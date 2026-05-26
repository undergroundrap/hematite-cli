use serde_json::Value;
use std::path::{Path, PathBuf};

const MAX_FINDINGS: usize = 200;
const MAX_FILE_SIZE: u64 = 1024 * 1024; // 1 MB

// Each entry: (label, regex-pattern)
const PATTERNS: &[(&str, &str)] = &[
    ("AWS Access Key", r"AKIA[0-9A-Z]{16}"),
    (
        "AWS Secret Key",
        r#"(?i)aws[_\-]?secret[_\-]?access[_\-]?key\s*[=:]\s*["']?[A-Za-z0-9/+]{40}["']?"#,
    ),
    (
        "GitHub Token",
        r"(ghp|ghs|gho|ghu|ghr|github_pat)_[A-Za-z0-9_]{36,}",
    ),
    ("Stripe Live Key", r"(sk|pk)_live_[A-Za-z0-9]{24,}"),
    ("Stripe Test Key", r"(sk|pk)_test_[A-Za-z0-9]{24,}"),
    (
        "Slack Webhook",
        r"hooks\.slack\.com/services/T[A-Z0-9]{8,}/B[A-Z0-9]{8,}/[A-Za-z0-9]{24,}",
    ),
    (
        "Private Key Block",
        r"-----BEGIN\s(?:RSA |EC |DSA |OPENSSH )?PRIVATE KEY-----",
    ),
    (
        "Generic API Key",
        r#"(?i)(?:api[_\-]?key|apikey|api[_\-]?secret|access[_\-]?token|auth[_\-]?token)\s*[=:]\s*["']?[A-Za-z0-9_\-]{20,}["']?"#,
    ),
    (
        "Database URL",
        r"(?i)(postgres|postgresql|mysql|mongodb|redis)://[^:\s]+:[^@\s]{6,}@",
    ),
    ("Bearer Token", r#"(?i)bearer\s+[A-Za-z0-9_\-\.]{20,}"#),
    (
        "Password Literal",
        r#"(?i)(?:password|passwd|pwd)\s*[=:]\s*["']?(?!your|test|example|changeme|placeholder|xxx|<)[A-Za-z0-9!@#$%^&*]{8,}["']?"#,
    ),
    ("Twilio Key", r"SK[0-9a-fA-F]{32}"),
    (
        "SendGrid Key",
        r"SG\.[A-Za-z0-9_\-]{22}\.[A-Za-z0-9_\-]{43}",
    ),
    (
        "Heroku API Key",
        r"[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}",
    ),
];

const SKIP_DIRS: &[&str] = &[
    ".git",
    "target",
    "node_modules",
    "vendor",
    ".venv",
    "venv",
    "__pycache__",
    "dist",
    ".next",
    ".nuxt",
    "build",
    "out",
];

const SKIP_EXTENSIONS: &[&str] = &[
    "png", "jpg", "jpeg", "gif", "ico", "svg", "woff", "woff2", "ttf", "otf", "eot", "mp3", "mp4",
    "wav", "ogg", "pdf", "zip", "tar", "gz", "bz2", "xz", "7z", "rar", "exe", "dll", "so", "dylib",
    "pdb", "lib", "a", "lock", // Cargo.lock / yarn.lock contain hashes, not secrets
];

// Known false-positive file names to skip
const SKIP_FILENAMES: &[&str] = &[
    "Cargo.lock",
    "yarn.lock",
    "package-lock.json",
    "poetry.lock",
    "*.min.js",
    "*.min.css",
];

struct Finding {
    file: String,
    line: usize,
    kind: String,
    snippet: String,
}

pub async fn execute(args: &Value) -> Result<String, String> {
    let scan_path = args.get("path").and_then(|v| v.as_str()).unwrap_or(".");

    let root = if let Some(r) = args.get("_root").and_then(|v| v.as_str()) {
        PathBuf::from(r)
    } else {
        crate::tools::file_ops::workspace_root()
    };

    let target = if scan_path == "." {
        root.clone()
    } else {
        root.join(scan_path)
    };

    if !target.exists() {
        return Err(format!(
            "secret_scanner: path not found: {}",
            target.display()
        ));
    }

    // Compile patterns
    let compiled: Vec<(String, regex::Regex)> = PATTERNS
        .iter()
        .filter_map(|(label, pat)| regex::Regex::new(pat).ok().map(|r| (label.to_string(), r)))
        .collect();

    let mut findings: Vec<Finding> = Vec::new();
    let mut files_scanned = 0usize;
    let mut files_skipped = 0usize;

    scan_dir(
        &target,
        &compiled,
        &mut findings,
        &mut files_scanned,
        &mut files_skipped,
    );

    format_report(&findings, files_scanned, files_skipped, &target)
}

fn scan_dir(
    dir: &Path,
    patterns: &[(String, regex::Regex)],
    findings: &mut Vec<Finding>,
    scanned: &mut usize,
    skipped: &mut usize,
) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

        if path.is_dir() {
            if SKIP_DIRS.contains(&name) {
                continue;
            }
            if findings.len() >= MAX_FINDINGS {
                return;
            }
            scan_dir(&path, patterns, findings, scanned, skipped);
        } else if path.is_file() {
            if should_skip_file(&path, name) {
                *skipped += 1;
                continue;
            }
            let meta = match std::fs::metadata(&path) {
                Ok(m) => m,
                Err(_) => continue,
            };
            if meta.len() > MAX_FILE_SIZE {
                *skipped += 1;
                continue;
            }
            scan_file(&path, patterns, findings);
            *scanned += 1;

            if findings.len() >= MAX_FINDINGS {
                return;
            }
        }
    }
}

