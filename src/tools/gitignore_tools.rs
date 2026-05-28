use serde_json::Value;

pub async fn execute(args: &Value) -> Result<String, String> {
    let action = if let Some(a) = args.get("action").and_then(|v| v.as_str()) {
        a.to_string()
    } else if args.get("path").is_some() || args.get("file").is_some() {
        "check".to_string()
    } else if args.get("language").is_some() || args.get("lang").is_some() {
        "generate".to_string()
    } else {
        "parse".to_string()
    };
    match action.as_str() {
        "parse" => parse_action(args),
        "check" => check_action(args),
        "generate" => generate_action(args),
        "explain" => explain_action(args),
        _ => Err(format!(
            "Unknown action '{}'. Valid: parse, check, generate, explain",
            action
        )),
    }
}

fn get_gitignore(args: &Value) -> Result<String, String> {
    args.get("text")
        .or_else(|| args.get("gitignore"))
        .or_else(|| args.get("content"))
        .or_else(|| args.get("input"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| "Missing 'text' — pass the .gitignore content as a string".to_string())
}

#[derive(Debug, Clone)]
struct GitignorePattern {
    raw: String,
    negated: bool,
    dir_only: bool,
    anchored: bool,
    segments: Vec<String>,
}

impl GitignorePattern {
    fn parse(raw: &str) -> Self {
        let mut s = raw.to_string();
        let negated = s.starts_with('!');
        if negated {
            s = s[1..].to_string();
        }
        let dir_only = s.ends_with('/');
        if dir_only {
            s = s.trim_end_matches('/').to_string();
        }
        // anchored = contains slash other than trailing
        let anchored = s.contains('/');
        let segments: Vec<String> = s.split('/').map(|p| p.to_string()).collect();
        GitignorePattern {
            raw: raw.to_string(),
            negated,
            dir_only,
            anchored,
            segments,
        }
    }

    fn is_global(&self) -> bool {
        !self.anchored && self.segments.len() == 1
    }
}

fn parse_gitignore(text: &str) -> Vec<GitignorePattern> {
    text.lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(|l| GitignorePattern::parse(l))
        .collect()
}

fn pattern_matches_path(pat: &GitignorePattern, path: &str) -> bool {
    let path_norm = path.replace('\\', "/");
    let path_lower = path_norm.to_lowercase();

    if pat.segments.len() == 1 {
        let seg = &pat.segments[0];
        let seg_lower = seg.to_lowercase();
        // match against last component or any component
        let filename = path_lower.split('/').last().unwrap_or(&path_lower);
        if glob_segment_match(&seg_lower, filename) {
            return true;
        }
        // also check any path component for global patterns
        if pat.is_global() {
            for component in path_lower.split('/') {
                if glob_segment_match(&seg_lower, component) {
                    return true;
                }
            }
        }
        return false;
    }

    // multi-segment: match path suffix
    let pat_str: String = pat
        .segments
        .iter()
        .map(|s| s.as_str())
        .collect::<Vec<_>>()
        .join("/");
    let pat_lower = pat_str.to_lowercase();
    if pat.anchored {
        // must match from root or from start of path
        path_lower == pat_lower
            || path_lower.starts_with(&format!("{}/", pat_lower))
            || path_lower.ends_with(&format!("/{}", pat_lower))
    } else {
        path_lower.contains(&pat_lower)
    }
}

fn glob_segment_match(pattern: &str, text: &str) -> bool {
    if pattern == "*" {
        return true;
    }
    if !pattern.contains('*') && !pattern.contains('?') && !pattern.contains('[') {
        return pattern == text;
    }
    glob_match(pattern, text)
}

fn glob_match(pattern: &str, text: &str) -> bool {
    let pb = pattern.as_bytes();
    let tb = text.as_bytes();
    let mut pi = 0usize;
    let mut ti = 0usize;
    let mut star_pi = usize::MAX;
    let mut star_ti = 0usize;
    while ti < tb.len() {
        if pi < pb.len() && (pb[pi] == b'?' || pb[pi] == tb[ti]) {
            pi += 1;
            ti += 1;
        } else if pi < pb.len() && pb[pi] == b'*' {
            star_pi = pi;
            star_ti = ti;
            pi += 1;
        } else if star_pi != usize::MAX {
            star_ti += 1;
            ti = star_ti;
            pi = star_pi + 1;
        } else {
            return false;
        }
    }
    while pi < pb.len() && pb[pi] == b'*' {
        pi += 1;
    }
    pi == pb.len()
}

fn parse_action(args: &Value) -> Result<String, String> {
    let text = get_gitignore(args)?;
    let lines: Vec<&str> = text.lines().collect();
    let mut out = format!(
        ".gitignore  [{} lines]\n{}\n\n",
        lines.len(),
        "=".repeat(44)
    );

    let mut section = String::new();
    let mut pattern_count = 0usize;
    let mut negated_count = 0usize;
    let mut comment_count = 0usize;

    for line in &lines {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            if !section.is_empty() {
                out += "\n";
            }
            section.clear();
            continue;
        }
        if trimmed.starts_with('#') {
            comment_count += 1;
            let comment = &trimmed[1..].trim().to_string();
            if !comment.is_empty() {
                if section.is_empty() {
                    section = comment.to_string();
                    out += &format!("[{}]\n", comment);
                }
            }
            continue;
        }
        let pat = GitignorePattern::parse(trimmed);
        pattern_count += 1;
        let flags = if pat.negated {
            negated_count += 1;
            " [UNIGNORE]"
        } else if pat.dir_only {
            " [DIR]"
        } else {
            ""
        };
        out += &format!("  {}{}\n", trimmed, flags);
    }

    out += &format!(
        "\n{} pattern(s)  {} comment(s)  {} negated\n",
        pattern_count, comment_count, negated_count
    );
    Ok(out)
}

