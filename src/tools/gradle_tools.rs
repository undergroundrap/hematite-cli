use serde_json::{json, Value};

pub fn make_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "action": {
                "type": "string",
                "description": "info (default) | deps | tasks | plugins | properties | validate"
            },
            "gradle": {
                "type": "string",
                "description": "Inline build.gradle or build.gradle.kts content"
            },
            "file": {
                "type": "string",
                "description": "Path to build.gradle, build.gradle.kts, or settings.gradle"
            },
            "configuration": {
                "type": "string",
                "description": "For deps: filter by configuration (implementation/testImplementation/api/etc.)"
            },
            "filter": {
                "type": "string",
                "description": "For deps/plugins/tasks: substring filter on name"
            }
        }
    })
}

pub async fn execute(args: &Value) -> Result<String, String> {
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("info");

    let (text, filename) = if let Some(f) = args.get("file").and_then(|v| v.as_str()) {
        let content =
            std::fs::read_to_string(f).map_err(|e| format!("Cannot read file '{}': {}", f, e))?;
        let fname = std::path::Path::new(f)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(f)
            .to_string();
        (content, fname)
    } else if let Some(t) = args
        .get("gradle")
        .or_else(|| args.get("text"))
        .and_then(|v| v.as_str())
    {
        (t.to_string(), "build.gradle".to_string())
    } else {
        return Err(
            "Provide 'file' (path to build.gradle or build.gradle.kts) or 'gradle' (inline content).".into(),
        );
    };

    let is_kts = filename.ends_with(".kts");

    match action {
        "deps" | "dependencies" => do_deps(&text, is_kts, args),
        "tasks" => do_tasks(&text, is_kts, args),
        "plugins" => do_plugins(&text, is_kts, args),
        "properties" | "props" | "ext" => do_properties(&text, is_kts),
        "validate" | "check" => do_validate(&text, is_kts),
        _ => do_info(&text, &filename, is_kts),
    }
}

// ── Tokenization helpers ──────────────────────────────────────────────────────

fn strip_line_comments(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut in_block = false;
    let mut in_str = false;
    let mut str_char = ' ';
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if in_block {
            if c == '*' && chars.peek() == Some(&'/') {
                chars.next();
                in_block = false;
            }
            // replace block comment chars with spaces to preserve offsets
            out.push(' ');
            continue;
        }
        if in_str {
            out.push(c);
            if c == str_char {
                in_str = false;
            }
            continue;
        }
        if (c == '"' || c == '\'') {
            in_str = true;
            str_char = c;
            out.push(c);
            continue;
        }
        if c == '/' && chars.peek() == Some(&'/') {
            // line comment — skip to end of line
            while let Some(nc) = chars.next() {
                if nc == '\n' {
                    out.push('\n');
                    break;
                }
            }
            continue;
        }
        if c == '/' && chars.peek() == Some(&'*') {
            chars.next();
            in_block = true;
            out.push(' ');
            out.push(' ');
            continue;
        }
        out.push(c);
    }
    out
}

// ── Dependency parsing ────────────────────────────────────────────────────────

#[derive(Debug)]
struct GradleDep {
    configuration: String,
    notation: String,
    group: String,
    artifact: String,
    version: String,
    kind: String, // "gav" | "project" | "files" | "platform"
}

fn parse_dep_notation(raw: &str) -> (String, String, String, String) {
    let s = raw.trim().trim_matches(|c| c == '"' || c == '\'');
    // Handle platform(), enforcedPlatform() wrappers
    let (wrapper, inner) = if s.starts_with("platform(") || s.starts_with("enforcedPlatform(") {
        let kind = if s.starts_with("enforcedPlatform") {
            "enforcedPlatform"
        } else {
            "platform"
        };
        let inner = s
            .find('(')
            .map(|i| s[i + 1..].trim_end_matches(')').trim())
            .unwrap_or(s);
        (kind, inner.trim_matches(|c| c == '"' || c == '\''))
    } else {
        ("", s)
    };

    if inner.starts_with("project(") || inner == "project" {
        let proj = inner.trim_start_matches("project(").trim_end_matches(')');
        return (
            String::new(),
            proj.trim_matches(|c| c == '"' || c == '\'').to_string(),
            String::new(),
            "project".into(),
        );
    }
    if inner.starts_with("files(") {
        return (
            String::new(),
            inner.to_string(),
            String::new(),
            "files".into(),
        );
    }

    // GAV string "group:artifact:version"
    let parts: Vec<&str> = inner.splitn(3, ':').collect();
    let group = parts.get(0).unwrap_or(&"").to_string();
    let artifact = parts.get(1).unwrap_or(&"").to_string();
    let version = parts.get(2).unwrap_or(&"").to_string();
    let kind = if wrapper.is_empty() {
        "gav".to_string()
    } else {
        wrapper.to_string()
    };
    (group, artifact, version, kind)
}

