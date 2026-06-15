use serde_json::{json, Value};
use std::collections::HashMap;

pub fn make_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "action": {
                "type": "string",
                "enum": ["parse", "authors", "frequency", "files", "summary"],
                "description": "Action to perform (default: parse)"
            },
            "log": { "type": "string", "description": "git log output as inline text" },
            "file": { "type": "string", "description": "Path to a saved git log file" },
            "author": { "type": "string", "description": "Filter commits by author name or email substring" },
            "limit": { "type": "integer", "description": "Max entries to show (default 30)" }
        }
    })
}

// ── data model ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Default)]
struct Commit {
    hash: String,
    author_name: String,
    author_email: String,
    date_iso: String,
    subject: String,
    /// files changed / insertions / deletions from --stat lines
    stat_files: u32,
    stat_ins: u32,
    stat_del: u32,
}

// ── parser ────────────────────────────────────────────────────────────────────

/// Parse git log output in several formats:
/// 1. Pipe-delimited: `%H|%an|%ae|%ai|%s` (most useful)
/// 2. Oneline: `<hash> <subject>`
/// 3. Traditional: `commit <hash>\nAuthor: ...\nDate: ...\n\n    <subject>`
///
/// Also absorbs `--stat` summary lines that follow a commit.
fn parse_log(text: &str) -> Vec<Commit> {
    let lines: Vec<&str> = text.lines().collect();
    let mut commits: Vec<Commit> = Vec::new();

    // Detect format by checking first non-empty line
    let first = lines
        .iter()
        .find(|l| !l.trim().is_empty())
        .copied()
        .unwrap_or("");
    let pipe_count = first.chars().filter(|&c| c == '|').count();

    if pipe_count >= 4 {
        // Format 1: pipe-delimited
        for line in &lines {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            // Could also have --stat lines interleaved; skip those
            if is_stat_line(line) {
                if let Some(last) = commits.last_mut() {
                    apply_stat_line(last, line);
                }
                continue;
            }
            let parts: Vec<&str> = line.splitn(5, '|').collect();
            if parts.len() >= 5 {
                commits.push(Commit {
                    hash: parts[0].to_string(),
                    author_name: parts[1].to_string(),
                    author_email: parts[2].to_string(),
                    date_iso: parts[3].to_string(),
                    subject: parts[4].to_string(),
                    ..Default::default()
                });
            } else if parts.len() == 4 {
                // hash|name|email|subject (no date)
                commits.push(Commit {
                    hash: parts[0].to_string(),
                    author_name: parts[1].to_string(),
                    author_email: parts[2].to_string(),
                    subject: parts[3].to_string(),
                    ..Default::default()
                });
            }
        }
    } else if first.len() > 7
        && first[..7].chars().all(|c| c.is_ascii_hexdigit())
        && first.contains(' ')
    {
        // Format 2: oneline (short hash + space + subject)
        for line in &lines {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            if is_stat_line(line) {
                if let Some(last) = commits.last_mut() {
                    apply_stat_line(last, line);
                }
                continue;
            }
            if let Some(sp) = line.find(' ') {
                let hash = &line[..sp];
                if hash.chars().all(|c| c.is_ascii_hexdigit()) {
                    commits.push(Commit {
                        hash: hash.to_string(),
                        subject: line[sp + 1..].to_string(),
                        ..Default::default()
                    });
                }
            }
        }
    } else {
        // Format 3: traditional verbose git log
        let mut current: Option<Commit> = None;
        let mut in_message = false;

        for line in &lines {
            let trimmed = line.trim();

            if trimmed.starts_with("commit ") && trimmed.len() > 7 {
                if let Some(c) = current.take() {
                    commits.push(c);
                }
                let hash = trimmed[7..].trim().to_string();
                current = Some(Commit {
                    hash,
                    ..Default::default()
                });
                in_message = false;
            } else if let Some(ref mut c) = current {
                if let Some(author) = trimmed.strip_prefix("Author: ") {
                    // "Name <email>"
                    if let Some(lt) = author.rfind('<') {
                        c.author_name = author[..lt].trim().to_string();
                        c.author_email = author[lt + 1..].trim_end_matches('>').to_string();
                    } else {
                        c.author_name = author.to_string();
                    }
                } else if let Some(date) = trimmed.strip_prefix("Date:") {
                    c.date_iso = date.trim().to_string();
                } else if trimmed.is_empty() {
                    in_message = !in_message;
                } else if in_message && c.subject.is_empty() {
                    c.subject = trimmed.to_string();
                } else if is_stat_line(trimmed) {
                    apply_stat_line(c, trimmed);
                }
            }
        }
        if let Some(c) = current {
            commits.push(c);
        }
    }

    commits
}