fn check_action(args: &Value) -> Result<String, String> {
    let text = get_gitignore(args)?;
    let path = args
        .get("path")
        .or_else(|| args.get("file"))
        .and_then(|v| v.as_str())
        .ok_or("Missing 'path' — the file path to test (e.g. 'dist/bundle.js')")?;

    let patterns = parse_gitignore(&text);
    let mut ignored = false;
    let mut matched_by: Option<String> = None;

    for pat in &patterns {
        if pattern_matches_path(pat, path) {
            if pat.negated {
                ignored = false;
                matched_by = Some(format!("!{} (unignored)", &pat.raw[1..]));
            } else {
                ignored = true;
                matched_by = Some(pat.raw.clone());
            }
        }
    }

    let mut out = format!(".gitignore Check\n{}\n\n", "=".repeat(44));
    out += &format!("Path: {}\n\n", path);
    out += if ignored {
        "Result: IGNORED\n"
    } else {
        "Result: NOT IGNORED\n"
    };
    if let Some(rule) = matched_by {
        out += &format!("Matched by: {}\n", rule);
    } else {
        out += "No matching pattern found — file is tracked by git.\n";
    }
    Ok(out)
}

fn generate_action(args: &Value) -> Result<String, String> {
    let lang = args
        .get("language")
        .or_else(|| args.get("lang"))
        .and_then(|v| v.as_str())
        .ok_or("Missing 'language' — e.g. 'rust', 'node', 'python', 'go', 'java'")?
        .to_lowercase();

    let (name, patterns) = match lang.as_str() {
        "rust" => (
            "Rust",
            vec![
                "# Build output",
                "/target/",
                "**/*.rs.bk",
                "",
                "# IDE",
                ".idea/",
                ".vscode/",
                "*.iml",
                "",
                "# Misc",
                ".DS_Store",
                "Thumbs.db",
            ],
        ),
        "node" | "nodejs" | "javascript" | "js" | "typescript" | "ts" => (
            "Node.js",
            vec![
                "# Dependencies",
                "node_modules/",
                "",
                "# Build output",
                "dist/",
                "build/",
                ".next/",
                ".nuxt/",
                "",
                "# Environment",
                ".env",
                ".env.local",
                ".env.*.local",
                "",
                "# Logs",
                "*.log",
                "npm-debug.log*",
                "yarn-debug.log*",
                "",
                "# IDE",
                ".idea/",
                ".vscode/",
                "",
                "# Misc",
                ".DS_Store",
                "Thumbs.db",
                "coverage/",
            ],
        ),
        "python" | "py" => (
            "Python",
            vec![
                "# Bytecode",
                "__pycache__/",
                "*.py[cod]",
                "*$py.class",
                "",
                "# Virtual envs",
                ".venv/",
                "venv/",
                "env/",
                ".env",
                "",
                "# Distribution",
                "dist/",
                "build/",
                "*.egg-info/",
                "*.egg",
                "",
                "# Testing",
                ".pytest_cache/",
                ".coverage",
                "htmlcov/",
                "",
                "# IDE",
                ".idea/",
                ".vscode/",
                "",
                "# Misc",
                ".DS_Store",
                "Thumbs.db",
            ],
        ),
        "go" | "golang" => (
            "Go",
            vec![
                "# Binaries",
                "*.exe",
                "*.exe~",
                "*.dll",
                "*.so",
                "*.dylib",
                "",
                "# Test binary",
                "*.test",
                "",
                "# Output",
                "*.out",
                "",
                "# Vendor",
                "/vendor/",
                "",
                "# IDE",
                ".idea/",
                ".vscode/",
                "",
                "# Misc",
                ".DS_Store",
                "Thumbs.db",
            ],
        ),
        "java" => (
            "Java",
            vec![
                "# Compiled",
                "*.class",
                "*.jar",
                "*.war",
                "*.nar",
                "*.ear",
                "*.zip",
                "",
                "# Maven",
                "target/",
                "pom.xml.tag",
                "pom.xml.releaseBackup",
                "",
                "# Gradle",
                ".gradle/",
                "build/",
                "",
                "# IDE",
                ".idea/",
                ".vscode/",
                "*.iml",
                "",
                "# Misc",
                ".DS_Store",
                "Thumbs.db",
            ],
        ),
        "dotnet" | "csharp" | "c#" | ".net" => (
            "C# / .NET",
            vec![
                "# Build output",
                "bin/",
                "obj/",
                "",
                "# NuGet",
                "*.nupkg",
                "packages/",
                "",
                "# IDE",
                ".vs/",
                ".idea/",
                "*.suo",
                "*.user",
                "",
                "# Misc",
                ".DS_Store",
                "Thumbs.db",
            ],
        ),
        "react" => (
            "React",
            vec![
                "node_modules/",
                "build/",
                ".env",
                ".env.local",
                ".env.development.local",
                ".env.test.local",
                ".env.production.local",
                "npm-debug.log*",
                "yarn-debug.log*",
                "yarn-error.log*",
                ".DS_Store",
                "Thumbs.db",
            ],
        ),
        "docker" => (
            "Docker",
            vec![
                "# Docker volumes and temp",
                ".dockerignore",
                "docker-compose.override.yml",
                "",
                "# Environment",
                ".env",
                ".env.*",
                "!.env.example",
                "",
                "# Misc",
                ".DS_Store",
                "Thumbs.db",
            ],
        ),
        _ => {
            return Err(format!(
            "Unknown language '{}'. Supported: rust, node, python, go, java, dotnet, react, docker",
            lang
        ))
        }
    };

    let content = patterns.join("\n");
    let mut out = format!(
        ".gitignore for {} ({} lines)\n{}\n\n",
        name,
        patterns.len(),
        "=".repeat(44)
    );
    out += &content;
    out += "\n";
    Ok(out)
}

