use std::path::{Component, Path, PathBuf};

pub async fn execute(args: &serde_json::Value) -> Result<String, String> {
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("parse");

    match action {
        "parse" => parse_path(args),
        "join" => join_paths(args),
        "normalize" => normalize(args),
        "relative" => relative(args),
        "basename" => basename(args),
        "stem" => stem(args),
        "extension" => extension(args),
        "is-absolute" => is_absolute(args),
        other => Err(format!(
            "path_tools: unknown action '{other}'. Valid: parse, join, normalize, relative, basename, stem, extension, is-absolute"
        )),
    }
}

// ── Helpers ────────────────────────────────────────────────────────────────────

fn get_path_str(args: &serde_json::Value) -> Result<String, String> {
    args.get("path")
        .or_else(|| args.get("input"))
        .or_else(|| args.get("text"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| "path_tools: 'path' is required".to_string())
}

fn logical_normalize(path: &Path) -> PathBuf {
    let mut result: Vec<Component> = Vec::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                if let Some(Component::Normal(_)) = result.last() {
                    result.pop();
                } else {
                    result.push(component);
                }
            }
            c => result.push(c),
        }
    }
    result.iter().collect()
}

fn compute_relative(from: &Path, to: &Path) -> Result<PathBuf, String> {
    let from_norm = logical_normalize(from);
    let to_norm = logical_normalize(to);

    let mut from_comps: Vec<Component> = from_norm.components().collect();
    let mut to_comps: Vec<Component> = to_norm.components().collect();

    // Paths on different roots (e.g. different Windows drives) cannot be made relative
    if from_comps.first() != to_comps.first()
        && from_comps
            .first()
            .map(|c| matches!(c, Component::Prefix(_) | Component::RootDir))
            .unwrap_or(false)
    {
        return Err(format!(
            "path_tools relative: '{}' and '{}' are on different roots — no relative path exists",
            from.display(),
            to.display()
        ));
    }

    // Strip common prefix
    while !from_comps.is_empty() && !to_comps.is_empty() && from_comps[0] == to_comps[0] {
        from_comps.remove(0);
        to_comps.remove(0);
    }

    let mut rel = PathBuf::new();
    for _ in &from_comps {
        rel.push("..");
    }
    for c in &to_comps {
        rel.push(c);
    }

    if rel.as_os_str().is_empty() {
        rel.push(".");
    }

    Ok(rel)
}

// ── Actions ────────────────────────────────────────────────────────────────────

fn parse_path(args: &serde_json::Value) -> Result<String, String> {
    let raw = get_path_str(args)?;
    let p = Path::new(&raw);

    let mut out = format!("PATH PARSE\n{}\n", "─".repeat(50));
    out.push_str(&format!("Input     : {raw}\n\n"));

    if let Some(parent) = p.parent() {
        out.push_str(&format!("Parent    : {}\n", parent.display()));
    } else {
        out.push_str("Parent    : (none)\n");
    }

    out.push_str(&format!(
        "Filename  : {}\n",
        p.file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default()
    ));
    out.push_str(&format!(
        "Stem      : {}\n",
        p.file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default()
    ));
    out.push_str(&format!(
        "Extension : {}\n",
        p.extension()
            .map(|e| e.to_string_lossy().into_owned())
            .unwrap_or_else(|| "(none)".to_string())
    ));
    out.push_str(&format!("Absolute  : {}\n", p.is_absolute()));

    let components: Vec<String> = p
        .components()
        .enumerate()
        .map(|(i, c)| format!("  [{}] {}", i, c.as_os_str().to_string_lossy()))
        .collect();
    if !components.is_empty() {
        out.push_str("\nComponents:\n");
        for c in &components {
            out.push_str(&format!("{c}\n"));
        }
    }

    let normalized = logical_normalize(p);
    let norm_str = normalized.to_string_lossy();
    if norm_str != raw {
        out.push_str(&format!("\nNormalized: {norm_str}\n"));
    }

    Ok(out)
}