fn should_skip_file(path: &Path, name: &str) -> bool {
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        if SKIP_EXTENSIONS.contains(&ext.to_lowercase().as_str()) {
            return true;
        }
    }
    SKIP_FILENAMES.iter().any(|pat| {
        if pat.starts_with("*.") {
            name.ends_with(&pat[1..])
        } else {
            name == *pat
        }
    })
}

fn scan_file(path: &Path, patterns: &[(String, regex::Regex)], findings: &mut Vec<Finding>) {
    let content = match std::fs::read(path) {
        Ok(b) => b,
        Err(_) => return,
    };

    // Skip binary files (null byte in first 8KB)
    let probe = &content[..content.len().min(8192)];
    if probe.contains(&0u8) {
        return;
    }

    let text = match std::str::from_utf8(&content) {
        Ok(s) => s,
        Err(_) => return,
    };

    let rel_path = path.to_string_lossy().to_string();

    for (line_num, line) in text.lines().enumerate() {
        // Skip obvious placeholder / example / test lines
        let lower = line.to_lowercase();
        if lower.contains("example")
            || lower.contains("placeholder")
            || lower.contains("your_")
            || lower.contains("your-")
            || lower.contains("<key>")
            || lower.contains("<token>")
            || lower.contains("xxxxxxx")
        {
            continue;
        }

        for (label, re) in patterns {
            if re.is_match(line) {
                // Redact the actual matched value to avoid logging real secrets
                let snippet = if line.len() > 120 {
                    format!("{}...", &line[..120])
                } else {
                    line.to_string()
                };
                findings.push(Finding {
                    file: rel_path.clone(),
                    line: line_num + 1,
                    kind: label.clone(),
                    snippet: redact_match(re, &snippet),
                });
                break; // one finding per line
            }
        }

        if findings.len() >= MAX_FINDINGS {
            return;
        }
    }
}

fn redact_match(re: &regex::Regex, line: &str) -> String {
    re.replace_all(line, |caps: &regex::Captures| {
        let m = caps.get(0).unwrap().as_str();
        let keep = m.len().min(6);
        format!("{}[...REDACTED...]", &m[..keep])
    })
    .to_string()
}

fn format_report(
    findings: &[Finding],
    scanned: usize,
    skipped: usize,
    root: &Path,
) -> Result<String, String> {
    if findings.is_empty() {
        return Ok(format!(
            "secret_scanner [CLEAN]: No secrets detected.\n\
             Scanned {scanned} file(s), skipped {skipped} (binary/large/lock).\n\
             Patterns checked: {}",
            PATTERNS.len()
        ));
    }

    let truncated = findings.len() >= MAX_FINDINGS;
    let mut out = format!(
        "SECRET SCAN: {} finding(s) across {} file(s) scanned\n\
         Root: {}\n\
         {}\n",
        findings.len(),
        scanned,
        root.display(),
        if truncated {
            format!("[WARNING: output capped at {MAX_FINDINGS} findings — fix high-priority items and re-scan]")
        } else {
            String::new()
        }
    );

    // Group by file
    let mut by_file: std::collections::BTreeMap<&str, Vec<&Finding>> = Default::default();
    for f in findings {
        by_file.entry(&f.file).or_default().push(f);
    }

    for (file, file_findings) in &by_file {
        out.push_str(&format!("\n{file}\n"));
        for f in file_findings {
            out.push_str(&format!(
                "  line {:>4}  [{}]\n            {}\n",
                f.line,
                f.kind,
                f.snippet.trim()
            ));
        }
    }

    out.push_str(&format!(
        "\n── Recommendation ──\n\
         1. Remove or rotate any real credentials found above.\n\
         2. Add .env and secret files to .gitignore.\n\
         3. Use environment variables or a secrets manager (HashiCorp Vault, AWS Secrets Manager).\n\
         4. Run `git filter-repo` or BFG Repo Cleaner if secrets were committed to history.\n\
         \nScanned: {scanned} files, skipped: {skipped} (binary/large/lock/build artifacts)."
    ));

    Ok(out.trim_end().to_string())
}