fn collect_deps(text: &str, _is_kts: bool) -> Vec<GradleDep> {
    let clean = strip_line_comments(text);
    let mut deps = Vec::new();

    // Match dependency configuration blocks
    // Groovy/KTS: dependencies { configName "g:a:v" }
    // Also: configName(group = "g", name = "a", version = "v")
    let dep_configs = [
        "implementation",
        "api",
        "compileOnly",
        "runtimeOnly",
        "testImplementation",
        "testCompileOnly",
        "testRuntimeOnly",
        "annotationProcessor",
        "kapt",
        "ksp",
        "debugImplementation",
        "releaseImplementation",
        "classpath",
        "provided",
        "compile",
        "runtime",
        "testCompile",
        "testRuntime",
        "androidTestImplementation",
        "androidTestCompileOnly",
        "integrationTestImplementation",
    ];

    let lines: Vec<&str> = clean.lines().collect();
    for line in &lines {
        let trimmed = line.trim();
        for config in &dep_configs {
            // Match: config "g:a:v" or config("g:a:v") or config 'g:a:v'
            if !trimmed.starts_with(config) {
                continue;
            }
            let after = trimmed[config.len()..].trim_start();
            if after.is_empty() {
                continue;
            }
            // Must start with ( or " or '
            let notation_raw = if after.starts_with('(') {
                // strip outer parens if present
                let inner = after.trim_start_matches('(').trim_end_matches(')');
                inner.trim()
            } else if after.starts_with('"') || after.starts_with('\'') {
                after
            } else {
                continue;
            };

            // Handle group:name:version map notation (Groovy)
            if notation_raw.contains("group:") || notation_raw.contains("group =") {
                let g = extract_map_val(notation_raw, "group");
                let a = extract_map_val(notation_raw, "name");
                let v = extract_map_val(notation_raw, "version");
                deps.push(GradleDep {
                    configuration: config.to_string(),
                    notation: format!("{}:{}:{}", g, a, v),
                    group: g,
                    artifact: a,
                    version: v,
                    kind: "gav".into(),
                });
                continue;
            }

            // Strip enclosing quotes / parens
            let stripped = strip_outer_quotes(notation_raw);
            if stripped.is_empty() || stripped.starts_with('{') {
                continue;
            }
            let (g, a, v, kind) = parse_dep_notation(stripped);
            let notation = if kind == "project" || kind == "files" {
                stripped.to_string()
            } else {
                format!("{}:{}:{}", g, a, v)
            };
            deps.push(GradleDep {
                configuration: config.to_string(),
                notation,
                group: g,
                artifact: a,
                version: v,
                kind,
            });
        }
    }
    deps
}

fn strip_outer_quotes(s: &str) -> &str {
    let s = s.trim();
    if (s.starts_with('"') && s.ends_with('"')) || (s.starts_with('\'') && s.ends_with('\'')) {
        &s[1..s.len() - 1]
    } else {
        s
    }
}

fn extract_map_val(s: &str, key: &str) -> String {
    // handles: group: "value" or group = "value"
    let patterns = [
        format!("{}: \"", key),
        format!("{}: '", key),
        format!("{} = \"", key),
        format!("{} = '", key),
    ];
    for pat in &patterns {
        if let Some(start) = s.find(pat.as_str()) {
            let after = &s[start + pat.len()..];
            let quote = if pat.ends_with('"') { '"' } else { '\'' };
            if let Some(end) = after.find(quote) {
                return after[..end].to_string();
            }
        }
    }
    String::new()
}

// ── Plugin parsing ────────────────────────────────────────────────────────────

#[derive(Debug)]
struct GradlePlugin {
    id: String,
    version: String,
    apply: bool,
}

