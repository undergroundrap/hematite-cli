use crate::agent::config::HematiteConfig;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use tokio::time::{timeout, Duration};

const SEARX_ROOT_ENV: &str = "HEMATITE_SEARX_ROOT";
const DEFAULT_SEARX_URL: &str = "http://localhost:8080";

#[derive(Clone, Debug, Default)]
pub struct SearxRuntimeSession {
    pub root: PathBuf,
    pub owned_by_session: bool,
    pub auto_stop_on_exit: bool,
    pub startup_summary: Option<String>,
    /// Docker Desktop was launched this session; background poller should
    /// watch for daemon readiness and then start SearXNG.
    pub docker_wake_pending: bool,
}

pub(crate) enum DockerState {
    Ready,
    MissingCli,
    DaemonUnavailable(String),
}

pub fn resolve_searx_root() -> PathBuf {
    if let Some(explicit) = std::env::var_os(SEARX_ROOT_ENV) {
        let candidate = PathBuf::from(explicit);
        if !candidate.as_os_str().is_empty() {
            return candidate;
        }
    }

    let home = std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));

    home.join(".hematite").join("searxng-local")
}

fn find_setup_script() -> Option<PathBuf> {
    let mut candidates = Vec::new();

    if let Ok(cwd) = std::env::current_dir() {
        candidates.push(cwd.join("scripts").join("setup-searxng.ps1"));
        candidates.push(cwd.join("setup-searxng.ps1"));
    }

    if let Ok(exe) = std::env::current_exe() {
        if let Some(exe_dir) = exe.parent() {
            candidates.push(exe_dir.join("setup-searxng.ps1"));
            candidates.push(exe_dir.join("scripts").join("setup-searxng.ps1"));
        }
    }

    candidates.into_iter().find(|path| Path::new(path).exists())
}

fn looks_like_local_searx_url(url: &str) -> bool {
    let lower = url.to_ascii_lowercase();
    lower.contains("localhost")
        || lower.contains("127.0.0.1")
        || lower.contains("[::1]")
        || !lower.contains("://")
}

/// Try to find Docker Desktop.exe on Windows. Checks the two most common
/// install locations (system-wide and per-user LOCALAPPDATA).
#[cfg(target_os = "windows")]
fn find_docker_desktop_exe() -> Option<PathBuf> {
    let mut candidates = vec![
        PathBuf::from(r"C:\Program Files\Docker\Docker\Docker Desktop.exe"),
        PathBuf::from(r"C:\Program Files (x86)\Docker\Docker\Docker Desktop.exe"),
    ];
    if let Some(local) = std::env::var_os("LOCALAPPDATA").map(PathBuf::from) {
        candidates.push(
            local
                .join("Programs")
                .join("Docker")
                .join("Docker")
                .join("Docker Desktop.exe"),
        );
    }
    candidates.into_iter().find(|p| p.exists())
}

pub(crate) fn docker_state() -> DockerState {
    match Command::new("docker")
        .args(["info", "--format", "{{.ServerVersion}}"])
        .output()
    {
        Ok(output) if output.status.success() => DockerState::Ready,
        Ok(output) => {
            let detail = String::from_utf8_lossy(&output.stderr).trim().to_string();
            DockerState::DaemonUnavailable(if detail.is_empty() {
                "Docker is installed but the daemon is not responding.".to_string()
            } else {
                detail
            })
        }
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => DockerState::MissingCli,
        Err(err) => DockerState::DaemonUnavailable(err.to_string()),
    }
}

fn ensure_scaffolded(root: &Path) -> Result<(), String> {
    let compose_path = root.join("docker-compose.yaml");
    let start_script = root.join("start_searx.bat");
    if compose_path.exists() && start_script.exists() {
        return Ok(());
    }

    let Some(script_path) = find_setup_script() else {
        return Err(
            "Local search bootstrap is unavailable: setup-searxng.ps1 could not be found."
                .to_string(),
        );
    };

    let output = Command::new("powershell")
        .arg("-ExecutionPolicy")
        .arg("Bypass")
        .arg("-File")
        .arg(script_path)
        .arg("-TargetRoot")
        .arg(root)
        .output()
        .map_err(|e| format!("Failed to scaffold local search: {}", e))?;

    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let detail = if !stderr.is_empty() { stderr } else { stdout };
        Err(format!("Failed to scaffold local search: {}", detail))
    }
}

