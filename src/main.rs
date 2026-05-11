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
            if auto_cmds.is_empty() {
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
                                    let verify_out = hematite::agent::report_export::generate_inspect_output(topic).await;
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
            let grade = if has_issues_final { "Issues found" } else { "All clear" };
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
            println!("Available sweep fixes ({}):\n", all.len());
            for fix in &all {
                println!("  \"{}\"", fix.label);
            }
            println!("\nRun one: hematite --fix-all --only \"<label>\"");
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

        use std::io::Write;
        let all_sweep = hematite::agent::report_export::sweep_auto_fixes();

        // --fix-all --only <label>: filter to the named fix
        let sweep: Vec<&hematite::agent::report_export::AutoFix> =
            if let Some(ref only_label) = cockpit.only {
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
        println!("Hematite maintenance sweep — {} checks\n", sweep.len());

        struct SweepEntry {
            label: &'static str,
            status: &'static str,  // "healthy", "fixed", "unresolved", "failed", "done"
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
            print!("  Checking {}... ", display);
            let _ = std::io::stdout().flush();
            let needs_fix = if let (Some(topic), Some(gone)) =
                (fix.verify_topic, fix.verify_gone)
            {
                let pre =
                    hematite::agent::report_export::generate_inspect_output(topic).await;
                pre.to_ascii_lowercase().contains(gone)
            } else {
                true
            };
            if !needs_fix {
                println!("OK");
                log.push(SweepEntry { label: fix.label, status: "healthy" });
                continue;
            }
            print!("needs fix — running... ");
            let _ = std::io::stdout().flush();
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
                            hematite::agent::report_export::generate_inspect_output(topic)
                                .await;
                        if post.to_ascii_lowercase().contains(gone) {
                            println!("\x1B[33m✗ still present\x1B[0m");
                            log.push(SweepEntry { label: fix.label, status: "unresolved" });
                        } else {
                            println!("\x1B[32m✓ resolved\x1B[0m");
                            verified += 1;
                            log.push(SweepEntry { label: fix.label, status: "fixed" });
                        }
                    } else {
                        println!("done");
                        verified += 1;
                        log.push(SweepEntry { label: fix.label, status: "done" });
                    }
                }
                Ok(s) => {
                    println!("failed (code {})", s.code().unwrap_or(1));
                    log.push(SweepEntry { label: fix.label, status: "failed" });
                }
                Err(e) => {
                    println!("error: {}", e);
                    log.push(SweepEntry { label: fix.label, status: "failed" });
                }
            }
        }
        println!();
        let summary = if applied == 0 {
            "All checks passed — nothing needed fixing.".to_string()
        } else {
            format!("{} fix(es) applied, {} verified resolved.", applied, verified)
        };
        println!("  {}", summary);

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
        let safe_ts: String = ts.chars().map(|c| if c.is_alphanumeric() || c == '-' { c } else { '_' }).collect();

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
            let json_str = serde_json::to_string_pretty(&json_obj)
                .unwrap_or_else(|_| "{}".to_string());
            report_path = report_dir.join(format!("sweep-{}.json", safe_ts));
            let _ = std::fs::write(&report_path, &json_str);
            report_content = json_str;
        } else {
            report_path = report_dir.join(format!("sweep-{}.md", safe_ts));
            let _ = std::fs::write(&report_path, &md);
            report_content = md.clone();
        }
        println!("Sweep report saved: {}", report_path.display());

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
        std::process::exit(if applied > 0 && verified < applied { 1 } else { 0 });
    }

    if cockpit.inventory {
        println!(
            "{}",
            hematite::agent::direct_answers::build_inspect_inventory()
        );
        return Ok(());
    }

    if cockpit.snapshots {
        let dir = hematite::tools::file_ops::hematite_dir().join("snapshots");
        if !dir.exists() {
            println!("No snapshots saved yet.");
            println!("Save one with: hematite --inspect <topic> --snapshot <name>");
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
        if entries.is_empty() {
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
                            if s < 60 { format!("{}s ago", s) }
                            else if s < 3600 { format!("{}m ago", s / 60) }
                            else if s < 86400 { format!("{}h ago", s / 3600) }
                            else { format!("{}d ago", s / 86400) }
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
        println!("--- {}  ({})", name_a, age_a);
        println!("+++ {}  ({})", name_b, age_b);
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
        }
        if !changed {
            println!("No differences between '{}' and '{}'.", name_a, name_b);
        }
        return Ok(());
    }

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

            let is_json_mode = cockpit
                .report_format
                .trim()
                .eq_ignore_ascii_case("json");

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
                        let _ = writeln!(
                            file,
                            "{}",
                            serde_json::to_string(&obj).unwrap_or_default()
                        );
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

        let diff_fmt = cockpit.report_format.trim().to_ascii_lowercase();

        if diff_fmt == "json" {
            use similar::{ChangeTag, TextDiff};
            let diff = TextDiff::from_lines(&snap_a, &snap_b);
            let mut diff_lines: Vec<String> = Vec::new();
            let mut changed = false;
            for group in diff.grouped_ops(2) {
                for op in &group {
                    for change in diff.iter_changes(op) {
                        let prefix = match change.tag() {
                            ChangeTag::Delete => { changed = true; "-" }
                            ChangeTag::Insert => { changed = true; "+" }
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
                "before": snap_a,
                "after": snap_b,
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