fn collect_plugins(text: &str, _is_kts: bool) -> Vec<GradlePlugin> {
    let clean = strip_line_comments(text);
    let mut plugins = Vec::new();

    // Plugins block: id("foo") version "bar"  or  id 'foo' version 'bar'
    // Also apply plugin: 'foo' (legacy)
    let lines: Vec<&str> = clean.lines().collect();
    for line in &lines {
        let t = line.trim();

        // Modern plugins block: id("foo.bar") version "1.2" apply false
        if t.starts_with("id(") || t.starts_with("id \"") || t.starts_with("id '") {
            let id = extract_first_string(t);
            let version = if let Some(vi) = t.find("version") {
                let after = &t[vi + "version".len()..].trim_start();
                extract_first_string(after)
            } else {
                String::new()
            };
            let apply = !t.contains("apply false") && !t.contains("apply(false)");
            if !id.is_empty() {
                plugins.push(GradlePlugin { id, version, apply });
            }
            continue;
        }

        // kotlin("jvm") style
        if t.starts_with("kotlin(") {
            let id = format!(
                "org.jetbrains.kotlin.{}",
                extract_first_string(&t["kotlin(".len()..])
            );
            let version = if let Some(vi) = t.find("version") {
                let after = &t[vi + "version".len()..].trim_start();
                extract_first_string(after)
            } else {
                String::new()
            };
            plugins.push(GradlePlugin {
                id,
                version,
                apply: true,
            });
            continue;
        }

        // Legacy: apply plugin: 'foo'
        if t.starts_with("apply plugin:") {
            let id = extract_first_string(&t["apply plugin:".len()..]);
            if !id.is_empty() {
                plugins.push(GradlePlugin {
                    id,
                    version: String::new(),
                    apply: true,
                });
            }
        }
    }
    plugins
}

fn extract_first_string(s: &str) -> String {
    let s = s.trim();
    if s.starts_with('"') {
        if let Some(end) = s[1..].find('"') {
            return s[1..end + 1].to_string();
        }
    } else if s.starts_with('\'') {
        if let Some(end) = s[1..].find('\'') {
            return s[1..end + 1].to_string();
        }
    } else if s.starts_with('(') {
        // strip paren then look for string
        return extract_first_string(&s[1..]);
    }
    String::new()
}

// ── Task parsing ──────────────────────────────────────────────────────────────

#[derive(Debug)]
struct GradleTask {
    name: String,
    task_type: String,
    depends_on: Vec<String>,
    description: String,
}

fn collect_tasks(text: &str, is_kts: bool) -> Vec<GradleTask> {
    let clean = strip_line_comments(text);
    let mut tasks = Vec::new();
    let lines: Vec<&str> = clean.lines().collect();

    for (i, line) in lines.iter().enumerate() {
        let t = line.trim();

        // Groovy: task foo { ... } or task foo(type: Bar) { ... }
        // KTS:    tasks.register("foo") { ... } or val foo by tasks.registering { ... }
        if !is_kts && t.starts_with("task ") {
            let rest = &t["task ".len()..];
            let name_end = rest
                .find(|c: char| c == '(' || c == ' ' || c == '{')
                .unwrap_or(rest.len());
            let name = rest[..name_end].trim().to_string();
            let task_type = if rest.contains("type:") {
                extract_groovy_type(rest)
            } else {
                String::new()
            };
            let depends_on = extract_depends_on_from_block(&lines, i);
            let description = extract_description_from_block(&lines, i);
            if !name.is_empty() {
                tasks.push(GradleTask {
                    name,
                    task_type,
                    depends_on,
                    description,
                });
            }
            continue;
        }

        // KTS: tasks.register("name") or tasks.named("name") or tasks.create("name")
        if t.starts_with("tasks.register(")
            || t.starts_with("tasks.named(")
            || t.starts_with("tasks.create(")
        {
            let name = extract_first_string(&t[t.find('(').unwrap_or(0) + 1..]);
            let task_type = extract_kts_task_type(t);
            let depends_on = extract_depends_on_from_block(&lines, i);
            let description = extract_description_from_block(&lines, i);
            if !name.is_empty() {
                tasks.push(GradleTask {
                    name,
                    task_type,
                    depends_on,
                    description,
                });
            }
        }
    }
    tasks
}

fn extract_groovy_type(s: &str) -> String {
    // type: Bar  or  (type: Bar)
    if let Some(pos) = s.find("type:") {
        let after = s[pos + 5..].trim();
        let end = after
            .find(|c: char| c == ')' || c == ',' || c == ' ' || c == '{')
            .unwrap_or(after.len());
        return after[..end].trim().to_string();
    }
    String::new()
}

