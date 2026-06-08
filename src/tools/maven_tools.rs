use serde_json::{json, Value};

pub fn make_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "action": {
                "type": "string",
                "description": "info (default) | deps | plugins | profiles | properties | validate"
            },
            "pom": {
                "type": "string",
                "description": "Inline pom.xml content"
            },
            "file": {
                "type": "string",
                "description": "Path to pom.xml or any POM file"
            },
            "scope": {
                "type": "string",
                "description": "For deps: filter by scope (compile/test/provided/runtime/system/import)"
            },
            "filter": {
                "type": "string",
                "description": "For deps/plugins: filter by groupId or artifactId substring"
            }
        }
    })
}

pub async fn execute(args: &Value) -> Result<String, String> {
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("info");

    let text = if let Some(f) = args.get("file").and_then(|v| v.as_str()) {
        std::fs::read_to_string(f).map_err(|e| format!("Cannot read file '{}': {}", f, e))?
    } else if let Some(t) = args
        .get("pom")
        .or_else(|| args.get("text"))
        .or_else(|| args.get("xml"))
        .and_then(|v| v.as_str())
    {
        t.to_string()
    } else {
        return Err("Provide 'file' (path to pom.xml) or 'pom' (inline XML content).".into());
    };

    match action {
        "deps" | "dependencies" => do_deps(&text, args),
        "plugins" => do_plugins(&text, args),
        "profiles" => do_profiles(&text),
        "properties" | "props" => do_properties(&text),
        "validate" | "check" => do_validate(&text),
        _ => do_info(&text),
    }
}

// ── XML helpers ──────────────────────────────────────────────────────────────

fn tag_first(xml: &str, tag: &str) -> Option<String> {
    let open = format!("<{}>", tag);
    let close = format!("</{}>", tag);
    let start = xml.find(&open)?;
    let content_start = start + open.len();
    let end = xml[content_start..].find(&close)?;
    Some(xml[content_start..content_start + end].trim().to_string())
}

#[allow(dead_code)]
fn tag_first_attr(xml: &str, tag: &str) -> Option<String> {
    // finds first occurrence of <tag or <tag with attrs
    let pat = format!("<{}", tag);
    let start = xml.find(&pat)?;
    let end = xml[start..].find('>')?;
    let slice = &xml[start..start + end + 1];
    Some(slice.to_string())
}

#[allow(dead_code)]
fn attr_val<'a>(tag_text: &'a str, attr: &str) -> Option<&'a str> {
    let pat = format!("{}=\"", attr);
    let start = tag_text.find(&pat)?;
    let content_start = start + pat.len();
    let end = tag_text[content_start..].find('"')?;
    Some(&tag_text[content_start..content_start + end])
}

fn between_tags<'a>(xml: &'a str, open: &str, close: &str) -> Vec<&'a str> {
    let mut results = Vec::new();
    let mut search = xml;
    while let Some(start) = search.find(open) {
        let after = &search[start + open.len()..];
        if let Some(end) = after.find(close) {
            results.push(&after[..end]);
            search = &after[end + close.len()..];
        } else {
            break;
        }
    }
    results
}

// ── Parsed types ─────────────────────────────────────────────────────────────

struct Dependency {
    group_id: String,
    artifact_id: String,
    version: String,
    scope: String,
    optional: bool,
    dep_type: String,
    classifier: String,
}

#[allow(dead_code)]
struct Plugin {
    group_id: String,
    artifact_id: String,
    version: String,
    inherited: bool,
}

struct Profile {
    id: String,
    activation: String,
    dep_count: usize,
    plugin_count: usize,
}

fn parse_dependency(block: &str) -> Dependency {
    let group_id = tag_first(block, "groupId").unwrap_or_default();
    let artifact_id = tag_first(block, "artifactId").unwrap_or_default();
    let version = tag_first(block, "version").unwrap_or_else(|| "(inherited)".into());
    let scope = tag_first(block, "scope").unwrap_or_else(|| "compile".into());
    let optional = tag_first(block, "optional")
        .map(|v| v == "true")
        .unwrap_or(false);
    let dep_type = tag_first(block, "type").unwrap_or_else(|| "jar".into());
    let classifier = tag_first(block, "classifier").unwrap_or_default();
    Dependency {
        group_id,
        artifact_id,
        version,
        scope,
        optional,
        dep_type,
        classifier,
    }
}

