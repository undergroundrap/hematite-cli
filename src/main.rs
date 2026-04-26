// Hematite: Frontier Precision Active.
use clap::Parser;
use crossterm::{
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use hematite::runtime::{
    build_runtime_bundle, run_agent_loop, spawn_runtime_profile_sync, AgentLoopConfig,
    AgentLoopRuntime, RuntimeBundle,
};
use hematite::{ui, CliCockpit};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::sync::Arc;

fn wants_version_report(args: &[String]) -> bool {
    args.len() == 2 && matches!(args[1].as_str(), "--version" | "-V")
}

fn report_indicates_issues(content: &str) -> bool {
    hematite::agent::report_export::report_has_issues_in_content(content)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    hematite::tools::hardening::pre_main_hardening();
    let raw_args: Vec<String> = std::env::args().collect();
    if wants_version_report(&raw_args) {
        println!("{}", hematite::hematite_version_report());
        return Ok(());
    }

    // Guard against inaccessible cwd (e.g. launched via desktop shortcut with no "Start in" path).
    // Windows can set the process cwd to a system folder like AppData\Local\ElevatedDiagnostics.
    // Relocate to home dir so all relative path resolution works correctly.
    let cwd_ok = std::env::current_dir()
        .map(|p| std::fs::read_dir(&p).is_ok())
        .unwrap_or(false);
    if !cwd_ok {
        let home = std::env::var_os("USERPROFILE")
            .or_else(|| std::env::var_os("HOME"))
            .map(std::path::PathBuf::from);
        if let Some(home) = home {
            let _ = std::env::set_current_dir(home);
        }
    }

    let cockpit = CliCockpit::parse();

    if cockpit.mcp_server {
        let edge = cockpit.edge_redact || cockpit.semantic_redact;
        let semantic = cockpit.semantic_redact;
        let semantic_url = cockpit.semantic_url.as_deref().unwrap_or(&cockpit.url);
        let semantic_model = cockpit.semantic_model.as_deref().unwrap_or("");
        hematite::agent::mcp_server::run_mcp_server(
            edge,
            semantic,
            &cockpit.url,
            semantic_url,
            semantic_model,
        )
        .await?;
        return Ok(());
    }

    if cockpit.report {
        let fmt = cockpit.report_format.trim().to_ascii_lowercase();
        if cockpit.open {
            let path = match fmt.as_str() {
                "json" => hematite::agent::report_export::save_report_json().await.1,
                "html" => hematite::agent::report_export::save_report_html().await.1,
                _ => {
                    hematite::agent::report_export::save_report_markdown()
                        .await
                        .1
                }
            };
            println!("Report saved: {}", path.display());
            open_path(&path);
        } else {
            let out = match fmt.as_str() {
                "json" => hematite::agent::report_export::generate_report_json().await,
                "html" => hematite::agent::report_export::generate_report_html().await,
                _ => hematite::agent::report_export::generate_report_markdown().await,
            };
            print!("{}", out);
        }
        return Ok(());
    }

    if cockpit.diagnose {
        let fmt = cockpit.report_format.trim().to_ascii_lowercase();
        let (content, path) = match fmt.as_str() {
            "html" => hematite::agent::report_export::save_diagnosis_report_html().await,
            _ => hematite::agent::report_export::save_diagnosis_report().await,
        };
        println!("Diagnosis saved: {}", path.display());
        if cockpit.open {
            open_path(&path);
        }
        std::process::exit(if report_indicates_issues(&content) {
            1
        } else {
            0
        });
    }

    if let Some(ref preset) = cockpit.triage {
        let preset_str = preset.as_str();
        let fmt = cockpit.report_format.trim().to_ascii_lowercase();
        let (content, path) = match fmt.as_str() {
            "html" => hematite::agent::report_export::save_triage_report_html(preset_str).await,
            _ => hematite::agent::report_export::save_triage_report(preset_str).await,
        };
        println!("Triage saved: {}", path.display());
        if cockpit.open {
            open_path(&path);
        }
        std::process::exit(if report_indicates_issues(&content) {
            1
        } else {
            0
        });
    }

    if let Some(ref issue) = cockpit.fix {
        let issue_str = issue.trim();

        if issue_str.eq_ignore_ascii_case("list") || issue_str.eq_ignore_ascii_case("help") {
            println!("hematite --fix: supported issue categories\n");
            for (category, keywords) in hematite::agent::report_export::fix_issue_categories() {
                println!("  {:<22} {}", category, keywords);
            }
            println!("\nExamples:");
            println!("  hematite --fix \"PC running slow\"");
            println!("  hematite --fix \"can't connect to internet\" --report-format html --open");
            println!("  hematite --fix \"BSOD after update\"");
            println!("  hematite --fix \"Outlook not opening\"");
            return Ok(());
        }

        if cockpit.dry_run {
            let topics = hematite::agent::report_export::fix_plan_topics(issue_str);
            println!("hematite --fix \"{}\": would inspect:\n", issue_str);
            for (i, (topic, label)) in topics.iter().enumerate() {
                println!("  [{}/{}] {} ({})", i + 1, topics.len(), label, topic);
            }
            println!("\nUp to 3 follow-up checks may be added automatically based on findings.");
            return Ok(());
        }

        let fmt = cockpit.report_format.trim().to_ascii_lowercase();
        let (content, path) = match fmt.as_str() {
            "html" => hematite::agent::report_export::save_fix_plan_html(issue).await,
            _ => hematite::agent::report_export::save_fix_plan(issue).await,
        };
        println!("Fix plan saved: {}", path.display());
        if cockpit.open {
            open_path(&path);
        }

        if cockpit.execute {
            let auto_cmds = hematite::agent::report_export::fix_plan_auto_commands(&content);
            if auto_cmds.is_empty() {
                println!("\nNo safe auto-fixes available for these findings.");
            } else {
                println!("\nFound {} safe auto-fix(es):", auto_cmds.len());
                for (i, (label, cmd)) in auto_cmds.iter().enumerate() {
                    println!("  [{}] {} — {}", i + 1, label, cmd);
                }
                print!("\nRun these now? [Y/n]: ");
                use std::io::Write;
                let _ = std::io::stdout().flush();
                let mut answer = String::new();
                let _ = std::io::stdin().read_line(&mut answer);
                if !answer.trim().eq_ignore_ascii_case("n") {
                    println!();
                    for (label, cmd) in &auto_cmds {
                        print!("  Running: {}... ", label);
                        let _ = std::io::stdout().flush();
                        let result = std::process::Command::new("cmd").args(["/C", cmd]).status();
                        match result {
                            Ok(s) if s.success() => println!("done"),
                            Ok(s) => println!("exited {}", s.code().unwrap_or(-1)),
                            Err(e) => println!("error: {}", e),
                        }
                    }
                }
            }
        }

        std::process::exit(if report_indicates_issues(&content) {
            1
        } else {
            0
        });
    }

    if let Some(ref cadence) = cockpit.schedule {
        let cadence_str = cadence.trim();

        if cadence_str == "status" {
            println!("{}", hematite::agent::scheduler::query_scheduled_task());
            return Ok(());
        }

        if cadence_str == "remove" {
            match hematite::agent::scheduler::remove_scheduled_task() {
                Ok(msg) => println!("{}", msg),
                Err(e) => {
                    eprintln!("Error: {}", e);
                    std::process::exit(1);
                }
            }
            return Ok(());
        }

        let exe_path = std::env::current_exe()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| "hematite".to_string());

        match hematite::agent::scheduler::register_scheduled_task(cadence_str, &exe_path) {
            Ok(msg) => println!("{}", msg),
            Err(e) => {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
        return Ok(());
    }

    if let Some(path) = cockpit.pdf_extract_helper.as_deref() {
        let code = hematite::memory::vein::run_pdf_extract_helper(std::path::Path::new(path));
        std::process::exit(code);
    }
    let local_soul = ui::hatch::generate_soul(cockpit.reroll.clone());

    if cockpit.stats {
        println!(
            "Species: {} | Wisdom: {} | Chaos: {}",
            local_soul.species, local_soul.wisdom, local_soul.chaos
        );
        return Ok(());
    }

    let RuntimeBundle {
        services,
        channels,
        watcher_guard: _watcher_guard,
    } = build_runtime_bundle(
        &cockpit,
        &local_soul.species,
        local_soul.snark,
        !cockpit.rusty,
    )
    .await?;

    let hematite::runtime::RuntimeServices {
        engine,
        gpu_state,
        git_state,
        voice_manager,
        swarm_coordinator,
        cancel_token,
        searx_session,
    } = services;

    let hematite::runtime::RuntimeChannels {
        specular_rx,
        agent_tx,
        agent_rx,
        swarm_tx,
        swarm_rx,
        user_input_tx,
        user_input_rx,
    } = channels;

    // VRAM Prewarming: trigger an asynchronous ping to the inference engine to force
    // the local LLM into GPU memory before the user even submits their first prompt.
    let prewarm_engine = engine.clone();
    tokio::spawn(async move {
        let _ = prewarm_engine.prewarm().await;
    });

    let tui_cancel_token = cancel_token.clone();

    tokio::spawn(run_agent_loop(
        AgentLoopRuntime {
            user_input_rx,
            agent_tx: agent_tx.clone(),
            services: hematite::runtime::RuntimeServices {
                engine: engine.clone(),
                gpu_state: gpu_state.clone(),
                git_state: git_state.clone(),
                voice_manager: voice_manager.clone(),
                swarm_coordinator: swarm_coordinator.clone(),
                cancel_token,
                searx_session: searx_session.clone(),
            },
        },
        AgentLoopConfig {
            yolo: cockpit.yolo,
            professional: !cockpit.rusty,
            brief: cockpit.brief,
            snark: local_soul.snark,
            chaos: local_soul.chaos,
            soul_personality: local_soul.personality.clone(),
            fast_model: cockpit.fast_model.clone(),
            think_model: cockpit.think_model.clone(),
        },
    ));

    let _runtime_profile_poller = spawn_runtime_profile_sync(engine.clone(), agent_tx.clone());

    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    std::io::stdout().execute(EnterAlternateScreen)?;
    std::io::stdout().execute(crossterm::event::EnableMouseCapture)?;
    enable_raw_mode()?;
    let mut terminal = Terminal::new(CrosstermBackend::new(std::io::stdout()))?;

    let _app_result = ui::tui::run_app(
        &mut terminal,
        specular_rx,
        agent_rx,
        user_input_tx,
        swarm_rx,
        swarm_tx,
        swarm_coordinator,
        Arc::new(std::sync::Mutex::new(std::time::Instant::now())),
        cockpit.clone(),
        local_soul,
        !cockpit.rusty,
        gpu_state,
        git_state,
        tui_cancel_token,
        voice_manager,
    )
    .await;

    disable_raw_mode()?;
    std::io::stdout().execute(crossterm::event::DisableMouseCapture)?;
    std::io::stdout().execute(LeaveAlternateScreen)?;

    // Flush any keystrokes buffered during inference so they don't ghost
    // into the next terminal session after Hematite exits.
    #[cfg(target_os = "windows")]
    {
        #[link(name = "kernel32")]
        extern "system" {
            fn GetStdHandle(nStdHandle: u32) -> *mut std::ffi::c_void;
            fn FlushConsoleInputBuffer(hConsoleInput: *mut std::ffi::c_void) -> i32;
        }
        const STD_INPUT_HANDLE: u32 = 0xFFFFFFF6; // (-10i32) as u32
        unsafe {
            let h = GetStdHandle(STD_INPUT_HANDLE);
            if !h.is_null() && h as isize != -1 {
                FlushConsoleInputBuffer(h);
            }
        }
    }

    if let Some(summary) =
        hematite::agent::searx_lifecycle::shutdown_searx_if_owned(&searx_session).await
    {
        eprintln!("{}", summary);
    }
    Ok(())
}

fn open_path(path: &std::path::Path) {
    #[cfg(target_os = "windows")]
    {
        let s = path.to_string_lossy().into_owned();
        let _ = std::process::Command::new("cmd")
            .args(["/c", "start", "", &s])
            .spawn();
    }
    #[cfg(not(target_os = "windows"))]
    {
        let opener = if cfg!(target_os = "macos") {
            "open"
        } else {
            "xdg-open"
        };
        let _ = std::process::Command::new(opener).arg(path).spawn();
    }
}

#[cfg(test)]
mod tests {
    use super::wants_version_report;

    #[test]
    fn detects_plain_version_flag() {
        assert!(wants_version_report(&[
            "hematite".into(),
            "--version".into()
        ]));
        assert!(wants_version_report(&["hematite".into(), "-V".into()]));
        assert!(!wants_version_report(&["hematite".into()]));
        assert!(!wants_version_report(&[
            "hematite".into(),
            "--version".into(),
            "--brief".into()
        ]));
    }
}