fn extract_kts_task_type(s: &str) -> String {
    // tasks.register<Copy>("name")  or  tasks.register("name", Copy::class)
    if let Some(lt) = s.find('<') {
        if let Some(gt) = s[lt..].find('>') {
            return s[lt + 1..lt + gt].trim().to_string();
        }
    }
    String::new()
}

fn extract_depends_on_from_block(lines: &[&str], start: usize) -> Vec<String> {
    let mut deps = Vec::new();
    // look in the next 20 lines for dependsOn
    for line in lines.iter().skip(start).take(20) {
        let t = line.trim();
        if t.starts_with("dependsOn") {
            // dependsOn "task1", "task2"  or  dependsOn(tasks.named("foo"))
            let rest =
                &t["dependsOn".len()..].trim_start_matches(|c| c == '(' || c == ':' || c == ' ');
            for part in rest.split(',') {
                let s = part
                    .trim()
                    .trim_matches(|c| c == '"' || c == '\'' || c == ')');
                if !s.is_empty() && !s.starts_with("tasks.") {
                    deps.push(s.to_string());
                }
            }
        }
        if t == "}" {
            break;
        }
    }
    deps
}

fn extract_description_from_block(lines: &[&str], start: usize) -> String {
    for line in lines.iter().skip(start).take(10) {
        let t = line.trim();
        if t.starts_with("description") {
            let rest = &t["description".len()..];
            let rest = rest.trim_start_matches(|c| c == ' ' || c == '=' || c == ':');
            return extract_first_string(rest);
        }
        if t == "}" {
            break;
        }
    }
    String::new()
}

// ── Property / ext parsing ────────────────────────────────────────────────────

fn collect_properties(text: &str, is_kts: bool) -> Vec<(String, String)> {
    let clean = strip_line_comments(text);
    let mut props = Vec::new();
    for line in clean.lines() {
        let t = line.trim();
        // Groovy ext: ext.foo = "bar"  or  ext { foo = "bar" }
        // KTS: val foo by extra("bar")  or  val foo: String by project
        if is_kts {
            if t.starts_with("val ") && t.contains(" by extra") {
                let rest = &t["val ".len()..];
                let name_end = rest
                    .find(|c: char| !c.is_alphanumeric() && c != '_')
                    .unwrap_or(rest.len());
                let name = rest[..name_end].to_string();
                let val = if let Some(pos) = t.find("extra(") {
                    extract_first_string(&t[pos + "extra(".len()..])
                } else {
                    String::new()
                };
                if !name.is_empty() {
                    props.push((name, val));
                }
                continue;
            }
        } else {
            // ext.foo = "bar"
            if t.starts_with("ext.") {
                let rest = &t["ext.".len()..];
                if let Some(eq) = rest.find('=') {
                    let name = rest[..eq].trim().to_string();
                    let val = extract_first_string(rest[eq + 1..].trim());
                    if !name.is_empty() {
                        props.push((name, val));
                    }
                }
                continue;
            }
        }
        // project.ext["foo"] = "bar" or project.extra["foo"] = "bar" (KTS)
        if t.contains("ext[") || t.contains("extra[") {
            let bracket_start = t.find('[').unwrap_or(0);
            let name = extract_first_string(&t[bracket_start + 1..]);
            let val = if let Some(eq) = t.find('=') {
                extract_first_string(t[eq + 1..].trim())
            } else {
                String::new()
            };
            if !name.is_empty() {
                props.push((name, val));
            }
        }
    }
    props
}

// ── Actions ──────────────────────────────────────────────────────────────────

