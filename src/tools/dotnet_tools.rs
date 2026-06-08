use serde_json::{json, Value};

pub fn make_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "action": {
                "type": "string",
                "enum": ["info", "packages", "targets", "references", "validate"],
                "description": "Action to perform (default: info)"
            },
            "file": { "type": "string", "description": "Path to .csproj, .fsproj, .vbproj, or .sln file" },
            "text": { "type": "string", "description": "Inline project XML content" },
            "xml":  { "type": "string", "description": "Alias for text" }
        }
    })
}

pub async fn execute(args: &Value) -> Result<String, String> {
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("info");
    match action {
        "info" => do_info(args),
        "packages" => do_packages(args),
        "targets" => do_targets(args),
        "references" => do_references(args),
        "validate" => do_validate(args),
        other => Err(format!(
            "Unknown action '{other}'. Choose: info, packages, targets, references, validate"
        )),
    }
}

// ── input loading ─────────────────────────────────────────────────────────────

fn load_text(args: &Value) -> Result<(String, String), String> {
    if let Some(f) = args.get("file").and_then(|v| v.as_str()) {
        let content =
            std::fs::read_to_string(f).map_err(|e| format!("Cannot read '{}': {e}", f))?;
        let kind = detect_kind(f);
        return Ok((content, kind));
    }
    let text = args
        .get("text")
        .or_else(|| args.get("xml"))
        .and_then(|v| v.as_str())
        .ok_or("Provide 'file' or 'text'.")?;
    Ok((text.to_string(), "csproj".to_string()))
}

fn detect_kind(path: &str) -> String {
    let p = path.to_lowercase();
    if p.ends_with(".sln") {
        "sln".into()
    } else if p.ends_with(".fsproj") {
        "fsproj".into()
    } else if p.ends_with(".vbproj") {
        "vbproj".into()
    } else {
        "csproj".into()
    }
}

// ── XML helpers ───────────────────────────────────────────────────────────────

/// Extract all occurrences of a tag's text content from XML.
fn tag_values(xml: &str, tag: &str) -> Vec<String> {
    let open = format!("<{tag}");
    let close = format!("</{tag}>");
    let mut out = Vec::new();
    let mut rest = xml;
    while let Some(pos) = rest.find(&open) {
        rest = &rest[pos..];
        if let Some(gt) = rest.find('>') {
            let after = &rest[gt + 1..];
            if let Some(end) = after.find(&close) {
                let val = after[..end].trim().to_string();
                if !val.is_empty() {
                    out.push(val);
                }
                rest = &after[end + close.len()..];
            } else {
                break;
            }
        } else {
            break;
        }
    }
    out
}

/// Get first occurrence of a tag's text content.
fn tag_first(xml: &str, tag: &str) -> Option<String> {
    tag_values(xml, tag).into_iter().next()
}

/// Extract attributes from the first occurrence of a tag.
fn tag_attrs(xml: &str, tag: &str) -> Option<String> {
    let open = format!("<{tag} ");
    let pos = xml.find(&open)?;
    let rest = &xml[pos + open.len()..];
    let end = rest.find('>')?;
    Some(rest[..end].to_string())
}

/// Get attribute value by name from an attribute string.
fn attr_val<'a>(attrs: &'a str, name: &str) -> Option<&'a str> {
    // Matches name="value" or name='value'
    let key = format!("{name}=");
    let pos = attrs.find(&key)?;
    let rest = &attrs[pos + key.len()..];
    let (_open, close) = if rest.starts_with('"') {
        ('"', '"')
    } else {
        ('\'', '\'')
    };
    let inner = &rest[1..];
    let end = inner.find(close)?;
    Some(&inner[..end])
}