fn parse_plugin(block: &str) -> Plugin {
    let group_id = tag_first(block, "groupId").unwrap_or_else(|| "org.apache.maven.plugins".into());
    let artifact_id = tag_first(block, "artifactId").unwrap_or_default();
    let version = tag_first(block, "version").unwrap_or_else(|| "(inherited)".into());
    let inherited = tag_first(block, "inherited")
        .map(|v| v != "false")
        .unwrap_or(true);
    Plugin {
        group_id,
        artifact_id,
        version,
        inherited,
    }
}

fn collect_dependencies(xml: &str) -> Vec<Dependency> {
    // Only from <dependencies> outside <dependencyManagement>
    // We strip <dependencyManagement> blocks first
    let no_dm = strip_section(xml, "dependencyManagement");
    between_tags(&no_dm, "<dependency>", "</dependency>")
        .iter()
        .map(|b| parse_dependency(b))
        .collect()
}

fn collect_managed_dependencies(xml: &str) -> Vec<Dependency> {
    if let Some(start) = xml.find("<dependencyManagement>") {
        let after = &xml[start..];
        if let Some(end) = after.find("</dependencyManagement>") {
            let block = &after[..end + "</dependencyManagement>".len()];
            return between_tags(block, "<dependency>", "</dependency>")
                .iter()
                .map(|b| parse_dependency(b))
                .collect();
        }
    }
    vec![]
}

fn strip_section(xml: &str, tag: &str) -> String {
    let open = format!("<{}>", tag);
    let close = format!("</{}>", tag);
    if let Some(start) = xml.find(&open) {
        if let Some(end) = xml[start..].find(&close) {
            let end_abs = start + end + close.len();
            return format!("{}{}", &xml[..start], &xml[end_abs..]);
        }
    }
    xml.to_string()
}

fn collect_plugins(xml: &str) -> Vec<Plugin> {
    // Only from <build><plugins>, not pluginManagement
    let no_pm = strip_section(xml, "pluginManagement");
    between_tags(&no_pm, "<plugin>", "</plugin>")
        .iter()
        .map(|b| parse_plugin(b))
        .collect()
}

fn collect_profiles(xml: &str) -> Vec<Profile> {
    between_tags(xml, "<profile>", "</profile>")
        .iter()
        .map(|b| {
            let id = tag_first(b, "id").unwrap_or_else(|| "(unnamed)".into());
            let mut activation_parts = Vec::new();
            if let Some(act) = xml.find("<activation>").map(|_| {
                between_tags(b, "<activation>", "</activation>")
                    .first()
                    .copied()
                    .unwrap_or("")
            }) {
                if act.contains("<activeByDefault>true</activeByDefault>") {
                    activation_parts.push("default");
                }
                if tag_first(act, "jdk").is_some() {
                    activation_parts.push("jdk");
                }
                if tag_first(act, "os").is_some() {
                    activation_parts.push("os");
                }
                if tag_first(act, "property").is_some() {
                    activation_parts.push("property");
                }
                if tag_first(act, "file").is_some() {
                    activation_parts.push("file");
                }
            }
            let activation = if activation_parts.is_empty() {
                "manual".into()
            } else {
                activation_parts.join("+")
            };
            let dep_count = between_tags(b, "<dependency>", "</dependency>").len();
            let plugin_count = between_tags(b, "<plugin>", "</plugin>").len();
            Profile {
                id,
                activation,
                dep_count,
                plugin_count,
            }
        })
        .collect()
}

// ── Actions ──────────────────────────────────────────────────────────────────

