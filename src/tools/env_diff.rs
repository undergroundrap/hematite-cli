use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

pub async fn execute(args: &Value) -> Result<String, String> {
    let root = if let Some(r) = args.get("_root").and_then(|v| v.as_str()) {
        PathBuf::from(r)
    } else {
        crate::tools::file_ops::workspace_root()
    };

    let file_a = args.get("file_a").and_then(|v| v.as_str());
    let file_b = args.get("file_b").and_then(|v| v.as_str());

    match (file_a, file_b) {
        (Some(a), Some(b)) => diff_two_files(a, b, &root),
        (Some(a), None) => diff_file_vs_process(a, &root),
        (None, None) => {
            // Auto-detect common env files in the workspace
            let candidates = &[
                ".env",
                ".env.local",
                ".env.development",
                ".env.production",
                ".env.test",
            ];
            let found: Vec<&str> = candidates
                .iter()
                .filter(|f| root.join(f).exists())
                .copied()
                .collect();
            match found.len() {
                0 => Ok("env_diff: no .env files found in the workspace root.\n\
                     Provide file_a and file_b to compare specific files, \
                     or file_a alone to compare against the live process environment."
                    .to_string()),
                1 => diff_file_vs_process(found[0], &root),
                _ => diff_two_files(found[0], found[1], &root),
            }
        }
        (None, Some(_)) => Err("env_diff: file_b requires file_a to also be specified.".into()),
    }
}

fn parse_env_file(path: &std::path::Path) -> Result<BTreeMap<String, String>, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|e| format!("env_diff: cannot read '{}': {e}", path.display()))?;

    let mut map = BTreeMap::new();
    for line in text.lines() {
        let line = line.trim();
        // Skip comments and blank lines
        if line.is_empty() || line.starts_with('#') || line.starts_with("//") {
            continue;
        }
        // Handle export VAR=val
        let line = line.strip_prefix("export ").unwrap_or(line).trim();
        if let Some(eq) = line.find('=') {
            let key = line[..eq].trim().to_string();
            let val = line[eq + 1..]
                .trim()
                .trim_matches('"')
                .trim_matches('\'')
                .to_string();
            map.insert(key, val);
        }
    }
    Ok(map)
}

fn get_process_env() -> BTreeMap<String, String> {
    std::env::vars().collect()
}

fn diff_two_files(a: &str, b: &str, root: &PathBuf) -> Result<String, String> {
    let path_a = if std::path::Path::new(a).is_absolute() {
        PathBuf::from(a)
    } else {
        root.join(a)
    };
    let path_b = if std::path::Path::new(b).is_absolute() {
        PathBuf::from(b)
    } else {
        root.join(b)
    };

    let map_a = parse_env_file(&path_a)?;
    let map_b = parse_env_file(&path_b)?;

    render_diff(
        &format!("{a} ({} keys)", map_a.len()),
        &format!("{b} ({} keys)", map_b.len()),
        &map_a,
        &map_b,
        true,
    )
}

fn diff_file_vs_process(file: &str, root: &PathBuf) -> Result<String, String> {
    let path = if std::path::Path::new(file).is_absolute() {
        PathBuf::from(file)
    } else {
        root.join(file)
    };

    let file_env = parse_env_file(&path)?;
    let proc_env = get_process_env();

    // For process comparison, only show keys that appear in the file
    // (the process has hundreds of vars; comparing all would be noise)
    let filtered_proc: BTreeMap<String, String> = file_env
        .keys()
        .filter_map(|k| proc_env.get(k).map(|v| (k.clone(), v.clone())))
        .collect();

    render_diff(
        &format!("{file} ({} keys)", file_env.len()),
        &format!("process env (filtered to file keys)"),
        &file_env,
        &filtered_proc,
        true,
    )
}

fn render_diff(
    label_a: &str,
    label_b: &str,
    a: &BTreeMap<String, String>,
    b: &BTreeMap<String, String>,
    redact_secrets: bool,
) -> Result<String, String> {
    let keys_a: BTreeSet<&String> = a.keys().collect();
    let keys_b: BTreeSet<&String> = b.keys().collect();

    let only_in_a: Vec<&String> = keys_a.difference(&keys_b).copied().collect();
    let only_in_b: Vec<&String> = keys_b.difference(&keys_a).copied().collect();
    let in_both: Vec<&String> = keys_a.intersection(&keys_b).copied().collect();

    let mut changed: Vec<(&String, &String, &String)> = Vec::new();
    let mut same_count = 0usize;
    for key in &in_both {
        let va = a.get(*key).unwrap();
        let vb = b.get(*key).unwrap();
        if va != vb {
            changed.push((key, va, vb));
        } else {
            same_count += 1;
        }
    }

    let total_changes = only_in_a.len() + only_in_b.len() + changed.len();

    let mut out = format!(
        "ENV DIFF\n{}\n\
         A: {label_a}\n\
         B: {label_b}\n\n\
         Summary: {} addition(s), {} removal(s), {} value change(s), {same_count} identical\n",
        "─".repeat(60),
        only_in_b.len(),
        only_in_a.len(),
        changed.len(),
    );

    if total_changes == 0 {
        out.push_str("\nAll matching keys are identical. No differences found.\n");
        return Ok(out);
    }

    if !only_in_b.is_empty() {
        out.push_str(&format!("\n[+] Only in B ({}):\n", only_in_b.len()));
        for key in &only_in_b {
            let val = redact(key, b.get(*key).unwrap(), redact_secrets);
            out.push_str(&format!("  + {key}={val}\n"));
        }
    }

    if !only_in_a.is_empty() {
        out.push_str(&format!("\n[-] Only in A ({}):\n", only_in_a.len()));
        for key in &only_in_a {
            let val = redact(key, a.get(*key).unwrap(), redact_secrets);
            out.push_str(&format!("  - {key}={val}\n"));
        }
    }

    if !changed.is_empty() {
        out.push_str(&format!("\n[~] Changed values ({}):\n", changed.len()));
        for (key, va, vb) in &changed {
            let va_r = redact(key, va, redact_secrets);
            let vb_r = redact(key, vb, redact_secrets);
            out.push_str(&format!("  ~ {key}\n      A: {va_r}\n      B: {vb_r}\n"));
        }
    }

    Ok(out)
}

fn redact(key: &str, val: &str, do_redact: bool) -> String {
    if !do_redact || val.is_empty() {
        return val.to_string();
    }
    let lower = key.to_lowercase();
    let is_secret = lower.contains("key")
        || lower.contains("secret")
        || lower.contains("token")
        || lower.contains("password")
        || lower.contains("passwd")
        || lower.contains("pwd")
        || lower.contains("api")
        || lower.contains("auth")
        || lower.contains("credential")
        || lower.contains("private");

    if is_secret && val.len() > 4 {
        format!("{}[...REDACTED]", &val[..val.len().min(4)])
    } else {
        val.to_string()
    }
}
