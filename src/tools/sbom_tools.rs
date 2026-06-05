use serde_json::Value;

pub fn make_schema() -> Value {
    serde_json::json!({
        "name": "sbom_tools",
        "description": "Parse and analyze Software Bill of Materials (SBOM) documents in CycloneDX JSON, SPDX JSON, and SPDX tag-value formats without external utilities.",
        "input_schema": {
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["info", "components", "licenses", "vulnerabilities", "validate"],
                    "description": "info (default): SBOM format, version, component count, tool info. components: full component list with name/version/license/purl. licenses: license distribution across all components. vulnerabilities: any vulnerability data (CycloneDX only). validate: check required SBOM fields."
                },
                "text": {
                    "type": "string",
                    "description": "SBOM content as a string (JSON or SPDX tag-value)."
                },
                "file": {
                    "type": "string",
                    "description": "Path to an SBOM file (.json, .spdx, .tv)."
                },
                "format": {
                    "type": "string",
                    "enum": ["auto", "cyclonedx", "spdx-json", "spdx-tv"],
                    "description": "Force a format; default 'auto' detects from content."
                },
                "limit": {
                    "type": "integer",
                    "description": "Max components to display (default 50)."
                }
            }
        }
    })
}

#[derive(Debug, Clone)]
enum SbomFormat {
    CycloneDx,
    SpdxJson,
    SpdxTv,
}

#[derive(Debug, Clone)]
struct Component {
    name: String,
    version: Option<String>,
    license: Option<String>,
    purl: Option<String>,
    component_type: Option<String>,
    supplier: Option<String>,
}

#[derive(Debug)]
struct Vulnerability {
    id: String,
    severity: Option<String>,
    description: Option<String>,
    affects: Vec<String>,
}

#[derive(Debug)]
struct Sbom {
    format: SbomFormat,
    spec_version: Option<String>,
    serial_number: Option<String>,
    version: Option<i64>,
    metadata_component: Option<String>,
    tool_name: Option<String>,
    timestamp: Option<String>,
    components: Vec<Component>,
    vulnerabilities: Vec<Vulnerability>,
    document_name: Option<String>,
    document_namespace: Option<String>,
    spdx_version: Option<String>,
    creator: Option<String>,
}

fn detect_format(text: &str) -> SbomFormat {
    let trimmed = text.trim_start();
    if trimmed.starts_with('{') {
        // JSON — check for CycloneDX or SPDX markers
        if text.contains("\"bomFormat\"")
            || text.contains("\"specVersion\"")
            || text.contains("CycloneDX")
        {
            return SbomFormat::CycloneDx;
        }
        if text.contains("\"spdxVersion\"") || text.contains("\"SPDXID\"") {
            return SbomFormat::SpdxJson;
        }
        // default JSON guess
        SbomFormat::CycloneDx
    } else if text.starts_with("SPDXVersion:")
        || text.contains("\nSPDXVersion:")
        || text.contains("SPDXID:")
    {
        SbomFormat::SpdxTv
    } else {
        SbomFormat::CycloneDx
    }
}