fn do_info(xml: &str) -> Result<String, String> {
    let group_id = tag_first(xml, "groupId").unwrap_or_else(|| "(none)".into());
    let artifact_id = tag_first(xml, "artifactId").unwrap_or_else(|| "(none)".into());
    let version = tag_first(xml, "version").unwrap_or_else(|| "(none)".into());
    let packaging = tag_first(xml, "packaging").unwrap_or_else(|| "jar".into());
    let name = tag_first(xml, "name").unwrap_or_default();
    let description = tag_first(xml, "description").unwrap_or_default();
    let url = tag_first(xml, "url").unwrap_or_default();

    let java_version = tag_first(xml, "java.version")
        .or_else(|| tag_first(xml, "maven.compiler.source"))
        .or_else(|| tag_first(xml, "maven.compiler.release"))
        .unwrap_or_default();

    // parent
    let parent_info = between_tags(xml, "<parent>", "</parent>").first().map(|b| {
        let pg = tag_first(b, "groupId").unwrap_or_default();
        let pa = tag_first(b, "artifactId").unwrap_or_default();
        let pv = tag_first(b, "version").unwrap_or_default();
        format!("{}:{}:{}", pg, pa, pv)
    });

    let deps = collect_dependencies(xml);
    let managed = collect_managed_dependencies(xml);
    let plugins = collect_plugins(xml);
    let profiles = collect_profiles(xml);
    let props_count = between_tags(xml, "<properties>", "</properties>")
        .first()
        .map(|b| {
            // count child elements crudely
            let mut count = 0usize;
            let mut s = *b;
            while let Some(p) = s.find('<') {
                let after = &s[p + 1..];
                if !after.starts_with('/') && !after.starts_with('!') {
                    count += 1;
                }
                s = &s[p + 1..];
            }
            count
        })
        .unwrap_or(0);

    let modules: Vec<String> = between_tags(xml, "<module>", "</module>")
        .iter()
        .map(|s| s.trim().to_string())
        .collect();

    let mut out = String::new();
    out.push_str("Maven POM\n");
    out.push_str(&"─".repeat(60));
    out.push('\n');
    out.push_str(&format!(
        "Coordinates  : {}:{}:{}\n",
        group_id, artifact_id, version
    ));
    out.push_str(&format!("Packaging    : {}\n", packaging));
    if !name.is_empty() {
        out.push_str(&format!("Name         : {}\n", name));
    }
    if !description.is_empty() {
        let desc_short = if description.len() > 80 {
            format!("{}...", &description[..77])
        } else {
            description.clone()
        };
        out.push_str(&format!("Description  : {}\n", desc_short));
    }
    if let Some(p) = parent_info {
        out.push_str(&format!("Parent       : {}\n", p));
    }
    if !java_version.is_empty() {
        out.push_str(&format!("Java version : {}\n", java_version));
    }
    if !url.is_empty() {
        out.push_str(&format!("URL          : {}\n", url));
    }
    out.push('\n');
    out.push_str("Counts\n");
    out.push_str(&format!("  Dependencies         : {}\n", deps.len()));
    if !managed.is_empty() {
        out.push_str(&format!("  Managed dependencies : {}\n", managed.len()));
    }
    out.push_str(&format!("  Build plugins        : {}\n", plugins.len()));
    out.push_str(&format!("  Properties           : {}\n", props_count));
    out.push_str(&format!("  Profiles             : {}\n", profiles.len()));
    if !modules.is_empty() {
        out.push_str(&format!("  Modules (multi)      : {}\n", modules.len()));
        for m in &modules {
            out.push_str(&format!("    • {}\n", m));
        }
    }

    // scope distribution
    if !deps.is_empty() {
        out.push('\n');
        out.push_str("Dependency scopes\n");
        let mut scope_counts: std::collections::HashMap<&str, usize> =
            std::collections::HashMap::new();
        for d in &deps {
            *scope_counts.entry(d.scope.as_str()).or_insert(0) += 1;
        }
        let mut pairs: Vec<_> = scope_counts.iter().collect();
        pairs.sort_by(|a, b| b.1.cmp(a.1));
        for (scope, count) in pairs {
            out.push_str(&format!("  {:<12} {}\n", scope, count));
        }
    }

    Ok(out)
}

