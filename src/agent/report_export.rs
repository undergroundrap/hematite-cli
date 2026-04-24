use serde_json::json;
use std::path::PathBuf;

const REPORT_TOPICS: &[(&str, &str)] = &[
    ("health_report", "System Health"),
    ("hardware", "Hardware"),
    ("storage", "Storage"),
    ("network", "Network"),
    ("security", "Security"),
    ("toolchains", "Developer Toolchains"),
];

pub async fn generate_report_markdown() -> String {
    let timestamp = now_timestamp_string();
    let mut hostname = hostname_from_env();
    let version = env!("CARGO_PKG_VERSION");
    let mut sections: Vec<(&str, String)> = Vec::new();

    for (topic, label) in REPORT_TOPICS {
        let args = json!({"topic": topic});
        let output = match crate::tools::host_inspect::inspect_host(&args).await {
            Ok(s) => {
                if *topic == "hardware" {
                    for line in s.lines() {
                        let ll = line.to_ascii_lowercase();
                        if ll.contains("hostname") || ll.contains("computer name") {
                            if let Some(val) = line.splitn(2, ':').nth(1) {
                                let h = val.trim().to_string();
                                if !h.is_empty() {
                                    hostname = h;
                                }
                            }
                        }
                    }
                }
                s
            }
            Err(e) => format!("Error: {}", e),
        };
        sections.push((label, output));
    }

    let section_refs: Vec<(&str, &str)> = sections.iter().map(|(l, o)| (*l, o.as_str())).collect();
    let action_plan = crate::agent::fix_recipes::format_action_plan(&section_refs);

    let mut md = String::new();
    md.push_str("# Hematite Diagnostic Report\n\n");
    md.push_str(&format!("**Generated:** {}  \n", timestamp));
    md.push_str(&format!("**Host:** {}  \n", hostname));
    md.push_str(&format!("**Hematite:** v{}  \n\n", version));
    md.push_str("---\n\n");

    md.push_str("## Action Plan\n\n");
    md.push_str(&action_plan);
    md.push_str("---\n\n");

    for (label, output) in &sections {
        md.push_str(&format!("## {}\n\n", label));
        md.push_str("```\n");
        md.push_str(output.trim_end());
        md.push_str("\n```\n\n");
    }

    md
}