/// Get all ItemGroup children matching a tag name, returning their attribute strings.
fn item_group_items<'a>(xml: &'a str, tag: &str) -> Vec<&'a str> {
    let open = format!("<{tag} ");
    let open2 = format!("<{tag}/>");
    let open3 = format!("<{tag}>");
    let mut out = Vec::new();
    let mut rest = xml;
    while let Some(pos) = rest
        .find(&open)
        .or_else(|| rest.find(&open2))
        .or_else(|| rest.find(&open3))
    {
        let slice = &rest[pos..];
        let end = slice.find('>').unwrap_or(slice.len() - 1);
        out.push(&slice[..end + 1]);
        rest = &slice[end + 1..];
    }
    out
}

// ── .sln parsing ──────────────────────────────────────────────────────────────

struct SlnProject {
    type_guid: String,
    name: String,
    path: String,
    guid: String,
}

fn parse_sln(text: &str) -> Vec<SlnProject> {
    let mut projects = Vec::new();
    for line in text.lines() {
        let t = line.trim();
        if !t.starts_with("Project(") {
            continue;
        }
        // Project("{TYPE-GUID}") = "Name", "Path", "{PROJ-GUID}"
        let parts: Vec<&str> = t.splitn(2, '=').collect();
        if parts.len() < 2 {
            continue;
        }
        let lhs = parts[0].trim();
        let rhs = parts[1].trim();
        let type_guid = lhs
            .trim_start_matches("Project(\"")
            .trim_end_matches("\")")
            .trim_matches('{')
            .trim_matches('}')
            .to_string();
        let fields: Vec<&str> = rhs.splitn(3, ',').collect();
        if fields.len() < 3 {
            continue;
        }
        let name = fields[0].trim().trim_matches('"').to_string();
        let path = fields[1].trim().trim_matches('"').to_string();
        let guid = fields[2]
            .trim()
            .trim_matches('"')
            .trim_matches('{')
            .trim_matches('}')
            .to_string();
        projects.push(SlnProject {
            type_guid,
            name,
            path,
            guid,
        });
    }
    projects
}

fn sln_type_label(type_guid: &str) -> &'static str {
    match type_guid.to_uppercase().as_str() {
        "FAE04EC0-301F-11D3-BF4B-00C04F79EFBC" => "C# project",
        "F2A71F9B-5D33-465A-A702-920D77279786" => "F# project",
        "F184B08F-C81C-45F6-A57F-5ABD9991F28F" => "VB.NET project",
        "2150E333-8FDC-42A3-9474-1A3956D46DE8" => "Solution folder",
        "8BC9CEB8-8B4A-11D0-8D11-00A0C91BC942" => "C++ project",
        "603C0E0B-DB56-11DC-BE95-000D561079B0" => "ASP.NET MVC",
        _ => "project",
    }
}

// ── actions ───────────────────────────────────────────────────────────────────