fn is_stat_line(line: &str) -> bool {
    // `3 files changed, 47 insertions(+), 2 deletions(-)`
    (line.contains("file changed") || line.contains("files changed"))
        && (line.contains("insertion") || line.contains("deletion"))
}

fn apply_stat_line(c: &mut Commit, line: &str) {
    // Extract numbers before "file", "insertion", "deletion"
    for part in line.split(',') {
        let part = part.trim();
        let num: u32 = part
            .split_whitespace()
            .next()
            .and_then(|w| w.parse().ok())
            .unwrap_or(0);
        if part.contains("file") {
            c.stat_files += num;
        } else if part.contains("insertion") {
            c.stat_ins += num;
        } else if part.contains("deletion") {
            c.stat_del += num;
        }
    }
}

// ── date helpers ──────────────────────────────────────────────────────────────

fn day_of_week_label(iso: &str) -> &'static str {
    // iso like "2024-03-15 14:22:00 +0000" — parse YYYY-MM-DD
    let parts: Vec<&str> = iso.trim().splitn(2, ' ').collect();
    if let Some(date) = parts.first() {
        let d: Vec<u32> = date.split('-').filter_map(|s| s.parse().ok()).collect();
        if d.len() == 3 {
            // Zeller's congruence for day of week (0=Sun)
            let (mut m, y, day) = (d[1], d[0], d[2]);
            let yr = if m < 3 { y - 1 } else { y };
            if m < 3 {
                m += 12;
            }
            let k = (yr % 100) as i32;
            let j = (yr / 100) as i32;
            let h = (day as i32 + (13 * (m as i32 + 1)) / 5 + k + k / 4 + j / 4 - 2 * j) % 7;
            let dow = ((h + 5) % 7) as usize; // 0=Mon
            return ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"][dow.min(6)];
        }
    }
    "?"
}

fn hour_of_day(iso: &str) -> Option<u32> {
    // "2024-03-15 14:22:00 +0000" → 14
    let parts: Vec<&str> = iso.trim().splitn(3, ' ').collect();
    if parts.len() >= 2 {
        let time = parts[1];
        time.split(':').next().and_then(|h| h.parse().ok())
    } else {
        None
    }
}

fn short_date(iso: &str) -> String {
    iso.trim().get(..10).unwrap_or(iso.trim()).to_string()
}

fn short_hash(hash: &str) -> &str {
    &hash[..hash.len().min(8)]
}

// ── actions ───────────────────────────────────────────────────────────────────

fn action_parse(commits: &[Commit], limit: usize) -> String {
    if commits.is_empty() {
        return "No commits found. Provide git log output via 'log' or 'file'.".into();
    }
    let mut out = format!("─── git log — {} commits ───\n\n", commits.len());
    out.push_str(&format!(
        "{:<9} {:<12} {:<22} {}\n",
        "HASH", "DATE", "AUTHOR", "SUBJECT"
    ));
    out.push_str(&"─".repeat(90));
    out.push('\n');
    for c in commits.iter().take(limit) {
        let name = if c.author_name.len() > 20 {
            format!("{}..", &c.author_name[..18])
        } else {
            c.author_name.clone()
        };
        let subj = if c.subject.len() > 50 {
            format!("{}..", &c.subject[..48])
        } else {
            c.subject.clone()
        };
        out.push_str(&format!(
            "{:<9} {:<12} {:<22} {}\n",
            short_hash(&c.hash),
            short_date(&c.date_iso),
            name,
            subj,
        ));
    }
    if commits.len() > limit {
        out.push_str(&format!(
            "\n… {} more (use 'limit' to show more)\n",
            commits.len() - limit
        ));
    }
    out
}