/// Run a full staged diagnosis — health_report → triage → targeted follow-ups → fix recipes.
/// No TUI, no model required. Output is self-contained markdown for cloud model ingestion.
pub async fn generate_diagnosis_report() -> String {
    let timestamp = now_timestamp_string();
    let hostname = hostname_from_env();
    let version = env!("CARGO_PKG_VERSION");

    // Phase 1: health_report
    let health_args = json!({"topic": "health_report"});
    let health_output = match crate::tools::host_inspect::inspect_host(&health_args).await {
        Ok(s) => s,
        Err(e) => format!("Error running health_report: {}", e),
    };

    // Phase 2: triage — find which topics need deeper investigation
    let follow_up_topics = crate::agent::diagnose::triage_follow_up_topics(&health_output);

    // Phase 3: run each targeted follow-up
    let mut follow_up_outputs: Vec<(&'static str, String)> = Vec::new();
    for topic in &follow_up_topics {
        let args = json!({"topic": topic});
        let output = match crate::tools::host_inspect::inspect_host(&args).await {
            Ok(s) => s,
            Err(e) => format!("Error: {}", e),
        };
        follow_up_outputs.push((*topic, output));
    }

    // Build section refs for fix recipe matching
    let mut section_refs: Vec<(&str, &str)> = vec![("health_report", health_output.as_str())];
    for (topic, output) in &follow_up_outputs {
        section_refs.push((*topic, output.as_str()));
    }
    let action_plan = crate::agent::fix_recipes::format_action_plan(&section_refs);

    let mut md = String::new();
    md.push_str("# Hematite Staged Diagnosis Report\n\n");
    md.push_str(&format!("**Generated:** {}  \n", timestamp));
    md.push_str(&format!("**Host:** {}  \n", hostname));
    md.push_str(&format!("**Hematite:** v{}  \n\n", version));
    md.push_str("---\n\n");

    md.push_str("## Action Plan\n\n");
    md.push_str(&action_plan);
    md.push_str("---\n\n");

    md.push_str("## System Health\n\n");
    md.push_str("```\n");
    md.push_str(health_output.trim_end());
    md.push_str("\n```\n\n");

    if !follow_up_outputs.is_empty() {
        md.push_str("## Targeted Investigation\n\n");
        for (topic, output) in &follow_up_outputs {
            md.push_str(&format!("### {}\n\n", topic));
            md.push_str("```\n");
            md.push_str(output.trim_end());
            md.push_str("\n```\n\n");
        }
    }

    md
}

pub async fn generate_report_json() -> String {
    let timestamp = now_timestamp_string();
    let hostname = hostname_from_env();
    let version = env!("CARGO_PKG_VERSION");
    let mut obj = serde_json::Map::new();
    obj.insert("generated".into(), json!(timestamp));
    obj.insert("host".into(), json!(hostname));
    obj.insert("hematite_version".into(), json!(version));

    for (topic, label) in REPORT_TOPICS {
        let args = json!({"topic": topic});
        let value = match crate::tools::host_inspect::inspect_host(&args).await {
            Ok(output) => json!({"label": label, "output": output}),
            Err(e) => json!({"label": label, "error": e}),
        };
        obj.insert(topic.to_string(), value);
    }

    serde_json::to_string_pretty(&serde_json::Value::Object(obj))
        .unwrap_or_else(|e| format!("{{\"error\": \"{}\"}}", e))
}

/// Runs diagnostic topics, writes to `.hematite/reports/health-<timestamp>.md`,
/// and returns `(markdown_content, saved_path)`.
pub async fn save_report_markdown() -> (String, PathBuf) {
    let md = generate_report_markdown().await;
    let path = report_path("md");
    ensure_parent(&path);
    let _ = std::fs::write(&path, &md);
    (md, path)
}

/// Same as `save_report_markdown` but JSON format.
pub async fn save_report_json() -> (String, PathBuf) {
    let json = generate_report_json().await;
    let path = report_path("json");
    ensure_parent(&path);
    let _ = std::fs::write(&path, &json);
    (json, path)
}

fn report_path(ext: &str) -> PathBuf {
    crate::tools::file_ops::hematite_dir()
        .join("reports")
        .join(format!("health-{}.{}", now_file_timestamp(), ext))
}

fn ensure_parent(path: &PathBuf) {
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
}

fn now_timestamp_string() -> String {
    let now = unix_now();
    let (y, mo, d, h, mi, s) = epoch_to_ymd_hms(now);
    format!("{:04}-{:02}-{:02} {:02}:{:02}:{:02} UTC", y, mo, d, h, mi, s)
}

fn now_file_timestamp() -> String {
    let now = unix_now();
    let (y, mo, d, h, mi, _s) = epoch_to_ymd_hms(now);
    format!("{:04}-{:02}-{:02}_{:02}-{:02}", y, mo, d, h, mi)
}

fn unix_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn hostname_from_env() -> String {
    std::env::var("COMPUTERNAME")
        .or_else(|_| std::env::var("HOSTNAME"))
        .unwrap_or_else(|_| "unknown".to_string())
}

/// Gregorian calendar decomposition of a Unix timestamp (accurate 1970–2100).
fn epoch_to_ymd_hms(epoch: u64) -> (u32, u32, u32, u32, u32, u32) {
    let s = (epoch % 60) as u32;
    let mi = ((epoch / 60) % 60) as u32;
    let h = ((epoch / 3600) % 24) as u32;
    let days = epoch / 86400;

    let years_400 = days / 146097;
    let rem = days % 146097;
    let years_100 = rem.min(146096) / 36524;
    let rem = rem - years_100 * 36524;
    let years_4 = rem / 1461;
    let rem = rem % 1461;
    let years_1 = rem.min(1460) / 365;
    let rem = rem - years_1 * 365;

    let year = (1970 + years_400 * 400 + years_100 * 100 + years_4 * 4 + years_1) as u32;
    let leap = u32::from(year % 4 == 0 && (year % 100 != 0 || year % 400 == 0));
    let month_days: [u32; 12] = [31, 28 + leap, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    let mut rem = rem as u32;
    let mut month = 1u32;
    for &md in &month_days {
        if rem < md {
            break;
        }
        rem -= md;
        month += 1;
    }
    let day = rem + 1;
    (year, month, day, h, mi, s)
}