fn do_info(text: &str, filename: &str, is_kts: bool) -> Result<String, String> {
    let clean = strip_line_comments(text);
    let plugins = collect_plugins(text, is_kts);
    let deps = collect_deps(text, is_kts);
    let tasks = collect_tasks(text, is_kts);
    let props = collect_properties(text, is_kts);

    // Detect group / version
    let group = extract_simple_val(&clean, "group");
    let version = extract_simple_val(&clean, "version");
    let description = extract_simple_val(&clean, "description");

    // Detect Java/Kotlin target
    let java_target = extract_simple_val(&clean, "sourceCompatibility")
        .or_else(|| extract_simple_val(&clean, "jvmTarget"))
        .or_else(|| extract_simple_val(&clean, "targetCompatibility"));

    // Detect repositories
    let repos: Vec<&str> = [
        "mavenCentral()",
        "google()",
        "mavenLocal()",
        "gradlePluginPortal()",
        "jcenter()",
    ]
    .iter()
    .filter(|r| clean.contains(*r))
    .copied()
    .collect();

    // Configuration distribution
    let mut config_counts: std::collections::HashMap<&str, usize> =
        std::collections::HashMap::new();
    for d in &deps {
        *config_counts.entry(d.configuration.as_str()).or_insert(0) += 1;
    }

    let mut out = String::new();
    let build_type = if is_kts {
        "Gradle Kotlin DSL"
    } else {
        "Gradle Groovy DSL"
    };
    out.push_str(&format!("{} — {}\n", build_type, filename));
    out.push_str(&"─".repeat(60));
    out.push('\n');
    if let Some(g) = &group {
        out.push_str(&format!("Group        : {}\n", g));
    }
    if let Some(v) = &version {
        out.push_str(&format!("Version      : {}\n", v));
    }
    if let Some(d) = &description {
        out.push_str(&format!("Description  : {}\n", d));
    }
    if let Some(jt) = &java_target {
        out.push_str(&format!("Java/JVM target: {}\n", jt));
    }
    out.push('\n');

    out.push_str("Counts\n");
    out.push_str(&format!("  Plugins       : {}\n", plugins.len()));
    out.push_str(&format!("  Dependencies  : {}\n", deps.len()));
    out.push_str(&format!("  Custom tasks  : {}\n", tasks.len()));
    out.push_str(&format!("  ext properties: {}\n", props.len()));

    if !repos.is_empty() {
        out.push('\n');
        out.push_str("Repositories\n");
        for r in &repos {
            out.push_str(&format!("  • {}\n", r));
        }
    }

    if !plugins.is_empty() {
        out.push('\n');
        out.push_str(&format!("Applied plugins ({})\n", plugins.len()));
        for p in &plugins {
            let ver = if p.version.is_empty() {
                String::new()
            } else {
                format!(" v{}", p.version)
            };
            let applied = if !p.apply { " [apply=false]" } else { "" };
            out.push_str(&format!("  • {}{}{}\n", p.id, ver, applied));
        }
    }

    if !config_counts.is_empty() {
        out.push('\n');
        out.push_str("Dependency configurations\n");
        let mut pairs: Vec<_> = config_counts.iter().collect();
        pairs.sort_by(|a, b| b.1.cmp(a.1));
        for (config, count) in pairs {
            out.push_str(&format!("  {:<30}  {}\n", config, count));
        }
    }

    Ok(out)
}

fn extract_simple_val(text: &str, key: &str) -> Option<String> {
    // group = "foo"  or  group = 'foo'  or  group("foo")
    for line in text.lines() {
        let t = line.trim();
        if t.starts_with(key) {
            let rest = &t[key.len()..].trim_start();
            if rest.starts_with('=') || rest.starts_with("(\"") || rest.starts_with("('") {
                let after = rest.trim_start_matches('=').trim_start_matches('(').trim();
                let val = extract_first_string(after);
                if !val.is_empty() {
                    return Some(val);
                }
            }
        }
    }
    None
}

