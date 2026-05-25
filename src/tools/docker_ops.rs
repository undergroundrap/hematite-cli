use serde_json::Value;
use std::time::Duration;

const TIMEOUT: u64 = 60;
const MAX_OUTPUT: usize = 10 * 1024;

pub async fn execute(args: &Value) -> Result<String, String> {
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .ok_or("docker_ops: 'action' is required (ps, ps-all, logs, start, stop, restart, rm, images, pull, inspect, build, exec, stats, compose-ps, compose-up, compose-down)")?;

    match action {
        "ps" => run_docker(&["ps", "--format", "table {{.ID}}\t{{.Names}}\t{{.Status}}\t{{.Ports}}"]).await,
        "ps-all" => run_docker(&["ps", "-a", "--format", "table {{.ID}}\t{{.Names}}\t{{.Status}}\t{{.Ports}}"]).await,
        "images" => run_docker(&["images", "--format", "table {{.Repository}}\t{{.Tag}}\t{{.ID}}\t{{.Size}}\t{{.CreatedSince}}"]).await,
        "stats" => run_docker_once_stats(args).await,
        "logs" => run_logs(args).await,
        "start" => run_container_action("start", args).await,
        "stop" => run_container_action("stop", args).await,
        "restart" => run_container_action("restart", args).await,
        "rm" => run_container_rm(args).await,
        "pull" => run_pull(args).await,
        "inspect" => run_inspect(args).await,
        "build" => run_build(args).await,
        "exec" => run_exec(args).await,
        "compose-ps" => run_compose(&["ps"], args).await,
        "compose-up" => run_compose_up(args).await,
        "compose-down" => run_compose(&["down"], args).await,
        other => Err(format!(
            "docker_ops: unknown action '{other}'. Valid: ps, ps-all, logs, start, stop, restart, rm, images, pull, inspect, build, exec, stats, compose-ps, compose-up, compose-down"
        )),
    }
}

async fn run_docker(cmd_args: &[&str]) -> Result<String, String> {
    let result = tokio::time::timeout(
        Duration::from_secs(TIMEOUT),
        tokio::process::Command::new("docker")
            .args(cmd_args)
            .output(),
    )
    .await;

    match result {
        Err(_) => Err(format!("docker_ops: timed out after {TIMEOUT}s")),
        Ok(Err(e)) => {
            if e.kind() == std::io::ErrorKind::NotFound {
                Err("docker_ops: Docker is not installed or not on PATH. Install from https://docs.docker.com/get-docker/".to_string())
            } else {
                Err(format!("docker_ops: failed to spawn docker: {e}"))
            }
        }
        Ok(Ok(output)) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            if !output.status.success() {
                let msg = if !stderr.is_empty() {
                    stderr.trim()
                } else {
                    stdout.trim()
                };
                return Err(format!("docker_ops: docker command failed: {msg}"));
            }
            let out = stdout.trim().to_string();
            Ok(if out.is_empty() {
                "(no output)".to_string()
            } else {
                cap(out)
            })
        }
    }
}

async fn run_logs(args: &Value) -> Result<String, String> {
    let container = require_container(args)?;
    let tail = args
        .get("tail")
        .and_then(|v| v.as_u64())
        .unwrap_or(100)
        .to_string();
    let timestamps = args
        .get("timestamps")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let mut cmd_args = vec!["logs", "--tail", &tail, container];
    if timestamps {
        cmd_args.push("--timestamps");
    }

    run_docker(&cmd_args)
        .await
        .map(|out| format!("docker_ops [LOGS] container={container}\n{out}"))
}

async fn run_container_action(action: &str, args: &Value) -> Result<String, String> {
    let container = require_container(args)?;
    run_docker(&[action, container])
        .await
        .map(|_| format!("docker_ops [{action}]: container '{container}' {action}ed successfully."))
}

async fn run_container_rm(args: &Value) -> Result<String, String> {
    let container = require_container(args)?;
    let force = args.get("force").and_then(|v| v.as_bool()).unwrap_or(false);
    let mut cmd_args = vec!["rm", container];
    if force {
        cmd_args.insert(1, "-f");
    }
    run_docker(&cmd_args)
        .await
        .map(|_| format!("docker_ops [rm]: container '{container}' removed."))
}

async fn run_pull(args: &Value) -> Result<String, String> {
    let image = args
        .get("image")
        .and_then(|v| v.as_str())
        .ok_or("docker_ops pull: 'image' is required")?;
    run_docker(&["pull", image])
        .await
        .map(|out| format!("docker_ops [pull] image={image}\n{out}"))
}