fn do_info(args: &Value) -> Result<String, String> {
    let (text, kind) = load_text(args)?;
    let mut out = String::new();

    if kind == "sln" {
        let projects = parse_sln(&text);
        let format_line = text
            .lines()
            .find(|l| {
                l.trim_start()
                    .starts_with("Microsoft Visual Studio Solution File")
            })
            .map(|l| l.trim().to_string())
            .unwrap_or_default();
        out.push_str("Solution Overview\n");
        out.push_str(&"─".repeat(40));
        out.push('\n');
        if !format_line.is_empty() {
            out.push_str(&format!("Format   : {format_line}\n"));
        }
        out.push_str(&format!("Projects : {}\n", projects.len()));
        let real_count = projects
            .iter()
            .filter(|p| p.type_guid.to_uppercase() != "2150E333-8FDC-42A3-9474-1A3956D46DE8")
            .count();
        out.push_str(&format!("Build    : {real_count} buildable project(s)\n\n"));
        out.push_str("Projects\n");
        out.push_str(&"─".repeat(40));
        out.push('\n');
        for p in &projects {
            let label = sln_type_label(&p.type_guid);
            out.push_str(&format!("  {:<30} [{label}]\n", p.name));
            out.push_str(&format!("    Path: {}\n", p.path));
        }
    } else {
        // csproj / fsproj / vbproj
        let sdk = tag_attrs(&text, "Project")
            .and_then(|a| attr_val(&a, "Sdk").map(|s| s.to_string()))
            .unwrap_or_default();
        let target_fw = tag_first(&text, "TargetFramework")
            .or_else(|| tag_first(&text, "TargetFrameworks"))
            .unwrap_or_default();
        let output = tag_first(&text, "OutputType").unwrap_or_else(|| "Library".into());
        let assembly = tag_first(&text, "AssemblyName").unwrap_or_default();
        let root_ns = tag_first(&text, "RootNamespace").unwrap_or_default();
        let nullable = tag_first(&text, "Nullable").unwrap_or_default();
        let lang_ver = tag_first(&text, "LangVersion").unwrap_or_default();
        let version = tag_first(&text, "Version")
            .or_else(|| tag_first(&text, "AssemblyVersion"))
            .unwrap_or_default();
        let pkg_count = tag_values(&text, "PackageReference").len();
        let proj_count = tag_values(&text, "ProjectReference").len();

        let ext = if kind == "fsproj" {
            "F#"
        } else if kind == "vbproj" {
            "VB.NET"
        } else {
            "C#"
        };

        out.push_str(&format!("{ext} Project Info\n"));
        out.push_str(&"─".repeat(40));
        out.push('\n');
        if !sdk.is_empty() {
            out.push_str(&format!("SDK              : {sdk}\n"));
        }
        if !target_fw.is_empty() {
            out.push_str(&format!("Target Framework : {target_fw}\n"));
        }
        if !output.is_empty() {
            out.push_str(&format!("Output Type      : {output}\n"));
        }
        if !assembly.is_empty() {
            out.push_str(&format!("Assembly Name    : {assembly}\n"));
        }
        if !root_ns.is_empty() {
            out.push_str(&format!("Root Namespace   : {root_ns}\n"));
        }
        if !version.is_empty() {
            out.push_str(&format!("Version          : {version}\n"));
        }
        if !nullable.is_empty() {
            out.push_str(&format!("Nullable         : {nullable}\n"));
        }
        if !lang_ver.is_empty() {
            out.push_str(&format!("LangVersion      : {lang_ver}\n"));
        }
        out.push_str(&format!("Package refs     : {pkg_count}\n"));
        out.push_str(&format!("Project refs     : {proj_count}\n"));

        let defines = tag_first(&text, "DefineConstants").unwrap_or_default();
        if !defines.is_empty() {
            out.push_str(&format!("Define Constants : {defines}\n"));
        }

        let gen_doc = tag_first(&text, "GenerateDocumentationFile").unwrap_or_default();
        if gen_doc.to_lowercase() == "true" {
            out.push_str("Documentation    : enabled\n");
        }

        let treat_warn = tag_first(&text, "TreatWarningsAsErrors").unwrap_or_default();
        if treat_warn.to_lowercase() == "true" {
            out.push_str("Warnings         : treated as errors\n");
        }

        let publish = tag_first(&text, "PublishSingleFile").unwrap_or_default();
        if publish.to_lowercase() == "true" {
            out.push_str("Publish          : single-file\n");
        }

        let self_contained = tag_first(&text, "SelfContained").unwrap_or_default();
        if self_contained.to_lowercase() == "true" {
            out.push_str("Self-contained   : true\n");
        }

        let invariant = tag_first(&text, "InvariantGlobalization").unwrap_or_default();
        if invariant.to_lowercase() == "true" {
            out.push_str("Globalization    : invariant\n");
        }
    }

    Ok(out)
}

