use serde_json::Value;

pub async fn execute(args: &Value) -> Result<String, String> {
    let action = if let Some(a) = args.get("action").and_then(|v| v.as_str()) {
        a.to_string()
    } else if args.get("version").is_some() {
        "get".to_string()
    } else {
        "list".to_string()
    };
    match action.as_str() {
        "list" => list_action(args),
        "get" => get_action(args),
        "latest" => latest_action(args),
        "validate" => validate_action(args),
        _ => Err(format!(
            "Unknown action '{}'. Valid: list, get, latest, validate",
            action
        )),
    }
}

fn get_changelog(args: &Value) -> Result<String, String> {
    args.get("text")
        .or_else(|| args.get("changelog"))
        .or_else(|| args.get("content"))
        .or_else(|| args.get("input"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| "Missing 'text' — pass the CHANGELOG.md content as a string".to_string())
}

#[derive(Debug, Clone)]
struct Release {
    version: String,
    date: Option<String>,
    yanked: bool,
    sections: Vec<(String, Vec<String>)>, // (section_name, items)
    raw_body: String,
}

fn parse_changelog(text: &str) -> Vec<Release> {
    let mut releases: Vec<Release> = Vec::new();
    let mut current: Option<Release> = None;
    let mut current_section: Option<String> = None;
    let mut current_items: Vec<String> = Vec::new();
    let mut body_lines: Vec<&str> = Vec::new();

    for line in text.lines() {
        let trimmed = line.trim();

        // Version heading: ## [x.y.z] or ## [Unreleased] with optional date
        if trimmed.starts_with("## ") {
            // Flush current section
            if let Some(ref mut rel) = current {
                if let Some(sec) = current_section.take() {
                    rel.sections.push((sec, current_items.drain(..).collect()));
                }
                rel.raw_body = body_lines.join("\n");
                releases.push(rel.clone());
            }
            body_lines.clear();
            current_items.clear();
            current_section = None;

            let heading = &trimmed[3..];
            let yanked = heading.to_uppercase().contains("[YANKED]");

            // Extract version and date
            let (version, date) = parse_version_heading(heading);
            current = Some(Release {
                version,
                date,
                yanked,
                sections: Vec::new(),
                raw_body: String::new(),
            });
            continue;
        }

        // Section heading within a release: ### Added / ### Changed / etc.
        if trimmed.starts_with("### ") {
            if let Some(ref mut rel) = current {
                if let Some(sec) = current_section.take() {
                    rel.sections.push((sec, current_items.drain(..).collect()));
                }
                current_section = Some(trimmed[4..].trim().to_string());
                current_items.clear();
            }
            body_lines.push(line);
            continue;
        }

        // List item
        if let Some(ref mut _rel) = current {
            if trimmed.starts_with("- ") || trimmed.starts_with("* ") {
                current_items.push(trimmed[2..].trim().to_string());
            }
            body_lines.push(line);
        }
    }

    // Flush last release
    if let Some(ref mut rel) = current {
        if let Some(sec) = current_section.take() {
            rel.sections.push((sec, current_items));
        }
        rel.raw_body = body_lines.join("\n");
        releases.push(rel.clone());
    }

    releases
}

fn parse_version_heading(heading: &str) -> (String, Option<String>) {
    // Format: [v1.2.3] - 2024-01-15  or  [Unreleased]  or  v1.2.3 - 2024-01-15
    let heading = heading.trim();

    // Remove [YANKED] suffix
    let heading = if let Some(pos) = heading.to_uppercase().find("[YANKED]") {
        heading[..pos].trim()
    } else {
        heading
    };

    // Extract bracketed version
    let (version_part, rest) = if heading.starts_with('[') {
        if let Some(close) = heading.find(']') {
            (&heading[1..close], heading[close + 1..].trim())
        } else {
            (heading, "")
        }
    } else {
        // No brackets — split on " - "
        if let Some(pos) = heading.find(" - ") {
            (&heading[..pos], heading[pos + 3..].trim())
        } else {
            (heading, "")
        }
    };

    let date = if rest.starts_with("- ") {
        Some(rest[2..].trim().to_string())
    } else if !rest.is_empty() {
        Some(rest.to_string())
    } else {
        None
    };

    (version_part.trim().to_string(), date)
}

fn list_action(args: &Value) -> Result<String, String> {
    let text = get_changelog(args)?;
    let releases = parse_changelog(&text);

    let mut out = format!(
        "CHANGELOG  [{} release(s)]\n{}\n\n",
        releases.len(),
        "=".repeat(44)
    );

    for rel in &releases {
        let date_str = rel.date.as_deref().unwrap_or("no date");
        let yanked_str = if rel.yanked { "  [YANKED]" } else { "" };
        let item_count: usize = rel.sections.iter().map(|(_, items)| items.len()).sum();
        out += &format!(
            "{:<20} {}  ({} item(s)){}\n",
            rel.version, date_str, item_count, yanked_str
        );
        for (sec, items) in &rel.sections {
            out += &format!("  {}: {} item(s)\n", sec, items.len());
        }
    }
    Ok(out)
}

fn get_action(args: &Value) -> Result<String, String> {
    let text = get_changelog(args)?;
    let query = args
        .get("version")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'version' — e.g. '1.2.3' or 'Unreleased'")?
        .to_lowercase();

    let releases = parse_changelog(&text);
    let rel = releases
        .iter()
        .find(|r| r.version.to_lowercase().contains(&query) || r.version.to_lowercase() == query)
        .ok_or_else(|| {
            format!(
                "Version '{}' not found. Use action='list' to see all versions.",
                query
            )
        })?;

    let date_str = rel.date.as_deref().unwrap_or("no date");
    let yanked_str = if rel.yanked { "  [YANKED]" } else { "" };
    let mut out = format!(
        "{} — {}{}\n{}\n\n",
        rel.version,
        date_str,
        yanked_str,
        "=".repeat(44)
    );
    out += &rel.raw_body;
    out += "\n";
    Ok(out)
}

fn latest_action(args: &Value) -> Result<String, String> {
    let text = get_changelog(args)?;
    let releases = parse_changelog(&text);

    // Skip "Unreleased" for latest
    let rel = releases
        .iter()
        .find(|r| !r.version.to_lowercase().contains("unreleased"))
        .or_else(|| releases.first())
        .ok_or("No releases found in changelog.")?;

    let date_str = rel.date.as_deref().unwrap_or("no date");
    let mut out = format!(
        "Latest: {} — {}\n{}\n\n",
        rel.version,
        date_str,
        "=".repeat(44)
    );
    out += &rel.raw_body;
    out += "\n";
    Ok(out)
}

fn validate_action(args: &Value) -> Result<String, String> {
    let text = get_changelog(args)?;
    let releases = parse_changelog(&text);
    let mut warnings: Vec<String> = Vec::new();

    // Check for Unreleased section
    let has_unreleased = releases
        .iter()
        .any(|r| r.version.to_lowercase().contains("unreleased"));
    if !has_unreleased {
        warnings
            .push("No [Unreleased] section found. Keep a Changelog recommends one.".to_string());
    }

    // Check for dates
    let missing_dates: Vec<&str> = releases
        .iter()
        .filter(|r| r.date.is_none() && !r.version.to_lowercase().contains("unreleased"))
        .map(|r| r.version.as_str())
        .collect();
    for v in &missing_dates {
        warnings.push(format!("Release '{}' is missing a date.", v));
    }

    // Check for known section names (Keep a Changelog standard)
    let known_sections = [
        "Added",
        "Changed",
        "Deprecated",
        "Removed",
        "Fixed",
        "Security",
    ];
    for rel in &releases {
        for (sec, _) in &rel.sections {
            if !known_sections.iter().any(|k| k.eq_ignore_ascii_case(sec)) {
                warnings.push(format!(
                    "Release '{}': non-standard section '{}'. Standard: {}",
                    rel.version,
                    sec,
                    known_sections.join(", ")
                ));
            }
        }
    }

    // Check for YANKED releases
    for rel in releases.iter().filter(|r| r.yanked) {
        warnings.push(format!("Release '{}' is marked as YANKED.", rel.version));
    }

    // Check for empty releases
    for rel in &releases {
        if rel.sections.is_empty() && !rel.version.to_lowercase().contains("unreleased") {
            warnings.push(format!(
                "Release '{}' has no content sections.",
                rel.version
            ));
        }
    }

    let mut out = format!("CHANGELOG Validation\n{}\n\n", "=".repeat(44));
    out += &format!(
        "Result: {}\n\n",
        if warnings.is_empty() {
            "VALID"
        } else {
            "VALID with warnings"
        }
    );
    out += &format!("{} release(s) parsed.\n", releases.len());
    if warnings.is_empty() {
        out += "No issues found.\n";
    } else {
        out += &format!("\n{} warning(s):\n", warnings.len());
        for w in &warnings {
            out += &format!("  [WARN] {}\n", w);
        }
    }
    Ok(out)
}