fn do_deps(xml: &str, args: &Value) -> Result<String, String> {
    let scope_filter = args
        .get("scope")
        .and_then(|v| v.as_str())
        .map(|s| s.to_lowercase());
    let name_filter = args
        .get("filter")
        .and_then(|v| v.as_str())
        .map(|s| s.to_lowercase());

    let deps = collect_dependencies(xml);
    let managed = collect_managed_dependencies(xml);

    let mut out = String::new();

    // Direct dependencies
    let filtered: Vec<_> = deps
        .iter()
        .filter(|d| {
            scope_filter
                .as_deref()
                .map(|sf| d.scope.to_lowercase() == sf)
                .unwrap_or(true)
        })
        .filter(|d| {
            name_filter
                .as_deref()
                .map(|f| {
                    d.group_id.to_lowercase().contains(f)
                        || d.artifact_id.to_lowercase().contains(f)
                })
                .unwrap_or(true)
        })
        .collect();

    if filtered.is_empty() {
        out.push_str("No dependencies match the given filters.\n");
    } else {
        out.push_str(&format!("Direct dependencies ({})\n", filtered.len()));
        out.push_str(&"─".repeat(80));
        out.push('\n');
        let g_w = filtered
            .iter()
            .map(|d| d.group_id.len())
            .max()
            .unwrap_or(10)
            .min(35);
        let a_w = filtered
            .iter()
            .map(|d| d.artifact_id.len())
            .max()
            .unwrap_or(10)
            .min(35);
        out.push_str(&format!(
            "{:<g_w$}  {:<a_w$}  {:<20}  {}\n",
            "groupId",
            "artifactId",
            "version",
            "scope",
            g_w = g_w,
            a_w = a_w
        ));
        out.push_str(&format!(
            "{:<g_w$}  {:<a_w$}  {:<20}  {}\n",
            "─".repeat(g_w),
            "─".repeat(a_w),
            "─".repeat(20),
            "─".repeat(10),
            g_w = g_w,
            a_w = a_w
        ));
        for d in &filtered {
            let mut extra = String::new();
            if d.optional {
                extra.push_str(" [optional]");
            }
            if d.dep_type != "jar" {
                extra.push_str(&format!(" [{}]", d.dep_type));
            }
            if !d.classifier.is_empty() {
                extra.push_str(&format!(" :{}", d.classifier));
            }
            out.push_str(&format!(
                "{:<g_w$}  {:<a_w$}  {:<20}  {}{}\n",
                truncate(&d.group_id, g_w),
                truncate(&d.artifact_id, a_w),
                truncate(&d.version, 20),
                d.scope,
                extra,
                g_w = g_w,
                a_w = a_w
            ));
        }
    }

    if !managed.is_empty() && scope_filter.is_none() && name_filter.is_none() {
        out.push('\n');
        out.push_str(&format!(
            "Managed dependencies ({}) — version pinning\n",
            managed.len()
        ));
        out.push_str(&"─".repeat(70));
        out.push('\n');
        let g_w = managed
            .iter()
            .map(|d| d.group_id.len())
            .max()
            .unwrap_or(10)
            .min(35);
        let a_w = managed
            .iter()
            .map(|d| d.artifact_id.len())
            .max()
            .unwrap_or(10)
            .min(35);
        out.push_str(&format!(
            "{:<g_w$}  {:<a_w$}  {}\n",
            "groupId",
            "artifactId",
            "version",
            g_w = g_w,
            a_w = a_w
        ));
        out.push_str(&format!(
            "{:<g_w$}  {:<a_w$}  {}\n",
            "─".repeat(g_w),
            "─".repeat(a_w),
            "─".repeat(20),
            g_w = g_w,
            a_w = a_w
        ));
        for d in &managed {
            out.push_str(&format!(
                "{:<g_w$}  {:<a_w$}  {}\n",
                truncate(&d.group_id, g_w),
                truncate(&d.artifact_id, a_w),
                d.version,
                g_w = g_w,
                a_w = a_w
            ));
        }
    }

    Ok(out)
}