fn parse_cyclonedx(json: &Value) -> Sbom {
    let spec_version = json["specVersion"].as_str().map(String::from);
    let serial_number = json["serialNumber"].as_str().map(String::from);
    let version = json["version"].as_i64();

    let metadata = &json["metadata"];
    let tool_name = metadata["tools"]
        .as_array()
        .and_then(|tools| tools.first())
        .and_then(|t| t["name"].as_str())
        .map(String::from)
        .or_else(|| {
            // CycloneDX 1.5+: tools as object with components
            metadata["tools"]["components"]
                .as_array()
                .and_then(|c| c.first())
                .and_then(|t| t["name"].as_str())
                .map(String::from)
        });
    let timestamp = metadata["timestamp"].as_str().map(String::from);
    let metadata_component = metadata["component"]["name"].as_str().map(|n| {
        let ver = metadata["component"]["version"].as_str().unwrap_or("");
        if ver.is_empty() {
            n.to_string()
        } else {
            format!("{} {}", n, ver)
        }
    });

    let mut components = Vec::new();
    if let Some(arr) = json["components"].as_array() {
        for c in arr {
            let name = c["name"].as_str().unwrap_or("?").to_string();
            let version = c["version"].as_str().map(String::from);
            let purl = c["purl"].as_str().map(String::from);
            let component_type = c["type"].as_str().map(String::from);
            let supplier = c["supplier"]["name"].as_str().map(String::from);
            let license = extract_cdx_license(c);
            components.push(Component {
                name,
                version,
                license,
                purl,
                component_type,
                supplier,
            });
        }
    }

    let mut vulnerabilities = Vec::new();
    if let Some(arr) = json["vulnerabilities"].as_array() {
        for v in arr {
            let id = v["id"].as_str().unwrap_or("?").to_string();
            let severity = v["ratings"]
                .as_array()
                .and_then(|r| r.first())
                .and_then(|r| r["severity"].as_str())
                .map(String::from);
            let description = v["description"].as_str().map(String::from);
            let affects = v["affects"]
                .as_array()
                .map(|a| {
                    a.iter()
                        .filter_map(|x| x["ref"].as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default();
            vulnerabilities.push(Vulnerability {
                id,
                severity,
                description,
                affects,
            });
        }
    }

    Sbom {
        format: SbomFormat::CycloneDx,
        spec_version,
        serial_number,
        version,
        metadata_component,
        tool_name,
        timestamp,
        components,
        vulnerabilities,
        document_name: None,
        document_namespace: None,
        spdx_version: None,
        creator: None,
    }
}

fn extract_cdx_license(c: &Value) -> Option<String> {
    // CycloneDX license can be under licenses[].license.id or licenses[].expression
    if let Some(arr) = c["licenses"].as_array() {
        let parts: Vec<String> = arr
            .iter()
            .filter_map(|l| {
                if let Some(id) = l["license"]["id"].as_str() {
                    Some(id.to_string())
                } else if let Some(name) = l["license"]["name"].as_str() {
                    Some(name.to_string())
                } else if let Some(expr) = l["expression"].as_str() {
                    Some(expr.to_string())
                } else {
                    None
                }
            })
            .collect();
        if !parts.is_empty() {
            return Some(parts.join(" OR "));
        }
    }
    None
}

fn parse_spdx_json(json: &Value) -> Sbom {
    let spdx_version = json["spdxVersion"].as_str().map(String::from);
    let document_name = json["name"].as_str().map(String::from);
    let document_namespace = json["documentNamespace"].as_str().map(String::from);
    let creator = json["creationInfo"]["creators"]
        .as_array()
        .and_then(|c| c.first())
        .and_then(|c| c.as_str())
        .map(String::from);
    let timestamp = json["creationInfo"]["created"].as_str().map(String::from);

    let mut components = Vec::new();
    if let Some(pkgs) = json["packages"].as_array() {
        for pkg in pkgs {
            let name = pkg["name"].as_str().unwrap_or("?").to_string();
            let version = pkg["versionInfo"].as_str().map(String::from);
            let supplier = pkg["supplier"].as_str().map(String::from);
            let purl = pkg["externalRefs"]
                .as_array()
                .and_then(|refs| {
                    refs.iter().find(|r| {
                        r["referenceCategory"].as_str() == Some("PACKAGE-MANAGER")
                            || r["referenceType"].as_str() == Some("purl")
                    })
                })
                .and_then(|r| r["referenceLocator"].as_str())
                .map(String::from);
            let license = pkg["licenseConcluded"]
                .as_str()
                .filter(|&l| l != "NOASSERTION" && l != "NONE")
                .or_else(|| {
                    pkg["licenseDeclared"]
                        .as_str()
                        .filter(|&l| l != "NOASSERTION" && l != "NONE")
                })
                .map(String::from);
            components.push(Component {
                name,
                version,
                license,
                purl,
                component_type: None,
                supplier,
            });
        }
    }

    Sbom {
        format: SbomFormat::SpdxJson,
        spec_version: spdx_version.clone(),
        serial_number: document_namespace.clone(),
        version: None,
        metadata_component: document_name.clone(),
        tool_name: creator.clone(),
        timestamp,
        components,
        vulnerabilities: Vec::new(),
        document_name,
        document_namespace,
        spdx_version,
        creator,
    }
}

fn parse_spdx_tv(text: &str) -> Sbom {
    let mut spdx_version = None;
    let mut document_name = None;
    let mut document_namespace = None;
    let mut creator = None;
    let mut timestamp = None;

    let mut components: Vec<Component> = Vec::new();
    let mut current: Option<Component> = None;

    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(pos) = line.find(':') {
            let key = line[..pos].trim();
            let val = line[pos + 1..].trim().to_string();
            match key {
                "SPDXVersion" => spdx_version = Some(val),
                "DocumentName" => document_name = Some(val),
                "DocumentNamespace" => document_namespace = Some(val),
                "Creator" if creator.is_none() => creator = Some(val),
                "Created" => timestamp = Some(val),
                "PackageName" => {
                    if let Some(c) = current.take() {
                        components.push(c);
                    }
                    current = Some(Component {
                        name: val,
                        version: None,
                        license: None,
                        purl: None,
                        component_type: None,
                        supplier: None,
                    });
                }
                "PackageVersion" => {
                    if let Some(c) = current.as_mut() {
                        c.version = Some(val);
                    }
                }
                "PackageSupplier" => {
                    if let Some(c) = current.as_mut() {
                        c.supplier = Some(val);
                    }
                }
                "PackageLicenseConcluded" | "PackageLicenseDeclared" => {
                    if let Some(c) = current.as_mut() {
                        if c.license.is_none() && val != "NOASSERTION" && val != "NONE" {
                            c.license = Some(val);
                        }
                    }
                }
                "ExternalRef" if val.contains("purl") => {
                    // ExternalRef: PACKAGE-MANAGER purl pkg:...
                    if let Some(c) = current.as_mut() {
                        if c.purl.is_none() {
                            if let Some(purl_start) = val.rfind("pkg:") {
                                c.purl = Some(val[purl_start..].to_string());
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }
    if let Some(c) = current.take() {
        components.push(c);
    }

    Sbom {
        format: SbomFormat::SpdxTv,
        spec_version: spdx_version.clone(),
        serial_number: document_namespace.clone(),
        version: None,
        metadata_component: document_name.clone(),
        tool_name: creator.clone(),
        timestamp,
        components,
        vulnerabilities: Vec::new(),
        document_name,
        document_namespace,
        spdx_version,
        creator,
    }
}

fn dispatch(action: &str, text: &str, fmt: &str, limit: usize) -> String {
    let detected_fmt = if fmt == "auto" {
        detect_format(text)
    } else {
        match fmt {
            "cyclonedx" => SbomFormat::CycloneDx,
            "spdx-json" => SbomFormat::SpdxJson,
            "spdx-tv" => SbomFormat::SpdxTv,
            _ => detect_format(text),
        }
    };

    let sbom = match detected_fmt {
        SbomFormat::CycloneDx => match serde_json::from_str::<Value>(text) {
            Ok(json) => parse_cyclonedx(&json),
            Err(e) => return format!("Error: CycloneDX JSON parse error: {}", e),
        },
        SbomFormat::SpdxJson => match serde_json::from_str::<Value>(text) {
            Ok(json) => parse_spdx_json(&json),
            Err(e) => return format!("Error: SPDX JSON parse error: {}", e),
        },
        SbomFormat::SpdxTv => parse_spdx_tv(text),
    };

    let fmt_name = match sbom.format {
        SbomFormat::CycloneDx => "CycloneDX",
        SbomFormat::SpdxJson => "SPDX (JSON)",
        SbomFormat::SpdxTv => "SPDX (tag-value)",
    };

    match action {
        "info" => {
            let mut out = String::from("SBOM INFO\n");
            out.push_str(&format!("  Format       : {}\n", fmt_name));
            if let Some(v) = &sbom.spec_version.as_ref().or(sbom.spdx_version.as_ref()) {
                out.push_str(&format!("  Version      : {}\n", v));
            }
            if let Some(n) = &sbom
                .document_name
                .as_ref()
                .or(sbom.metadata_component.as_ref())
            {
                out.push_str(&format!("  Document     : {}\n", n));
            }
            if let Some(sn) = &sbom.serial_number {
                out.push_str(&format!("  Serial/NS    : {}\n", sn));
            }
            if let Some(v) = sbom.version {
                out.push_str(&format!("  Revision     : {}\n", v));
            }
            if let Some(ts) = &sbom.timestamp {
                out.push_str(&format!("  Created      : {}\n", ts));
            }
            if let Some(tool) = &sbom.tool_name.as_ref().or(sbom.creator.as_ref()) {
                out.push_str(&format!("  Tool/Creator : {}\n", tool));
            }
            out.push_str(&format!("  Components   : {}\n", sbom.components.len()));
            let with_license = sbom
                .components
                .iter()
                .filter(|c| c.license.is_some())
                .count();
            out.push_str(&format!(
                "  With License : {} ({:.0}%)\n",
                with_license,
                if sbom.components.is_empty() {
                    0.0
                } else {
                    with_license as f64 / sbom.components.len() as f64 * 100.0
                }
            ));
            let with_purl = sbom.components.iter().filter(|c| c.purl.is_some()).count();
            out.push_str(&format!("  With PURL    : {}\n", with_purl));
            if !sbom.vulnerabilities.is_empty() {
                out.push_str(&format!(
                    "  Vulnerabilities: {}\n",
                    sbom.vulnerabilities.len()
                ));
            }
            out
        }
        "components" => {
            if sbom.components.is_empty() {
                return "No components found in SBOM".to_string();
            }
            let display: Vec<&Component> = sbom.components.iter().take(limit).collect();
            let total = sbom.components.len();
            let mut out = format!("COMPONENTS ({}/{})\n", display.len(), total);
            out.push_str(&format!(
                "  {:<35} {:<20} {:<25} {}\n",
                "Name", "Version", "License", "PURL prefix"
            ));
            out.push_str(&format!("  {}\n", "-".repeat(95)));
            for c in &display {
                let name = if c.name.len() > 33 {
                    format!("{}…", &c.name[..32])
                } else {
                    c.name.clone()
                };
                let ver = c.version.as_deref().unwrap_or("-");
                let lic = c.license.as_deref().unwrap_or("-");
                let lic_display = if lic.len() > 23 {
                    format!("{}…", &lic[..22])
                } else {
                    lic.to_string()
                };
                let purl_prefix = c
                    .purl
                    .as_deref()
                    .map(|p| {
                        // show "pkg:npm/..." type prefix
                        let trimmed = p.trim_start_matches("pkg:");
                        let slash = trimmed.find('/').unwrap_or(trimmed.len().min(12));
                        format!("pkg:{}", &trimmed[..slash])
                    })
                    .unwrap_or_default();
                out.push_str(&format!(
                    "  {:<35} {:<20} {:<25} {}\n",
                    name,
                    if ver.len() > 18 { &ver[..18] } else { ver },
                    lic_display,
                    purl_prefix
                ));
            }
            if total > limit {
                out.push_str(&format!("\n  ... {} more components\n", total - limit));
            }
            out
        }
        "licenses" => {
            let mut counts: std::collections::HashMap<String, usize> =
                std::collections::HashMap::new();
            let mut no_license = 0usize;
            for c in &sbom.components {
                match &c.license {
                    Some(l) => {
                        *counts.entry(l.clone()).or_insert(0) += 1;
                    }
                    None => no_license += 1,
                }
            }
            let total = sbom.components.len();
            let mut sorted: Vec<(String, usize)> = counts.into_iter().collect();
            sorted.sort_by(|a, b| b.1.cmp(&a.1));

            let mut out = format!("LICENSE DISTRIBUTION ({} components)\n\n", total);
            let bar_width = 30usize;
            for (lic, count) in sorted.iter().take(limit) {
                let pct = if total == 0 {
                    0.0
                } else {
                    *count as f64 / total as f64 * 100.0
                };
                let fill = (pct / 100.0 * bar_width as f64) as usize;
                let bar = format!("{}{}", "█".repeat(fill), "░".repeat(bar_width - fill));
                out.push_str(&format!(
                    "  {:30} {:4} ({:5.1}%) {}\n",
                    lic, count, pct, bar
                ));
            }
            if no_license > 0 {
                let pct = no_license as f64 / total as f64 * 100.0;
                let fill = (pct / 100.0 * bar_width as f64) as usize;
                let bar = format!("{}{}", "░".repeat(fill), " ".repeat(bar_width - fill));
                out.push_str(&format!(
                    "  {:30} {:4} ({:5.1}%) {} (no license)\n",
                    "(unknown/unlicensed)", no_license, pct, bar
                ));
            }
            out.push_str(&format!(
                "\n  Unique license expressions: {}\n",
                sorted.len()
            ));
            out
        }
        "vulnerabilities" => {
            if sbom.vulnerabilities.is_empty() {
                match sbom.format {
                    SbomFormat::CycloneDx => {
                        return "No vulnerabilities section in this CycloneDX SBOM".to_string()
                    }
                    _ => {
                        return "Vulnerability data is only available in CycloneDX SBOMs"
                            .to_string()
                    }
                }
            }
            let mut out = format!("VULNERABILITIES ({})\n\n", sbom.vulnerabilities.len());
            let mut severity_counts: std::collections::HashMap<String, usize> =
                std::collections::HashMap::new();
            for v in &sbom.vulnerabilities {
                let sev = v.severity.clone().unwrap_or_else(|| "unknown".to_string());
                *severity_counts.entry(sev).or_insert(0) += 1;
            }
            let order = ["critical", "high", "medium", "low", "info", "unknown"];
            for sev in &order {
                if let Some(count) = severity_counts.get(*sev) {
                    out.push_str(&format!("  {:8}: {}\n", sev.to_uppercase(), count));
                }
            }
            out.push('\n');
            for v in sbom.vulnerabilities.iter().take(limit) {
                out.push_str(&format!(
                    "  {} [{}]\n",
                    v.id,
                    v.severity.as_deref().unwrap_or("?")
                ));
                if let Some(desc) = &v.description {
                    let snippet = if desc.len() > 100 {
                        format!("{}…", &desc[..99])
                    } else {
                        desc.clone()
                    };
                    out.push_str(&format!("    {}\n", snippet));
                }
                if !v.affects.is_empty() {
                    out.push_str(&format!("    Affects: {}\n", v.affects.join(", ")));
                }
            }
            out
        }
        "validate" => {
            let mut issues: Vec<String> = Vec::new();
            let mut warnings: Vec<String> = Vec::new();

            match sbom.format {
                SbomFormat::CycloneDx => {
                    if sbom.spec_version.is_none() {
                        issues.push("ERROR: missing 'specVersion'".to_string());
                    }
                    if sbom.serial_number.is_none() {
                        warnings.push(
                            "WARN: missing 'serialNumber' (recommended for SBOM identity)"
                                .to_string(),
                        );
                    }
                    if sbom.components.is_empty() {
                        warnings.push("WARN: no components listed".to_string());
                    }
                    if sbom.tool_name.is_none() {
                        warnings.push("WARN: no tool information in metadata".to_string());
                    }
                    if sbom.timestamp.is_none() {
                        warnings.push("WARN: no timestamp in metadata".to_string());
                    }
                }
                SbomFormat::SpdxJson | SbomFormat::SpdxTv => {
                    if sbom.spdx_version.is_none() {
                        issues.push("ERROR: missing SPDXVersion".to_string());
                    }
                    if sbom.document_name.is_none() {
                        issues.push("ERROR: missing DocumentName".to_string());
                    }
                    if sbom.document_namespace.is_none() {
                        issues.push("ERROR: missing DocumentNamespace".to_string());
                    }
                    if sbom.components.is_empty() {
                        warnings.push("WARN: no packages listed".to_string());
                    }
                    if sbom.creator.is_none() {
                        warnings.push("WARN: no Creator field".to_string());
                    }
                    if sbom.timestamp.is_none() {
                        warnings.push("WARN: no Created timestamp".to_string());
                    }
                }
            }

            // license coverage check
            let no_license = sbom
                .components
                .iter()
                .filter(|c| c.license.is_none())
                .count();
            if no_license > 0 && !sbom.components.is_empty() {
                let pct = no_license * 100 / sbom.components.len();
                warnings.push(format!(
                    "WARN: {}/{} components ({:}%) have no license information",
                    no_license,
                    sbom.components.len(),
                    pct
                ));
            }

            let verdict = if issues.is_empty() && warnings.is_empty() {
                "VALID"
            } else if issues.is_empty() {
                "WARNINGS"
            } else {
                "INVALID"
            };

            let mut out = format!("SBOM VALIDATION: {}\n\n", verdict);
            out.push_str(&format!("  Format      : {}\n", fmt_name));
            out.push_str(&format!("  Components  : {}\n", sbom.components.len()));
            let with_lic = sbom
                .components
                .iter()
                .filter(|c| c.license.is_some())
                .count();
            out.push_str(&format!("  With License: {}\n", with_lic));

            if !issues.is_empty() || !warnings.is_empty() {
                out.push_str("\nFindings:\n");
                for i in &issues {
                    out.push_str(&format!("  {}\n", i));
                }
                for w in &warnings {
                    out.push_str(&format!("  {}\n", w));
                }
            }
            out
        }
        _ => format!("Error: unknown action '{}'", action),
    }
}

pub async fn execute(args: &Value) -> Result<String, String> {
    let action = args["action"].as_str().unwrap_or("info");
    let fmt = args["format"].as_str().unwrap_or("auto");
    let limit = args["limit"].as_u64().unwrap_or(50) as usize;

    let text: String = if let Some(path) = args["file"].as_str() {
        match std::fs::read_to_string(path) {
            Ok(s) => s,
            Err(e) => return Ok(format!("Error reading file '{}': {}", path, e)),
        }
    } else if let Some(t) = args["text"].as_str() {
        t.to_string()
    } else {
        return Ok(
            "Error: provide 'file' (path to SBOM file) or 'text' (SBOM content string)".to_string(),
        );
    };

    if text.trim().is_empty() {
        return Ok("Error: empty input".to_string());
    }

    Ok(dispatch(action, &text, fmt, limit))
}