fn do_packages(args: &Value) -> Result<String, String> {
    let (text, kind) = load_text(args)?;
    if kind == "sln" {
        return Ok("Use a .csproj/.fsproj/.vbproj file for 'packages'. Solutions list projects, not packages directly.".into());
    }
    let items = item_group_items(&text, "PackageReference");
    if items.is_empty() {
        return Ok("No PackageReference entries found.".into());
    }
    let mut rows: Vec<(String, String, String)> = Vec::new();
    for item in &items {
        let name = attr_val(item, "Include").unwrap_or("?").to_string();
        let version = attr_val(item, "Version").unwrap_or("").to_string();
        let cond = attr_val(item, "Condition").unwrap_or("").to_string();
        rows.push((name, version, cond));
    }
    rows.sort_by(|a, b| a.0.cmp(&b.0));
    let w_name = rows.iter().map(|r| r.0.len()).max().unwrap_or(10).max(7);
    let w_ver = rows.iter().map(|r| r.1.len()).max().unwrap_or(7).max(7);
    let mut out = String::new();
    out.push_str(&format!("NuGet Packages ({} total)\n", rows.len()));
    out.push_str(&"─".repeat(w_name + w_ver + 4));
    out.push('\n');
    out.push_str(&format!(
        "{:<w_name$}  {:<w_ver$}  Condition\n",
        "Package", "Version"
    ));
    out.push_str(&format!(
        "{:<w_name$}  {:<w_ver$}  ─────────\n",
        "─".repeat(w_name),
        "─".repeat(w_ver)
    ));
    for (name, ver, cond) in &rows {
        if cond.is_empty() {
            out.push_str(&format!("{:<w_name$}  {:<w_ver$}\n", name, ver));
        } else {
            out.push_str(&format!("{:<w_name$}  {:<w_ver$}  {cond}\n", name, ver));
        }
    }
    // wildcard / floating versions
    let wildcards: Vec<&str> = rows
        .iter()
        .filter(|(_, v, _)| v.contains('*') || v.starts_with("$("))
        .map(|(n, _, _)| n.as_str())
        .collect();
    if !wildcards.is_empty() {
        out.push_str(&format!(
            "\n⚠ Wildcard/variable versions: {}\n",
            wildcards.join(", ")
        ));
    }
    Ok(out)
}

fn do_targets(args: &Value) -> Result<String, String> {
    let (text, kind) = load_text(args)?;
    if kind == "sln" {
        return do_sln_targets(&text);
    }
    let target_fw = tag_first(&text, "TargetFramework")
        .or_else(|| tag_first(&text, "TargetFrameworks"))
        .unwrap_or_else(|| "unspecified".into());
    let output = tag_first(&text, "OutputType").unwrap_or_else(|| "Library".into());

    let mut out = String::new();
    out.push_str("Build Targets & Configurations\n");
    out.push_str(&"─".repeat(40));
    out.push('\n');
    out.push_str(&format!("Target Framework : {target_fw}\n"));
    out.push_str(&format!("Output Type      : {output}\n"));

    // PropertyGroups with Condition (debug/release configs)
    let mut rest = text.as_str();
    let mut configs: Vec<(String, Vec<(String, String)>)> = Vec::new();
    while let Some(pos) = rest.find("<PropertyGroup") {
        let slice = &rest[pos..];
        let cond_start = slice.find("Condition");
        let close_tag = slice.find("</PropertyGroup>");
        match (cond_start, close_tag) {
            (Some(cs), Some(ct)) if cs < ct => {
                let cond_str = &slice[cs..];
                let eq = cond_str.find('=').unwrap_or(0);
                let val_rest = &cond_str[eq + 1..].trim_matches('"').trim_matches('\'');
                let end_q = val_rest
                    .find('"')
                    .or_else(|| val_rest.find('\''))
                    .unwrap_or(val_rest.len());
                let cond_val = val_rest[..end_q].to_string();
                let body = &slice[..ct];
                let props = extract_simple_props(body);
                if !cond_val.is_empty() {
                    configs.push((cond_val, props));
                }
            }
            _ => {}
        }
        rest = &rest[pos + 14..];
    }

    if !configs.is_empty() {
        out.push('\n');
        out.push_str("Conditional Configurations\n");
        for (cond, props) in &configs {
            out.push_str(&format!("  [{cond}]\n"));
            for (k, v) in props {
                out.push_str(&format!("    {k} = {v}\n"));
            }
        }
    }

    // MSBuild Targets defined
    let target_names = collect_target_names(&text);
    if !target_names.is_empty() {
        out.push('\n');
        out.push_str("Custom MSBuild Targets\n");
        for t in &target_names {
            out.push_str(&format!("  {t}\n"));
        }
    }

    Ok(out)
}