fn do_deps(text: &str, is_kts: bool, args: &Value) -> Result<String, String> {
    let config_filter = args
        .get("configuration")
        .and_then(|v| v.as_str())
        .map(|s| s.to_lowercase());
    let name_filter = args
        .get("filter")
        .and_then(|v| v.as_str())
        .map(|s| s.to_lowercase());

    let all_deps = collect_deps(text, is_kts);
    let filtered: Vec<_> = all_deps
        .iter()
        .filter(|d| {
            config_filter
                .as_deref()
                .map(|f| d.configuration.to_lowercase().contains(f))
                .unwrap_or(true)
        })
        .filter(|d| {
            name_filter
                .as_deref()
                .map(|f| {
                    d.group.to_lowercase().contains(f)
                        || d.artifact.to_lowercase().contains(f)
                        || d.notation.to_lowercase().contains(f)
                })
                .unwrap_or(true)
        })
        .collect();

    if filtered.is_empty() {
        return Ok("No dependencies match the given filters.\n".into());
    }

    let mut out = String::new();
    out.push_str(&format!("Dependencies ({})\n", filtered.len()));
    out.push_str(&"─".repeat(80));
    out.push('\n');

    // Group by configuration (insertion-ordered unique list)
    let configs: Vec<&str> = {
        let mut seen: Vec<&str> = Vec::new();
        for d in &filtered {
            let s = d.configuration.as_str();
            if !seen.contains(&s) {
                seen.push(s);
            }
        }
        seen
    };

    for config in configs {
        let group_deps: Vec<_> = filtered
            .iter()
            .filter(|d| d.configuration == config)
            .collect();
        out.push_str(&format!("\n{}  ({})\n", config, group_deps.len()));
        out.push_str(&"─".repeat(50));
        out.push('\n');
        for d in group_deps {
            match d.kind.as_str() {
                "project" => out.push_str(&format!("  project({}) [local]\n", d.artifact)),
                "files" => out.push_str(&format!("  {} [file]\n", d.notation)),
                "platform" => out.push_str(&format!(
                    "  {}:{}:{}  [platform BOM]\n",
                    d.group, d.artifact, d.version
                )),
                "enforcedPlatform" => out.push_str(&format!(
                    "  {}:{}:{}  [enforcedPlatform BOM]\n",
                    d.group, d.artifact, d.version
                )),
                _ => {
                    let ver = if d.version.is_empty() {
                        "(no version)"
                    } else {
                        &d.version
                    };
                    out.push_str(&format!("  {}:{}  {}\n", d.group, d.artifact, ver));
                }
            }
        }
    }

    Ok(out)
}

// Minimal LinkedHashSet-like collection (insertion-ordered unique values)
mod std {
    pub use ::std::*;
    pub struct LinkedHashSet<T>(Vec<T>);
    impl<T: PartialEq> LinkedHashSet<T> {
        pub fn new() -> Self {
            LinkedHashSet(Vec::new())
        }
        pub fn insert(&mut self, v: T) {
            if !self.0.contains(&v) {
                self.0.push(v);
            }
        }
        pub fn into_iter(self) -> impl Iterator<Item = T> {
            self.0.into_iter()
        }
    }
}

fn do_tasks(text: &str, is_kts: bool, args: &Value) -> Result<String, String> {
    let name_filter = args
        .get("filter")
        .and_then(|v| v.as_str())
        .map(|s| s.to_lowercase());

    let tasks = collect_tasks(text, is_kts);
    let filtered: Vec<_> = tasks
        .iter()
        .filter(|t| {
            name_filter
                .as_deref()
                .map(|f| t.name.to_lowercase().contains(f))
                .unwrap_or(true)
        })
        .collect();

    if filtered.is_empty() {
        return Ok("No custom tasks found (only standard lifecycle tasks).\n\nCommon Gradle tasks: build, test, clean, assemble, check, jar, run, publish\n".into());
    }

    let mut out = String::new();
    out.push_str(&format!("Custom tasks ({})\n", filtered.len()));
    out.push_str(&"─".repeat(60));
    out.push('\n');
    for t in &filtered {
        out.push_str(&format!("\ntask: {}\n", t.name));
        if !t.task_type.is_empty() {
            out.push_str(&format!("  type       : {}\n", t.task_type));
        }
        if !t.depends_on.is_empty() {
            out.push_str(&format!("  dependsOn  : {}\n", t.depends_on.join(", ")));
        }
        if !t.description.is_empty() {
            out.push_str(&format!("  description: {}\n", t.description));
        }
    }
    Ok(out)
}

