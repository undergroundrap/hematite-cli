use serde_json::Value;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub async fn execute(args: &Value) -> Result<String, String> {
    let root = if let Some(r) = args.get("_root").and_then(|v| v.as_str()) {
        PathBuf::from(r)
    } else {
        crate::tools::file_ops::workspace_root()
    };

    let mut sections: Vec<String> = Vec::new();
    let mut manifests_found = 0usize;

    // ── Rust (Cargo.toml) ────────────────────────────────────────────────────
    if let Some(s) = audit_cargo(&root) {
        sections.push(s);
        manifests_found += 1;
    }

    // ── Node.js (package.json) ───────────────────────────────────────────────
    if let Some(s) = audit_npm(&root) {
        sections.push(s);
        manifests_found += 1;
    }

    // ── Python (requirements.txt / pyproject.toml / setup.cfg) ──────────────
    if let Some(s) = audit_python(&root) {
        sections.push(s);
        manifests_found += 1;
    }

    // ── Go (go.mod) ──────────────────────────────────────────────────────────
    if let Some(s) = audit_go(&root) {
        sections.push(s);
        manifests_found += 1;
    }

    if manifests_found == 0 {
        return Ok("dependency_audit: no supported manifest files found.\n\
             Supports: Cargo.toml (Rust), package.json (Node.js), \
             requirements.txt / pyproject.toml (Python), go.mod (Go)."
            .to_string());
    }

    let mut out = format!(
        "DEPENDENCY AUDIT\n{}\n\
         Manifests scanned: {manifests_found}\n\n",
        "─".repeat(60)
    );
    out.push_str(&sections.join("\n"));
    out.push_str(
        "\n── Recommendations ──\n\
         • Run `cargo update` / `npm update` / `pip install --upgrade` to update dependencies.\n\
         • Use `cargo audit` (Rust), `npm audit` (Node), or `safety check` (Python) for CVE scanning.\n\
         • Pin major versions in production; allow minor/patch updates for security fixes.\n\
         • Add a CI step (Dependabot / Renovate) to catch outdated deps automatically.",
    );

    Ok(out)
}

// ── Cargo.toml ───────────────────────────────────────────────────────────────

fn audit_cargo(root: &Path) -> Option<String> {
    let cargo_toml = root.join("Cargo.toml");
    let text = std::fs::read_to_string(&cargo_toml).ok()?;

    let mut deps: BTreeMap<String, String> = BTreeMap::new();
    let mut section = "";
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            section = if trimmed.contains("dependencies") {
                "dep"
            } else {
                ""
            };
            continue;
        }
        if section != "dep" || trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if let Some((name, rest)) = trimmed.split_once('=') {
            let name = name.trim().to_string();
            let version = extract_cargo_version(rest.trim());
            deps.insert(name, version);
        }
    }

    let mut flags: Vec<String> = Vec::new();

    // Flag wildcard versions
    for (name, ver) in &deps {
        if ver == "*" {
            flags.push(format!(
                "  [WILDCARD] {name} = \"*\" — unpinned, any version will match"
            ));
        }
        // Flag very old-looking major versions for common crates
        flag_outdated_crate(name, ver, &mut flags);
    }

    // Check for Cargo.lock
    let lock_exists = root.join("Cargo.lock").exists();
    let workspace_lock = root.join("..").join("Cargo.lock").exists();

    let mut out = format!(
        "RUST (Cargo.toml)\n\
         Dependencies listed: {}\n\
         Cargo.lock present : {}\n",
        deps.len(),
        if lock_exists || workspace_lock {
            "yes"
        } else {
            "NO — add to version control for reproducible builds"
        }
    );

    if flags.is_empty() {
        out.push_str("No obvious issues detected.\n");
    } else {
        out.push_str("Issues found:\n");
        for f in &flags {
            out.push_str(f);
            out.push('\n');
        }
    }

    // Detect dev-only security-relevant crates that should be non-dev
    if text.contains("openssl") && !text.contains("rustls") {
        out.push_str(
            "  [INFO] openssl dependency found — consider rustls for pure-Rust TLS \
             (no C library required, better portability).\n",
        );
    }
    if text.contains("unsafe") && text.contains("[profile.release]") {
        // Can't check unsafe usage from toml, skip
    }

    out.push('\n');
    Some(out)
}

fn extract_cargo_version(rest: &str) -> String {
    let rest = rest.trim().trim_matches('"');
    if rest.starts_with('{') {
        // { version = "1.0", features = [...] }
        if let Some(start) = rest.find("version") {
            let after = &rest[start + 7..].trim_start_matches([' ', '=', '"', ' ']);
            if let Some(end) = after.find(['"', ',', '}']) {
                return after[..end].to_string();
            }
        }
        return "(complex)".to_string();
    }
    rest.trim_matches('"').to_string()
}