fn do_sln_targets(text: &str) -> Result<String, String> {
    let projects = parse_sln(text);
    let buildable: Vec<&SlnProject> = projects
        .iter()
        .filter(|p| p.type_guid.to_uppercase() != "2150E333-8FDC-42A3-9474-1A3956D46DE8")
        .collect();
    let mut out = String::from("Solution Build Projects\n");
    out.push_str(&"─".repeat(40));
    out.push('\n');
    for p in buildable {
        out.push_str(&format!("  {} → {}\n", p.name, p.path));
    }
    Ok(out)
}

fn extract_simple_props(xml: &str) -> Vec<(String, String)> {
    let mut props = Vec::new();
    let skip = ["PropertyGroup", "Condition", "DefineConstants"];
    let mut rest = xml;
    while let Some(pos) = rest.find('<') {
        let slice = &rest[pos + 1..];
        let name_end = slice.find(['>', ' ', '/']).unwrap_or(slice.len());
        let tag = &slice[..name_end];
        if !tag.is_empty() && !tag.starts_with('/') && !tag.starts_with('!') && !skip.contains(&tag)
        {
            let close = format!("</{tag}>");
            if let Some(gt) = slice.find('>') {
                let after = &slice[gt + 1..];
                if let Some(end) = after.find(&close) {
                    let val = after[..end].trim().to_string();
                    if !val.is_empty() {
                        props.push((tag.to_string(), val));
                    }
                    rest = &after[end + close.len()..];
                    continue;
                }
            }
        }
        rest = &rest[pos + 1..];
    }
    props
}

fn collect_target_names(xml: &str) -> Vec<String> {
    let mut names = Vec::new();
    let mut rest = xml;
    while let Some(pos) = rest.find("<Target ") {
        let slice = &rest[pos..];
        if let Some(gt) = slice.find('>') {
            let tag_body = &slice[..gt];
            if let Some(n) = attr_val(tag_body, "Name") {
                names.push(n.to_string());
            }
            rest = &slice[gt + 1..];
        } else {
            break;
        }
    }
    names
}

fn do_references(args: &Value) -> Result<String, String> {
    let (text, kind) = load_text(args)?;
    if kind == "sln" {
        let projects = parse_sln(&text);
        let mut out = String::from("Solution Project References\n");
        out.push_str(&"─".repeat(40));
        out.push('\n');
        for p in &projects {
            let label = sln_type_label(&p.type_guid);
            out.push_str(&format!("  {:<28} [{label}]  {}\n", p.name, p.guid));
        }
        return Ok(out);
    }

    let proj_refs = item_group_items(&text, "ProjectReference");
    let framework_refs = item_group_items(&text, "Reference");
    let mut out = String::new();

    if !proj_refs.is_empty() {
        out.push_str(&format!("Project References ({})\n", proj_refs.len()));
        out.push_str(&"─".repeat(40));
        out.push('\n');
        for item in &proj_refs {
            let path = attr_val(item, "Include").unwrap_or("?");
            out.push_str(&format!("  {path}\n"));
        }
    }

    if !framework_refs.is_empty() {
        if !out.is_empty() {
            out.push('\n');
        }
        out.push_str(&format!("Assembly References ({})\n", framework_refs.len()));
        out.push_str(&"─".repeat(40));
        out.push('\n');
        for item in &framework_refs {
            let name = attr_val(item, "Include").unwrap_or("?");
            let hint = {
                // look for HintPath child — simple search
                let search = format!("Include=\"{name}\"");
                if let Some(pos) = text.find(&search) {
                    let after = &text[pos..];
                    tag_values(after, "HintPath")
                        .into_iter()
                        .next()
                        .unwrap_or_default()
                } else {
                    String::new()
                }
            };
            if hint.is_empty() {
                out.push_str(&format!("  {name}\n"));
            } else {
                out.push_str(&format!("  {name}  →  {hint}\n"));
            }
        }
    }

    if out.is_empty() {
        out.push_str("No project or assembly references found.\n");
    }
    Ok(out)
}