fn explain_action(args: &Value) -> Result<String, String> {
    let text = get_gitignore(args)?;
    let patterns = parse_gitignore(&text);

    let mut out = format!(".gitignore Explanation\n{}\n\n", "=".repeat(44));
    for pat in &patterns {
        out += &format!("Pattern: {}\n", pat.raw);
        if pat.negated {
            out += "  Effect:  UNIGNORES — re-includes files previously ignored\n";
        } else {
            out += "  Effect:  IGNORES — git will not track matching files\n";
        }
        if pat.dir_only {
            out += "  Scope:   Directories only (trailing /)\n";
        }
        if pat.anchored {
            out += "  Scope:   Anchored to repository root (contains /)\n";
        } else {
            out += "  Scope:   Matches anywhere in the tree\n";
        }
        let has_glob = pat.raw.contains('*') || pat.raw.contains('?') || pat.raw.contains('[');
        if has_glob {
            if pat.raw.contains("**") {
                out += "  Glob:    ** matches zero or more path segments\n";
            }
            if pat.raw.contains('*') && !pat.raw.contains("**") {
                out += "  Glob:    * matches within a single path segment\n";
            }
            if pat.raw.contains('?') {
                out += "  Glob:    ? matches any single character\n";
            }
        }
        out += "\n";
    }
    if patterns.is_empty() {
        out += "No patterns found (file is empty or only comments).\n";
    }
    Ok(out)
}