fn flag_outdated_crate(name: &str, ver: &str, flags: &mut Vec<String>) {
    // Known minimum expected major versions for common Rust crates (as of 2025)
    let known_minimums: &[(&str, u64, &str)] = &[
        ("tokio", 1, "1.x"),
        ("serde", 1, "1.x"),
        ("serde_json", 1, "1.x"),
        ("reqwest", 0, "0.12+"),
        ("clap", 4, "4.x"),
        ("axum", 0, "0.7+"),
        ("actix-web", 4, "4.x"),
        ("sqlx", 0, "0.7+"),
        ("hyper", 1, "1.x"),
        ("tracing", 0, "0.1+"),
    ];
    let major = ver
        .trim_start_matches(['^', '~', '=', '>', '<', ' '])
        .split('.')
        .next()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(0);

    for (crate_name, min_major, expected) in known_minimums {
        if name == *crate_name && major < *min_major {
            flags.push(format!(
                "  [OUTDATED] {name} = \"{ver}\" — expected {expected}; upgrade recommended"
            ));
        }
    }
}

// ── package.json ─────────────────────────────────────────────────────────────

fn audit_npm(root: &Path) -> Option<String> {
    let pkg_json = root.join("package.json");
    let text = std::fs::read_to_string(&pkg_json).ok()?;
    let json: Value = serde_json::from_str(&text).ok()?;

    let mut dep_count = 0usize;
    let mut dev_dep_count = 0usize;
    let mut issues: Vec<String> = Vec::new();

    for key in &["dependencies", "peerDependencies"] {
        if let Some(deps) = json.get(key).and_then(|v| v.as_object()) {
            for (name, ver) in deps {
                dep_count += 1;
                let ver_str = ver.as_str().unwrap_or("?");
                flag_npm_version(name, ver_str, &mut issues);
            }
        }
    }
    if let Some(deps) = json.get("devDependencies").and_then(|v| v.as_object()) {
        dev_dep_count = deps.len();
    }

    let node_modules = root.join("node_modules");
    let lock_yarn = root.join("yarn.lock").exists();
    let lock_npm = root.join("package-lock.json").exists();
    let lock_pnpm = root.join("pnpm-lock.yaml").exists();
    let has_lock = lock_yarn || lock_npm || lock_pnpm;

    let name = json
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("(unnamed)");
    let version = json.get("version").and_then(|v| v.as_str()).unwrap_or("?");
    let engines = json
        .get("engines")
        .map(|e| format!("{e}"))
        .unwrap_or_default();

    let mut out = format!(
        "NODE.JS (package.json)\n\
         Package         : {name} v{version}\n\
         Dependencies    : {dep_count} prod, {dev_dep_count} dev\n\
         Lock file       : {}\n",
        if has_lock {
            if lock_yarn {
                "yarn.lock"
            } else if lock_npm {
                "package-lock.json"
            } else {
                "pnpm-lock.yaml"
            }
        } else {
            "MISSING — run npm install / yarn install"
        }
    );
    if !engines.is_empty() {
        out.push_str(&format!("Node engines req : {engines}\n"));
    }
    if node_modules.exists() {
        out.push_str("node_modules     : present\n");
    } else {
        out.push_str("node_modules     : NOT found — run npm install\n");
    }

    if issues.is_empty() {
        out.push_str("No obvious version issues detected.\n");
    } else {
        out.push_str("Issues found:\n");
        for i in &issues {
            out.push_str(i);
            out.push('\n');
        }
    }
    out.push('\n');
    Some(out)
}

fn flag_npm_version(name: &str, ver: &str, issues: &mut Vec<String>) {
    if ver == "*" || ver == "latest" {
        issues.push(format!(
            "  [PINNING] {name}: \"{ver}\" — unpinned; specify an exact or range version"
        ));
        return;
    }
    // Warn on known deprecated packages
    let deprecated = &[
        "request",
        "node-uuid",
        "querystring",
        "url",
        "punycode",
        "rimraf@2",
        "glob@7",
        "lodash@3",
        "lodash@4.17.0",
    ];
    for dep in deprecated {
        if dep.contains('@') {
            let parts: Vec<&str> = dep.splitn(2, '@').collect();
            if name == parts[0] && ver.starts_with(parts[1]) {
                issues.push(format!(
                    "  [DEPRECATED] {name}@{ver} — known deprecated version"
                ));
            }
        } else if name == *dep {
            issues.push(format!(
                "  [DEPRECATED] {name} — package is deprecated; find an alternative"
            ));
        }
    }
}

// ── Python ───────────────────────────────────────────────────────────────────

