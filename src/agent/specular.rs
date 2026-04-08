use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::PathBuf;
use std::process::Command;
use tokio::sync::mpsc;

#[derive(Debug, Clone)]
pub enum SpecularEvent {
    FileChanged(PathBuf),
    SyntaxError { path: PathBuf, details: String },
}

/// Spawns the OS file watcher asynchronously and transmits internal alerts into via the mpsc channel
pub fn spawn_watcher(
    tx: mpsc::Sender<SpecularEvent>,
) -> Result<RecommendedWatcher, Box<dyn std::error::Error>> {
    let (std_tx, std_rx) = std::sync::mpsc::channel();

    let mut watcher = notify::RecommendedWatcher::new(std_tx, Config::default())?;

    // Attach physically to the source directory to intercept code additions
    if std::path::Path::new("./src").exists() {
        watcher.watch(std::path::Path::new("./src"), RecursiveMode::Recursive)?;
    }

    // Spawn a transparent background OS thread to translate synchronous notify events
    // perfectly over into the async tokio run cycle bounds.
    std::thread::spawn(move || {
        // Initial heartbeat to populate the UI Radar
        let _ = tx.blocking_send(SpecularEvent::FileChanged(PathBuf::from("./src")));

        for res in std_rx {
            match res {
                Ok(Event { kind, paths, .. }) => {
                    if kind.is_modify() {
                        for path in paths {
                            if let Some(ext) = path.extension() {
                                if ext == "rs" {
                                    // 1) File modification identified locally
                                    let _ =
                                        tx.blocking_send(SpecularEvent::FileChanged(path.clone()));

                                    // 2) Trigger pseudo-cargo check
                                    let output = Command::new("cargo").arg("check").output();

                                    if let Ok(cmd_out) = output {
                                        if !cmd_out.status.success() {
                                            // 3) Proactive anomaly! Harvest compiler crash and send to AgentLoop
                                            let error_str =
                                                String::from_utf8_lossy(&cmd_out.stderr)
                                                    .to_string();
                                            let _ = tx.blocking_send(SpecularEvent::SyntaxError {
                                                path: path.clone(),
                                                details: error_str,
                                            });
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                Err(e) => println!("Specular internal tracking error: {:?}", e),
            }
        }
    });

    Ok(watcher)
}