fn do_plugins(xml: &str, args: &Value) -> Result<String, String> {
    let name_filter = args
        .get("filter")
        .and_then(|v| v.as_str())
        .map(|s| s.to_lowercase());

    let plugins = collect_plugins(xml);
    // Also check pluginManagement
    let pm_plugins: Vec<Plugin> = between_tags(xml, "<pluginManagement>", "</pluginManagement>")
        .iter()
        .flat_map(|b| {
            between_tags(b, "<plugin>", "</plugin>")
                .iter()
                .map(|pb| parse_plugin(pb))
                .collect::<Vec<_>>()
        })
        .collect();

    let mut out = String::new();

    let filtered: Vec<_> = plugins
        .iter()
        .filter(|p| {
            name_filter
                .as_deref()
                .map(|f| {
                    p.group_id.to_lowercase().contains(f)
                        || p.artifact_id.to_lowercase().contains(f)
                })
                .unwrap_or(true)
        })
        .collect();

    if filtered.is_empty() {
        out.push_str("No build plugins found.\n");
    } else {
        out.push_str(&format!("Build plugins ({})\n", filtered.len()));
        out.push_str(&"─".repeat(70));
        out.push('\n');
        let g_w = filtered
            .iter()
            .map(|p| p.group_id.len())
            .max()
            .unwrap_or(10)
            .min(40);
        let a_w = filtered
            .iter()
            .map(|p| p.artifact_id.len())
            .max()
            .unwrap_or(10)
            .min(40);
        out.push_str(&format!(
            "{:<g_w$}  {:<a_w$}  {}\n",
            "groupId",
            "artifactId",
            "version",
            g_w = g_w,
            a_w = a_w
        ));
        out.push_str(&format!(
            "{:<g_w$}  {:<a_w$}  {}\n",
            "─".repeat(g_w),
            "─".repeat(a_w),
            "─".repeat(20),
            g_w = g_w,
            a_w = a_w
        ));
        for p in &filtered {
            out.push_str(&format!(
                "{:<g_w$}  {:<a_w$}  {}\n",
                truncate(&p.group_id, g_w),
                truncate(&p.artifact_id, a_w),
                p.version,
                g_w = g_w,
                a_w = a_w
            ));
        }
    }

    if !pm_plugins.is_empty() {
        out.push('\n');
        out.push_str(&format!(
            "Plugin management ({}) — version pinning\n",
            pm_plugins.len()
        ));
        out.push_str(&"─".repeat(60));
        out.push('\n');
        for p in &pm_plugins {
            out.push_str(&format!(
                "  {}:{}  {}\n",
                p.group_id, p.artifact_id, p.version
            ));
        }
    }

    Ok(out)
}

fn do_profiles(xml: &str) -> Result<String, String> {
    let profiles = collect_profiles(xml);
    if profiles.is_empty() {
        return Ok("No profiles defined in this POM.\n".into());
    }
    let mut out = String::new();
    out.push_str(&format!("Profiles ({})\n", profiles.len()));
    out.push_str(&"─".repeat(60));
    out.push('\n');
    out.push_str(&format!(
        "{:<25}  {:<20}  deps  plugins\n",
        "id", "activation"
    ));
    out.push_str(&format!(
        "{:<25}  {:<20}  ────  ───────\n",
        "─".repeat(25),
        "─".repeat(20)
    ));
    for p in &profiles {
        out.push_str(&format!(
            "{:<25}  {:<20}  {:>4}  {:>7}\n",
            truncate(&p.id, 25),
            p.activation,
            p.dep_count,
            p.plugin_count
        ));
    }
    out.push_str("\nActivate with: mvn -P <profile-id> ...\n");
    Ok(out)
}

fn do_properties(xml: &str) -> Result<String, String> {
    let props_blocks = between_tags(xml, "<properties>", "</properties>");
    if props_blocks.is_empty() {
        return Ok("No <properties> section found.\n".into());
    }
    let block = props_blocks[0];
    let mut props: Vec<(String, String)> = Vec::new();
    let mut s = block;
    while let Some(open_start) = s.find('<') {
        let rest = &s[open_start + 1..];
        if rest.starts_with('/') || rest.starts_with('!') || rest.starts_with('?') {
            s = &s[open_start + 1..];
            continue;
        }
        if let Some(tag_end) = rest.find('>') {
            let tag_name = rest[..tag_end].trim();
            // skip tags with attributes (e.g. <foo bar="baz">)
            let bare_tag = if let Some(sp) = tag_name.find([' ', '\t']) {
                &tag_name[..sp]
            } else {
                tag_name
            };
            let close = format!("</{}>", bare_tag);
            let after_open = &rest[tag_end + 1..];
            if let Some(close_pos) = after_open.find(&close) {
                let value = after_open[..close_pos].trim().to_string();
                props.push((bare_tag.to_string(), value));
                s = &after_open[close_pos + close.len()..];
            } else {
                s = &s[open_start + 1..];
            }
        } else {
            break;
        }
    }

    if props.is_empty() {
        return Ok("No properties found in <properties> section.\n".into());
    }

    let mut out = String::new();
    out.push_str(&format!("Properties ({})\n", props.len()));
    out.push_str(&"─".repeat(60));
    out.push('\n');
    let key_w = props
        .iter()
        .map(|(k, _)| k.len())
        .max()
        .unwrap_or(20)
        .min(40);
    for (k, v) in &props {
        out.push_str(&format!(
            "{:<key_w$}  {}\n",
            truncate(k, key_w),
            if v.len() > 60 {
                format!("{}...", &v[..57])
            } else {
                v.clone()
            },
            key_w = key_w
        ));
    }
    Ok(out)
}

