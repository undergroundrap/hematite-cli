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

fn snapshot_path(name: &str) -> std::path::PathBuf {
    let safe: String = name
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    hematite::tools::file_ops::hematite_dir()
        .join("snapshots")
        .join(format!("{}.txt", safe))
}

fn wants_version_report(args: &[String]) -> bool {
    args.len() == 2 && matches!(args[1].as_str(), "--version" | "-V")
}

fn report_indicates_issues(content: &str) -> bool {
    hematite::agent::report_export::report_has_issues_in_content(content)
}

fn print_health_banner(content: &str) {
    let score = hematite::agent::report_export::score_health_from_content(content);
    let bar = match score.grade {
        'A' => "██████████ A",
        'B' => "████████░░ B",
        'C' => "██████░░░░ C",
        'D' => "████░░░░░░ D",
        _ => "██░░░░░░░░ F",
    };
    println!();
    println!("  Health Score  {}  — {}", bar, score.label);
    println!("  {}", score.summary_line());
}

fn print_fix_suggestions(content: &str) {
    let suggestions = hematite::agent::report_export::suggest_fix_commands(content);
    if !suggestions.is_empty() {
        println!();
        println!("  Next steps — run a targeted fix plan:");
        for s in &suggestions {
            println!("    {}", s.trim());
        }
        println!();
    }
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
        if cockpit.open || fmt == "html" {
            let (out, path) = match fmt.as_str() {
                "json" => hematite::agent::report_export::save_report_json().await,
                "html" => hematite::agent::report_export::save_report_html().await,
                _ => hematite::agent::report_export::save_report_markdown().await,
            };
            println!("Report saved: {}", path.display());
            if cockpit.open {
                open_path(&path);
            }
            if cockpit.clipboard {
                copy_to_clipboard(&out);
                println!("Copied to clipboard.");
            }
            if cockpit.notify {
                show_toast("Hematite Report", "Diagnostic report complete.");
            }
        } else {
            let out = match fmt.as_str() {
                "json" => hematite::agent::report_export::generate_report_json().await,
                _ => hematite::agent::report_export::generate_report_markdown().await,
            };
            if let Some(ref out_path) = cockpit.output {
                write_output_copy(&out, out_path);
            } else {
                print!("{}", out);
            }
            if cockpit.clipboard {
                copy_to_clipboard(&out);
                println!("Copied to clipboard.");
            }
            if cockpit.notify {
                show_toast("Hematite Report", "Diagnostic report complete.");
            }
        }
        return Ok(());
    }

    if cockpit.diagnose {
        if cockpit.dry_run {
            let topics = hematite::agent::report_export::report_topics();
            println!(
                "hematite --diagnose --dry-run: phase 1 inspects {} topic(s)\n",
                topics.len()
            );
            for (i, (topic, label)) in topics.iter().enumerate() {
                println!("  [{}/{}] {} ({})", i + 1, topics.len(), label, topic);
            }
            println!(
                "\nPhase 2 topics are determined dynamically from phase 1 results (disk/RAM/security flags)."
            );
            println!("Remove --dry-run to run the full staged diagnosis.");
            return Ok(());
        }

        let fmt = cockpit.report_format.trim().to_ascii_lowercase();
        let (content, path) = match fmt.as_str() {
            "html" => hematite::agent::report_export::save_diagnosis_report_html().await,
            "json" => hematite::agent::report_export::save_diagnosis_report_json().await,
            _ => hematite::agent::report_export::save_diagnosis_report().await,
        };
        let has_issues = report_indicates_issues(&content);
        if !cockpit.quiet || has_issues {
            println!("Diagnosis saved: {}", path.display());
            print_health_banner(&content);
            print_fix_suggestions(&content);
        }
        if let Some(ref out_path) = cockpit.output {
            write_output_copy(&content, out_path);
        }
        if cockpit.clipboard {
            copy_to_clipboard(&content);
            println!("Copied to clipboard.");
        }
        if cockpit.notify {
            let score = hematite::agent::report_export::score_health_from_content(&content);
            let body = format!("Grade {} — {}", score.grade, score.summary_line());
            show_toast("Hematite Diagnosis", &body);
        }
        if cockpit.open {
            open_path(&path);
        }
        std::process::exit(if has_issues { 1 } else { 0 });
    }

    if let Some(ref preset) = cockpit.triage {
        let preset_str = preset.as_str();

        if cockpit.dry_run {
            let topics = hematite::agent::report_export::triage_topics_for_preset(preset_str);
            println!(
                "hematite --triage{} --dry-run: {} topic(s) would be inspected\n",
                if preset_str == "default" {
                    String::new()
                } else {
                    format!(" {}", preset_str)
                },
                topics.len()
            );
            for (i, (topic, label)) in topics.iter().enumerate() {
                println!("  [{}/{}] {} ({})", i + 1, topics.len(), label, topic);
            }
            println!("\nRemove --dry-run to run the triage.");
            return Ok(());
        }

        let fmt = cockpit.report_format.trim().to_ascii_lowercase();
        let (content, path) = match fmt.as_str() {
            "html" => hematite::agent::report_export::save_triage_report_html(preset_str).await,
            "json" => hematite::agent::report_export::save_triage_report_json(preset_str).await,
            _ => hematite::agent::report_export::save_triage_report(preset_str).await,
        };
        let has_issues = report_indicates_issues(&content);
        if !cockpit.quiet || has_issues {
            println!("Triage saved: {}", path.display());
            print_health_banner(&content);
            print_fix_suggestions(&content);
        }
        if let Some(ref out_path) = cockpit.output {
            write_output_copy(&content, out_path);
        }
        if cockpit.clipboard {
            copy_to_clipboard(&content);
            println!("Copied to clipboard.");
        }
        if cockpit.notify {
            let score = hematite::agent::report_export::score_health_from_content(&content);
            let body = format!("Grade {} — {}", score.grade, score.summary_line());
            show_toast("Hematite Triage", &body);
        }
        if cockpit.open {
            open_path(&path);
        }
        std::process::exit(if has_issues { 1 } else { 0 });
    }

    if let Some(ref issue) = cockpit.fix {
        let issue_str = issue.trim();

        if issue_str.eq_ignore_ascii_case("list") || issue_str.eq_ignore_ascii_case("help") {
            println!(
                "hematite --fix: {} supported issue categories (no model required)\n",
                hematite::agent::report_export::fix_issue_categories().len()
            );
            for (category, keywords) in hematite::agent::report_export::fix_issue_categories() {
                // Use the first keyword phrase as the example argument
                let example = keywords.split(',').next().unwrap_or(keywords).trim();
                println!("  {:<26}  hematite --fix \"{}\"", category, example);
            }
            println!("\nAdd --report-format html --open for a browser report.");
            println!("Add --dry-run to preview which checks would run.");
            println!("Add --execute to run safe auto-fixes after the plan.");
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
        let (content, path) = if fmt == "html" {
            hematite::agent::report_export::save_fix_plan_html(issue).await
        } else if fmt == "json" {
            hematite::agent::report_export::save_fix_plan_json(issue).await
        } else {
            let (summary, md, path) =
                hematite::agent::report_export::save_fix_plan_with_summary(issue).await;
            let has_issues = report_indicates_issues(&md);
            if !cockpit.quiet || has_issues {
                println!("\n{}", summary.trim_end());
            }
            (md, path)
        };
        let has_issues_final = report_indicates_issues(&content);
        if !cockpit.quiet || has_issues_final {
            println!("\nFix plan saved: {}", path.display());
            if fmt != "json" {
                print_health_banner(&content);
                print_fix_suggestions(&content);
            }
        }
        if cockpit.clipboard {
            copy_to_clipboard(&content);
            println!("Copied to clipboard.");
        }
        if cockpit.open {
            open_path(&path);
        }

        if cockpit.execute {
            let auto_cmds = hematite::agent::report_export::fix_plan_auto_commands(&content);
            if fmt == "json" {
                // JSON mode: auto-apply without prompt, emit structured results
                let mut results: Vec<serde_json::Value> = Vec::new();
                for fix in &auto_cmds {
                    let cmd_status = std::process::Command::new("cmd")
                        .args(["/C", fix.cmd])
                        .stdout(std::process::Stdio::null())
                        .stderr(std::process::Stdio::null())
                        .status();
                    let ok = matches!(cmd_status, Ok(s) if s.success());
                    let verified = if ok {
                        if let (Some(topic), Some(gone)) = (fix.verify_topic, fix.verify_gone) {
                            let verify_out =
                                hematite::agent::report_export::generate_inspect_output(topic)
                                    .await;
                            Some(!verify_out.to_ascii_lowercase().contains(gone))
                        } else {
                            None
                        }
                    } else {
                        None
                    };
                    results.push(serde_json::json!({
                        "label": fix.label,
                        "status": if ok { "ok" } else { "failed" },
                        "verified_resolved": verified,
                    }));
                }
                let json_out = serde_json::json!({
                    "issue": issue_str,
                    "fixes_applied": results,
                });
                println!(
                    "{}",
                    serde_json::to_string_pretty(&json_out).unwrap_or_default()
                );
            } else if auto_cmds.is_empty() {
                println!("\nNo safe auto-fixes available for these findings.");
            } else {
                println!("\nFound {} safe auto-fix(es):", auto_cmds.len());
                for (i, fix) in auto_cmds.iter().enumerate() {
                    println!("  [{}] {}", i + 1, fix.label);
                }
                use std::io::Write;
                let approved = if cockpit.yes {
                    println!("\nApplying fixes automatically (--yes)...");
                    true
                } else {
                    print!("\nRun these now? [Y/n]: ");
                    let _ = std::io::stdout().flush();
                    let mut answer = String::new();
                    let _ = std::io::stdin().read_line(&mut answer);
                    !answer.trim().eq_ignore_ascii_case("n")
                };
                if approved {
                    println!();
                    for fix in &auto_cmds {
                        print!("  Running: {}... ", fix.label);
                        let _ = std::io::stdout().flush();
                        let status = std::process::Command::new("cmd")
                            .args(["/C", fix.cmd])
                            .stdout(std::process::Stdio::null())
                            .stderr(std::process::Stdio::null())
                            .status();
                        match status {
                            Ok(s) if s.success() => {
                                println!("OK");
                                if let (Some(topic), Some(gone)) =
                                    (fix.verify_topic, fix.verify_gone)
                                {
                                    print!("    Verifying {}... ", topic);
                                    let _ = std::io::stdout().flush();
                                    let verify_out =
                                        hematite::agent::report_export::generate_inspect_output(
                                            topic,
                                        )
                                        .await;
                                    if verify_out.to_ascii_lowercase().contains(gone) {
                                        println!("\x1B[33m✗ Still present\x1B[0m — run: hematite --fix \"{}\"", issue_str);
                                    } else {
                                        println!("\x1B[32m✓ Verified resolved\x1B[0m");
                                    }
                                }
                            }
                            Ok(s) => println!("Failed (code {})", s.code().unwrap_or(1)),
                            Err(e) => println!("Error: {}", e),
                        }
                    }
                    println!("\nAuto-fix run complete.");
                }
            }
        }

        if let Some(ref out_path) = cockpit.output {
            write_output_copy(&content, out_path);
        }
        if cockpit.notify {
            let grade = if has_issues_final {
                "Issues found"
            } else {
                "All clear"
            };
            show_toast("Hematite Fix Plan", &format!("{} — {}", grade, issue_str));
        }
        std::process::exit(if has_issues_final { 1 } else { 0 });
    }

    if cockpit.fix_all {
        // --fix-all --schedule [cadence]: register a Windows scheduled task for the sweep
        if let Some(ref cadence) = cockpit.schedule {
            let cadence_str = cadence.trim();
            if cadence_str == "status" {
                println!("{}", hematite::agent::scheduler::query_sweep_task());
                return Ok(());
            }
            if cadence_str == "remove" {
                match hematite::agent::scheduler::remove_sweep_task() {
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
            match hematite::agent::scheduler::register_sweep_task(cadence_str, &exe_path) {
                Ok(msg) => println!("{}", msg),
                Err(e) => {
                    eprintln!("Error: {}", e);
                    std::process::exit(1);
                }
            }
            return Ok(());
        }

        // --fix-all --list: print available fix labels and exit
        if cockpit.only.as_deref() == Some("list") || cockpit.only.as_deref() == Some("help") {
            let all = hematite::agent::report_export::sweep_auto_fixes();
            let fmt = cockpit.report_format.trim().to_ascii_lowercase();
            if fmt == "json" {
                let arr: Vec<serde_json::Value> = all
                    .iter()
                    .map(|f| {
                        serde_json::json!({
                            "label": f.label,
                            "verify_topic": f.verify_topic,
                            "verify_gone": f.verify_gone,
                        })
                    })
                    .collect();
                let out = serde_json::to_string_pretty(&serde_json::Value::Array(arr))
                    .unwrap_or_else(|_| "[]".to_string());
                if let Some(ref out_path) = cockpit.output {
                    write_output_copy(&out, out_path);
                } else {
                    println!("{}", out);
                }
            } else {
                println!("Available sweep fixes ({}):\n", all.len());
                for fix in &all {
                    println!("  \"{}\"", fix.label);
                }
                println!("\nRun one: hematite --fix-all --only \"<label>\"");
            }
            return Ok(());
        }

        // --fix-all --dry-run: preview what would run without executing
        if cockpit.dry_run {
            let all = hematite::agent::report_export::sweep_auto_fixes();
            let preview: Vec<_> = if let Some(ref only_label) = cockpit.only {
                let lower = only_label.to_ascii_lowercase();
                all.iter()
                    .filter(|f| f.label.to_ascii_lowercase().contains(&lower))
                    .copied()
                    .collect()
            } else {
                all
            };
            println!(
                "hematite --fix-all --dry-run: {} fix(es) would run\n",
                preview.len()
            );
            for (i, fix) in preview.iter().enumerate() {
                println!("  [{}] {}", i + 1, fix.label);
                if let Some(topic) = fix.verify_topic {
                    println!("       verify-topic: {}", topic);
                }
                println!("       cmd: {}", fix.cmd);
            }
            println!("\nRemove --dry-run to execute the sweep.");
            return Ok(());
        }

        let all_sweep = hematite::agent::report_export::sweep_auto_fixes();

        // --fix-all --only <label>: filter to the named fix
        let sweep: Vec<&hematite::agent::report_export::AutoFix> = if let Some(ref only_label) =
            cockpit.only
        {
            let label_lower = only_label.to_ascii_lowercase();
            let matches: Vec<_> = all_sweep
                .iter()
                .filter(|f| f.label.to_ascii_lowercase().contains(&label_lower))
                .copied()
                .collect();
            if matches.is_empty() {
                eprintln!(
                        "No sweep fix found matching {:?}.\nRun `hematite --fix-all --only list` to see all labels.",
                        only_label
                    );
                std::process::exit(1);
            }
            matches
        } else {
            all_sweep
        };

        let ts = hematite::agent::report_export::timestamp_label();
        let quiet_sweep = cockpit.quiet;
        // In quiet mode, buffer progress lines; flush only if there are unresolved issues.
        let mut progress_buf: Vec<String> = Vec::new();
        let emit = |buf: &mut Vec<String>, line: String| {
            if quiet_sweep {
                buf.push(line);
            } else {
                println!("{}", line);
            }
        };
        emit(
            &mut progress_buf,
            format!("Hematite maintenance sweep — {} checks\n", sweep.len()),
        );

        struct SweepEntry {
            label: &'static str,
            status: &'static str, // "healthy", "fixed", "unresolved", "failed", "done"
        }
        let mut log: Vec<SweepEntry> = Vec::new();
        let mut applied = 0usize;
        let mut verified = 0usize;

        for fix in &sweep {
            let display = fix
                .label
                .trim_start_matches("Restart ")
                .trim_start_matches("Flush ")
                .trim_start_matches("Clear ")
                .trim_start_matches("Resync ")
                .trim_start_matches("Empty ")
                .trim_start_matches("Start ");
            let needs_fix = if let (Some(topic), Some(gone)) = (fix.verify_topic, fix.verify_gone) {
                let pre = hematite::agent::report_export::generate_inspect_output(topic).await;
                pre.to_ascii_lowercase().contains(gone)
            } else {
                true
            };
            if !needs_fix {
                emit(&mut progress_buf, format!("  Checking {}... OK", display));
                log.push(SweepEntry {
                    label: fix.label,
                    status: "healthy",
                });
                continue;
            }
            let status = std::process::Command::new("cmd")
                .args(["/C", fix.cmd])
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status();
            applied += 1;
            match status {
                Ok(s) if s.success() => {
                    if let (Some(topic), Some(gone)) = (fix.verify_topic, fix.verify_gone) {
                        let post =
                            hematite::agent::report_export::generate_inspect_output(topic).await;
                        if post.to_ascii_lowercase().contains(gone) {
                            emit(
                                &mut progress_buf,
                                format!("  Checking {}... needs fix — ✗ still present", display),
                            );
                            log.push(SweepEntry {
                                label: fix.label,
                                status: "unresolved",
                            });
                        } else {
                            emit(
                                &mut progress_buf,
                                format!("  Checking {}... needs fix — ✓ resolved", display),
                            );
                            verified += 1;
                            log.push(SweepEntry {
                                label: fix.label,
                                status: "fixed",
                            });
                        }
                    } else {
                        emit(
                            &mut progress_buf,
                            format!("  Checking {}... needs fix — done", display),
                        );
                        verified += 1;
                        log.push(SweepEntry {
                            label: fix.label,
                            status: "done",
                        });
                    }
                }
                Ok(s) => {
                    emit(
                        &mut progress_buf,
                        format!(
                            "  Checking {}... needs fix — failed (code {})",
                            display,
                            s.code().unwrap_or(1)
                        ),
                    );
                    log.push(SweepEntry {
                        label: fix.label,
                        status: "failed",
                    });
                }
                Err(e) => {
                    emit(
                        &mut progress_buf,
                        format!("  Checking {}... needs fix — error: {}", display, e),
                    );
                    log.push(SweepEntry {
                        label: fix.label,
                        status: "failed",
                    });
                }
            }
        }
        let summary = if applied == 0 {
            "All checks passed — nothing needed fixing.".to_string()
        } else {
            format!(
                "{} fix(es) applied, {} verified resolved.",
                applied, verified
            )
        };
        let has_unresolved = applied > verified;
        emit(&mut progress_buf, String::new());
        emit(&mut progress_buf, format!("  {}", summary));

        // Flush buffered progress if quiet mode suppressed it but there are issues
        if quiet_sweep && has_unresolved {
            for line in &progress_buf {
                println!("{}", line);
            }
        }

        // Build and save the sweep report
        let hostname = std::env::var("COMPUTERNAME")
            .or_else(|_| std::env::var("HOSTNAME"))
            .unwrap_or_else(|_| "unknown".to_string());
        let mut md = format!(
            "# Hematite Maintenance Sweep\n\nDate: {}  \nMachine: {}\n\n## Results\n\n| Check | Result |\n|---|---|\n",
            ts, hostname
        );
        for e in &log {
            let icon = match e.status {
                "healthy" => "OK — skipped",
                "fixed" => "Fixed — verified resolved",
                "unresolved" => "Fixed — still present",
                "done" => "Fixed — applied",
                _ => "Failed",
            };
            md.push_str(&format!("| {} | {} |\n", e.label, icon));
        }
        md.push_str(&format!("\n## Summary\n\n{}\n", summary));

        let fmt = cockpit.report_format.trim().to_ascii_lowercase();
        let report_dir = hematite::tools::file_ops::hematite_dir().join("reports");
        let _ = std::fs::create_dir_all(&report_dir);
        let safe_ts: String = ts
            .chars()
            .map(|c| {
                if c.is_alphanumeric() || c == '-' {
                    c
                } else {
                    '_'
                }
            })
            .collect();

        let report_content: String;
        let report_path: std::path::PathBuf;

        if fmt == "html" {
            let html = hematite::agent::html_template::build_html_shell(
                "Hematite Maintenance Sweep",
                &hematite::hematite_version(),
                &hematite::agent::html_template::markdown_to_html(&md),
            );
            report_path = report_dir.join(format!("sweep-{}.html", safe_ts));
            let _ = std::fs::write(&report_path, &html);
            report_content = md.clone();
        } else if fmt == "json" {
            let checks: Vec<serde_json::Value> = log
                .iter()
                .map(|e| {
                    serde_json::json!({
                        "label": e.label,
                        "status": e.status,
                    })
                })
                .collect();
            let json_obj = serde_json::json!({
                "generated": ts,
                "host": hostname,
                "hematite_version": hematite::hematite_version(),
                "checks_run": log.len(),
                "applied": applied,
                "verified": verified,
                "unresolved": applied.saturating_sub(verified),
                "summary": summary,
                "checks": checks,
            });
            let json_str =
                serde_json::to_string_pretty(&json_obj).unwrap_or_else(|_| "{}".to_string());
            report_path = report_dir.join(format!("sweep-{}.json", safe_ts));
            let _ = std::fs::write(&report_path, &json_str);
            report_content = json_str;
        } else {
            report_path = report_dir.join(format!("sweep-{}.md", safe_ts));
            let _ = std::fs::write(&report_path, &md);
            report_content = md.clone();
        }
        if !quiet_sweep || has_unresolved {
            println!("Sweep report saved: {}", report_path.display());
        }

        if let Some(ref out_path) = cockpit.output {
            write_output_copy(&report_content, out_path);
        }
        if cockpit.clipboard {
            copy_to_clipboard(&report_content);
            println!("Copied to clipboard.");
        }
        if cockpit.notify {
            let toast_body = if applied == 0 {
                "All checks passed — nothing needed fixing.".to_string()
            } else if verified == applied {
                format!("{} fix(es) applied — all verified resolved.", applied)
            } else {
                format!(
                    "{} fix(es) applied, {} unresolved — action needed.",
                    applied,
                    applied - verified
                )
            };
            show_toast("Hematite Sweep", &toast_body);
        }
        if cockpit.open {
            open_path(&report_path);
        }
        std::process::exit(if applied > 0 && verified < applied {
            1
        } else {
            0
        });
    }

    if cockpit.inventory {
        let fmt = cockpit.report_format.trim().to_ascii_lowercase();
        let out = if fmt == "json" {
            hematite::agent::direct_answers::build_inspect_inventory_json()
        } else {
            hematite::agent::direct_answers::build_inspect_inventory()
        };
        if let Some(ref out_path) = cockpit.output {
            write_output_copy(&out, out_path);
        } else {
            println!("{}", out);
        }
        return Ok(());
    }

    if cockpit.snapshots {
        let dir = hematite::tools::file_ops::hematite_dir().join("snapshots");
        let fmt = cockpit.report_format.trim().to_ascii_lowercase();

        if !dir.exists() {
            if fmt == "json" {
                println!("[]");
            } else {
                println!("No snapshots saved yet.");
                println!("Save one with: hematite --inspect <topic> --snapshot <name>");
            }
            return Ok(());
        }
        let mut entries: Vec<_> = std::fs::read_dir(&dir)
            .into_iter()
            .flatten()
            .flatten()
            .filter(|e| e.path().extension().is_some_and(|x| x == "txt"))
            .collect();
        entries.sort_by_key(|e| {
            e.metadata()
                .and_then(|m| m.modified())
                .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
        });
        entries.reverse();

        if fmt == "json" {
            let arr: Vec<serde_json::Value> = entries
                .iter()
                .map(|e| {
                    let name = e.file_name();
                    let stem = std::path::Path::new(&name)
                        .file_stem()
                        .map(|s| s.to_string_lossy().into_owned())
                        .unwrap_or_default();
                    let size_bytes = e.metadata().map(|m| m.len()).unwrap_or(0);
                    let age_secs = e
                        .metadata()
                        .and_then(|m| m.modified())
                        .ok()
                        .and_then(|t| t.elapsed().ok())
                        .map(|d| d.as_secs())
                        .unwrap_or(0);
                    serde_json::json!({
                        "name": stem,
                        "size_bytes": size_bytes,
                        "age_secs": age_secs,
                    })
                })
                .collect();
            let out = serde_json::to_string_pretty(&serde_json::Value::Array(arr))
                .unwrap_or_else(|_| "[]".to_string());
            if let Some(ref out_path) = cockpit.output {
                write_output_copy(&out, out_path);
            } else {
                println!("{}", out);
            }
        } else if entries.is_empty() {
            println!("No snapshots saved yet.");
        } else {
            println!("Saved snapshots ({}):\n", entries.len());
            for e in &entries {
                let name = e.file_name();
                let stem = std::path::Path::new(&name)
                    .file_stem()
                    .map(|s| s.to_string_lossy().into_owned())
                    .unwrap_or_default();
                let size = e.metadata().map(|m| m.len()).unwrap_or(0);
                let age = e
                    .metadata()
                    .and_then(|m| m.modified())
                    .ok()
                    .and_then(|t| t.elapsed().ok())
                    .map(|d| {
                        let s = d.as_secs();
                        if s < 60 {
                            format!("{}s ago", s)
                        } else if s < 3600 {
                            format!("{}m ago", s / 60)
                        } else if s < 86400 {
                            format!("{}h ago", s / 3600)
                        } else {
                            format!("{}d ago", s / 86400)
                        }
                    })
                    .unwrap_or_else(|| "?".to_string());
                println!("  {:30}  {:>6} B  {}", stem, size, age);
            }
            println!("\nDiff against live: hematite --diff <topic> --from <name>");
            println!("Diff two saved:    hematite --compare <name1>,<name2>");
        }
        return Ok(());
    }

    if let Some(ref names_csv) = cockpit.compare {
        let parts: Vec<&str> = names_csv.splitn(2, ',').collect();
        if parts.len() != 2 {
            eprintln!(
                "Error: --compare requires two comma-separated snapshot names.\n\
                 Example: hematite --compare before-update,after-update\n\
                 Run `hematite --snapshots` to list available snapshots."
            );
            std::process::exit(1);
        }
        let (name_a, name_b) = (parts[0].trim(), parts[1].trim());
        let load_snap = |name: &str| {
            let path = snapshot_path(name);
            match std::fs::read_to_string(&path) {
                Ok(content) => {
                    let age = path
                        .metadata()
                        .and_then(|m| m.modified())
                        .ok()
                        .and_then(|t| t.elapsed().ok())
                        .map(|d| {
                            let s = d.as_secs();
                            if s < 60 {
                                format!("{}s ago", s)
                            } else if s < 3600 {
                                format!("{}m ago", s / 60)
                            } else if s < 86400 {
                                format!("{}h ago", s / 3600)
                            } else {
                                format!("{}d ago", s / 86400)
                            }
                        })
                        .unwrap_or_else(|| "saved".to_string());
                    Ok((content, age))
                }
                Err(e) => Err(format!("Cannot load snapshot '{}': {}", name, e)),
            }
        };
        let (snap_a, age_a) = match load_snap(name_a) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("{}", e);
                eprintln!("Run `hematite --snapshots` to list available snapshots.");
                std::process::exit(1);
            }
        };
        let (snap_b, age_b) = match load_snap(name_b) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("{}", e);
                eprintln!("Run `hematite --snapshots` to list available snapshots.");
                std::process::exit(1);
            }
        };
        let fmt = cockpit.report_format.trim().to_ascii_lowercase();
        let snap_a_f = apply_field_filter(&snap_a, cockpit.field.as_deref());
        let snap_b_f = apply_field_filter(&snap_b, cockpit.field.as_deref());
        use similar::{ChangeTag, TextDiff};
        let diff = TextDiff::from_lines(snap_a_f.as_ref(), snap_b_f.as_ref());
        if fmt == "json" {
            let mut diff_lines: Vec<String> = Vec::new();
            let mut changed = false;
            for group in diff.grouped_ops(2) {
                for op in &group {
                    for change in diff.iter_changes(op) {
                        let prefix = match change.tag() {
                            ChangeTag::Delete => {
                                changed = true;
                                "-"
                            }
                            ChangeTag::Insert => {
                                changed = true;
                                "+"
                            }
                            ChangeTag::Equal => " ",
                        };
                        diff_lines.push(format!("{}{}", prefix, change));
                    }
                }
            }
            let obj = serde_json::json!({
                "snapshot_a": format!("{} ({})", name_a, age_a),
                "snapshot_b": format!("{} ({})", name_b, age_b),
                "changed": changed,
                "diff_lines": diff_lines,
                "before": snap_a_f.as_ref(),
                "after": snap_b_f.as_ref(),
            });
            let out = serde_json::to_string_pretty(&obj)
                .unwrap_or_else(|e| format!("{{\"error\": \"{}\"}}", e));
            if let Some(ref out_path) = cockpit.output {
                write_output_copy(&out, out_path);
            } else {
                println!("{}", out);
            }
            if cockpit.clipboard {
                copy_to_clipboard(&out);
                println!("Copied to clipboard.");
            }
        } else {
            println!("--- {}  ({})", name_a, age_a);
            println!("+++ {}  ({})", name_b, age_b);
            println!();
            let mut changed = false;
            for group in diff.grouped_ops(2) {
                for op in &group {
                    for change in diff.iter_changes(op) {
                        match change.tag() {
                            ChangeTag::Delete => {
                                print!("\x1B[31m- {}\x1B[0m", change);
                                changed = true;
                            }
                            ChangeTag::Insert => {
                                print!("\x1B[32m+ {}\x1B[0m", change);
                                changed = true;
                            }
                            ChangeTag::Equal => {
                                print!("  {}", change);
                            }
                        }
                    }
                }
            }
            if !changed {
                println!("No differences between '{}' and '{}'.", name_a, name_b);
            }
        }
        return Ok(());
    }

    const AUDIT_DEFAULT_TOPICS: &str =
        "services,startup_items,ports,scheduled_tasks,shares,firewall_rules,processes,connections";

    if let Some(ref audit_name) = cockpit.audit_start.clone() {
        let topics = cockpit
            .audit_topics
            .as_deref()
            .unwrap_or(AUDIT_DEFAULT_TOPICS);
        eprintln!("Starting audit session '{}'...", audit_name);
        eprintln!("Topics: {}", topics);
        let content = hematite::agent::report_export::generate_inspect_output(topics).await;
        let before_name = format!("{}_before", audit_name);
        let snap_path = snapshot_path(&before_name);
        if let Some(parent) = snap_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(&snap_path, content.as_str());
        let meta_path = {
            let safe: String = audit_name
                .chars()
                .map(|c| {
                    if c.is_alphanumeric() || c == '-' || c == '_' {
                        c
                    } else {
                        '_'
                    }
                })
                .collect();
            hematite::tools::file_ops::hematite_dir()
                .join("snapshots")
                .join(format!("{}.audit.json", safe))
        };
        let started_at = hematite::agent::report_export::timestamp_label();
        let meta = format!(
            "{{\"topics\":\"{}\",\"started_at\":\"{}\"}}",
            topics.replace('"', "\\\""),
            started_at
        );
        let _ = std::fs::write(&meta_path, meta);
        println!("Audit session '{}' started.", audit_name);
        println!("Baseline: {}", snap_path.display());
        println!("Topics:   {}", topics);
        println!();
        println!("Make your changes, then run:");
        println!("  hematite --audit-end {}", audit_name);
        return Ok(());
    }

    if let Some(ref audit_name) = cockpit.audit_end.clone() {
        let meta_path = {
            let safe: String = audit_name
                .chars()
                .map(|c| {
                    if c.is_alphanumeric() || c == '-' || c == '_' {
                        c
                    } else {
                        '_'
                    }
                })
                .collect();
            hematite::tools::file_ops::hematite_dir()
                .join("snapshots")
                .join(format!("{}.audit.json", safe))
        };
        let meta_str = match std::fs::read_to_string(&meta_path) {
            Ok(s) => s,
            Err(_) => {
                eprintln!(
                    "Error: no audit session named '{}' found.\n\
                     Start one with: hematite --audit-start {}",
                    audit_name, audit_name
                );
                std::process::exit(1);
            }
        };
        // Parse topics from metadata JSON without pulling in serde_json.
        let topics = meta_str
            .split('"')
            .skip_while(|s| *s != "topics")
            .nth(2)
            .unwrap_or(AUDIT_DEFAULT_TOPICS);
        let started_at = meta_str
            .split('"')
            .skip_while(|s| *s != "started_at")
            .nth(2)
            .unwrap_or("unknown");

        let before_name = format!("{}_before", audit_name);
        let before_path = snapshot_path(&before_name);
        let snap_a = match std::fs::read_to_string(&before_path) {
            Ok(s) => s,
            Err(_) => {
                eprintln!(
                    "Error: baseline snapshot '{}' not found.\n\
                     Start a new session with: hematite --audit-start {}",
                    before_name, audit_name
                );
                std::process::exit(1);
            }
        };

        eprintln!(
            "Audit session '{}': capturing post-change snapshot...",
            audit_name
        );
        eprintln!("Topics: {}", topics);
        let snap_b = hematite::agent::report_export::generate_inspect_output(topics).await;
        let after_name = format!("{}_after", audit_name);
        let after_path = snapshot_path(&after_name);
        let _ = std::fs::write(&after_path, snap_b.as_str());

        use similar::{ChangeTag, TextDiff};
        let diff = TextDiff::from_lines(&snap_a, &snap_b);
        let mut diff_md = String::new();
        let mut any_changed = false;
        for group in diff.grouped_ops(3) {
            for op in &group {
                for change in diff.iter_changes(op) {
                    match change.tag() {
                        ChangeTag::Delete => {
                            any_changed = true;
                            diff_md.push('-');
                            diff_md.push(' ');
                            diff_md.push_str(change.value());
                        }
                        ChangeTag::Insert => {
                            any_changed = true;
                            diff_md.push('+');
                            diff_md.push(' ');
                            diff_md.push_str(change.value());
                        }
                        ChangeTag::Equal => {
                            diff_md.push(' ');
                            diff_md.push(' ');
                            diff_md.push_str(change.value());
                        }
                    }
                }
            }
            diff_md.push('\n');
        }

        let ended_at = hematite::agent::report_export::timestamp_label();
        let change_summary = if any_changed {
            "Changes detected — review the diff below."
        } else {
            "No changes detected across all topics."
        };

        let mut report = String::new();
        report.push_str(&format!("# Change Audit: {}\n\n", audit_name));
        report.push_str(&format!("**Started:** {}\n", started_at));
        report.push_str(&format!("**Completed:** {}\n", ended_at));
        report.push_str(&format!("**Topics:** {}\n\n", topics));
        report.push_str(&format!("## Result\n\n{}\n\n", change_summary));
        if any_changed {
            report.push_str("## Diff\n\n```diff\n");
            report.push_str(&diff_md);
            report.push_str("```\n");
        }

        let report_dir = hematite::tools::file_ops::hematite_dir().join("reports");
        let _ = std::fs::create_dir_all(&report_dir);
        let safe_name: String = audit_name
            .chars()
            .map(|c| {
                if c.is_alphanumeric() || c == '-' || c == '_' {
                    c
                } else {
                    '_'
                }
            })
            .collect();
        let safe_ts: String = ended_at
            .chars()
            .map(|c| {
                if c.is_alphanumeric() || c == '-' {
                    c
                } else {
                    '_'
                }
            })
            .collect();
        let fmt = cockpit.report_format.trim().to_ascii_lowercase();
        let report_path = if fmt == "html" {
            let html = hematite::agent::html_template::build_html_shell(
                &format!("Change Audit: {}", audit_name),
                &hematite::hematite_version(),
                &hematite::agent::html_template::markdown_to_html(&report),
            );
            let p = report_dir.join(format!("audit-{}-{}.html", safe_name, safe_ts));
            let _ = std::fs::write(&p, &html);
            p
        } else {
            let p = report_dir.join(format!("audit-{}-{}.md", safe_name, safe_ts));
            let _ = std::fs::write(&p, &report);
            p
        };

        print!("{}", report);
        println!("Audit report saved: {}", report_path.display());

        if cockpit.clipboard {
            copy_to_clipboard(&report);
            println!("Copied to clipboard.");
        }
        if cockpit.open {
            open_path(&report_path);
        }
        if cockpit.notify {
            show_toast(
                "Hematite Audit",
                if any_changed {
                    "Changes detected"
                } else {
                    "No changes"
                },
            );
        }
        // Clean up the metadata file now that the session is complete.
        let _ = std::fs::remove_file(&meta_path);
        return Ok(());
    }

    // ── Alert Rules ───────────────────────────────────────────────────────────

    fn alert_rules_path() -> std::path::PathBuf {
        hematite::tools::file_ops::hematite_dir().join("alert_rules.json")
    }

    #[derive(Debug)]
    struct AlertRule {
        id: u64,
        label: String,
        topic: String,
        pattern: String,
        negate: bool,
    }

    fn load_alert_rules() -> Vec<AlertRule> {
        let path = alert_rules_path();
        let content = match std::fs::read_to_string(&path) {
            Ok(s) => s,
            Err(_) => return Vec::new(),
        };
        let arr: Vec<serde_json::Value> = serde_json::from_str(&content).unwrap_or_default();
        arr.into_iter()
            .filter_map(|v| {
                Some(AlertRule {
                    id: v["id"].as_u64()?,
                    label: v["label"].as_str()?.to_string(),
                    topic: v["topic"].as_str()?.to_string(),
                    pattern: v["pattern"].as_str()?.to_string(),
                    negate: v["negate"].as_bool().unwrap_or(false),
                })
            })
            .collect()
    }

    fn save_alert_rules(rules: &[AlertRule]) {
        let arr: Vec<serde_json::Value> = rules
            .iter()
            .map(|r| {
                serde_json::json!({
                    "id": r.id,
                    "label": r.label,
                    "topic": r.topic,
                    "pattern": r.pattern,
                    "negate": r.negate,
                })
            })
            .collect();
        let json = serde_json::to_string_pretty(&serde_json::Value::Array(arr))
            .unwrap_or_else(|_| "[]".to_string());
        let _ = std::fs::write(alert_rules_path(), json);
    }

    if let Some(ref spec) = cockpit.alert_rule_add.clone() {
        let (topic, pattern) = if let Some(pos) = spec.find(':') {
            (
                spec[..pos].trim().to_string(),
                spec[pos + 1..].trim().to_string(),
            )
        } else {
            eprintln!(
                "Error: --alert-rule-add requires TOPIC:PATTERN format.\n\
                 Example: hematite --alert-rule-add thermal:throttl --alert-rule-label \"CPU Throttling\""
            );
            std::process::exit(1);
        };
        if topic.is_empty() || pattern.is_empty() {
            eprintln!("Error: both topic and pattern must be non-empty.");
            std::process::exit(1);
        }
        let mut rules = load_alert_rules();
        let next_id = rules.iter().map(|r| r.id).max().unwrap_or(0) + 1;
        let label = cockpit
            .alert_rule_label
            .as_deref()
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("{} matches \"{}\"", topic, pattern));
        let negate = cockpit.alert_rule_negate;
        rules.push(AlertRule {
            id: next_id,
            label: label.clone(),
            topic: topic.clone(),
            pattern: pattern.clone(),
            negate,
        });
        save_alert_rules(&rules);
        let neg_note = if negate { " (fires when ABSENT)" } else { "" };
        println!("Alert rule #{} added: \"{}\"", next_id, label);
        println!("  Topic:   {}", topic);
        println!("  Pattern: \"{}\"{}", pattern, neg_note);
        println!();
        println!("Evaluate now:    hematite --alert-rule-run");
        println!("Schedule hourly: hematite --alert-rule-run --schedule hourly");
        return Ok(());
    }

    if cockpit.alert_rules {
        let rules = load_alert_rules();
        if rules.is_empty() {
            println!("No alert rules defined.");
            println!();
            println!(
                "Add a rule:  hematite --alert-rule-add TOPIC:PATTERN --alert-rule-label \"Name\""
            );
            println!("Examples:");
            println!(
                "  hematite --alert-rule-add thermal:throttl --alert-rule-label \"CPU Throttling\""
            );
            println!("  hematite --alert-rule-add startup_items:HKCU\\\\Software --alert-rule-label \"New user startup entry\"");
            println!("  hematite --alert-rule-add services:Defender --alert-rule-negate --alert-rule-label \"Defender stopped\"");
            return Ok(());
        }
        println!(
            "Alert Rules ({} rule{})",
            rules.len(),
            if rules.len() == 1 { "" } else { "s" }
        );
        println!("{}", "\u{2500}".repeat(64));
        for r in &rules {
            let neg = if r.negate {
                "  [fires when ABSENT]"
            } else {
                ""
            };
            println!(
                "  {:>3}  {:<28}  {:<20}  \"{}\"{}",
                r.id, r.label, r.topic, r.pattern, neg
            );
        }
        println!();
        println!("Evaluate all:    hematite --alert-rule-run");
        println!("Remove rule N:   hematite --alert-rule-remove <ID>");
        println!("Schedule hourly: hematite --alert-rule-run --schedule hourly");
        return Ok(());
    }

    if let Some(remove_id) = cockpit.alert_rule_remove {
        let mut rules = load_alert_rules();
        let before = rules.len();
        rules.retain(|r| r.id != remove_id);
        if rules.len() == before {
            eprintln!(
                "No alert rule with ID {} found. Run `hematite --alert-rules` to list rules.",
                remove_id
            );
            std::process::exit(1);
        }
        save_alert_rules(&rules);
        println!("Alert rule #{} removed.", remove_id);
        return Ok(());
    }

    if cockpit.alert_rule_run {
        if let Some(ref cadence) = cockpit.schedule {
            let cs = cadence.trim();
            let exe_path = std::env::current_exe()
                .unwrap_or_else(|_| std::path::PathBuf::from("hematite"))
                .to_string_lossy()
                .to_string();
            match cs {
                "remove" => match hematite::agent::scheduler::remove_alert_task() {
                    Ok(msg) => println!("{}", msg),
                    Err(e) => eprintln!("Error: {}", e),
                },
                "status" => println!("{}", hematite::agent::scheduler::query_alert_task()),
                _ => match hematite::agent::scheduler::register_alert_task(cs, &exe_path) {
                    Ok(msg) => println!("{}", msg),
                    Err(e) => eprintln!("Error: {}", e),
                },
            }
            return Ok(());
        }

        let rules = load_alert_rules();
        if rules.is_empty() {
            println!("No alert rules defined. Add one with --alert-rule-add.");
            return Ok(());
        }

        let mut fired = 0usize;
        for rule in &rules {
            let output = hematite::agent::report_export::generate_inspect_output(&rule.topic).await;
            let matched = output
                .to_ascii_lowercase()
                .contains(&rule.pattern.to_ascii_lowercase());
            let should_fire = if rule.negate { !matched } else { matched };
            let status = if should_fire { "ALERT" } else { "ok" };
            println!(
                "[{}] {} — topic:{} pattern:\"{}\"{}",
                status,
                rule.label,
                rule.topic,
                rule.pattern,
                if rule.negate { " (negate)" } else { "" }
            );
            if should_fire {
                fired += 1;
                show_toast(&format!("Hematite Alert: {}", rule.label), &rule.pattern);
            }
        }
        println!();
        if fired == 0 {
            println!("All {} rule(s) OK — no alerts fired.", rules.len());
        } else {
            println!("{} alert(s) fired out of {} rule(s).", fired, rules.len());
        }
        return Ok(());
    }

    // ── End Alert Rules ───────────────────────────────────────────────────────

    // ── Timeline ──────────────────────────────────────────────────────────────

    const TIMELINE_TOPICS: &str = "health_report,startup_items,ports,services";

    fn timeline_dir() -> std::path::PathBuf {
        hematite::tools::file_ops::hematite_dir().join("timeline")
    }

    fn timeline_entry_path(date: &str) -> std::path::PathBuf {
        let safe: String = date
            .chars()
            .map(|c| {
                if c.is_alphanumeric() || c == '-' {
                    c
                } else {
                    '_'
                }
            })
            .collect();
        timeline_dir().join(format!("{}.txt", safe))
    }

    fn timeline_index_path() -> std::path::PathBuf {
        timeline_dir().join("index.jsonl")
    }

    fn timeline_today_date() -> String {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let days = now / 86400;
        let years_400 = days / 146097;
        let rem = days % 146097;
        let years_100 = rem.min(146096) / 36524;
        let rem = rem - years_100 * 36524;
        let years_4 = rem / 1461;
        let rem = rem % 1461;
        let years_1 = rem.min(1460) / 365;
        let rem = rem - years_1 * 365;
        let year = 1970 + years_400 * 400 + years_100 * 100 + years_4 * 4 + years_1;
        let leap = u64::from(year % 4 == 0 && (year % 100 != 0 || year % 400 == 0));
        let month_days: [u64; 12] = [31, 28 + leap, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
        let mut rem = rem;
        let mut month = 1u64;
        for &md in &month_days {
            if rem < md {
                break;
            }
            rem -= md;
            month += 1;
        }
        format!("{:04}-{:02}-{:02}", year, month, rem + 1)
    }

    struct TimelineEntry {
        date: String,
        grade: char,
        summary: String,
    }

    fn load_timeline_index() -> Vec<TimelineEntry> {
        let path = timeline_index_path();
        let content = match std::fs::read_to_string(&path) {
            Ok(s) => s,
            Err(_) => return Vec::new(),
        };
        content
            .lines()
            .filter(|l| !l.trim().is_empty())
            .filter_map(|line| {
                let date = line
                    .split('"')
                    .skip_while(|s| *s != "date")
                    .nth(2)?
                    .to_string();
                let grade = line
                    .split('"')
                    .skip_while(|s| *s != "grade")
                    .nth(2)?
                    .chars()
                    .next()
                    .unwrap_or('?');
                let summary = line
                    .split('"')
                    .skip_while(|s| *s != "summary")
                    .nth(2)
                    .unwrap_or("")
                    .to_string();
                Some(TimelineEntry {
                    date,
                    grade,
                    summary,
                })
            })
            .collect()
    }

    fn append_timeline_index(date: &str, grade: char, summary: &str) {
        let path = timeline_index_path();
        let line = format!(
            "{{\"date\":\"{}\",\"grade\":\"{}\",\"summary\":\"{}\"}}\n",
            date,
            grade,
            summary.replace('"', "'")
        );
        use std::io::Write;
        if let Ok(mut f) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
        {
            let _ = f.write_all(line.as_bytes());
        }
    }

    if cockpit.timeline_capture {
        if let Some(ref cadence) = cockpit.schedule {
            let cadence_str = cadence.trim();
            if cadence_str == "daily" || cadence_str == "remove" || cadence_str == "status" {
                let exe_path = std::env::current_exe()
                    .unwrap_or_else(|_| std::path::PathBuf::from("hematite"))
                    .to_string_lossy()
                    .to_string();
                match cadence_str {
                    "remove" => match hematite::agent::scheduler::remove_timeline_task() {
                        Ok(msg) => println!("{}", msg),
                        Err(e) => eprintln!("Error: {}", e),
                    },
                    "status" => println!("{}", hematite::agent::scheduler::query_timeline_task()),
                    _ => match hematite::agent::scheduler::register_timeline_task(&exe_path) {
                        Ok(msg) => println!("{}", msg),
                        Err(e) => eprintln!("Error: {}", e),
                    },
                }
                return Ok(());
            }
        }

        let today = timeline_today_date();
        let entry_path = timeline_entry_path(&today);

        if entry_path.exists() && !cockpit.yes {
            println!("Timeline entry for {} already exists.", today);
            println!("Path: {}", entry_path.display());
            println!("Use --yes to overwrite.");
            return Ok(());
        }

        let _ = std::fs::create_dir_all(timeline_dir());
        eprintln!("Capturing timeline snapshot for {}...", today);
        eprintln!("Topics: {}", TIMELINE_TOPICS);
        let content =
            hematite::agent::report_export::generate_inspect_output(TIMELINE_TOPICS).await;
        let _ = std::fs::write(&entry_path, content.as_str());

        let score = hematite::agent::report_export::score_health_from_content(&content);
        let summary = score.summary_line();

        // Rebuild index removing any existing entry for today, then append.
        let index_path = timeline_index_path();
        let existing = match std::fs::read_to_string(&index_path) {
            Ok(s) => s
                .lines()
                .filter(|l| !l.contains(&format!("\"date\":\"{}\"", today)))
                .map(|l| format!("{}\n", l))
                .collect::<String>(),
            Err(_) => String::new(),
        };
        let _ = std::fs::write(&index_path, existing);
        append_timeline_index(&today, score.grade, &summary);

        println!("Timeline snapshot: {}", today);
        println!("Health grade:      {} — {}", score.grade, summary);
        println!("Saved:             {}", entry_path.display());
        return Ok(());
    }

    if cockpit.timeline {
        let entries = load_timeline_index();
        if entries.is_empty() {
            println!("No timeline entries yet.");
            println!();
            println!("Start capturing daily snapshots:");
            println!("  hematite --timeline-capture");
            println!("  hematite --timeline-capture --schedule daily  (automated, daily at 03:00)");
            return Ok(());
        }
        let first = entries.first().map(|e| e.date.as_str()).unwrap_or("");
        let last = entries.last().map(|e| e.date.as_str()).unwrap_or("");
        println!(
            "Machine State Timeline  ({} entries, {} \u{2192} {})",
            entries.len(),
            first,
            last
        );
        println!("{}", "\u{2500}".repeat(60));
        for e in &entries {
            println!("  {}  {}  {}", e.date, e.grade, e.summary);
        }
        println!();
        println!("Diff two dates:  hematite --timeline-diff DATE1,DATE2");
        println!("Capture today:   hematite --timeline-capture");
        return Ok(());
    }

    if cockpit.timeline_trend {
        let entries = load_timeline_index();
        if entries.is_empty() {
            println!("No timeline entries yet. Run `hematite --timeline-capture` to start.");
            return Ok(());
        }

        let grade_score = |g: char| -> i32 {
            match g {
                'A' => 6,
                'B' => 5,
                'C' => 4,
                'D' => 3,
                'F' => 2,
                _ => 1,
            }
        };
        let grade_bar_width = |g: char| -> usize {
            match g {
                'A' => 30,
                'B' => 24,
                'C' => 18,
                'D' => 12,
                'F' => 6,
                _ => 3,
            }
        };
        let grade_color = |g: char| -> &'static str {
            match g {
                'A' => "\x1B[32m",
                'B' => "\x1B[92m",
                'C' => "\x1B[33m",
                'D' => "\x1B[31m",
                'F' => "\x1B[91m",
                _ => "",
            }
        };
        let reset = "\x1B[0m";

        let first_date = entries.first().map(|e| e.date.as_str()).unwrap_or("");
        let last_date = entries.last().map(|e| e.date.as_str()).unwrap_or("");
        println!(
            "Machine Health Trend \u{2014} {} entries ({} \u{2192} {})\n",
            entries.len(),
            first_date,
            last_date
        );

        // Bar chart
        println!(" {:<12}  {:^5}  Health", "Date", "Grade");
        println!(" {}", "\u{2500}".repeat(62));
        for e in &entries {
            let w = grade_bar_width(e.grade);
            let bar: String = "\u{2588}".repeat(w);
            let color = grade_color(e.grade);
            let summary_short = if e.summary.chars().count() > 36 {
                let truncated: String = e.summary.chars().take(33).collect();
                format!("{}...", truncated)
            } else {
                e.summary.clone()
            };
            println!(
                " {:<12}  {}{:^5}{}  {}{}{}  {}",
                e.date, color, e.grade, reset, color, bar, reset, summary_short
            );
        }

        // Sparkline
        let spark: String = entries
            .iter()
            .map(|e| match e.grade {
                'A' => "\u{2588}",
                'B' => "\u{2586}",
                'C' => "\u{2584}",
                'D' => "\u{2582}",
                'F' => "\u{2581}",
                _ => "\u{2591}",
            })
            .collect::<Vec<_>>()
            .join(" ");
        println!("\n Sparkline: {}", spark);

        // Worst / best / trajectory
        let worst = entries.iter().min_by_key(|e| grade_score(e.grade));
        let best = entries.iter().max_by_key(|e| grade_score(e.grade));
        let first_score = grade_score(entries[0].grade);
        let last_score = grade_score(entries[entries.len() - 1].grade);
        let trajectory = if last_score > first_score {
            "\u{2191} improving"
        } else if last_score < first_score {
            "\u{2193} degraded"
        } else {
            "\u{2192} stable"
        };

        // Recent trend (up to last 7 entries)
        let recent_trend = if entries.len() >= 3 {
            let window = &entries[entries.len().saturating_sub(7)..];
            let rs = grade_score(window[0].grade);
            let re = grade_score(window[window.len() - 1].grade);
            if re > rs {
                "improving recently"
            } else if re < rs {
                "degrading recently"
            } else {
                "stable recently"
            }
        } else {
            "not enough data for recent trend"
        };

        println!("\n Summary:");
        if let Some(w) = worst {
            let color = grade_color(w.grade);
            println!("   Worst:   {}{}{} ({})", color, w.grade, reset, w.date);
        }
        if let Some(b) = best {
            let color = grade_color(b.grade);
            println!("   Best:    {}{}{} ({})", color, b.grade, reset, b.date);
        }
        let last = &entries[entries.len() - 1];
        let lc = grade_color(last.grade);
        println!(
            "   Latest:  {}{}{} {} \u{2014} {}",
            lc, last.grade, reset, trajectory, recent_trend
        );
        println!(
            "\n Tip: run `hematite --timeline-diff DATE` to see what changed on a specific day."
        );

        return Ok(());
    }

    if let Some(ref spec) = cockpit.timeline_diff.clone() {
        let parts: Vec<&str> = spec.splitn(2, ',').collect();
        let (date_a, date_b) = if parts.len() == 2 {
            (parts[0].trim().to_string(), parts[1].trim().to_string())
        } else {
            // Single date: find the previous entry from the index.
            let date_a_str = parts[0].trim().to_string();
            let entries = load_timeline_index();
            let prev = entries
                .windows(2)
                .find(|w| w[1].date == date_a_str)
                .map(|w| w[0].date.clone());
            match prev {
                Some(p) => (p, date_a_str),
                None => {
                    eprintln!(
                        "Error: no previous timeline entry found before '{}'.\n\
                         Run `hematite --timeline` to see available entries.\n\
                         To diff two specific dates: hematite --timeline-diff DATE1,DATE2",
                        date_a_str
                    );
                    std::process::exit(1);
                }
            }
        };

        let load = |date: &str| -> Result<String, String> {
            let p = timeline_entry_path(date);
            std::fs::read_to_string(&p).map_err(|_| {
                format!(
                    "No timeline entry for '{}'. Run `hematite --timeline` to see available dates.",
                    date
                )
            })
        };

        let snap_a = match load(&date_a) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("{}", e);
                std::process::exit(1);
            }
        };
        let snap_b = match load(&date_b) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("{}", e);
                std::process::exit(1);
            }
        };

        println!("--- {}", date_a);
        println!("+++ {}", date_b);
        println!();

        use similar::{ChangeTag, TextDiff};
        let diff = TextDiff::from_lines(&snap_a, &snap_b);
        let mut changed = false;
        for group in diff.grouped_ops(2) {
            for op in &group {
                for change in diff.iter_changes(op) {
                    match change.tag() {
                        ChangeTag::Delete => {
                            print!("\x1B[31m- {}\x1B[0m", change);
                            changed = true;
                        }
                        ChangeTag::Insert => {
                            print!("\x1B[32m+ {}\x1B[0m", change);
                            changed = true;
                        }
                        ChangeTag::Equal => {
                            print!("  {}", change);
                        }
                    }
                }
            }
            println!();
        }
        if !changed {
            println!("No differences between {} and {}.", date_a, date_b);
        }
        return Ok(());
    }

    // ── End Timeline ──────────────────────────────────────────────────────────

    // ── Diagnose Why ─────────────────────────────────────────────────────────

    if let Some(ref symptom) = cockpit.diagnose_why.clone() {
        use hematite::agent::diagnose_why::{build_diagnosis, match_symptom};

        let group = match match_symptom(symptom.trim()) {
            Some(g) => g,
            None => {
                eprintln!(
                    "No matching symptom category for: \"{}\"\n\
                     Try describing the symptom differently, e.g.:\n\
                       hematite --diagnose-why \"PC running slow\"\n\
                       hematite --diagnose-why \"blue screen\"\n\
                       hematite --diagnose-why \"no internet\"",
                    symptom.trim()
                );
                std::process::exit(1);
            }
        };

        let topics_csv = group.topics.join(",");
        eprintln!(
            "Symptom: {} | Category: {} | Topics: {}",
            symptom.trim(),
            group.category,
            topics_csv
        );
        eprintln!("Running {} topic(s)...", group.topics.len());

        let raw_output = hematite::agent::report_export::generate_inspect_output(&topics_csv).await;
        let diagnosis = build_diagnosis(group, &raw_output);

        // ── Format report ────────────────────────────────────────────────────
        let mut md = String::new();
        md.push_str(&format!("# Diagnose Why: {}\n\n", symptom.trim()));
        md.push_str(&format!("**Category:** {}\n", diagnosis.category));
        md.push_str(&format!(
            "**Topics inspected:** {}\n\n",
            diagnosis.topics_run.join(", ")
        ));

        // ── Root cause correlation (shown first when present) ────────────────
        if !diagnosis.root_causes.is_empty() {
            md.push_str("## Root Cause Correlation\n\n");
            md.push_str(
                "> These findings co-occur across multiple topics and likely share one root cause.\n\n",
            );
            for (i, rc) in diagnosis.root_causes.iter().enumerate() {
                md.push_str(&format!(
                    "### [{}] [{}] {}\n\n",
                    i + 1,
                    rc.confidence,
                    rc.summary
                ));
                md.push_str(&format!("{}\n\n", rc.detail));
                if !rc.unified_steps.is_empty() {
                    md.push_str("**Consolidated fix steps:**\n\n");
                    for (n, step) in rc.unified_steps.iter().enumerate() {
                        md.push_str(&format!("{}. {}\n", n + 1, step));
                    }
                    md.push('\n');
                }
            }
            md.push_str("---\n\n");
        }

        if diagnosis.findings.is_empty() {
            md.push_str("## Result\n\nNo actionable issues detected across all topics.\n");
        } else {
            md.push_str("## Probable Causes (ranked by severity)\n\n");
            for (i, f) in diagnosis.findings.iter().enumerate() {
                md.push_str(&format!("### [{}] {} — {}\n\n", i + 1, f.severity, f.title));
                if !f.evidence.is_empty() {
                    md.push_str("**Evidence:**\n");
                    for line in &f.evidence {
                        md.push_str(&format!("> {}\n", line));
                    }
                    md.push('\n');
                }
                if !f.steps.is_empty() {
                    md.push_str("**Steps to fix:**\n");
                    for (n, step) in f.steps.iter().enumerate() {
                        md.push_str(&format!("{}. {}\n", n + 1, step));
                    }
                    md.push('\n');
                }
                if let Some(cmd) = f.dig_deeper {
                    md.push_str(&format!("**Dig deeper:** `{}`\n\n", cmd));
                }
            }
        }

        println!("{}", md.trim_end());

        // ── Save report ──────────────────────────────────────────────────────
        let ts = hematite::agent::report_export::timestamp_label();
        let safe_ts: String = ts
            .chars()
            .map(|c| {
                if c.is_alphanumeric() || c == '-' {
                    c
                } else {
                    '_'
                }
            })
            .collect();
        let report_dir = hematite::tools::file_ops::hematite_dir().join("reports");
        let _ = std::fs::create_dir_all(&report_dir);
        let fmt = cockpit.report_format.trim().to_ascii_lowercase();
        let report_path = if fmt == "html" {
            let html = hematite::agent::html_template::build_html_shell(
                &format!("Diagnose Why: {}", symptom.trim()),
                &hematite::hematite_version(),
                &hematite::agent::html_template::markdown_to_html(&md),
            );
            let p = report_dir.join(format!("diagnose-why-{}.html", safe_ts));
            let _ = std::fs::write(&p, &html);
            p
        } else {
            let p = report_dir.join(format!("diagnose-why-{}.md", safe_ts));
            let _ = std::fs::write(&p, &md);
            p
        };
        println!("\nDiagnosis saved: {}", report_path.display());

        if let Some(ref out_path) = cockpit.output {
            write_output_copy(&md, out_path);
        }
        if cockpit.clipboard {
            copy_to_clipboard(&md);
            println!("Copied to clipboard.");
        }
        if cockpit.open {
            open_path(&report_path);
        }
        if cockpit.notify {
            let summary = if diagnosis.findings.is_empty() {
                "No issues detected.".to_string()
            } else {
                format!(
                    "{} probable cause(s) found — top: {}",
                    diagnosis.findings.len(),
                    diagnosis.findings[0].title
                )
            };
            show_toast("Hematite Diagnose Why", &summary);
        }
        std::process::exit(if diagnosis.findings.is_empty() { 0 } else { 1 });
    }

    // ── End Diagnose Why ──────────────────────────────────────────────────────

    if let Some(ref topics_csv) = cockpit.watch {
        let interval = cockpit.watch_interval.max(1);
        let alert_pat = cockpit.alert.as_deref().map(|p| p.to_ascii_lowercase());

        let max_cycles = cockpit.count;
        let stop_label = match max_cycles {
            Some(n) => format!("{} cycle(s)", n),
            None => "Ctrl+C to stop".to_string(),
        };
        if let Some(ref pat) = alert_pat {
            eprintln!(
                "Watching: {} | alert: {:?} | interval: {}s | {}",
                topics_csv, pat, interval, stop_label
            );
        } else {
            eprintln!(
                "Watching: {} | interval: {}s | {}",
                topics_csv, interval, stop_label
            );
        }

        let mut cycle: u64 = 0;
        loop {
            use std::io::Write;

            let ts = {
                use std::time::{SystemTime, UNIX_EPOCH};
                let s = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();
                let h = ((s / 3600) % 24) as u32;
                let m = ((s / 60) % 60) as u32;
                let sec = (s % 60) as u32;
                format!("{:02}:{:02}:{:02} UTC", h, m, sec)
            };

            let raw_content =
                hematite::agent::report_export::generate_inspect_output(topics_csv).await;
            let content = apply_field_filter(&raw_content, cockpit.field.as_deref());

            let is_json_mode = cockpit.report_format.trim().eq_ignore_ascii_case("json");

            if is_json_mode {
                // JSON mode: emit one newline-delimited JSON object per cycle
                let alert_matched = alert_pat
                    .as_ref()
                    .map(|p| raw_content.to_ascii_lowercase().contains(p.as_str()))
                    .unwrap_or(false);
                let obj = serde_json::json!({
                    "timestamp": ts,
                    "cycle": cycle + 1,
                    "topics": topics_csv.as_str(),
                    "alert_matched": alert_matched,
                    "output": content.as_ref(),
                });
                println!(
                    "{}",
                    serde_json::to_string(&obj).unwrap_or_else(|_| "{}".to_string())
                );
                let _ = std::io::stdout().flush();
                if alert_matched && cockpit.notify {
                    if let Some(ref pat) = alert_pat {
                        show_toast(
                            "Hematite Alert",
                            &format!("Pattern {:?} matched at {}", pat, ts),
                        );
                    }
                }
            } else if let Some(ref pat) = alert_pat {
                if raw_content.to_ascii_lowercase().contains(pat.as_str()) {
                    // Match: ring bell, clear screen, print (field-filtered) output
                    print!("\x1B[2J\x1B[H\x07");
                    let _ = std::io::stdout().flush();
                    println!(
                        "\x1B[32mALERT\x1B[0m — pattern {:?} matched at {} | {}\n",
                        pat, ts, stop_label
                    );
                    if cockpit.notify {
                        show_toast(
                            "Hematite Alert",
                            &format!("Pattern {:?} matched at {}", pat, ts),
                        );
                    }
                    print!("{}", content);
                } else {
                    // No match: single heartbeat line, no clear
                    println!("  [{}]  no match for {:?}", ts, pat);
                }
            } else {
                // No alert filter: clear and display as before
                print!("\x1B[2J\x1B[H");
                let _ = std::io::stdout().flush();
                println!(
                    "Hematite Watch — {} | every {}s | {}\n",
                    ts, interval, stop_label
                );
                print!("{}", content);
            }

            // Append to --output log file if specified (NDJSON for json mode, timestamped blocks otherwise).
            if let Some(ref out_path) = cockpit.output {
                use std::io::Write as _;
                let path = std::path::Path::new(out_path);
                if let Some(parent) = path.parent() {
                    if !parent.as_os_str().is_empty() {
                        let _ = std::fs::create_dir_all(parent);
                    }
                }
                if let Ok(mut file) = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(path)
                {
                    if is_json_mode {
                        let alert_matched = alert_pat
                            .as_ref()
                            .map(|p| raw_content.to_ascii_lowercase().contains(p.as_str()))
                            .unwrap_or(false);
                        let obj = serde_json::json!({
                            "timestamp": ts,
                            "cycle": cycle + 1,
                            "topics": topics_csv.as_str(),
                            "alert_matched": alert_matched,
                            "output": content.as_ref(),
                        });
                        let _ =
                            writeln!(file, "{}", serde_json::to_string(&obj).unwrap_or_default());
                    } else {
                        let _ = writeln!(file, "=== {} (cycle {}) ===", ts, cycle + 1);
                        let _ = write!(file, "{}", content.as_ref());
                        let _ = writeln!(file);
                    }
                }
            }

            let _ = std::io::stdout().flush();
            cycle += 1;
            if let Some(max) = max_cycles {
                if cycle >= max {
                    break;
                }
            }
            tokio::time::sleep(std::time::Duration::from_secs(interval)).await;
        }
    }

    if let Some(ref topics_csv) = cockpit.diff {
        let after_secs = cockpit.diff_after.max(1);

        let ts = |secs: u64| {
            let h = ((secs / 3600) % 24) as u32;
            let m = ((secs / 60) % 60) as u32;
            let s = (secs % 60) as u32;
            format!("{:02}:{:02}:{:02} UTC", h, m, s)
        };

        let now = || {
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs()
        };

        // If --from <name> is given, load snapshot A from disk
        let (snap_a, ts_a) = if let Some(ref from_name) = cockpit.from {
            let path = snapshot_path(from_name);
            match std::fs::read_to_string(&path) {
                Ok(content) => {
                    eprintln!("Loaded snapshot A from: {}", path.display());
                    let age = path
                        .metadata()
                        .and_then(|m| m.modified())
                        .ok()
                        .and_then(|t| t.elapsed().ok())
                        .map(|d| {
                            let s = d.as_secs();
                            if s < 60 {
                                format!("{}s ago", s)
                            } else if s < 3600 {
                                format!("{}m ago", s / 60)
                            } else if s < 86400 {
                                format!("{}h ago", s / 3600)
                            } else {
                                format!("{}d ago", s / 86400)
                            }
                        })
                        .unwrap_or_else(|| "saved".to_string());
                    (content, format!("{} ({})", from_name, age))
                }
                Err(e) => {
                    eprintln!("Error loading snapshot '{}': {}", from_name, e);
                    eprintln!("Run `hematite --snapshots` to list available snapshots.");
                    std::process::exit(1);
                }
            }
        } else {
            eprintln!("Taking snapshot A ({})...", topics_csv);
            let s = hematite::agent::report_export::generate_inspect_output(topics_csv).await;
            let t = ts(now());
            eprintln!(
                "Snapshot A taken at {}. Waiting {}s for snapshot B...",
                t, after_secs
            );
            tokio::time::sleep(std::time::Duration::from_secs(after_secs)).await;
            (s, t)
        };

        eprintln!("Taking snapshot B...");
        let snap_b = hematite::agent::report_export::generate_inspect_output(topics_csv).await;
        let ts_b = ts(now());

        // Apply --field filter to both snapshots before diffing so the diff only
        // shows lines matching the pattern (e.g. --diff resource_load --field cpu).
        let snap_a_f = apply_field_filter(&snap_a, cockpit.field.as_deref());
        let snap_b_f = apply_field_filter(&snap_b, cockpit.field.as_deref());

        let diff_fmt = cockpit.report_format.trim().to_ascii_lowercase();

        if diff_fmt == "json" {
            use similar::{ChangeTag, TextDiff};
            let diff = TextDiff::from_lines(snap_a_f.as_ref(), snap_b_f.as_ref());
            let mut diff_lines: Vec<String> = Vec::new();
            let mut changed = false;
            for group in diff.grouped_ops(2) {
                for op in &group {
                    for change in diff.iter_changes(op) {
                        let prefix = match change.tag() {
                            ChangeTag::Delete => {
                                changed = true;
                                "-"
                            }
                            ChangeTag::Insert => {
                                changed = true;
                                "+"
                            }
                            ChangeTag::Equal => " ",
                        };
                        diff_lines.push(format!("{}{}", prefix, change));
                    }
                }
            }
            let obj = serde_json::json!({
                "topics": topics_csv.as_str(),
                "snapshot_a": ts_a,
                "snapshot_b": ts_b,
                "changed": changed,
                "diff_lines": diff_lines,
                "before": snap_a_f.as_ref(),
                "after": snap_b_f.as_ref(),
            });
            let out = serde_json::to_string_pretty(&obj)
                .unwrap_or_else(|e| format!("{{\"error\": \"{}\"}}", e));
            if let Some(ref out_path) = cockpit.output {
                write_output_copy(&out, out_path);
            } else {
                println!("{}", out);
            }
            if cockpit.clipboard {
                copy_to_clipboard(&out);
                println!("Copied to clipboard.");
            }
        } else {
            println!("--- Snapshot A  ({})", ts_a);
            println!("+++ Snapshot B  ({})", ts_b);
            println!();

            use similar::{ChangeTag, TextDiff};
            let diff = TextDiff::from_lines(snap_a_f.as_ref(), snap_b_f.as_ref());
            let mut changed = false;
            for group in diff.grouped_ops(2) {
                for op in &group {
                    for change in diff.iter_changes(op) {
                        match change.tag() {
                            ChangeTag::Delete => {
                                print!("\x1B[31m- {}\x1B[0m", change);
                                changed = true;
                            }
                            ChangeTag::Insert => {
                                print!("\x1B[32m+ {}\x1B[0m", change);
                                changed = true;
                            }
                            ChangeTag::Equal => {
                                print!("  {}", change);
                            }
                        }
                    }
                }
                println!();
            }

            if !changed {
                println!("No changes detected between snapshots.");
            }
        }
        return Ok(());
    }

    if let Some(ref topics_csv) = cockpit.inspect {
        let fmt = cockpit.report_format.trim().to_ascii_lowercase();

        if let Some(ref snap_name) = cockpit.snapshot {
            // Snapshots are always plain text (for later --diff --from comparisons).
            let raw_content =
                hematite::agent::report_export::generate_inspect_output(topics_csv).await;
            let content = apply_field_filter(&raw_content, cockpit.field.as_deref());
            let snap_path = snapshot_path(snap_name);
            if let Some(parent) = snap_path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            match std::fs::write(&snap_path, content.as_ref()) {
                Ok(()) => println!("Snapshot saved: {}", snap_path.display()),
                Err(e) => eprintln!("Failed to save snapshot: {}", e),
            }
        } else if cockpit.open || fmt == "html" {
            // HTML or --open: save to file and optionally launch.
            let (_, path) =
                hematite::agent::report_export::run_inspect_topics(topics_csv, &fmt, true).await;
            if let Some(p) = path {
                println!("Inspect report saved: {}", p.display());
                if cockpit.open {
                    open_path(&p);
                }
            }
        } else {
            // Default (md/txt) and JSON: generate and print to stdout.
            let raw_content = if fmt == "json" {
                hematite::agent::report_export::generate_inspect_output_json(topics_csv).await
            } else {
                hematite::agent::report_export::generate_inspect_output(topics_csv).await
            };
            let content = apply_field_filter(&raw_content, cockpit.field.as_deref());
            if let Some(ref out_path) = cockpit.output {
                write_output_copy(&content, out_path);
            } else {
                print!("{}", content);
            }
            if cockpit.clipboard {
                copy_to_clipboard(&content);
                println!("Copied to clipboard.");
            }
            if cockpit.notify {
                show_toast("Hematite Inspect", &format!("Done: {}", topics_csv));
            }
        }
        return Ok(());
    }

    if let Some(ref query) = cockpit.query {
        let fmt = cockpit.report_format.trim().to_ascii_lowercase();
        let content = if fmt == "json" {
            hematite::agent::report_export::generate_query_output_json(query).await
        } else {
            hematite::agent::report_export::generate_query_output(query).await
        };
        let filtered = apply_field_filter(&content, cockpit.field.as_deref());
        if let Some(ref out_path) = cockpit.output {
            write_output_copy(&filtered, out_path);
        } else {
            print!("{}", filtered);
        }
        if cockpit.clipboard {
            copy_to_clipboard(&filtered);
            println!("Copied to clipboard.");
        }
        if cockpit.notify {
            show_toast("Hematite Query", &format!("Done: {}", query.trim()));
        }
        return Ok(());
    }

    if let Some(ref expr) = cockpit.compute {
        match hematite::tools::scientific::compute_expr(expr.trim()).await {
            Ok(result) => println!("{}", result.trim()),
            Err(e) => {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
        return Ok(());
    }

    if let Some(ref expr) = cockpit.convert {
        let result = hematite::tools::math_util::unit_convert(expr.trim());
        println!("{}", result.trim());
        if cockpit.clipboard {
            copy_to_clipboard(&result);
            println!("Copied to clipboard.");
        }
        return Ok(());
    }

    if let Some(ref query) = cockpit.periodic {
        match hematite::tools::scientific::lookup_element(query.trim()) {
            Ok(result) => println!("{}", result),
            Err(e) => {
                eprintln!("{}", e);
                std::process::exit(1);
            }
        }
        return Ok(());
    }

    if let Some(ref target) = cockpit.hash {
        let algo = cockpit.hash_algo.as_deref().unwrap_or("all");
        match hematite::tools::scientific::hash_input(target.trim(), algo).await {
            Ok(result) => {
                println!("{}", result.trim());
                if cockpit.clipboard {
                    copy_to_clipboard(&result);
                    println!("Copied to clipboard.");
                }
            }
            Err(e) => {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
        return Ok(());
    }

    if let Some(ref text) = cockpit.encode {
        let codec = cockpit.codec.as_deref().unwrap_or("base64");
        match hematite::tools::scientific::encode_decode(text, codec, false).await {
            Ok(result) => println!("{}", result.trim()),
            Err(e) => {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
        return Ok(());
    }

    if let Some(ref text) = cockpit.decode {
        let codec = cockpit.codec.as_deref().unwrap_or("base64");
        match hematite::tools::scientific::encode_decode(text, codec, true).await {
            Ok(result) => println!("{}", result.trim()),
            Err(e) => {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
        return Ok(());
    }

    if let Some(ref query) = cockpit.formula {
        println!(
            "{}",
            hematite::tools::scientific_ext::search_formulas(query.trim())
        );
        return Ok(());
    }

    if let Some(ref kind) = cockpit.random {
        let length = cockpit.length.unwrap_or(match kind.trim() {
            "password" | "pwd" | "pass" => 20,
            "pin" => 6,
            "bytes" => 16,
            _ => 32,
        });
        let count = cockpit.count.unwrap_or(1) as usize;
        let extra = cockpit.random_args.as_deref().unwrap_or("");
        match hematite::tools::scientific_ext::generate_random(kind.trim(), length, count, extra)
            .await
        {
            Ok(result) => {
                println!("{}", result.trim());
                if cockpit.clipboard {
                    copy_to_clipboard(&result);
                    println!("Copied to clipboard.");
                }
            }
            Err(e) => {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
        return Ok(());
    }

    if let Some(ref pair) = cockpit.diff_data {
        let parts: Vec<&str> = pair.splitn(2, ',').collect();
        if parts.len() < 2 {
            eprintln!(
                "Error: --diff-data requires two comma-separated file paths.\n\
                 Example: hematite --diff-data before.csv,after.csv"
            );
            std::process::exit(1);
        }
        let path_a = parts[0].trim();
        let path_b = parts[1].trim();
        let key_col = cockpit.diff_key.as_deref().unwrap_or("");
        eprintln!("Comparing {} → {}...", path_a, path_b);
        match hematite::tools::scientific_ext::diff_data(path_a, path_b, key_col).await {
            Ok(result) => {
                println!("{}", result.trim());
                if cockpit.clipboard {
                    copy_to_clipboard(&result);
                    println!("Copied to clipboard.");
                }
            }
            Err(e) => {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
        return Ok(());
    }

    if let Some(ref file_path) = cockpit.describe {
        let cols = cockpit.column.as_deref().unwrap_or("");
        let output = "";
        match hematite::tools::data_tools::describe_stats(file_path.trim(), cols, output).await {
            Ok(result) => {
                println!("{}", result.trim());
                if cockpit.clipboard {
                    copy_to_clipboard(&result);
                    println!("Copied to clipboard.");
                }
            }
            Err(e) => {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
        return Ok(());
    }

    if let Some(ref op) = cockpit.matrix {
        // New unified format: --matrix 'det [[1,2],[3,4]]'
        // Legacy format: --matrix det --matrix-a '[[1,2],[3,4]]' [--matrix-b '[[5],[6]]']
        let query = if cockpit.matrix_a.is_some() {
            let mat_a = cockpit.matrix_a.as_deref().unwrap_or("").trim().to_string();
            let mat_b = cockpit.matrix_b.as_deref().unwrap_or("").trim().to_string();
            if mat_b.is_empty() {
                format!("{} {}", op.trim(), mat_a)
            } else {
                format!("{} {} {}", op.trim(), mat_a, mat_b)
            }
        } else {
            op.trim().to_string()
        };
        let result = hematite::tools::math_util::matrix_calc(&query);
        println!("{}", result.trim());
        if cockpit.clipboard {
            copy_to_clipboard(&result);
            println!("Copied to clipboard.");
        }
        return Ok(());
    }

    if let Some(ref eq) = cockpit.solve {
        let var = cockpit.solve_var.as_deref().unwrap_or("x");
        let (x0, x1) = cockpit
            .solve_range
            .as_deref()
            .and_then(|s| {
                let mut it = s.splitn(2, ',');
                let lo = it.next()?.trim().parse::<f64>().ok()?;
                let hi = it.next()?.trim().parse::<f64>().ok()?;
                Some((lo, hi))
            })
            .unwrap_or((-1000.0, 1000.0));
        match hematite::tools::scientific_ext::solve_equation(eq.trim(), var, x0, x1).await {
            Ok(result) => {
                println!("{}", result.trim());
                if cockpit.clipboard {
                    copy_to_clipboard(&result);
                    println!("Copied to clipboard.");
                }
            }
            Err(e) => {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
        return Ok(());
    }

    if let Some(ref file_path) = cockpit.curve_fit {
        let x_col = cockpit.fit_x.as_deref().unwrap_or("");
        let y_col = cockpit.fit_y.as_deref().unwrap_or("");
        let model = cockpit.fit_model.as_deref().unwrap_or("auto");
        match hematite::tools::scientific_ext::curve_fit(file_path.trim(), x_col, y_col, model)
            .await
        {
            Ok(result) => {
                println!("{}", result.trim());
                if cockpit.clipboard {
                    copy_to_clipboard(&result);
                    println!("Copied to clipboard.");
                }
            }
            Err(e) => {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
        return Ok(());
    }

    if let Some(ref expr) = cockpit.integrate {
        let var = cockpit.int_var.as_deref().unwrap_or("x");
        let n = cockpit.int_n.unwrap_or(1000);
        let parse_bound = |s: Option<&str>| -> Option<f64> {
            let s = s?.trim();
            // allow "pi", "e", "2*pi", etc via simple substitutions
            let s = s
                .replace("pi", &std::f64::consts::PI.to_string())
                .replace('e', &std::f64::consts::E.to_string());
            s.parse::<f64>().ok()
        };
        let lo = parse_bound(cockpit.int_from.as_deref()).unwrap_or(0.0);
        let hi = parse_bound(cockpit.int_to.as_deref()).unwrap_or(1.0);
        match hematite::tools::scientific_ext::integrate(expr.trim(), var, lo, hi, n).await {
            Ok(result) => {
                println!("{}", result.trim());
                if cockpit.clipboard {
                    copy_to_clipboard(&result);
                    println!("Copied to clipboard.");
                }
            }
            Err(e) => {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
        return Ok(());
    }

    if let Some(ref expr) = cockpit.differentiate {
        let var = cockpit.int_var.as_deref().unwrap_or("x");
        let order = cockpit.order.unwrap_or(1);
        let at_str = cockpit.at.as_deref().unwrap_or("0");
        let at = at_str
            .trim()
            .replace("pi", &std::f64::consts::PI.to_string())
            .replace('e', &std::f64::consts::E.to_string())
            .parse::<f64>()
            .unwrap_or_else(|_| {
                eprintln!(
                    "Error: --at must be a number (got '{}'). Use --at 3.14 or --at pi",
                    at_str
                );
                std::process::exit(1);
            });
        match hematite::tools::scientific_ext::differentiate(expr.trim(), var, at, order).await {
            Ok(result) => {
                println!("{}", result.trim());
                if cockpit.clipboard {
                    copy_to_clipboard(&result);
                    println!("Copied to clipboard.");
                }
            }
            Err(e) => {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
        return Ok(());
    }

    if let Some(ref file_path) = cockpit.profile {
        match hematite::tools::scientific_ext::data_profile(file_path.trim()).await {
            Ok(result) => {
                println!("{}", result.trim());
                if cockpit.clipboard {
                    copy_to_clipboard(&result);
                    println!("Copied to clipboard.");
                }
            }
            Err(e) => {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
        return Ok(());
    }

    if let Some(n) = cockpit.prime {
        let result = hematite::tools::math_util::prime_info(n);
        println!("{}", result.trim());
        if cockpit.clipboard {
            copy_to_clipboard(&result);
            println!("Copied to clipboard.");
        }
        return Ok(());
    }

    if let Some(ref kind) = cockpit.sequence {
        let count = cockpit.seq_count.unwrap_or(10);
        let start = cockpit.seq_start.unwrap_or(1.0);
        let step = cockpit.seq_step.unwrap_or(1.0);
        let result = hematite::tools::math_util::generate_sequence(kind.trim(), count, start, step);
        println!("{}", result.trim());
        if cockpit.clipboard {
            copy_to_clipboard(&result);
            println!("Copied to clipboard.");
        }
        return Ok(());
    }

    if let Some(ref nk) = cockpit.choose {
        let parts: Vec<&str> = nk.split([',', ' ']).filter(|s| !s.is_empty()).collect();
        if parts.len() < 2 {
            eprintln!("Error: --choose requires two integers N and K (e.g. --choose '10 3' or --choose '10,3')");
            std::process::exit(1);
        }
        let n = parts[0].trim().parse::<u64>().unwrap_or_else(|_| {
            eprintln!("Invalid N: {}", parts[0]);
            std::process::exit(1);
        });
        let k = parts[1].trim().parse::<u64>().unwrap_or_else(|_| {
            eprintln!("Invalid K: {}", parts[1]);
            std::process::exit(1);
        });
        let result = hematite::tools::math_util::combinatorics(n, k);
        println!("{}", result.trim());
        if cockpit.clipboard {
            copy_to_clipboard(&result);
            println!("Copied to clipboard.");
        }
        return Ok(());
    }

    if let Some(ref expr) = cockpit.truth_table {
        let result = hematite::tools::math_util::truth_table(expr.trim());
        println!("{}", result.trim());
        if cockpit.clipboard {
            copy_to_clipboard(&result);
            println!("Copied to clipboard.");
        }
        return Ok(());
    }

    if let Some(ref nk) = cockpit.gcd {
        let parts: Vec<&str> = nk.split([',', ' ']).filter(|s| !s.is_empty()).collect();
        if parts.len() < 2 {
            eprintln!("Error: --gcd requires two integers (e.g. --gcd '48,18' or --gcd '48 18')");
            std::process::exit(1);
        }
        let a = parts[0].trim().parse::<u128>().unwrap_or_else(|_| {
            eprintln!("Invalid A: {}", parts[0]);
            std::process::exit(1);
        });
        let b = parts[1].trim().parse::<u128>().unwrap_or_else(|_| {
            eprintln!("Invalid B: {}", parts[1]);
            std::process::exit(1);
        });
        let result = hematite::tools::math_util::gcd_lcm(a, b);
        println!("{}", result.trim());
        if cockpit.clipboard {
            copy_to_clipboard(&result);
            println!("Copied to clipboard.");
        }
        return Ok(());
    }

    if let Some(ref input) = cockpit.roman {
        let result = hematite::tools::math_util::roman_info(input.trim());
        println!("{}", result.trim());
        if cockpit.clipboard {
            copy_to_clipboard(&result);
            println!("Copied to clipboard.");
        }
        return Ok(());
    }

    if let Some(ref input) = cockpit.base_convert {
        let from = cockpit.base_from.unwrap_or(10);
        let to = cockpit.base_to.unwrap_or(2);
        let result = hematite::tools::math_util::base_convert(input.trim(), from, to);
        println!("{}", result.trim());
        if cockpit.clipboard {
            copy_to_clipboard(&result);
            println!("Copied to clipboard.");
        }
        return Ok(());
    }

    if let Some(ref input) = cockpit.date {
        let result = hematite::tools::math_util::date_calc(input.trim());
        println!("{}", result.trim());
        if cockpit.clipboard {
            copy_to_clipboard(&result);
            println!("Copied to clipboard.");
        }
        return Ok(());
    }

    if let Some(ref cidr) = cockpit.subnet {
        let result = hematite::tools::math_util::subnet_calc(cidr.trim());
        println!("{}", result.trim());
        if cockpit.clipboard {
            copy_to_clipboard(&result);
            println!("Copied to clipboard.");
        }
        return Ok(());
    }

    if let Some(ref input) = cockpit.color {
        let result = hematite::tools::math_util::color_convert(input.trim());
        println!("{}", result.trim());
        if cockpit.clipboard {
            copy_to_clipboard(&result);
            println!("Copied to clipboard.");
        }
        return Ok(());
    }

    if let Some(ref formula) = cockpit.mw {
        let result = hematite::tools::math_util::molecular_weight(formula.trim());
        println!("{}", result.trim());
        if cockpit.clipboard {
            copy_to_clipboard(&result);
            println!("Copied to clipboard.");
        }
        return Ok(());
    }

    if let Some(ref query) = cockpit.r#const {
        let result = hematite::tools::math_util::physical_const(query.trim());
        println!("{}", result.trim());
        if cockpit.clipboard {
            copy_to_clipboard(&result);
            println!("Copied to clipboard.");
        }
        return Ok(());
    }

    if let Some(ref query) = cockpit.normal {
        let result = hematite::tools::math_util::stat_normal(query.trim());
        println!("{}", result.trim());
        if cockpit.clipboard {
            copy_to_clipboard(&result);
            println!("Copied to clipboard.");
        }
        return Ok(());
    }

    if let Some(ref expr) = cockpit.vectors {
        let result = hematite::tools::math_util::vector_calc(expr.trim());
        println!("{}", result.trim());
        if cockpit.clipboard {
            copy_to_clipboard(&result);
            println!("Copied to clipboard.");
        }
        return Ok(());
    }

    if let Some(ref query) = cockpit.number_theory {
        let result = hematite::tools::math_util::number_theory(query.trim());
        println!("{}", result.trim());
        if cockpit.clipboard {
            copy_to_clipboard(&result);
            println!("Copied to clipboard.");
        }
        return Ok(());
    }

    if let Some(ref query) = cockpit.simulate {
        let result = hematite::tools::math_util::simulate(query.trim());
        println!("{}", result.trim());
        if cockpit.clipboard {
            copy_to_clipboard(&result);
            println!("Copied to clipboard.");
        }
        return Ok(());
    }

    if let Some(ref query) = cockpit.graph {
        let result = hematite::tools::math_util::graph_theory(query.trim());
        println!("{}", result.trim());
        if cockpit.clipboard {
            copy_to_clipboard(&result);
            println!("Copied to clipboard.");
        }
        return Ok(());
    }

    if let Some(ref query) = cockpit.symbolic {
        let result = hematite::tools::math_util::symbolic_calc(query.trim());
        println!("{}", result.trim());
        if cockpit.clipboard {
            copy_to_clipboard(&result);
            println!("Copied to clipboard.");
        }
        return Ok(());
    }

    if let Some(ref query) = cockpit.finance {
        let result = hematite::tools::math_util::finance_calc(query.trim());
        println!("{}", result.trim());
        if cockpit.clipboard {
            copy_to_clipboard(&result);
            println!("Copied to clipboard.");
        }
        return Ok(());
    }

    if let Some(ref query) = cockpit.logic {
        let result = hematite::tools::math_util::logic_calc(query.trim());
        println!("{}", result.trim());
        if cockpit.clipboard {
            copy_to_clipboard(&result);
            println!("Copied to clipboard.");
        }
        return Ok(());
    }

    if let Some(ref query) = cockpit.signal {
        let result = hematite::tools::math_util::signal_calc(query.trim());
        println!("{}", result.trim());
        if cockpit.clipboard {
            copy_to_clipboard(&result);
            println!("Copied to clipboard.");
        }
        return Ok(());
    }

    if let Some(ref query) = cockpit.interpolate {
        let result = hematite::tools::math_util::interpolate_calc(query.trim());
        println!("{}", result.trim());
        if cockpit.clipboard {
            copy_to_clipboard(&result);
            println!("Copied to clipboard.");
        }
        return Ok(());
    }

    if let Some(ref query) = cockpit.units {
        let result = hematite::tools::math_util::units_calc(query.trim());
        println!("{}", result.trim());
        if cockpit.clipboard {
            copy_to_clipboard(&result);
            println!("Copied to clipboard.");
        }
        return Ok(());
    }

    if let Some(ref query) = cockpit.ode {
        let result = hematite::tools::math_util::ode_solve(query.trim());
        println!("{}", result.trim());
        if cockpit.clipboard {
            copy_to_clipboard(&result);
            println!("Copied to clipboard.");
        }
        return Ok(());
    }

    if let Some(ref query) = cockpit.optimize {
        let result = hematite::tools::math_util::optimize_calc(query.trim());
        println!("{}", result.trim());
        if cockpit.clipboard {
            copy_to_clipboard(&result);
            println!("Copied to clipboard.");
        }
        return Ok(());
    }

    if let Some(ref group1) = cockpit.hypothesis {
        let test_type = cockpit.hypothesis_test.as_deref().unwrap_or("one-t");
        let group2 = cockpit.hypothesis_group2.as_deref().unwrap_or("");
        let alpha = cockpit.hypothesis_alpha.unwrap_or(0.05);
        let mu = cockpit.hypothesis_mu.unwrap_or(0.0);
        match hematite::tools::data_tools::hypothesis_test(
            test_type,
            group1.trim(),
            group2.trim(),
            alpha,
            mu,
        )
        .await
        {
            Ok(result) => {
                println!("{}", result.trim());
                if cockpit.clipboard {
                    copy_to_clipboard(&result);
                    println!("Copied to clipboard.");
                }
            }
            Err(e) => eprintln!("hypothesis-test error: {}", e),
        }
        return Ok(());
    }

    if let Some(ref file_path) = cockpit.classify {
        let label = cockpit.classify_label.as_deref().unwrap_or("");
        let feats = cockpit.classify_cols.as_deref().unwrap_or("");
        let predict = cockpit.classify_predict.as_deref().unwrap_or("");
        let k = cockpit.classify_k.unwrap_or(3);
        let method = cockpit.classify_method.as_deref().unwrap_or("knn");
        match hematite::tools::data_tools::classify_data(
            file_path.trim(),
            label,
            feats,
            predict,
            k,
            method,
        )
        .await
        {
            Ok(result) => {
                println!("{}", result.trim());
                if cockpit.clipboard {
                    copy_to_clipboard(&result);
                    println!("Copied to clipboard.");
                }
            }
            Err(e) => eprintln!("classify error: {}", e),
        }
        return Ok(());
    }

    if let Some(ref file_path) = cockpit.polyfit {
        let x_col = cockpit.polyfit_x.as_deref().unwrap_or("");
        let y_col = cockpit.polyfit_y.as_deref().unwrap_or("");
        let degree = cockpit.polyfit_degree.unwrap_or(1);
        let predict = cockpit.polyfit_predict.as_deref().unwrap_or("");
        match hematite::tools::data_tools::regression_analysis(
            file_path.trim(),
            x_col,
            y_col,
            degree,
            predict,
        )
        .await
        {
            Ok(result) => {
                println!("{}", result.trim());
                if cockpit.clipboard {
                    copy_to_clipboard(&result);
                    println!("Copied to clipboard.");
                }
            }
            Err(e) => eprintln!("polyfit error: {}", e),
        }
        return Ok(());
    }

    if let Some(ref query) = cockpit.probability {
        let result = hematite::tools::math_util::prob_calc(query.trim());
        println!("{}", result.trim());
        if cockpit.clipboard {
            copy_to_clipboard(&result);
            println!("Copied to clipboard.");
        }
        return Ok(());
    }

    if let Some(ref query) = cockpit.bitwise {
        let result = hematite::tools::math_util::bitwise_calc(query.trim());
        println!("{}", result.trim());
        if cockpit.clipboard {
            copy_to_clipboard(&result);
            println!("Copied to clipboard.");
        }
        return Ok(());
    }

    if let Some(ref query) = cockpit.set {
        let result = hematite::tools::math_util::set_calc(query.trim());
        println!("{}", result.trim());
        if cockpit.clipboard {
            copy_to_clipboard(&result);
            println!("Copied to clipboard.");
        }
        return Ok(());
    }

    if let Some(ref query) = cockpit.cipher {
        let result = hematite::tools::math_util::cipher_calc(query.trim());
        println!("{}", result.trim());
        if cockpit.clipboard {
            copy_to_clipboard(&result);
            println!("Copied to clipboard.");
        }
        return Ok(());
    }

    if let Some(ref text) = cockpit.text_stats {
        let result = hematite::tools::math_util::text_stats(text.trim());
        println!("{}", result.trim());
        if cockpit.clipboard {
            copy_to_clipboard(&result);
            println!("Copied to clipboard.");
        }
        return Ok(());
    }

    if let Some(ref query) = cockpit.levenshtein {
        let result = hematite::tools::math_util::string_dist(query.trim());
        println!("{}", result.trim());
        if cockpit.clipboard {
            copy_to_clipboard(&result);
            println!("Copied to clipboard.");
        }
        return Ok(());
    }

    if let Some(ref query) = cockpit.number_format {
        let result = hematite::tools::math_util::number_format(query.trim());
        println!("{}", result.trim());
        if cockpit.clipboard {
            copy_to_clipboard(&result);
            println!("Copied to clipboard.");
        }
        return Ok(());
    }

    if let Some(ref query) = cockpit.sort_viz {
        let result = hematite::tools::math_util::sort_viz(query.trim());
        println!("{}", result.trim());
        if cockpit.clipboard {
            copy_to_clipboard(&result);
            println!("Copied to clipboard.");
        }
        return Ok(());
    }

    if let Some(ref text) = cockpit.checksum {
        let result = hematite::tools::math_util::checksum_calc(text.trim());
        println!("{}", result.trim());
        if cockpit.clipboard {
            copy_to_clipboard(&result);
            println!("Copied to clipboard.");
        }
        return Ok(());
    }

    if let Some(ref text) = cockpit.validate {
        let result = hematite::tools::math_util::validate_calc(text.trim());
        println!("{}", result.trim());
        if cockpit.clipboard {
            copy_to_clipboard(&result);
            println!("Copied to clipboard.");
        }
        return Ok(());
    }

    if let Some(ref numbers) = cockpit.dstats {
        let result = hematite::tools::math_util::stats_calc(numbers.trim());
        println!("{}", result.trim());
        if cockpit.clipboard {
            copy_to_clipboard(&result);
            println!("Copied to clipboard.");
        }
        return Ok(());
    }

    if let Some(ref query) = cockpit.geometry {
        let result = hematite::tools::math_util::geometry_calc(query.trim());
        println!("{}", result.trim());
        if cockpit.clipboard {
            copy_to_clipboard(&result);
            println!("Copied to clipboard.");
        }
        return Ok(());
    }

    if let Some(ref query) = cockpit.electrical {
        let result = hematite::tools::math_util::electrical_calc(query.trim());
        println!("{}", result.trim());
        if cockpit.clipboard {
            copy_to_clipboard(&result);
            println!("Copied to clipboard.");
        }
        return Ok(());
    }

    if let Some(ref query) = cockpit.physics {
        let result = hematite::tools::math_util::physics_calc(query.trim());
        println!("{}", result.trim());
        if cockpit.clipboard {
            copy_to_clipboard(&result);
            println!("Copied to clipboard.");
        }
        return Ok(());
    }

    if let Some(ref query) = cockpit.chemistry {
        let result = hematite::tools::math_util::chemistry_calc(query.trim());
        println!("{}", result.trim());
        if cockpit.clipboard {
            copy_to_clipboard(&result);
            println!("Copied to clipboard.");
        }
        return Ok(());
    }

    if let Some(ref query) = cockpit.combinatorics {
        let result = hematite::tools::math_util::combinatorics_calc(query.trim());
        println!("{}", result.trim());
        if cockpit.clipboard {
            copy_to_clipboard(&result);
            println!("Copied to clipboard.");
        }
        return Ok(());
    }

    if let Some(ref file_path) = cockpit.fourier {
        let col = cockpit.fourier_col.as_deref().unwrap_or("");
        let top_n = cockpit.fourier_top.unwrap_or(10);
        let sample_rate = cockpit.fourier_rate.unwrap_or(1.0);
        match hematite::tools::data_tools::fourier_analysis(
            file_path.trim(),
            col,
            top_n,
            sample_rate,
        )
        .await
        {
            Ok(result) => {
                println!("{}", result.trim());
                if cockpit.clipboard {
                    copy_to_clipboard(&result);
                    println!("Copied to clipboard.");
                }
            }
            Err(e) => eprintln!("Error: {}", e),
        }
        return Ok(());
    }

    if let Some(ref file_path) = cockpit.cluster {
        let k = cockpit.cluster_k.unwrap_or(3);
        let cols_str = cockpit.cluster_cols.as_deref().unwrap_or("");
        let cols: Vec<&str> = if cols_str.is_empty() {
            vec![]
        } else {
            cols_str.split(',').map(str::trim).collect()
        };
        let output = cockpit.cluster_output.as_deref().unwrap_or("");
        match hematite::tools::data_tools::cluster_kmeans(file_path.trim(), k, &cols, 300, output)
            .await
        {
            Ok(result) => {
                println!("{}", result.trim());
                if cockpit.clipboard {
                    copy_to_clipboard(&result);
                    println!("Copied to clipboard.");
                }
            }
            Err(e) => eprintln!("Error: {}", e),
        }
        return Ok(());
    }

    if let Some(ref file_path) = cockpit.normalize {
        let method = cockpit.normalize_method.as_deref().unwrap_or("minmax");
        let cols_str = cockpit.normalize_cols.as_deref().unwrap_or("");
        let cols: Vec<&str> = if cols_str.is_empty() {
            vec![]
        } else {
            cols_str.split(',').map(str::trim).collect()
        };
        let output = cockpit.normalize_output.as_deref().unwrap_or("");
        match hematite::tools::data_tools::normalize_dataset(
            file_path.trim(),
            method,
            &cols,
            output,
        )
        .await
        {
            Ok(result) => {
                println!("{}", result.trim());
                if cockpit.clipboard {
                    copy_to_clipboard(&result);
                    println!("Copied to clipboard.");
                }
            }
            Err(e) => eprintln!("Error: {}", e),
        }
        return Ok(());
    }

    if let Some(ref file_path) = cockpit.pca {
        let n_comp = cockpit.pca_components.unwrap_or(3);
        let cols_str = cockpit.pca_cols.as_deref().unwrap_or("");
        let cols: Vec<&str> = if cols_str.is_empty() {
            vec![]
        } else {
            cols_str.split(',').map(str::trim).collect()
        };
        let output = cockpit.pca_output.as_deref().unwrap_or("");
        match hematite::tools::data_tools::pca_analyze(file_path.trim(), n_comp, &cols, output)
            .await
        {
            Ok(result) => {
                println!("{}", result.trim());
                if cockpit.clipboard {
                    copy_to_clipboard(&result);
                    println!("Copied to clipboard.");
                }
            }
            Err(e) => eprintln!("Error: {}", e),
        }
        return Ok(());
    }

    if let Some(ref file_path) = cockpit.percentile {
        let col = cockpit.percentile_col.as_deref().unwrap_or("");
        match hematite::tools::data_tools::percentile_report(file_path.trim(), col).await {
            Ok(result) => {
                println!("{}", result.trim());
                if cockpit.clipboard {
                    copy_to_clipboard(&result);
                    println!("Copied to clipboard.");
                }
            }
            Err(e) => eprintln!("Error: {}", e),
        }
        return Ok(());
    }

    if let Some(ref file_path) = cockpit.pivot {
        let row_col = cockpit.pivot_row.as_deref().unwrap_or("");
        let col_col = cockpit.pivot_col.as_deref().unwrap_or("");
        let val_col = cockpit.pivot_val.as_deref().unwrap_or("");
        let agg = cockpit.pivot_agg.as_deref().unwrap_or("count");
        match hematite::tools::data_tools::pivot_table(
            file_path.trim(),
            row_col,
            col_col,
            val_col,
            agg,
        )
        .await
        {
            Ok(result) => {
                println!("{}", result.trim());
                if cockpit.clipboard {
                    copy_to_clipboard(&result);
                    println!("Copied to clipboard.");
                }
            }
            Err(e) => eprintln!("Error: {}", e),
        }
        return Ok(());
    }

    if let Some(ref file_path) = cockpit.regression {
        let target = cockpit.regression_target.as_deref().unwrap_or("");
        let preds_str = cockpit.regression_predictors.as_deref().unwrap_or("");
        let preds: Vec<&str> = if preds_str.is_empty() {
            vec![]
        } else {
            preds_str.split(',').map(str::trim).collect()
        };
        match hematite::tools::data_tools::linear_regression(file_path.trim(), &preds, target).await
        {
            Ok(result) => {
                println!("{}", result.trim());
                if cockpit.clipboard {
                    copy_to_clipboard(&result);
                    println!("Copied to clipboard.");
                }
            }
            Err(e) => eprintln!("Error: {}", e),
        }
        return Ok(());
    }

    if let Some(ref file_path) = cockpit.outliers {
        let col = cockpit.outlier_col.as_deref().unwrap_or("");
        let output = cockpit.outlier_output.as_deref().unwrap_or("");
        match hematite::tools::data_tools::detect_outliers(file_path.trim(), col, output).await {
            Ok(result) => {
                println!("{}", result.trim());
                if cockpit.clipboard {
                    copy_to_clipboard(&result);
                    println!("Copied to clipboard.");
                }
            }
            Err(e) => eprintln!("Error: {}", e),
        }
        return Ok(());
    }

    if let Some(ref file_path) = cockpit.plot {
        let x_col = cockpit.plot_x.as_deref().unwrap_or("");
        let y_col = cockpit.plot_y.as_deref().unwrap_or("");
        let chart_type = cockpit.plot_type.as_deref().unwrap_or("line");
        let title = cockpit.plot_title.as_deref().unwrap_or("");
        let output = cockpit.plot_output.as_deref().unwrap_or("");
        match hematite::tools::data_tools::plot_chart(
            file_path.trim(),
            x_col,
            y_col,
            chart_type,
            title,
            output,
        )
        .await
        {
            Ok(result) => {
                println!("{}", result.trim());
                // Auto-open if --open flag set
                if cockpit.open {
                    let svg_path = if output.is_empty() {
                        let base = std::path::Path::new(file_path.trim())
                            .with_extension("")
                            .to_string_lossy()
                            .to_string();
                        format!("{}_plot.svg", base)
                    } else {
                        output.to_string()
                    };
                    open_path(std::path::Path::new(&svg_path));
                }
                if cockpit.clipboard {
                    copy_to_clipboard(&result);
                    println!("Copied to clipboard.");
                }
            }
            Err(e) => eprintln!("Error: {}", e),
        }
        return Ok(());
    }

    if let Some(ref file_path) = cockpit.sample {
        let n = cockpit.sample_n.unwrap_or(0);
        let frac = cockpit.sample_frac.unwrap_or(0.0);
        let seed = cockpit.sample_seed.unwrap_or(42);
        let split = cockpit.split.unwrap_or(0.0);
        let out_dir = cockpit.sample_output.as_deref().unwrap_or("");
        match hematite::tools::data_tools::sample_data(
            file_path.trim(),
            n,
            frac,
            seed,
            split,
            out_dir,
        )
        .await
        {
            Ok(result) => {
                println!("{}", result.trim());
                if cockpit.clipboard {
                    copy_to_clipboard(&result);
                    println!("Copied to clipboard.");
                }
            }
            Err(e) => eprintln!("Error: {}", e),
        }
        return Ok(());
    }

    if let Some(ref file_path) = cockpit.correlation {
        let method = cockpit.corr_method.as_deref().unwrap_or("pearson");
        match hematite::tools::data_tools::correlation_matrix(file_path.trim(), method).await {
            Ok(result) => {
                println!("{}", result.trim());
                if cockpit.clipboard {
                    copy_to_clipboard(&result);
                    println!("Copied to clipboard.");
                }
            }
            Err(e) => eprintln!("Error: {}", e),
        }
        return Ok(());
    }

    if let Some(ref file_path) = cockpit.timeseries {
        let date_col = cockpit.ts_date.as_deref().unwrap_or("");
        let val_col = cockpit.ts_value.as_deref().unwrap_or("");
        let window = cockpit.ts_window.unwrap_or(7);
        match hematite::tools::data_tools::timeseries_analyze(
            file_path.trim(),
            date_col,
            val_col,
            window,
        )
        .await
        {
            Ok(result) => {
                println!("{}", result.trim());
                if cockpit.clipboard {
                    copy_to_clipboard(&result);
                    println!("Copied to clipboard.");
                }
            }
            Err(e) => eprintln!("Error: {}", e),
        }
        return Ok(());
    }

    if let Some(ref file_path) = cockpit.query_data {
        let file_path = file_path.trim();
        let sql = cockpit.sql.as_deref().unwrap_or("").trim().to_string();
        if sql.is_empty() {
            eprintln!(
                "Error: --query-data requires --sql \"SELECT ...\"\n\
                 The file is loaded as a table named 'data'.\n\
                 Example: hematite --query-data employees.csv --sql \"SELECT department, COUNT(*) FROM data GROUP BY department\""
            );
            std::process::exit(1);
        }
        match hematite::tools::scientific::query_data(file_path, &sql).await {
            Ok(result) => {
                println!("{}", result.trim());
                if cockpit.clipboard {
                    copy_to_clipboard(&result);
                    println!("Copied to clipboard.");
                }
            }
            Err(e) => {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
        return Ok(());
    }

    if let Some(ref path_str) = cockpit.analyze {
        let path_str = path_str.trim();
        eprintln!("Analyzing: {}...", path_str);
        let profile = match hematite::tools::scientific::analyze_dataset(path_str).await {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        };

        let ts = hematite::agent::report_export::timestamp_label();
        let safe_ts: String = ts
            .chars()
            .map(|c| {
                if c.is_alphanumeric() || c == '-' {
                    c
                } else {
                    '_'
                }
            })
            .collect();
        let fmt = cockpit.report_format.trim().to_ascii_lowercase();
        let report_dir = hematite::tools::file_ops::hematite_dir().join("reports");
        let _ = std::fs::create_dir_all(&report_dir);

        let (content, report_path) = if fmt == "html" {
            let html = hematite::agent::html_template::build_html_shell(
                &format!("Dataset Analysis: {}", path_str),
                &hematite::hematite_version(),
                &hematite::agent::html_template::markdown_to_html(&profile),
            );
            let p = report_dir.join(format!("analysis-{}.html", safe_ts));
            let _ = std::fs::write(&p, &html);
            (html, p)
        } else {
            let p = report_dir.join(format!("analysis-{}.md", safe_ts));
            let _ = std::fs::write(&p, &profile);
            (profile.clone(), p)
        };

        if let Some(ref out_path) = cockpit.output {
            write_output_copy(&profile, out_path);
        } else if fmt != "html" {
            print!("{}", profile);
        }
        println!("\nAnalysis saved: {}", report_path.display());
        if cockpit.clipboard {
            copy_to_clipboard(&content);
            println!("Copied to clipboard.");
        }
        if cockpit.open {
            open_path(&report_path);
        }
        if cockpit.notify {
            show_toast(
                "Hematite Analyze",
                &format!("Profile complete: {}", path_str),
            );
        }
        return Ok(());
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

/// Filter content to lines matching `pattern` (case-insensitive). Returns the filtered string.
/// If `pattern` is None, returns the full content unchanged.
fn apply_field_filter<'a>(content: &'a str, pattern: Option<&str>) -> std::borrow::Cow<'a, str> {
    match pattern {
        None => std::borrow::Cow::Borrowed(content),
        Some(pat) => {
            let lower_pat = pat.to_ascii_lowercase();
            let filtered: String = content
                .lines()
                .filter(|line| line.to_ascii_lowercase().contains(&lower_pat))
                .collect::<Vec<_>>()
                .join("\n");
            std::borrow::Cow::Owned(if filtered.is_empty() {
                format!("(no lines matched {:?})", pat)
            } else {
                filtered
            })
        }
    }
}

/// Copy report content to a user-specified output path, creating parent dirs if needed.
fn write_output_copy(content: &str, output_path: &str) {
    let path = std::path::Path::new(output_path);
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            let _ = std::fs::create_dir_all(parent);
        }
    }
    match std::fs::write(path, content) {
        Ok(()) => println!("Output written: {}", path.display()),
        Err(e) => eprintln!("Failed to write --output {}: {}", path.display(), e),
    }
}

fn copy_to_clipboard(text: &str) {
    use std::io::Write;
    #[cfg(target_os = "windows")]
    let prog: (&str, Vec<&str>) = ("clip", vec![]);
    #[cfg(target_os = "macos")]
    let prog: (&str, Vec<&str>) = ("pbcopy", vec![]);
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    let prog: (&str, Vec<&str>) = ("xclip", vec!["-selection", "clipboard"]);
    if let Ok(mut child) = std::process::Command::new(prog.0)
        .args(&prog.1)
        .stdin(std::process::Stdio::piped())
        .spawn()
    {
        if let Some(mut stdin) = child.stdin.take() {
            let _ = stdin.write_all(text.as_bytes());
        }
        let _ = child.wait();
    }
}

/// Show a native Windows desktop notification using the WinRT Toast API via
/// PowerShell. No-ops on non-Windows platforms and when PowerShell is absent.
fn show_toast(title: &str, body: &str) {
    #[cfg(target_os = "windows")]
    {
        // Escape single-quotes so they don't break the PowerShell string literals.
        let safe_title = title.replace('\'', "\\'");
        let safe_body = body.replace('\'', "\\'");
        let script = format!(
            "$ErrorActionPreference='SilentlyContinue';\
            [Windows.UI.Notifications.ToastNotificationManager,Windows.UI.Notifications,ContentType=WindowsRuntime]|Out-Null;\
            $t=[Windows.UI.Notifications.ToastNotificationManager]::GetTemplateContent([Windows.UI.Notifications.ToastTemplateType]::ToastText02);\
            $n=$t.GetElementsByTagName('text');\
            $n.Item(0).InnerText='{title}';\
            $n.Item(1).InnerText='{body}';\
            $toast=[Windows.UI.Notifications.ToastNotification]::new($t);\
            [Windows.UI.Notifications.ToastNotificationManager]::CreateToastNotifier('Hematite').Show($toast)",
            title = safe_title,
            body = safe_body,
        );
        let _ = std::process::Command::new("powershell")
            .args(["-NoProfile", "-NonInteractive", "-Command", &script])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn();
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = (title, body);
    }
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