fn action_authors(commits: &[Commit]) -> String {
    if commits.is_empty() {
        return "No commits found.".into();
    }
    let mut counts: HashMap<String, (u32, String)> = HashMap::new();
    for c in commits {
        let e = counts
            .entry(c.author_name.clone())
            .or_insert_with(|| (0, c.author_email.clone()));
        e.0 += 1;
    }
    let mut ranked: Vec<(u32, String, String)> = counts
        .into_iter()
        .map(|(name, (n, email))| (n, name, email))
        .collect();
    ranked.sort_by_key(|b| std::cmp::Reverse(b.0));

    let total = commits.len() as f64;
    let mut out = format!(
        "─── Author Leaderboard — {} commits total ───\n\n",
        commits.len()
    );
    out.push_str(&format!(
        "{:<4} {:<25} {:<28} {:>6}  BAR\n",
        "#", "AUTHOR", "EMAIL", "COMMITS"
    ));
    out.push_str(&"─".repeat(80));
    out.push('\n');
    for (i, (n, name, email)) in ranked.iter().enumerate() {
        let pct = (*n as f64 / total * 100.0) as usize;
        let bar = "█".repeat(pct.min(40));
        let email_short = if email.len() > 26 {
            format!("{}..", &email[..24])
        } else {
            email.clone()
        };
        out.push_str(&format!(
            "{:<4} {:<25} {:<28} {:>6}  {}\n",
            i + 1,
            &name[..name.len().min(24)],
            email_short,
            n,
            bar,
        ));
    }
    out
}

fn action_frequency(commits: &[Commit]) -> String {
    if commits.is_empty() {
        return "No commits found.".into();
    }
    // day-of-week heatmap
    let days = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
    let mut dow_counts = [0u32; 7];
    let mut hour_counts = [0u32; 24];

    for c in commits {
        if c.date_iso.is_empty() {
            continue;
        }
        let dow_label = day_of_week_label(&c.date_iso);
        if let Some(pos) = days.iter().position(|&d| d == dow_label) {
            dow_counts[pos] += 1;
        }
        if let Some(h) = hour_of_day(&c.date_iso) {
            if (h as usize) < 24 {
                hour_counts[h as usize] += 1;
            }
        }
    }

    let max_dow = *dow_counts.iter().max().unwrap_or(&1).max(&1);
    let max_hour = *hour_counts.iter().max().unwrap_or(&1).max(&1);

    let mut out = "─── Commit Frequency ───\n\n".to_string();
    out.push_str("Day of Week\n");
    out.push_str(&"─".repeat(50));
    out.push('\n');
    for (i, day) in days.iter().enumerate() {
        let n = dow_counts[i];
        let bar_len = (n as usize * 30) / max_dow as usize;
        let bar = "█".repeat(bar_len);
        out.push_str(&format!("  {:<3}  {:>4}  {}\n", day, n, bar));
    }

    out.push_str("\nHour of Day (UTC)\n");
    out.push_str(&"─".repeat(50));
    out.push('\n');
    for (h, &n) in hour_counts.iter().enumerate() {
        if n == 0 {
            continue;
        }
        let bar_len = (n as usize * 30) / max_hour as usize;
        let bar = "█".repeat(bar_len);
        out.push_str(&format!("  {:02}:00  {:>4}  {}\n", h, n, bar));
    }
    out
}

fn action_files(commits: &[Commit]) -> String {
    let with_stats: Vec<&Commit> = commits.iter().filter(|c| c.stat_files > 0).collect();
    if with_stats.is_empty() {
        return "No file change stats found.\n\
                Re-run git log with --stat to include file change counts:\n\
                  git log --format='%H|%an|%ae|%ai|%s' --stat > log.txt"
            .into();
    }
    let total_files: u32 = commits.iter().map(|c| c.stat_files).sum();
    let total_ins: u32 = commits.iter().map(|c| c.stat_ins).sum();
    let total_del: u32 = commits.iter().map(|c| c.stat_del).sum();
    let mut out = format!(
        "─── File Change Stats — {} commits with stats ───\n\n\
         Total: {} file-changes, +{} insertions, -{} deletions\n\n",
        with_stats.len(),
        total_files,
        total_ins,
        total_del,
    );
    out.push_str(&format!(
        "{:<9} {:<12} {:>8} {:>8} {:>8}  SUBJECT\n",
        "HASH", "DATE", "FILES", "+INS", "-DEL"
    ));
    out.push_str(&"─".repeat(80));
    out.push('\n');
    let mut sorted: Vec<&Commit> = commits.iter().filter(|c| c.stat_files > 0).collect();
    sorted.sort_by_key(|b| std::cmp::Reverse(b.stat_files));
    for c in sorted.iter().take(30) {
        let subj = if c.subject.len() > 32 {
            format!("{}..", &c.subject[..30])
        } else {
            c.subject.clone()
        };
        out.push_str(&format!(
            "{:<9} {:<12} {:>8} {:>8} {:>8}  {}\n",
            short_hash(&c.hash),
            short_date(&c.date_iso),
            c.stat_files,
            c.stat_ins,
            c.stat_del,
            subj,
        ));
    }
    out
}

