use serde_json::Value;

pub async fn execute(args: &Value) -> Result<String, String> {
    let action = if let Some(a) = args.get("action").and_then(|v| v.as_str()) {
        a.to_string()
    } else if args.get("url").is_some() || args.get("path").is_some() {
        "check".to_string()
    } else {
        "parse".to_string()
    };
    match action.as_str() {
        "parse" => parse_action(args),
        "check" => check_action(args),
        "validate" => validate_action(args),
        "summary" => summary_action(args),
        _ => Err(format!(
            "Unknown action '{}'. Valid: parse, check, validate, summary",
            action
        )),
    }
}

fn get_text(args: &Value) -> Result<String, String> {
    args.get("text")
        .or_else(|| args.get("input"))
        .or_else(|| args.get("robots"))
        .or_else(|| args.get("content"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| "Missing 'text' — pass the robots.txt content as a string".to_string())
}

#[derive(Debug, Clone)]
struct UserAgentBlock {
    agents: Vec<String>,
    disallows: Vec<String>,
    allows: Vec<String>,
    crawl_delay: Option<f64>,
}

fn parse_robots(text: &str) -> (Vec<UserAgentBlock>, Vec<String>, Vec<String>) {
    let mut blocks: Vec<UserAgentBlock> = Vec::new();
    let mut sitemaps: Vec<String> = Vec::new();
    let mut hosts: Vec<String> = Vec::new();

    let mut current_agents: Vec<String> = Vec::new();
    let mut current_disallows: Vec<String> = Vec::new();
    let mut current_allows: Vec<String> = Vec::new();
    let mut current_crawl_delay: Option<f64> = None;
    let mut in_block = false;

    for raw_line in text.lines() {
        let line = if let Some(comment_pos) = raw_line.find('#') {
            raw_line[..comment_pos].trim()
        } else {
            raw_line.trim()
        };

        if line.is_empty() {
            if in_block && !current_agents.is_empty() {
                blocks.push(UserAgentBlock {
                    agents: current_agents.clone(),
                    disallows: current_disallows.clone(),
                    allows: current_allows.clone(),
                    crawl_delay: current_crawl_delay,
                });
                current_agents.clear();
                current_disallows.clear();
                current_allows.clear();
                current_crawl_delay = None;
                in_block = false;
            }
            continue;
        }

        let lower = line.to_lowercase();
        if let Some(val) = field_value(&lower, line, "user-agent:") {
            if in_block && !current_agents.is_empty() {
                blocks.push(UserAgentBlock {
                    agents: current_agents.clone(),
                    disallows: current_disallows.clone(),
                    allows: current_allows.clone(),
                    crawl_delay: current_crawl_delay,
                });
                current_agents.clear();
                current_disallows.clear();
                current_allows.clear();
                current_crawl_delay = None;
            }
            current_agents.push(val.trim().to_string());
            in_block = true;
        } else if let Some(val) = field_value(&lower, line, "disallow:") {
            current_disallows.push(val.trim().to_string());
        } else if let Some(val) = field_value(&lower, line, "allow:") {
            current_allows.push(val.trim().to_string());
        } else if let Some(val) = field_value(&lower, line, "crawl-delay:") {
            current_crawl_delay = val.trim().parse::<f64>().ok();
        } else if field_value(&lower, line, "sitemap:").is_some() {
            sitemaps.push(
                line[line.to_lowercase().find("sitemap:").unwrap() + 8..]
                    .trim()
                    .to_string(),
            );
        } else if let Some(val) = field_value(&lower, line, "host:") {
            hosts.push(val.trim().to_string());
        }
    }

    // flush final block
    if in_block && !current_agents.is_empty() {
        blocks.push(UserAgentBlock {
            agents: current_agents,
            disallows: current_disallows,
            allows: current_allows,
            crawl_delay: current_crawl_delay,
        });
    }

    (blocks, sitemaps, hosts)
}

fn field_value<'a>(lower: &'a str, original: &'a str, field: &str) -> Option<&'a str> {
    if lower.starts_with(field) {
        Some(&original[field.len()..])
    } else {
        None
    }
}

fn path_matches_rule(path: &str, rule: &str) -> bool {
    if rule.is_empty() {
        return false;
    }
    // Rule ending in $ means exact match
    let (rule_pat, exact) = if rule.ends_with('$') {
        (&rule[..rule.len() - 1], true)
    } else {
        (rule, false)
    };
    // Wildcard matching
    if !rule_pat.contains('*') {
        if exact {
            path == rule_pat
        } else {
            path.starts_with(rule_pat)
        }
    } else {
        wildcard_match(path, rule_pat, exact)
    }
}

fn wildcard_match(path: &str, pattern: &str, exact: bool) -> bool {
    let parts: Vec<&str> = pattern.split('*').collect();
    if parts.is_empty() {
        return true;
    }
    let mut pos = 0usize;
    let path_bytes = path.as_bytes();
    for (i, part) in parts.iter().enumerate() {
        if part.is_empty() {
            continue;
        }
        if i == 0 {
            if !path.starts_with(part) {
                return false;
            }
            pos = part.len();
        } else {
            if let Some(found) = path[pos..].find(part) {
                pos += found + part.len();
            } else {
                return false;
            }
        }
    }
    if exact {
        pos == path_bytes.len()
    } else {
        true
    }
}

fn parse_action(args: &Value) -> Result<String, String> {
    let text = get_text(args)?;
    let (blocks, sitemaps, hosts) = parse_robots(&text);
    let _total_lines = text
        .lines()
        .filter(|l| !l.trim().is_empty() && !l.trim().starts_with('#'))
        .count();

    let mut out = format!("robots.txt\n{}\n\n", "=".repeat(44));
    out += &format!(
        "{} user-agent block(s)  {} sitemap(s)\n\n",
        blocks.len(),
        sitemaps.len()
    );

    for block in &blocks {
        let agents_str = block.agents.join(", ");
        out += &format!("User-agent: {}\n", agents_str);
        if let Some(d) = block.crawl_delay {
            out += &format!("  Crawl-delay: {}\n", d);
        }
        if block.disallows.is_empty() && block.allows.is_empty() {
            out += "  (no rules)\n";
        }
        for path in &block.allows {
            out += &format!(
                "  Allow:    {}\n",
                if path.is_empty() {
                    "(empty — allows all)"
                } else {
                    path
                }
            );
        }
        for path in &block.disallows {
            out += &format!(
                "  Disallow: {}\n",
                if path.is_empty() {
                    "(empty — disallows nothing)"
                } else {
                    path
                }
            );
        }
        out += "\n";
    }

    if !sitemaps.is_empty() {
        out += "Sitemaps:\n";
        for s in &sitemaps {
            out += &format!("  {}\n", s);
        }
        out += "\n";
    }
    if !hosts.is_empty() {
        out += "Host directives:\n";
        for h in &hosts {
            out += &format!("  {}\n", h);
        }
        out += "\n";
    }
    Ok(out)
}

fn check_action(args: &Value) -> Result<String, String> {
    let text = get_text(args)?;
    let url_or_path = args
        .get("url")
        .or_else(|| args.get("path"))
        .and_then(|v| v.as_str())
        .ok_or("Missing 'url' or 'path' — the path to test (e.g. '/admin/login')")?;
    let agent = args
        .get("agent")
        .or_else(|| args.get("user_agent"))
        .or_else(|| args.get("ua"))
        .and_then(|v| v.as_str())
        .unwrap_or("*");

    // Extract path from URL if full URL given
    let check_path = if url_or_path.starts_with("http://") || url_or_path.starts_with("https://") {
        if let Some(path_start) = url_or_path
            .find("//")
            .and_then(|i| url_or_path[i + 2..].find('/').map(|j| i + 2 + j))
        {
            &url_or_path[path_start..]
        } else {
            "/"
        }
    } else {
        url_or_path
    };

    let (blocks, _, _) = parse_robots(&text);

    // Find matching blocks: exact agent match first, then wildcard
    let exact_block = blocks
        .iter()
        .find(|b| b.agents.iter().any(|a| a.eq_ignore_ascii_case(agent)));
    let wildcard_block = blocks.iter().find(|b| b.agents.iter().any(|a| a == "*"));

    let target_block = exact_block.or(wildcard_block);

    let mut out = format!("robots.txt Check\n{}\n\n", "=".repeat(44));
    out += &format!("Path:       {}\n", check_path);
    out += &format!("User-agent: {}\n\n", agent);

    if let Some(block) = target_block {
        let matched_agent = if exact_block.is_some() { agent } else { "*" };
        out += &format!("Matched block: User-agent: {}\n\n", matched_agent);

        // RFC 9309: most-specific matching rule wins; Allow beats Disallow of equal length
        let mut best_allow: Option<&str> = None;
        let mut best_disallow: Option<&str> = None;

        for allow_rule in &block.allows {
            if path_matches_rule(check_path, allow_rule) {
                if best_allow.map(|b: &str| b.len()).unwrap_or(0) <= allow_rule.len() {
                    best_allow = Some(allow_rule);
                }
            }
        }
        for disallow_rule in &block.disallows {
            if !disallow_rule.is_empty() && path_matches_rule(check_path, disallow_rule) {
                if best_disallow.map(|b: &str| b.len()).unwrap_or(0) <= disallow_rule.len() {
                    best_disallow = Some(disallow_rule);
                }
            }
        }

        let allowed = match (best_allow, best_disallow) {
            (Some(a), Some(d)) => a.len() >= d.len(), // Allow wins ties
            (Some(_), None) => true,
            (None, Some(_)) => false,
            (None, None) => true, // no matching rule = allowed
        };

        out += if allowed {
            "Result: ALLOWED\n"
        } else {
            "Result: BLOCKED\n"
        };
        if let Some(r) = best_allow {
            out += &format!("  Matching Allow rule:    {}\n", r);
        }
        if let Some(r) = best_disallow {
            out += &format!("  Matching Disallow rule: {}\n", r);
        }
        if best_allow.is_none() && best_disallow.is_none() {
            out += "  No matching rules — allowed by default\n";
        }
    } else {
        out += "No matching user-agent block found.\n";
        out += "Result: ALLOWED (no applicable rules)\n";
    }
    Ok(out)
}

fn validate_action(args: &Value) -> Result<String, String> {
    let text = get_text(args)?;
    let mut warnings: Vec<String> = Vec::new();
    let mut line_num = 0usize;

    for raw_line in text.lines() {
        line_num += 1;
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let lower = line.to_lowercase();
        let known_fields = [
            "user-agent:",
            "disallow:",
            "allow:",
            "crawl-delay:",
            "sitemap:",
            "host:",
            "request-rate:",
            "visit-time:",
            "noindex:",
        ];
        let known = known_fields.iter().any(|f| lower.starts_with(f));
        if !known {
            if line.contains(':') {
                warnings.push(format!("Line {}: unknown directive '{}'", line_num, line));
            } else {
                warnings.push(format!(
                    "Line {}: unrecognized line (missing directive prefix?): '{}'",
                    line_num, line
                ));
            }
        }
        // Check Allow/Disallow paths start with /
        if lower.starts_with("disallow:") || lower.starts_with("allow:") {
            let colon_pos = line.find(':').unwrap_or(0);
            let path = line[colon_pos + 1..].trim();
            if !path.is_empty() && !path.starts_with('/') && !path.starts_with('*') {
                warnings.push(format!(
                    "Line {}: path '{}' should start with '/' or '*'",
                    line_num, path
                ));
            }
        }
    }

    let (blocks, sitemaps, _) = parse_robots(&text);
    let has_wildcard = blocks.iter().any(|b| b.agents.iter().any(|a| a == "*"));
    if !has_wildcard {
        warnings.push("No 'User-agent: *' block — crawlers not explicitly addressed may use their own defaults".to_string());
    }
    for block in &blocks {
        let _all_empty_disallow = block.disallows.iter().all(|d| d.is_empty());
        let has_disallow_slash = block.disallows.iter().any(|d| d == "/");
        if has_disallow_slash {
            warnings.push(format!(
                "User-agent '{}': Disallow: / blocks the entire site",
                block.agents.join(", ")
            ));
        }
    }
    for sitemap in &sitemaps {
        if !sitemap.starts_with("http://") && !sitemap.starts_with("https://") {
            warnings.push(format!(
                "Sitemap '{}' should be an absolute URL starting with http:// or https://",
                sitemap
            ));
        }
    }

    let mut out = format!("robots.txt Validation\n{}\n\n", "=".repeat(44));
    out += &format!(
        "Result: {}\n\n",
        if warnings.is_empty() {
            "VALID"
        } else {
            "VALID with warnings"
        }
    );
    if warnings.is_empty() {
        out += "No issues found.\n";
    } else {
        out += &format!("{} warning(s):\n", warnings.len());
        for w in &warnings {
            out += &format!("  [WARN] {}\n", w);
        }
    }
    Ok(out)
}

fn summary_action(args: &Value) -> Result<String, String> {
    let text = get_text(args)?;
    let (blocks, sitemaps, _) = parse_robots(&text);

    let mut out = format!("robots.txt Summary\n{}\n\n", "=".repeat(44));
    out += &format!(
        "{} block(s)  {} sitemap(s)\n\n",
        blocks.len(),
        sitemaps.len()
    );
    out += &format!(
        "{:<30} {:>8} {:>8} {}\n",
        "User-agent", "Allows", "Disallows", "Crawl-delay"
    );
    out += &format!("{}\n", "-".repeat(62));
    for block in &blocks {
        let agent_str = block.agents.join(", ");
        let short = if agent_str.len() > 28 {
            format!("{}...", &agent_str[..25])
        } else {
            agent_str
        };
        let delay_str = block
            .crawl_delay
            .map(|d| format!("{}s", d))
            .unwrap_or_else(|| "-".to_string());
        out += &format!(
            "{:<30} {:>8} {:>8} {}\n",
            short,
            block.allows.len(),
            block.disallows.len(),
            delay_str
        );
    }
    if !sitemaps.is_empty() {
        out += "\nSitemaps:\n";
        for s in &sitemaps {
            out += &format!("  {}\n", s);
        }
    }
    Ok(out)
}