fn join_paths(args: &serde_json::Value) -> Result<String, String> {
    // Accept 'base' + 'parts' array, or 'paths' array, or positional 'a'/'b'/'c'
    let mut result = PathBuf::new();

    if let Some(base) = args.get("base").and_then(|v| v.as_str()) {
        result.push(base);
        if let Some(parts) = args.get("parts").and_then(|v| v.as_array()) {
            for p in parts {
                if let Some(s) = p.as_str() {
                    result.push(s);
                }
            }
        }
    } else if let Some(arr) = args.get("paths").and_then(|v| v.as_array()) {
        for p in arr {
            if let Some(s) = p.as_str() {
                result.push(s);
            }
        }
    } else {
        return Err(
            "path_tools join: 'base' + 'parts' array, or 'paths' array is required".to_string(),
        );
    }

    let joined = result.to_string_lossy();
    let normalized = logical_normalize(&result);

    let mut out = format!("PATH JOIN\n{}\n", "─".repeat(50));
    out.push_str(&format!("Joined    : {joined}\n"));
    let norm_str = normalized.to_string_lossy();
    if norm_str != joined {
        out.push_str(&format!("Normalized: {norm_str}\n"));
    }
    Ok(out)
}

fn normalize(args: &serde_json::Value) -> Result<String, String> {
    let raw = get_path_str(args)?;
    let p = Path::new(&raw);
    let normalized = logical_normalize(p);
    let norm_str = normalized.to_string_lossy();

    let mut out = format!("PATH NORMALIZE\n{}\n", "─".repeat(50));
    out.push_str(&format!("Input      : {raw}\n"));
    out.push_str(&format!("Normalized : {norm_str}\n"));
    if raw == norm_str {
        out.push_str("(already normalized)\n");
    }
    Ok(out)
}

fn relative(args: &serde_json::Value) -> Result<String, String> {
    let from_str = args
        .get("from")
        .and_then(|v| v.as_str())
        .ok_or("path_tools relative: 'from' is required")?;
    let to_str = args
        .get("to")
        .and_then(|v| v.as_str())
        .ok_or("path_tools relative: 'to' is required")?;

    let from = Path::new(from_str);
    let to = Path::new(to_str);
    let rel = compute_relative(from, to)?;

    let mut out = format!("PATH RELATIVE\n{}\n", "─".repeat(50));
    out.push_str(&format!("From     : {from_str}\n"));
    out.push_str(&format!("To       : {to_str}\n"));
    out.push_str(&format!("Relative : {}\n", rel.display()));
    Ok(out)
}

fn basename(args: &serde_json::Value) -> Result<String, String> {
    let raw = get_path_str(args)?;
    let p = Path::new(&raw);
    let name = p
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();

    let mut out = format!("PATH BASENAME\n{}\n", "─".repeat(50));
    out.push_str(&format!("Input    : {raw}\n"));
    out.push_str(&format!("Basename : {name}\n"));
    Ok(out)
}

fn stem(args: &serde_json::Value) -> Result<String, String> {
    let raw = get_path_str(args)?;
    // Optional 'suffix' to strip a specific extension
    let p = Path::new(&raw);
    let file_stem = p
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();

    let mut out = format!("PATH STEM\n{}\n", "─".repeat(50));
    out.push_str(&format!("Input : {raw}\n"));
    out.push_str(&format!("Stem  : {file_stem}\n"));
    Ok(out)
}

fn extension(args: &serde_json::Value) -> Result<String, String> {
    let raw = get_path_str(args)?;
    let p = Path::new(&raw);

    // Optional 'replace' to swap the extension
    let replace = args.get("replace").and_then(|v| v.as_str());

    let current_ext = p
        .extension()
        .map(|e| e.to_string_lossy().into_owned())
        .unwrap_or_default();

    let mut out = format!("PATH EXTENSION\n{}\n", "─".repeat(50));
    out.push_str(&format!("Input     : {raw}\n"));
    out.push_str(&format!(
        "Extension : {}\n",
        if current_ext.is_empty() {
            "(none)"
        } else {
            &current_ext
        }
    ));

    if let Some(new_ext) = replace {
        let new_path = p.with_extension(new_ext);
        out.push_str(&format!("Replaced  : {}\n", new_path.display()));
    }

    Ok(out)
}

fn is_absolute(args: &serde_json::Value) -> Result<String, String> {
    let raw = get_path_str(args)?;
    let p = Path::new(&raw);
    let abs = p.is_absolute();

    let mut out = format!("PATH IS-ABSOLUTE\n{}\n", "─".repeat(50));
    out.push_str(&format!("Input    : {raw}\n"));
    out.push_str(&format!(
        "Result   : {}\n",
        if abs {
            "YES — absolute path"
        } else {
            "NO — relative path"
        }
    ));
    Ok(out)
}