fn do_plugins(text: &str, is_kts: bool, args: &Value) -> Result<String, String> {
    let name_filter = args
        .get("filter")
        .and_then(|v| v.as_str())
        .map(|s| s.to_lowercase());

    let plugins = collect_plugins(text, is_kts);
    let filtered: Vec<_> = plugins
        .iter()
        .filter(|p| {
            name_filter
                .as_deref()
                .map(|f| p.id.to_lowercase().contains(f))
                .unwrap_or(true)
        })
        .collect();

    if filtered.is_empty() {
        return Ok("No plugins found.\n".into());
    }

    let mut out = String::new();
    out.push_str(&format!("Plugins ({})\n", filtered.len()));
    out.push_str(&"─".repeat(60));
    out.push('\n');
    let id_w = filtered
        .iter()
        .map(|p| p.id.len())
        .max()
        .unwrap_or(20)
        .min(50);
    out.push_str(&format!(
        "{:<id_w$}  {:<20}  apply\n",
        "id",
        "version",
        id_w = id_w
    ));
    out.push_str(&format!(
        "{:<id_w$}  {:<20}  ─────\n",
        "─".repeat(id_w),
        "─".repeat(20),
        id_w = id_w
    ));
    for p in &filtered {
        let ver = if p.version.is_empty() {
            "(inherited)"
        } else {
            &p.version
        };
        let apply = if p.apply { "yes" } else { "no" };
        out.push_str(&format!(
            "{:<id_w$}  {:<20}  {}\n",
            truncate(&p.id, id_w),
            ver,
            apply,
            id_w = id_w
        ));
    }
    Ok(out)
}

fn do_properties(text: &str, is_kts: bool) -> Result<String, String> {
    let props = collect_properties(text, is_kts);
    if props.is_empty() {
        return Ok("No ext/extra properties found.\n".into());
    }
    let mut out = String::new();
    out.push_str(&format!("Extra properties ({})\n", props.len()));
    out.push_str(&"─".repeat(50));
    out.push('\n');
    let key_w = props
        .iter()
        .map(|(k, _)| k.len())
        .max()
        .unwrap_or(20)
        .min(40);
    for (k, v) in &props {
        out.push_str(&format!("{:<key_w$}  {}\n", k, v, key_w = key_w));
    }
    Ok(out)
}

fn do_validate(text: &str, is_kts: bool) -> Result<String, String> {
    let clean = strip_line_comments(text);
    let mut warnings = Vec::new();

    // No group defined
    if extract_simple_val(&clean, "group").is_none() {
        warnings.push("No 'group' defined — required for library/publication artifacts".into());
    }

    // No version defined
    if extract_simple_val(&clean, "version").is_none() {
        warnings.push("No 'version' defined — required for publication".into());
    }

    // No repositories
    let has_repo = clean.contains("mavenCentral()")
        || clean.contains("google()")
        || clean.contains("mavenLocal()")
        || clean.contains("gradlePluginPortal()")
        || clean.contains("jcenter()")
        || clean.contains("maven {");
    if !has_repo {
        warnings.push("No repositories configured — dependencies won't resolve".into());
    }

    // jcenter is deprecated
    if clean.contains("jcenter()") {
        warnings.push(
            "jcenter() is deprecated and read-only since March 2021 — migrate to mavenCentral()"
                .into(),
        );
    }

    // Deps without version
    let deps = collect_deps(text, is_kts);
    for d in &deps {
        if d.kind == "gav" && d.version.is_empty() {
            warnings.push(format!(
                "{}:{} — no version specified; use a BOM or version catalog to manage it",
                d.group, d.artifact
            ));
        }
    }

    // Duplicate deps (same group:artifact in same config)
    let mut seen: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for d in &deps {
        if d.kind == "gav" {
            let key = format!("{}:{}:{}", d.configuration, d.group, d.artifact);
            *seen.entry(key).or_insert(0) += 1;
        }
    }
    for (key, count) in &seen {
        if *count > 1 {
            warnings.push(format!(
                "Duplicate dependency: {} appears {} times",
                key, count
            ));
        }
    }

    // Detect usage of deprecated compile/runtime configurations
    let deprecated_configs = ["compile ", "runtime ", "testCompile ", "testRuntime "];
    for config in &deprecated_configs {
        if clean.contains(config) {
            warnings.push(format!(
                "'{}' configuration is deprecated — use '{}' instead",
                config.trim(),
                match config.trim() {
                    "compile" => "implementation or api",
                    "runtime" => "runtimeOnly",
                    "testCompile" => "testImplementation",
                    "testRuntime" => "testRuntimeOnly",
                    _ => "modern equivalent",
                }
            ));
        }
    }

    let mut out = String::new();
    if warnings.is_empty() {
        out.push_str("VALID — no issues found.\n");
    } else {
        out.push_str(&format!("WARNINGS ({})\n", warnings.len()));
        out.push_str(&"─".repeat(60));
        out.push('\n');
        for w in &warnings {
            out.push_str(&format!("  ⚠  {}\n", w));
        }
    }
    Ok(out)
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}…", &s[..max.saturating_sub(1)])
    }
}