fn action_summary(commits: &[Commit]) -> String {
    if commits.is_empty() {
        return "No commits found.".into();
    }
    let authors: std::collections::HashSet<&str> =
        commits.iter().map(|c| c.author_name.as_str()).collect();

    let first_date = commits
        .last()
        .map(|c| short_date(&c.date_iso))
        .unwrap_or_default();
    let last_date = commits
        .first()
        .map(|c| short_date(&c.date_iso))
        .unwrap_or_default();

    // Approximate weeks between first and last
    let weeks = {
        let parse_yw = |s: &str| -> Option<(i64, i64)> {
            let parts: Vec<i64> = s.split('-').filter_map(|x| x.parse().ok()).collect();
            if parts.len() >= 2 {
                Some((parts[0], parts[1]))
            } else {
                None
            }
        };
        if let (Some((y1, m1)), Some((y2, m2))) = (parse_yw(&first_date), parse_yw(&last_date)) {
            let months = (y2 - y1) * 12 + (m2 - m1);
            (months * 4).max(1)
        } else {
            1i64
        }
    };
    let per_week = commits.len() as f64 / weeks as f64;

    let mut out = "─── git log Summary ───\n\n".to_string();
    out.push_str(&format!("  Total commits : {}\n", commits.len()));
    out.push_str(&format!("  Authors       : {}\n", authors.len()));
    out.push_str(&format!(
        "  Date range    : {} → {}\n",
        first_date, last_date
    ));
    out.push_str(&format!("  Approx. rate  : {:.1} commits/week\n", per_week));

    let total_ins: u32 = commits.iter().map(|c| c.stat_ins).sum();
    let total_del: u32 = commits.iter().map(|c| c.stat_del).sum();
    if total_ins > 0 || total_del > 0 {
        out.push_str(&format!("  Lines added   : +{}\n", total_ins));
        out.push_str(&format!("  Lines removed : -{}\n", total_del));
    }

    // Top 3 authors
    let mut counts: HashMap<&str, u32> = HashMap::new();
    for c in commits {
        *counts.entry(c.author_name.as_str()).or_default() += 1;
    }
    let mut ranked: Vec<(&str, u32)> = counts.into_iter().collect();
    ranked.sort_by_key(|b| std::cmp::Reverse(b.1));
    out.push_str("\n  Top contributors:\n");
    for (name, n) in ranked.iter().take(5) {
        out.push_str(&format!("    {:<25} {} commits\n", name, n));
    }
    out
}

// ── entry point ───────────────────────────────────────────────────────────────

pub async fn execute(args: &Value) -> Result<String, String> {
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("parse");
    let limit = args
        .get("limit")
        .and_then(|v| v.as_u64())
        .map(|n| n as usize)
        .unwrap_or(30);
    let author_filter = args
        .get("author")
        .and_then(|v| v.as_str())
        .map(|s| s.to_lowercase());

    // Load text
    let raw = if let Some(t) = args.get("log").and_then(|v| v.as_str()) {
        t.to_string()
    } else if let Some(p) = args.get("file").and_then(|v| v.as_str()) {
        std::fs::read_to_string(p).map_err(|e| format!("Cannot read '{}': {}", p, e))?
    } else {
        return Err(
            "Provide 'log' (inline git log text) or 'file' (path to saved git log output).\n\
             Recommended format: git log --format='%H|%an|%ae|%ai|%s' > log.txt\n\
             For file churn:    git log --format='%H|%an|%ae|%ai|%s' --stat > log.txt"
                .into(),
        );
    };

    let mut commits = parse_log(&raw);

    // Apply author filter
    if let Some(ref af) = author_filter {
        commits.retain(|c| {
            c.author_name.to_lowercase().contains(af.as_str())
                || c.author_email.to_lowercase().contains(af.as_str())
        });
    }

    let result = match action {
        "parse" => action_parse(&commits, limit),
        "authors" => action_authors(&commits),
        "frequency" => action_frequency(&commits),
        "files" => action_files(&commits),
        "summary" => action_summary(&commits),
        other => {
            return Err(format!(
                "Unknown action '{}'. Choose: parse, authors, frequency, files, summary.",
                other
            ))
        }
    };

    Ok(result)
}