pub(crate) fn docker_compose_up(root: &Path) -> Result<(), String> {
    let output = Command::new("docker")
        .args(["compose", "up", "-d"])
        .current_dir(root)
        .output()
        .map_err(|e| format!("Failed to start local search: {}", e))?;

    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let detail = if !stderr.is_empty() { stderr } else { stdout };
        Err(format!("Local search start failed: {}", detail))
    }
}

fn docker_compose_down(root: &Path) -> Result<(), String> {
    let output = Command::new("docker")
        .args(["compose", "down"])
        .current_dir(root)
        .output()
        .map_err(|e| format!("Failed to stop local search: {}", e))?;

    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let detail = if !stderr.is_empty() { stderr } else { stdout };
        Err(format!("Local search stop failed: {}", detail))
    }
}

pub(crate) async fn wait_for_searx(url: &str) -> bool {
    for _ in 0..20 {
        if is_searx_responding(url).await {
            return true;
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
    false
}

/// Checks if SearXNG is responding at the configured URL.
pub async fn is_searx_responding(url: &str) -> bool {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_millis(500))
        .build()
        .unwrap_or_default();

    match timeout(Duration::from_millis(600), client.get(url).send()).await {
        Ok(Ok(resp)) => resp.status().is_success() || resp.status().as_u16() == 403, // 403 is fine, SearXNG might block generic UA but it's alive
        _ => false,
    }
}

/// Automatically boots SearXNG if it's offline and the user has auto-start enabled.
pub async fn boot_searx_if_needed(config: &HematiteConfig) -> SearxRuntimeSession {
    let url = config.searx_url.as_deref().unwrap_or(DEFAULT_SEARX_URL);
    let root = resolve_searx_root();
    let mut session = SearxRuntimeSession {
        root: root.clone(),
        owned_by_session: false,
        auto_stop_on_exit: config.auto_stop_searx,
        startup_summary: None,
        docker_wake_pending: false,
    };

    if !config.auto_start_searx {
        return session;
    }

    if !looks_like_local_searx_url(url) {
        return session;
    }

    // Check if it's already alive.
    if is_searx_responding(url).await {
        return session;
    }

    if let Err(err) = ensure_scaffolded(&root) {
        session.startup_summary = Some(err);
        return session;
    }

    match docker_state() {
        DockerState::MissingCli => {
            session.startup_summary = Some(
                "Local search is unavailable: Docker Desktop is not installed. Install it from https://www.docker.com/products/docker-desktop or set `auto_start_searx` to false in `.hematite/settings.json`.".to_string(),
            );
            return session;
        }
        DockerState::DaemonUnavailable(_detail) => {
            #[cfg(target_os = "windows")]
            if let Some(exe) = find_docker_desktop_exe() {
                let launched = std::process::Command::new(&exe).spawn().is_ok();
                if launched {
                    session.docker_wake_pending = true;
                    session.startup_summary = Some(
                        "Local search: Docker Desktop wasn't running — launching it now. \
                        SearXNG will auto-start once Docker is ready (~30–60s). \
                        Falling back to Jina until then."
                            .to_string(),
                    );
                    return session;
                }
            }
            session.startup_summary = Some(format!(
                "Local search is unavailable: Docker is installed but not running. \
                Start Docker Desktop, then relaunch Hematite or run `docker compose up -d` in `{}`.",
                root.display()
            ));
            return session;
        }
        DockerState::Ready => {}
    }

    if let Err(err) = docker_compose_up(&root) {
        session.startup_summary = Some(err);
        return session;
    }

    if wait_for_searx(url).await {
        session.owned_by_session = true;
        session.startup_summary = Some(format!(
            "Local search auto-started: SearXNG is now live at {} (root: {}). Hematite started this stack in the current session{}.",
            url,
            root.display(),
            if config.auto_stop_searx {
                " and will stop it on exit"
            } else {
                ""
            }
        ));
    } else {
        session.startup_summary = Some(format!(
            "Local search was started from `{}`, but {} never became reachable. Check `docker compose logs` in that folder.",
            root.display(),
            url
        ));
    }

    session
}

pub async fn shutdown_searx_if_owned(session: &SearxRuntimeSession) -> Option<String> {
    if !session.owned_by_session || !session.auto_stop_on_exit {
        return None;
    }

    match docker_compose_down(&session.root) {
        Ok(()) => Some(format!(
            "Stopped session-owned local search stack at {}.",
            session.root.display()
        )),
        Err(err) => Some(err),
    }
}