fn audit_python(root: &Path) -> Option<String> {
    let req_txt = root.join("requirements.txt");
    let pyproject = root.join("pyproject.toml");
    let setup_cfg = root.join("setup.cfg");
    let setup_py = root.join("setup.py");

    let mut packages: Vec<(String, String)> = Vec::new();
    let source: &str;

    if req_txt.exists() {
        source = "requirements.txt";
        if let Ok(text) = std::fs::read_to_string(&req_txt) {
            for line in text.lines() {
                let line = line.trim();
                if line.is_empty() || line.starts_with('#') || line.starts_with('-') {
                    continue;
                }
                let (name, ver) = split_python_req(line);
                packages.push((name, ver));
            }
        }
    } else if pyproject.exists() {
        source = "pyproject.toml";
        if let Ok(text) = std::fs::read_to_string(&pyproject) {
            let mut in_deps = false;
            for line in text.lines() {
                let trimmed = line.trim();
                if trimmed.contains("dependencies") && trimmed.contains('=') {
                    in_deps = true;
                    continue;
                }
                if in_deps {
                    if trimmed.starts_with('[') {
                        in_deps = false;
                        continue;
                    }
                    let clean = trimmed.trim_matches(['"', '\'', ',', ' ']);
                    if !clean.is_empty() && !clean.starts_with('#') {
                        let (name, ver) = split_python_req(clean);
                        packages.push((name, ver));
                    }
                }
            }
        }
    } else if setup_cfg.exists() || setup_py.exists() {
        source = if setup_cfg.exists() {
            "setup.cfg"
        } else {
            "setup.py"
        };
    } else {
        return None;
    }

    let venv = root.join(".venv").exists() || root.join("venv").exists();
    let mut issues: Vec<String> = Vec::new();

    for (name, ver) in &packages {
        flag_python_package(name, ver, &mut issues);
    }

    let mut out = format!(
        "PYTHON ({source})\n\
         Packages listed : {}\n\
         Virtual env     : {}\n",
        packages.len(),
        if venv {
            "present"
        } else {
            "not found (.venv / venv)"
        }
    );

    if issues.is_empty() {
        out.push_str("No obvious version issues detected.\n");
    } else {
        out.push_str("Issues found:\n");
        for i in &issues {
            out.push_str(i);
            out.push('\n');
        }
    }
    out.push('\n');
    Some(out)
}

fn split_python_req(line: &str) -> (String, String) {
    for sep in &["==", ">=", "<=", "~=", "!=", ">", "<", "@"] {
        if let Some(pos) = line.find(sep) {
            return (
                line[..pos].trim().to_string(),
                line[pos..].trim().to_string(),
            );
        }
    }
    (line.trim().to_string(), "unpinned".to_string())
}

fn flag_python_package(name: &str, ver: &str, issues: &mut Vec<String>) {
    let lower = name.to_lowercase();
    if ver == "unpinned" {
        issues.push(format!(
            "  [PINNING] {name}: no version pin — add ==X.Y.Z for reproducible installs"
        ));
        return;
    }
    // Known deprecated / insecure
    let deprecated = &[
        "pycrypto", // replaced by pycryptodome
        "md5",      // stdlib-only, never a pip package
        "sha",      // deprecated stdlib
        "urllib3",  // ok but flag very old
        "requests", // ok but note if using 1.x
    ];
    for dep in deprecated {
        if lower == *dep {
            issues.push(format!(
                "  [INFO] {name}: review for deprecation or known CVEs"
            ));
        }
    }
}

// ── go.mod ───────────────────────────────────────────────────────────────────

fn audit_go(root: &Path) -> Option<String> {
    let go_mod = root.join("go.mod");
    let text = std::fs::read_to_string(&go_mod).ok()?;

    let mut module_name = String::new();
    let mut go_version = String::new();
    let mut direct_deps: Vec<(String, String)> = Vec::new();
    let mut indirect_deps = 0usize;
    let mut in_require = false;

    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("module ") {
            module_name = trimmed[7..].trim().to_string();
        } else if trimmed.starts_with("go ") {
            go_version = trimmed[3..].trim().to_string();
        } else if trimmed == "require (" {
            in_require = true;
        } else if trimmed == ")" {
            in_require = false;
        } else if in_require && !trimmed.is_empty() && !trimmed.starts_with("//") {
            let parts: Vec<&str> = trimmed.splitn(3, ' ').collect();
            if parts.len() >= 2 {
                let is_indirect = trimmed.contains("// indirect");
                if is_indirect {
                    indirect_deps += 1;
                } else {
                    direct_deps.push((parts[0].to_string(), parts[1].to_string()));
                }
            }
        } else if trimmed.starts_with("require ") && !trimmed.contains('(') {
            let rest = &trimmed[8..];
            let parts: Vec<&str> = rest.splitn(3, ' ').collect();
            if parts.len() >= 2 {
                direct_deps.push((parts[0].to_string(), parts[1].to_string()));
            }
        }
    }

    let go_sum = root.join("go.sum").exists();
    let vendor = root.join("vendor").exists();

    let out = format!(
        "GO (go.mod)\n\
         Module          : {module_name}\n\
         Go version      : {go_version}\n\
         Direct deps     : {}\n\
         Indirect deps   : {indirect_deps}\n\
         go.sum present  : {}\n\
         vendor dir      : {}\n\
         No obvious issues detected (run `go mod tidy` and `govulncheck ./...` for full audit).\n\n",
        direct_deps.len(),
        if go_sum { "yes" } else { "NO — run go mod tidy" },
        if vendor { "yes (vendored)" } else { "no" }
    );

    Some(out)
}