fn do_validate(xml: &str) -> Result<String, String> {
    let mut warnings: Vec<String> = Vec::new();

    // Required fields
    if tag_first(xml, "groupId").is_none() {
        warnings.push("Missing <groupId> — required unless inherited from parent".into());
    }
    if tag_first(xml, "artifactId").is_none() {
        warnings.push("Missing <artifactId> — required".into());
    }
    if tag_first(xml, "version").is_none() {
        warnings.push("Missing <version> — required unless inherited from parent".into());
    }

    // Wildcard/range versions in dependencies
    let deps = collect_dependencies(xml);
    for d in &deps {
        if d.version.contains('[') || d.version.contains('(') {
            warnings.push(format!(
                "{}:{} — version range '{}' is fragile; pin an exact version instead",
                d.group_id, d.artifact_id, d.version
            ));
        }
        if d.version == "LATEST" || d.version == "RELEASE" {
            warnings.push(format!(
                "{}:{} — '{}' resolves differently per environment; pin a fixed version",
                d.group_id, d.artifact_id, d.version
            ));
        }
        if d.version == "(inherited)"
            && collect_managed_dependencies(xml)
                .iter()
                .all(|m| m.group_id != d.group_id || m.artifact_id != d.artifact_id)
        {
            // version missing and not in dependencyManagement
            warnings.push(format!(
                "{}:{} — no version specified and not in dependencyManagement",
                d.group_id, d.artifact_id
            ));
        }
    }

    // Duplicate dependencies (same groupId:artifactId, different version/scope)
    let mut seen: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for d in &deps {
        let key = format!("{}:{}", d.group_id, d.artifact_id);
        *seen.entry(key).or_insert(0) += 1;
    }
    for (key, count) in &seen {
        if *count > 1 {
            warnings.push(format!(
                "Duplicate dependency: {} appears {} times",
                key, count
            ));
        }
    }

    // Missing maven compiler source/target consistency
    let source = tag_first(xml, "maven.compiler.source");
    let target = tag_first(xml, "maven.compiler.target");
    let release = tag_first(xml, "maven.compiler.release");
    if source.is_none() && target.is_none() && release.is_none() {
        warnings.push(
            "No Java version configured — add <maven.compiler.release>21</maven.compiler.release> in <properties>".into(),
        );
    }
    if let (Some(src), Some(tgt)) = (source.as_deref(), target.as_deref()) {
        if src != tgt {
            warnings.push(format!(
                "maven.compiler.source ({}) != maven.compiler.target ({}) — use <release> instead",
                src, tgt
            ));
        }
    }

    // Check for snapshot parent version
    if let Some(parent_block) = between_tags(xml, "<parent>", "</parent>").first() {
        if let Some(pv) = tag_first(parent_block, "version") {
            if pv.contains("SNAPSHOT") {
                warnings.push(format!(
                    "Parent version '{}' is a SNAPSHOT — avoid SNAPSHOT parents in releases",
                    pv
                ));
            }
        }
    }

    // Check for snapshot deps in non-snapshot artifact
    if let Some(own_version) = tag_first(xml, "version") {
        if !own_version.contains("SNAPSHOT") {
            for d in &deps {
                if d.version.contains("SNAPSHOT") {
                    warnings.push(format!(
                        "{}:{} is a SNAPSHOT dependency in a release artifact",
                        d.group_id, d.artifact_id
                    ));
                }
            }
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