async fn run_inspect(args: &Value) -> Result<String, String> {
    let target = args
        .get("container")
        .or_else(|| args.get("image"))
        .and_then(|v| v.as_str())
        .ok_or("docker_ops inspect: 'container' or 'image' is required")?;
    let format = args
        .get("format")
        .and_then(|v| v.as_str())
        .unwrap_or("{{json .}}");
    let raw = run_docker(&["inspect", "--format", format, target]).await?;
    // Try pretty-printing JSON
    if let Ok(json) = serde_json::from_str::<Value>(&raw) {
        if let Ok(pretty) = serde_json::to_string_pretty(&json) {
            return Ok(format!("docker_ops [inspect] {target}\n{pretty}"));
        }
    }
    Ok(format!("docker_ops [inspect] {target}\n{raw}"))
}

async fn run_build(args: &Value) -> Result<String, String> {
    let context = args.get("context").and_then(|v| v.as_str()).unwrap_or(".");
    let tag = args.get("tag").and_then(|v| v.as_str());
    let dockerfile = args.get("dockerfile").and_then(|v| v.as_str());
    let no_cache = args
        .get("no_cache")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let mut cmd_args = vec!["build", context];
    let tag_owned;
    if let Some(t) = tag {
        tag_owned = format!("-t{t}");
        cmd_args.push(&tag_owned);
    }
    let df_owned;
    if let Some(df) = dockerfile {
        df_owned = format!("-f{df}");
        cmd_args.push(&df_owned);
    }
    if no_cache {
        cmd_args.push("--no-cache");
    }

    run_docker(&cmd_args)
        .await
        .map(|out| format!("docker_ops [build] context={context}\n{out}"))
}

async fn run_exec(args: &Value) -> Result<String, String> {
    let container = require_container(args)?;
    let command = args
        .get("command")
        .and_then(|v| v.as_str())
        .ok_or("docker_ops exec: 'command' is required")?;

    let shell_args: Vec<&str> = command.split_whitespace().collect();
    let mut cmd_args = vec!["exec", container];
    cmd_args.extend(shell_args.iter().copied());

    run_docker(&cmd_args)
        .await
        .map(|out| format!("docker_ops [exec] {container}: {command}\n{out}"))
}

async fn run_docker_once_stats(args: &Value) -> Result<String, String> {
    let mut cmd_args = vec![
        "stats",
        "--no-stream",
        "--format",
        "table {{.Name}}\t{{.CPUPerc}}\t{{.MemUsage}}\t{{.MemPerc}}\t{{.NetIO}}\t{{.BlockIO}}",
    ];
    let container_owned;
    if let Some(c) = args.get("container").and_then(|v| v.as_str()) {
        container_owned = c.to_string();
        cmd_args.push(&container_owned);
    }
    run_docker(&cmd_args)
        .await
        .map(|out| format!("docker_ops [stats]\n{out}"))
}

async fn run_compose(compose_args: &[&str], args: &Value) -> Result<String, String> {
    let file = args.get("file").and_then(|v| v.as_str());
    let mut cmd_args = vec!["compose"];
    let file_owned;
    if let Some(f) = file {
        cmd_args.push("-f");
        file_owned = f.to_string();
        cmd_args.push(&file_owned);
    }
    cmd_args.extend(compose_args.iter().copied());
    run_docker(&cmd_args).await
}

async fn run_compose_up(args: &Value) -> Result<String, String> {
    let file = args.get("file").and_then(|v| v.as_str());
    let detach = args.get("detach").and_then(|v| v.as_bool()).unwrap_or(true);
    let build = args.get("build").and_then(|v| v.as_bool()).unwrap_or(false);

    let mut cmd_args = vec!["compose"];
    let file_owned;
    if let Some(f) = file {
        cmd_args.push("-f");
        file_owned = f.to_string();
        cmd_args.push(&file_owned);
    }
    cmd_args.push("up");
    if detach {
        cmd_args.push("-d");
    }
    if build {
        cmd_args.push("--build");
    }
    run_docker(&cmd_args).await.map(|out| {
        format!(
            "docker_ops [compose-up]{}\n{out}",
            if detach { " (detached)" } else { "" }
        )
    })
}

fn require_container<'a>(args: &'a Value) -> Result<&'a str, String> {
    args.get("container")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "docker_ops: 'container' is required for this action".to_string())
}

fn cap(s: String) -> String {
    if s.len() > MAX_OUTPUT {
        format!(
            "{}...\n[truncated — {} bytes total]",
            &s[..MAX_OUTPUT],
            s.len()
        )
    } else {
        s
    }
}