fn do_validate(args: &Value) -> Result<String, String> {
    let (text, kind) = load_text(args)?;
    let mut issues: Vec<String> = Vec::new();

    if kind == "sln" {
        let projects = parse_sln(&text);
        let has_format = text
            .lines()
            .any(|l| l.contains("Microsoft Visual Studio Solution File"));
        if !has_format {
            issues.push("Missing 'Microsoft Visual Studio Solution File' header line".into());
        }
        if projects.is_empty() {
            issues.push("No Project entries found".into());
        }
        let real: Vec<&SlnProject> = projects
            .iter()
            .filter(|p| p.type_guid.to_uppercase() != "2150E333-8FDC-42A3-9474-1A3956D46DE8")
            .collect();
        if real.is_empty() {
            issues.push("No buildable (non-folder) projects".into());
        }
        // Check for duplicate GUIDs
        let mut guids: Vec<&str> = projects.iter().map(|p| p.guid.as_str()).collect();
        guids.sort();
        let mut prev = "";
        for g in &guids {
            if *g == prev {
                issues.push(format!("Duplicate project GUID: {g}"));
            }
            prev = g;
        }
        // Check for relative .sln path on Windows (backslash expected)
        for p in &real {
            if p.path.contains('/') && !p.path.starts_with("http") {
                issues.push(format!(
                    "'{}' uses forward slashes — expect backslashes on Windows",
                    p.path
                ));
            }
        }
    } else {
        let sdk =
            tag_attrs(&text, "Project").and_then(|a| attr_val(&a, "Sdk").map(|s| s.to_string()));
        if sdk.is_none() {
            issues.push("Missing Sdk attribute on <Project> — is this a SDK-style project?".into());
        }
        let fw =
            tag_first(&text, "TargetFramework").or_else(|| tag_first(&text, "TargetFrameworks"));
        if fw.is_none() {
            issues.push("No <TargetFramework> or <TargetFrameworks> specified".into());
        }

        // Wildcard package versions
        let items = item_group_items(&text, "PackageReference");
        for item in &items {
            let ver = attr_val(item, "Version").unwrap_or("");
            let name = attr_val(item, "Include").unwrap_or("?");
            if ver.contains('*') {
                issues.push(format!("Wildcard version on {name}: '{ver}' — pins may cause non-deterministic restores"));
            }
            if ver.is_empty() {
                issues.push(format!(
                    "PackageReference '{name}' has no Version attribute"
                ));
            }
        }

        // Hardcoded connection strings / secrets in PropertyGroups
        let lower = text.to_lowercase();
        if lower.contains("password=")
            || lower.contains("connectionstring")
            || lower.contains("api_key")
            || lower.contains("apikey")
        {
            issues.push("Possible credential or connection string in project file — move to user secrets or environment variables".into());
        }

        // Deprecated elements
        if text.contains("<DotNetCliToolReference") {
            issues.push("<DotNetCliToolReference> is deprecated — migrate to <PackageReference> for .NET Core 3.0+".into());
        }
        if text.contains("<AutoGenerateBindingRedirects") {
            issues.push("<AutoGenerateBindingRedirects> is a .NET Framework concept — not needed in SDK-style projects".into());
        }

        // Both TargetFramework and TargetFrameworks
        if tag_first(&text, "TargetFramework").is_some()
            && tag_first(&text, "TargetFrameworks").is_some()
        {
            issues.push(
                "Both <TargetFramework> and <TargetFrameworks> defined — use only one".into(),
            );
        }
    }

    let mut out = String::new();
    if issues.is_empty() {
        out.push_str("✓ VALID — no issues found\n");
    } else {
        out.push_str(&format!("INVALID — {} issue(s) found\n", issues.len()));
        out.push_str(&"─".repeat(40));
        out.push('\n');
        for issue in &issues {
            out.push_str(&format!("  ⚠ {issue}\n"));
        }
    }
    Ok(out)
}
