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
    let score = crate::agent::fix_recipes::score_health(&section_refs);
    let action_plan = crate::agent::fix_recipes::format_action_plan(&section_refs);

    let mut md = String::new();
    md.push_str("# Hematite Diagnostic Report\n\n");
    md.push_str(&format!("**Generated:** {}  \n", timestamp));
    md.push_str(&format!("**Host:** {}  \n", hostname));
    md.push_str(&format!("**Hematite:** v{}  \n", version));
    md.push_str(&format!(
        "**Health Score:** {} — {}  \n\n",
        score.grade, score.label
    ));
    md.push_str(&format!("> {}\n\n", score.summary_line()));
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

struct DiagnosisData {
    timestamp: String,
    hostname: String,
    health_output: String,
    follow_up_outputs: Vec<(&'static str, String)>,
}

async fn run_diagnosis_phases() -> DiagnosisData {
    let timestamp = now_timestamp_string();
    let hostname = hostname_from_env();

    let health_args = json!({"topic": "health_report"});
    let health_output = match crate::tools::host_inspect::inspect_host(&health_args).await {
        Ok(s) => s,
        Err(e) => format!("Error running health_report: {}", e),
    };

    let follow_up_topics = crate::agent::diagnose::triage_follow_up_topics(&health_output);

    let mut follow_up_outputs: Vec<(&'static str, String)> = Vec::new();
    for topic in &follow_up_topics {
        let args = json!({"topic": topic});
        let output = match crate::tools::host_inspect::inspect_host(&args).await {
            Ok(s) => s,
            Err(e) => format!("Error: {}", e),
        };
        follow_up_outputs.push((*topic, output));
    }

    DiagnosisData { timestamp, hostname, health_output, follow_up_outputs }
}

/// Run a full staged diagnosis — health_report → triage → targeted follow-ups → fix recipes.
/// No TUI, no model required. Output is self-contained markdown for cloud model ingestion.
pub async fn generate_diagnosis_report() -> String {
    let version = env!("CARGO_PKG_VERSION");
    let data = run_diagnosis_phases().await;

    let mut section_refs: Vec<(&str, &str)> =
        vec![("health_report", data.health_output.as_str())];
    for (topic, output) in &data.follow_up_outputs {
        section_refs.push((*topic, output.as_str()));
    }
    let score = crate::agent::fix_recipes::score_health(&section_refs);
    let action_plan = crate::agent::fix_recipes::format_action_plan(&section_refs);

    let mut md = String::new();
    md.push_str("# Hematite Staged Diagnosis Report\n\n");
    md.push_str(&format!("**Generated:** {}  \n", data.timestamp));
    md.push_str(&format!("**Host:** {}  \n", data.hostname));
    md.push_str(&format!("**Hematite:** v{}  \n", version));
    md.push_str(&format!("**Health Score:** {} — {}  \n\n", score.grade, score.label));
    md.push_str(&format!("> {}\n\n", score.summary_line()));
    md.push_str("---\n\n");
    md.push_str("## Action Plan\n\n");
    md.push_str(&action_plan);
    md.push_str("---\n\n");
    md.push_str("## System Health\n\n```\n");
    md.push_str(data.health_output.trim_end());
    md.push_str("\n```\n\n");

    if !data.follow_up_outputs.is_empty() {
        md.push_str("## Targeted Investigation\n\n");
        for (topic, output) in &data.follow_up_outputs {
            md.push_str(&format!("### {}\n\n```\n", topic));
            md.push_str(output.trim_end());
            md.push_str("\n```\n\n");
        }
    }

    md
}

/// Same as generate_diagnosis_report but outputs a self-contained HTML file.
pub async fn generate_diagnosis_report_html() -> String {
    let version = env!("CARGO_PKG_VERSION");
    let data = run_diagnosis_phases().await;

    let mut section_refs: Vec<(&str, &str)> =
        vec![("health_report", data.health_output.as_str())];
    for (topic, output) in &data.follow_up_outputs {
        section_refs.push((*topic, output.as_str()));
    }
    let score = crate::agent::fix_recipes::score_health(&section_refs);
    let action_plan_html = crate::agent::fix_recipes::format_action_plan_html(&section_refs);

    let mut sections: Vec<(&str, String)> =
        vec![("System Health", data.health_output.clone())];
    for (topic, output) in &data.follow_up_outputs {
        sections.push((*topic, output.clone()));
    }

    build_html_document(
        "Hematite Staged Diagnosis",
        &data.timestamp,
        &data.hostname,
        version,
        &score,
        &action_plan_html,
        &sections,
    )
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

/// Self-contained HTML diagnostic report — double-clickable, no external deps.
pub async fn generate_report_html() -> String {
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
    let score = crate::agent::fix_recipes::score_health(&section_refs);
    let action_plan_html = crate::agent::fix_recipes::format_action_plan_html(&section_refs);

    build_html_document(
        "Hematite Diagnostic Report",
        &timestamp,
        &hostname,
        version,
        &score,
        &action_plan_html,
        &sections,
    )
}

pub async fn save_report_html() -> (String, PathBuf) {
    let html = generate_report_html().await;
    let path = report_path("html");
    ensure_parent(&path);
    let _ = std::fs::write(&path, &html);
    (html, path)
}

pub async fn save_diagnosis_report() -> (String, PathBuf) {
    let md = generate_diagnosis_report().await;
    let path = crate::tools::file_ops::hematite_dir()
        .join("reports")
        .join(format!("diagnosis-{}.md", now_file_timestamp()));
    ensure_parent(&path);
    let _ = std::fs::write(&path, &md);
    (md, path)
}

pub async fn save_diagnosis_report_html() -> (String, PathBuf) {
    let html = generate_diagnosis_report_html().await;
    let path = crate::tools::file_ops::hematite_dir()
        .join("reports")
        .join(format!("diagnosis-{}.html", now_file_timestamp()));
    ensure_parent(&path);
    let _ = std::fs::write(&path, &html);
    (html, path)
}

fn build_html_document(
    title: &str,
    timestamp: &str,
    hostname: &str,
    version: &str,
    score: &crate::agent::fix_recipes::HealthScore,
    action_plan_html: &str,
    sections: &[(&str, String)],
) -> String {
    let css = r#":root{--bg:#000;--fg:#fff;--dim:#6b6b6b;--line:#1a1a1a;--line-2:#262626}
*{box-sizing:border-box;margin:0;padding:0}
html{scrollbar-width:thin;scrollbar-color:#2a2a2a #000}
::-webkit-scrollbar{width:8px}::-webkit-scrollbar-track{background:#000}::-webkit-scrollbar-thumb{background:#222;border-radius:999px;border:2px solid #000}::-webkit-scrollbar-thumb:hover{background:#333}
body{font-family:'Inter',-apple-system,BlinkMacSystemFont,'Segoe UI',sans-serif;-webkit-font-smoothing:antialiased;-moz-osx-font-smoothing:grayscale;background:var(--bg);color:var(--fg);padding:2.5rem 1.5rem;min-height:100vh}
.wrap{max-width:900px;margin:0 auto}
header{background:#0a0a0a;border:1px solid var(--line-2);border-radius:18px;padding:2rem 2.25rem;margin-bottom:1rem}
h1{font-size:1.35rem;font-weight:600;letter-spacing:-0.025em;color:var(--fg);margin-bottom:.6rem}
.meta{font-size:.775rem;color:var(--dim);margin-bottom:1.25rem;display:flex;flex-wrap:wrap;gap:.4rem 1.5rem;letter-spacing:-0.005em}
.score-row{display:flex;align-items:center;gap:1rem;flex-wrap:wrap}
.grade{font-size:2rem;font-weight:800;width:3rem;height:3rem;border-radius:10px;display:flex;align-items:center;justify-content:center;flex-shrink:0;letter-spacing:-0.02em}
.gA{background:#14532d;color:#4ade80}.gB{background:#166534;color:#86efac}.gC{background:#78350f;color:#fbbf24}.gD{background:#7c2d12;color:#fb923c}.gF{background:#7f1d1d;color:#f87171}
.score-info h2{font-size:1rem;font-weight:600;letter-spacing:-0.02em;color:var(--fg)}.score-info p{color:#a3a3a3;font-size:.85rem;margin-top:.2rem;letter-spacing:-0.005em}
section{background:#0a0a0a;border:1px solid var(--line-2);border-radius:18px;padding:2rem 2.25rem;margin-bottom:1rem}
section>h2{font-size:.85rem;font-weight:600;letter-spacing:-0.01em;color:#d4d4d4;margin-bottom:1.25rem;padding-bottom:.75rem;border-bottom:1px solid var(--line)}
.recipe{padding:1rem 1.25rem;border-left:3px solid var(--line-2);border-radius:0 10px 10px 0;margin-bottom:.75rem;background:#111}
.recipe:last-child{margin-bottom:0}
.sev-action{border-left-color:#dc2626}.sev-investigate{border-left-color:#d97706}.sev-monitor{border-left-color:#3b82f6}
.recipe h3{font-size:.875rem;font-weight:600;letter-spacing:-0.015em;margin-bottom:.7rem;display:flex;align-items:center;gap:.5rem;flex-wrap:wrap;color:var(--fg)}
.badge{font-size:.65rem;font-weight:700;padding:.2rem .5rem;border-radius:5px;letter-spacing:.02em}
.b-action{background:#7f1d1d;color:#f87171}.b-investigate{background:#78350f;color:#fbbf24}.b-monitor{background:#1e3a5f;color:#93c5fd}
.recipe ol{padding-left:1.2rem;color:#d4d4d4}
.recipe li{margin-bottom:.4rem;line-height:1.6;font-size:.85rem}
.dig-deeper{font-size:.75rem;color:#4b4b4b;margin-top:.7rem}
.dig-deeper code{background:var(--line);padding:.1rem .3rem;border-radius:3px;font-size:.75rem;color:#6b6b6b}
.healthy{color:#4ade80;font-weight:500;font-size:.875rem;padding:.4rem 0;letter-spacing:-0.01em}
details{border:1px solid var(--line);border-radius:10px;margin-bottom:.6rem;overflow:hidden}
details:last-child{margin-bottom:0}
summary{cursor:pointer;font-weight:500;font-size:.8rem;color:#a3a3a3;padding:.7rem 1rem;background:#111;list-style:none;user-select:none;letter-spacing:-0.005em;transition:color 150ms ease,background 150ms ease}
summary::-webkit-details-marker{display:none}
summary::before{content:'▶  ';font-size:.6rem;color:var(--dim)}
details[open] summary::before{content:'▼  '}
summary:hover{background:#161616;color:var(--fg)}
pre{font-family:'Cascadia Code','JetBrains Mono','Fira Code',Consolas,monospace;font-size:.75rem;background:#000;color:#a3a3a3;padding:1.25rem;overflow-x:auto;white-space:pre-wrap;word-break:break-word;line-height:1.6;margin:0;border-top:1px solid var(--line)}
footer{text-align:center;color:var(--dim);font-size:.725rem;margin-top:1.5rem;padding-top:1rem;letter-spacing:-0.005em}
@media(max-width:640px){body{padding:1.5rem .75rem}header,section{padding:1.5rem;border-radius:14px}}"#;

    let mut sections_html = String::new();
    for (label, output) in sections {
        sections_html.push_str(&format!(
            "<details><summary>{}</summary><pre>{}</pre></details>\n",
            he(label),
            he(output.trim_end())
        ));
    }

    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1">
<title>{title} — {hostname}</title>
<style>{css}</style>
</head>
<body>
<div class="wrap">
<header>
<h1>{title}</h1>
<div class="meta">
  <span>Generated: {timestamp}</span>
  <span>Host: {hostname}</span>
  <span>Hematite v{version}</span>
</div>
<div class="score-row">
  <div class="grade g{grade}">{grade}</div>
  <div class="score-info">
    <h2>Health Score: {grade} — {label}</h2>
    <p>{summary}</p>
  </div>
</div>
</header>
<section>
<h2>Action Plan</h2>
{action_plan_html}
</section>
<section>
<h2>Diagnostic Data</h2>
{sections_html}
</section>
</div>
<footer>Generated by Hematite v{version}</footer>
</body>
</html>"#,
        title = he(title),
        hostname = he(hostname),
        timestamp = he(timestamp),
        version = he(version),
        grade = score.grade,
        label = he(score.label),
        summary = he(&score.summary_line()),
        action_plan_html = action_plan_html,
        sections_html = sections_html,
        css = css,
    )
}

fn he(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
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
