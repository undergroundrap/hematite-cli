const TASK_NAME: &str = "Hematite Health Check";

pub fn register_scheduled_task(cadence: &str, exe_path: &str) -> Result<String, String> {
    #[cfg(not(target_os = "windows"))]
    {
        let _ = (cadence, exe_path);
        return Err("Scheduled tasks require Windows (schtasks.exe).\n\
             On Linux/macOS use cron instead:\n\
               hematite --triage --report-format html"
            .into());
    }

    #[cfg(target_os = "windows")]
    {
        let task_run = format!("\"{}\" --triage --report-format html", exe_path);

        let (schedule_type, extra_args, label): (&str, &[&str], &str) = match cadence {
            "daily" => ("daily", &[], "daily at 08:00"),
            _ => ("weekly", &["/d", "MON"], "weekly on Monday at 08:00"),
        };

        let mut args: Vec<String> = vec![
            "/create".into(),
            "/tn".into(),
            TASK_NAME.into(),
            "/tr".into(),
            task_run.clone(),
            "/sc".into(),
            schedule_type.into(),
            "/st".into(),
            "08:00".into(),
        ];
        for a in extra_args {
            args.push(a.to_string());
        }
        args.push("/f".into());

        let out = std::process::Command::new("schtasks")
            .args(&args)
            .output()
            .map_err(|e| format!("Failed to run schtasks: {}", e))?;

        if out.status.success() {
            let reports_dir = crate::tools::file_ops::hematite_dir().join("reports");
            Ok(format!(
                "Task \"{}\" registered — runs {}.\n\
                 Action: {}\n\
                 Reports will save to: {}\n\
                 Run `hematite --schedule status` to confirm.",
                TASK_NAME,
                label,
                task_run,
                reports_dir.display()
            ))
        } else {
            let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
            let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
            Err(if !stderr.is_empty() { stderr } else { stdout })
        }
    }
}

pub fn remove_scheduled_task() -> Result<String, String> {
    #[cfg(not(target_os = "windows"))]
    return Err("Scheduled tasks require Windows.".into());

    #[cfg(target_os = "windows")]
    {
        let out = std::process::Command::new("schtasks")
            .args(["/delete", "/tn", TASK_NAME, "/f"])
            .output()
            .map_err(|e| format!("Failed to run schtasks: {}", e))?;

        if out.status.success() {
            Ok(format!("Task \"{}\" removed.", TASK_NAME))
        } else {
            let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
            Err(if !stderr.is_empty() {
                stderr
            } else {
                format!("Task \"{}\" not found — nothing to remove.", TASK_NAME)
            })
        }
    }
}

pub fn query_scheduled_task() -> String {
    #[cfg(not(target_os = "windows"))]
    return "Scheduled tasks are Windows-only. Use cron for recurring triage:\n\
            hematite --triage --report-format html"
        .to_string();

    #[cfg(target_os = "windows")]
    {
        let out = std::process::Command::new("schtasks")
            .args(["/query", "/tn", TASK_NAME, "/fo", "LIST"])
            .output();

        match out {
            Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).trim().to_string(),
            Ok(o) => {
                let stderr = String::from_utf8_lossy(&o.stderr).to_ascii_lowercase();
                if stderr.contains("cannot find") || stderr.contains("does not exist") {
                    format!("Task \"{}\" is not registered.", TASK_NAME)
                } else {
                    format!(
                        "Not registered: {}",
                        String::from_utf8_lossy(&o.stderr).trim()
                    )
                }
            }
            Err(e) => format!("Error querying task: {}", e),
        }
    }
}
