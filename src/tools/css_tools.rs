use regex::Regex;
use serde_json::Value;

pub async fn execute(args: &Value) -> Result<String, String> {
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("parse");
    match action {
        "parse" | "list" => action_parse(args),
        "validate" => action_validate(args),
        "vars" | "variables" => action_vars(args),
        "stats" => action_stats(args),
        "minify" => action_minify(args),
        other => Err(format!(
            "css_tools: unknown action '{other}'. Valid: parse, validate, vars, stats, minify"
        )),
    }
}

// ── I/O ───────────────────────────────────────────────────────────────────────

fn get_css(args: &Value) -> Result<String, String> {
    if let Some(s) = args
        .get("text")
        .or_else(|| args.get("css"))
        .or_else(|| args.get("input"))
        .and_then(|v| v.as_str())
    {
        return Ok(s.to_string());
    }
    if let Some(path) = args.get("file").and_then(|v| v.as_str()) {
        return std::fs::read_to_string(path)
            .map_err(|e| format!("css_tools: cannot read '{path}': {e}"));
    }
    Err("css_tools: 'text'/'css' (inline CSS) or 'file' (file path) is required".to_string())
}

// ── Data structures ───────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
struct CssRule {
    selector: String,
    declarations: Vec<(String, String)>, // (property, value)
    line: usize,
}

#[derive(Debug, Clone)]
struct AtRule {
    kind: String,        // "media", "keyframes", etc.
    params: String,      // everything after @keyword up to the block
    inner: Vec<CssRule>, // nested rules (only meaningful for block at-rules)
    #[allow(dead_code)]
    line: usize,
}

#[derive(Debug, Default)]
struct ParsedCss {
    rules: Vec<CssRule>,
    at_rules: Vec<AtRule>,
}

// ── Preprocessor ──────────────────────────────────────────────────────────────

/// Strip block comments and non-standard line comments from CSS text.
fn strip_comments(src: &str) -> String {
    let mut out = String::with_capacity(src.len());
    let chars: Vec<char> = src.chars().collect();
    let n = chars.len();
    let mut i = 0;
    while i < n {
        if i + 1 < n && chars[i] == '/' && chars[i + 1] == '*' {
            // Block comment — advance until */
            i += 2;
            while i + 1 < n && !(chars[i] == '*' && chars[i + 1] == '/') {
                if chars[i] == '\n' {
                    out.push('\n'); // preserve line count for line numbers
                }
                i += 1;
            }
            i += 2; // skip */
        } else if i + 1 < n && chars[i] == '/' && chars[i + 1] == '/' {
            // Line comment — skip to end of line
            while i < n && chars[i] != '\n' {
                i += 1;
            }
        } else {
            out.push(chars[i]);
            i += 1;
        }
    }
    out
}

// ── Parser ────────────────────────────────────────────────────────────────────

fn parse_css(src: &str) -> ParsedCss {
    let cleaned = strip_comments(src);
    let mut result = ParsedCss::default();
    let mut line_no = 1usize;
    let chars: Vec<char> = cleaned.chars().collect();
    let n = chars.len();
    let mut i = 0;

    while i < n {
        // Skip whitespace, tracking line numbers
        while i < n && chars[i].is_whitespace() {
            if chars[i] == '\n' {
                line_no += 1;
            }
            i += 1;
        }
        if i >= n {
            break;
        }

        // At-rule
        if chars[i] == '@' {
            let start_line = line_no;
            i += 1;
            let keyword = read_ident(&chars, &mut i, &mut line_no);
            // skip whitespace
            while i < n && chars[i].is_whitespace() {
                if chars[i] == '\n' {
                    line_no += 1;
                }
                i += 1;
            }
            // Read params up to { or ;
            let params = read_until_brace_or_semi(&chars, &mut i, &mut line_no);
            if i < n && chars[i] == '{' {
                i += 1; // consume {
                        // Collect block content, respecting nested braces
                let block = read_block(&chars, &mut i, &mut line_no);
                let inner = parse_rules_in_block(&block);
                result.at_rules.push(AtRule {
                    kind: keyword,
                    params: params.trim().to_string(),
                    inner,
                    line: start_line,
                });
            } else {
                // Simple at-rule ending with ;
                if i < n && chars[i] == ';' {
                    i += 1;
                }
                result.at_rules.push(AtRule {
                    kind: keyword,
                    params: params.trim().to_string(),
                    inner: Vec::new(),
                    line: start_line,
                });
            }
            continue;
        }

        // Regular rule: selector { declarations }
        let start_line = line_no;
        let selector_raw = read_until_brace_or_eof(&chars, &mut i, &mut line_no);
        let selector = selector_raw.trim().to_string();
        if selector.is_empty() {
            break;
        }
        if i >= n || chars[i] != '{' {
            break;
        }
        i += 1; // consume {
        let block = read_block(&chars, &mut i, &mut line_no);
        let declarations = parse_declarations(&block);
        result.rules.push(CssRule {
            selector,
            declarations,
            line: start_line,
        });
    }

    result
}

fn read_ident(chars: &[char], i: &mut usize, line_no: &mut usize) -> String {
    let mut s = String::new();
    while *i < chars.len() && (chars[*i].is_alphanumeric() || chars[*i] == '-' || chars[*i] == '_')
    {
        if chars[*i] == '\n' {
            *line_no += 1;
        }
        s.push(chars[*i]);
        *i += 1;
    }
    s
}

fn read_until_brace_or_semi(chars: &[char], i: &mut usize, line_no: &mut usize) -> String {
    let mut s = String::new();
    while *i < chars.len() && chars[*i] != '{' && chars[*i] != ';' {
        if chars[*i] == '\n' {
            *line_no += 1;
        }
        s.push(chars[*i]);
        *i += 1;
    }
    s
}

fn read_until_brace_or_eof(chars: &[char], i: &mut usize, line_no: &mut usize) -> String {
    let mut s = String::new();
    let mut depth = 0i32;
    while *i < chars.len() {
        match chars[*i] {
            '{' if depth == 0 => break,
            '{' => {
                depth += 1;
                s.push(chars[*i]);
            }
            '}' => {
                depth -= 1;
                s.push(chars[*i]);
            }
            '\n' => {
                *line_no += 1;
                s.push(chars[*i]);
            }
            c => s.push(c),
        }
        *i += 1;
    }
    s
}

/// Reads until the matching closing brace (after the opening `{` has been consumed).
fn read_block(chars: &[char], i: &mut usize, line_no: &mut usize) -> String {
    let mut s = String::new();
    let mut depth = 1i32;
    while *i < chars.len() && depth > 0 {
        match chars[*i] {
            '{' => {
                depth += 1;
                s.push(chars[*i]);
            }
            '}' => {
                depth -= 1;
                if depth > 0 {
                    s.push(chars[*i]);
                }
            }
            '\n' => {
                *line_no += 1;
                s.push(chars[*i]);
            }
            c => s.push(c),
        }
        *i += 1;
    }
    s
}

/// Parse declarations from a block body string.
fn parse_declarations(block: &str) -> Vec<(String, String)> {
    let mut decls = Vec::new();
    for raw in block.split(';') {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Some(colon) = trimmed.find(':') {
            let prop = trimmed[..colon].trim().to_lowercase();
            let val = trimmed[colon + 1..].trim().to_string();
            if !prop.is_empty() {
                decls.push((prop, val));
            }
        }
    }
    decls
}

/// Parse regular rules out of a block (used for @media, @supports, etc.).
fn parse_rules_in_block(block: &str) -> Vec<CssRule> {
    // Lightweight: parse selector { ... } patterns inside the block
    let mut rules = Vec::new();
    let chars: Vec<char> = block.chars().collect();
    let n = chars.len();
    let mut i = 0;
    let mut line_no = 1usize;

    while i < n {
        while i < n && chars[i].is_whitespace() {
            if chars[i] == '\n' {
                line_no += 1;
            }
            i += 1;
        }
        if i >= n {
            break;
        }
        let start_line = line_no;
        let sel = read_until_brace_or_eof(&chars, &mut i, &mut line_no)
            .trim()
            .to_string();
        if sel.is_empty() || i >= n || chars[i] != '{' {
            break;
        }
        i += 1;
        let inner = read_block(&chars, &mut i, &mut line_no);
        let decls = parse_declarations(&inner);
        rules.push(CssRule {
            selector: sel,
            declarations: decls,
            line: start_line,
        });
    }
    rules
}

// ── action_parse ──────────────────────────────────────────────────────────────

fn action_parse(args: &Value) -> Result<String, String> {
    let css = get_css(args)?;
    let parsed = parse_css(&css);

    let mut out = format!("CSS Parse Result\n{}\n\n", "=".repeat(60));

    // At-rule summary
    if !parsed.at_rules.is_empty() {
        let mut at_counts: std::collections::BTreeMap<String, usize> =
            std::collections::BTreeMap::new();
        for ar in &parsed.at_rules {
            *at_counts.entry(ar.kind.clone()).or_insert(0) += 1;
        }
        let summary: Vec<String> = at_counts
            .iter()
            .map(|(k, v)| format!("@{} ×{}", k, v))
            .collect();
        out += &format!(
            "At-rules: {} ({})\n",
            parsed.at_rules.len(),
            summary.join(", ")
        );
    }
    out += &format!("Rules: {}\n", parsed.rules.len());
    out += "\n";

    // Top-level rules
    for rule in &parsed.rules {
        out += &format!("Selector: {}\n", rule.selector);
        out += &format!(
            "  Line: {} | Properties: {}\n",
            rule.line,
            rule.declarations.len()
        );
        for (prop, val) in rule.declarations.iter().take(4) {
            out += &format!("  {}: {}\n", prop, truncate_val(val, 60));
        }
        if rule.declarations.len() > 4 {
            out += &format!("  … and {} more\n", rule.declarations.len() - 4);
        }
        out += "\n";
    }

    // At-rules with nested content
    for ar in &parsed.at_rules {
        if ar.inner.is_empty() {
            out += &format!("@{} {}\n  (no nested rules)\n\n", ar.kind, ar.params);
        } else {
            out += &format!(
                "@{} {} — {} nested rule(s)\n",
                ar.kind,
                ar.params,
                ar.inner.len()
            );
            for rule in ar.inner.iter().take(3) {
                out += &format!(
                    "  {:<40} ({} propert{})\n",
                    rule.selector,
                    rule.declarations.len(),
                    if rule.declarations.len() == 1 {
                        "y"
                    } else {
                        "ies"
                    }
                );
            }
            if ar.inner.len() > 3 {
                out += &format!("  … and {} more rules\n", ar.inner.len() - 3);
            }
            out += "\n";
        }
    }

    Ok(out)
}

fn truncate_val(s: &str, max: usize) -> &str {
    if s.len() <= max {
        s
    } else {
        &s[..max]
    }
}

// ── action_validate ───────────────────────────────────────────────────────────

fn action_validate(args: &Value) -> Result<String, String> {
    let css = get_css(args)?;
    let parsed = parse_css(&css);
    let mut warnings: Vec<String> = Vec::new();

    // All rules (top-level + nested inside at-rules)
    let mut all_rules: Vec<&CssRule> = parsed.rules.iter().collect();
    for ar in &parsed.at_rules {
        for r in &ar.inner {
            all_rules.push(r);
        }
    }

    // Duplicate selectors
    let mut selector_lines: std::collections::HashMap<String, Vec<usize>> =
        std::collections::HashMap::new();
    for rule in &all_rules {
        selector_lines
            .entry(rule.selector.clone())
            .or_default()
            .push(rule.line);
    }
    for (sel, lines) in &selector_lines {
        if lines.len() > 1 {
            let line_list: Vec<String> = lines.iter().map(|l| l.to_string()).collect();
            warnings.push(format!(
                "Duplicate selector '{}' defined {} times (lines: {})",
                sel,
                lines.len(),
                line_list.join(", ")
            ));
        }
    }

    // Empty rule blocks
    for rule in &all_rules {
        if rule.declarations.is_empty() {
            warnings.push(format!(
                "Empty rule block for '{}' (line {}) — no declarations",
                rule.selector, rule.line
            ));
        }
    }

    // Duplicate properties within the same rule
    for rule in &all_rules {
        let mut seen: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
        for (prop, _) in &rule.declarations {
            let count = seen.entry(prop.clone()).or_insert(0);
            *count += 1;
        }
        for (prop, n) in &seen {
            if *n > 1 {
                warnings.push(format!(
                    "Duplicate property '{}' in '{}' (line {}) — later value silently overrides",
                    prop, rule.selector, rule.line
                ));
            }
        }
    }

    // !important overuse
    let important_count: usize = all_rules
        .iter()
        .flat_map(|r| r.declarations.iter())
        .filter(|(_, v)| v.contains("!important"))
        .count();
    if important_count > 5 {
        warnings.push(format!(
            "High !important usage: {} declarations — may indicate specificity problems",
            important_count
        ));
    }

    // Vendor prefix without standard property
    let vendor_prefixes = ["-webkit-", "-moz-", "-ms-", "-o-"];
    for rule in &all_rules {
        let props: Vec<&str> = rule.declarations.iter().map(|(p, _)| p.as_str()).collect();
        for prop in &props {
            for prefix in &vendor_prefixes {
                if let Some(base) = prop.strip_prefix(prefix) {
                    if !props.contains(&base) {
                        warnings.push(format!(
                            "Vendor-prefixed '{}' in '{}' (line {}) has no corresponding standard '{}' property",
                            prop, rule.selector, rule.line, base
                        ));
                    }
                }
            }
        }
    }

    // Very long selector chains (depth > 4 by counting spaces between parts)
    for rule in &all_rules {
        let depth = rule.selector.split_whitespace().count();
        if depth > 4 {
            warnings.push(format!(
                "Deep selector chain ({} levels) '{}' (line {}) — high specificity may cause maintenance issues",
                depth, &rule.selector[..rule.selector.len().min(50)], rule.line
            ));
        }
    }

    // Invalid hex color lengths
    let hex_re = Regex::new(r"#([0-9a-fA-F]+)").unwrap();
    for rule in &all_rules {
        for (prop, val) in &rule.declarations {
            if prop == "color"
                || prop.contains("color")
                || prop.contains("background")
                || prop.contains("border")
            {
                for cap in hex_re.captures_iter(val) {
                    let hex = cap.get(1).map(|m| m.as_str()).unwrap_or("");
                    if !matches!(hex.len(), 3 | 4 | 6 | 8) {
                        warnings.push(format!(
                            "Invalid hex color '#{}' in '{}' at '{}' (line {}) — valid lengths: 3, 4, 6, 8 digits",
                            hex, prop, rule.selector, rule.line
                        ));
                    }
                }
            }
        }
    }

    // z-index > 9999
    for rule in &all_rules {
        for (prop, val) in &rule.declarations {
            if prop == "z-index" {
                if let Ok(z) = val.trim().parse::<i64>() {
                    if z > 9999 {
                        warnings.push(format!(
                            "z-index {} in '{}' (line {}) is very large — often a sign of specificity wars",
                            z, rule.selector, rule.line
                        ));
                    }
                }
            }
        }
    }

    // Unknown pseudo-elements (common typos)
    let known_pseudo_elements = [
        "::before",
        "::after",
        "::first-line",
        "::first-letter",
        "::selection",
        "::placeholder",
        "::marker",
        "::backdrop",
        "::cue",
        "::slotted",
        "::part",
        "::spelling-error",
        "::grammar-error",
    ];
    let pseudo_re = Regex::new(r"::[a-zA-Z\-]+").unwrap();
    for rule in &all_rules {
        for cap in pseudo_re.captures_iter(&rule.selector) {
            let pseudo = cap.get(0).map(|m| m.as_str()).unwrap_or("");
            if !known_pseudo_elements.contains(&pseudo) {
                warnings.push(format!(
                    "Unknown pseudo-element '{}' in selector '{}' (line {}) — check for typos",
                    pseudo, rule.selector, rule.line
                ));
            }
        }
    }

    let verdict = if warnings.is_empty() {
        "VALID — no issues found"
    } else {
        "WARNINGS FOUND"
    };

    let mut out = format!("CSS Validation\n{}\n\n", "=".repeat(60));
    out += &format!("Result: {}\n\n", verdict);
    if !warnings.is_empty() {
        out += &format!("{} warning(s):\n", warnings.len());
        for w in &warnings {
            out += &format!("  [WARN]  {}\n", w);
        }
    } else {
        out += "No issues detected.\n";
    }
    Ok(out)
}

// ── action_vars ───────────────────────────────────────────────────────────────

fn action_vars(args: &Value) -> Result<String, String> {
    let css = get_css(args)?;
    let parsed = parse_css(&css);

    let all_rules: Vec<&CssRule> = {
        let mut v: Vec<&CssRule> = parsed.rules.iter().collect();
        for ar in &parsed.at_rules {
            for r in &ar.inner {
                v.push(r);
            }
        }
        v
    };

    // Collect definitions (properties starting with --)
    let mut definitions: Vec<(String, String, String)> = Vec::new(); // (name, value, selector)
    for rule in &all_rules {
        for (prop, val) in &rule.declarations {
            if prop.starts_with("--") {
                definitions.push((prop.clone(), val.clone(), rule.selector.clone()));
            }
        }
    }

    // Collect usages (var(--name) in values)
    let var_re = Regex::new(r"var\(\s*(--[a-zA-Z0-9_\-]+)\s*(?:,[^)]+)?\)").unwrap();
    let mut usages: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for rule in &all_rules {
        for (_, val) in &rule.declarations {
            for cap in var_re.captures_iter(val) {
                let name = cap.get(1).map(|m| m.as_str()).unwrap_or("").to_string();
                *usages.entry(name).or_insert(0) += 1;
            }
        }
    }

    let defined_names: std::collections::HashSet<String> =
        definitions.iter().map(|(n, _, _)| n.clone()).collect();

    let mut out = format!("CSS Custom Properties (Variables)\n{}\n\n", "=".repeat(60));

    // Definitions
    out += &format!("Defined Variables ({}):\n", definitions.len());
    if definitions.is_empty() {
        out += "  (none found)\n";
    } else {
        out += &format!("  {:<35} {:<35} {}\n", "Variable", "Value", "Selector");
        out += &format!("  {}\n", "─".repeat(85));
        for (name, val, sel) in &definitions {
            out += &format!(
                "  {:<35} {:<35} {}\n",
                name,
                truncate_val(val, 34),
                &sel[..sel.len().min(40)]
            );
        }
    }
    out += "\n";

    // Usage counts
    out += &format!("Variable Usages ({} distinct):\n", usages.len());
    if usages.is_empty() {
        out += "  (none found)\n";
    } else {
        let mut usage_list: Vec<(&String, &usize)> = usages.iter().collect();
        usage_list.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
        for (name, n) in &usage_list {
            out += &format!(
                "  {:35} {} use{}\n",
                name,
                n,
                if **n == 1 { "" } else { "s" }
            );
        }
    }
    out += "\n";

    // Variables used but not defined
    let undefined: Vec<&String> = usages
        .keys()
        .filter(|name| !defined_names.contains(*name))
        .collect();
    if !undefined.is_empty() {
        out += &format!("Potentially Undefined ({}):\n", undefined.len());
        for name in &undefined {
            out += &format!(
                "  {} — used but not defined in this file (may be defined elsewhere)\n",
                name
            );
        }
    }

    Ok(out)
}

// ── action_stats ──────────────────────────────────────────────────────────────

fn action_stats(args: &Value) -> Result<String, String> {
    let css = get_css(args)?;
    let parsed = parse_css(&css);

    let all_rules: Vec<&CssRule> = {
        let mut v: Vec<&CssRule> = parsed.rules.iter().collect();
        for ar in &parsed.at_rules {
            for r in &ar.inner {
                v.push(r);
            }
        }
        v
    };

    let total_rules = all_rules.len();
    let total_decls: usize = all_rules.iter().map(|r| r.declarations.len()).sum();
    let unique_selectors: std::collections::HashSet<&str> =
        all_rules.iter().map(|r| r.selector.as_str()).collect();

    // At-rule breakdown
    let mut at_counts: std::collections::BTreeMap<String, usize> =
        std::collections::BTreeMap::new();
    for ar in &parsed.at_rules {
        *at_counts.entry(ar.kind.clone()).or_insert(0) += 1;
    }

    // Top properties
    let mut prop_counts: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();
    for rule in &all_rules {
        for (prop, _) in &rule.declarations {
            *prop_counts.entry(prop.clone()).or_insert(0) += 1;
        }
    }
    let mut prop_list: Vec<(&String, &usize)> = prop_counts.iter().collect();
    prop_list.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));

    // Selector complexity
    let id_sels = all_rules
        .iter()
        .filter(|r| r.selector.contains('#'))
        .count();
    let class_sels = all_rules
        .iter()
        .filter(|r| r.selector.contains('.'))
        .count();
    let elem_sels = all_rules
        .iter()
        .filter(|r| {
            !r.selector.contains('.') && !r.selector.contains('#') && !r.selector.is_empty()
        })
        .count();
    let deep_sels = all_rules
        .iter()
        .filter(|r| r.selector.split_whitespace().count() > 2)
        .count();

    // Color values
    let color_re =
        Regex::new(r"(?i)(#[0-9a-f]{3,8}|rgb\([^)]+\)|hsl\([^)]+\)|rgba\([^)]+\)|hsla\([^)]+\))")
            .unwrap();
    let mut colors: std::collections::HashSet<String> = std::collections::HashSet::new();
    for rule in &all_rules {
        for (_, val) in &rule.declarations {
            for cap in color_re.captures_iter(val) {
                colors.insert(cap.get(0).map(|m| m.as_str()).unwrap_or("").to_lowercase());
            }
        }
    }

    // !important count
    let important_count: usize = all_rules
        .iter()
        .flat_map(|r| r.declarations.iter())
        .filter(|(_, v)| v.contains("!important"))
        .count();

    // CSS var count
    let var_re = Regex::new(r"var\(--[^)]+\)").unwrap();
    let var_count: usize = all_rules
        .iter()
        .flat_map(|r| r.declarations.iter())
        .filter(|(_, v)| var_re.is_match(v))
        .count();

    // Size estimates
    let original_size = css.len();
    let gzip_estimate = (original_size as f64 * 0.3) as usize;

    let mut out = format!("CSS Statistics\n{}\n\n", "=".repeat(60));
    out += &format!("Total rules:         {}\n", total_rules);
    out += &format!("Total declarations:  {}\n", total_decls);
    out += &format!("Unique selectors:    {}\n", unique_selectors.len());
    out += &format!("At-rules:            {}\n", parsed.at_rules.len());
    out += &format!("!important count:    {}\n", important_count);
    out += &format!("CSS variable usages: {}\n", var_count);
    out += &format!("Unique colors found: {}\n", colors.len());
    out += &format!(
        "File size:           {} bytes (est. ~{} bytes gzipped)\n",
        original_size, gzip_estimate
    );
    out += "\n";

    if !at_counts.is_empty() {
        out += "At-rule breakdown:\n";
        for (kind, n) in &at_counts {
            out += &format!("  @{:<20} {}\n", kind, n);
        }
        out += "\n";
    }

    out += "Top 10 properties:\n";
    for (prop, n) in prop_list.iter().take(10) {
        out += &format!("  {:<30} {}\n", prop, n);
    }
    out += "\n";

    out += "Selector types:\n";
    out += &format!("  ID selectors (#):          {}\n", id_sels);
    out += &format!("  Class selectors (.):       {}\n", class_sels);
    out += &format!("  Element/other selectors:   {}\n", elem_sels);
    out += &format!("  Deep (> 2 levels):         {}\n", deep_sels);

    if !colors.is_empty() {
        out += "\nColor values found:\n";
        let mut color_list: Vec<&String> = colors.iter().collect();
        color_list.sort();
        for chunk in color_list.chunks(6) {
            out += &format!(
                "  {}\n",
                chunk
                    .iter()
                    .map(|s| s.as_str())
                    .collect::<Vec<_>>()
                    .join("  ")
            );
        }
    }

    Ok(out)
}

// ── action_minify ─────────────────────────────────────────────────────────────

fn action_minify(args: &Value) -> Result<String, String> {
    let css = get_css(args)?;
    let original_size = css.len();

    // Strip comments
    let no_comments = strip_comments(&css);

    // Collapse whitespace sequences to single spaces/nothing where safe
    let mut minified = String::with_capacity(no_comments.len());
    let chars: Vec<char> = no_comments.chars().collect();
    let n = chars.len();
    let mut i = 0;

    // Tokens around which whitespace is not needed
    let no_space_around = |c: char| matches!(c, '{' | '}' | ':' | ';' | ',' | '>' | '+' | '~');

    while i < n {
        let c = chars[i];
        if c.is_whitespace() {
            // Collapse all consecutive whitespace to a single space
            while i < n && chars[i].is_whitespace() {
                i += 1;
            }
            // Check the previous and next non-space chars
            let prev = minified.chars().last();
            let next = chars.get(i).copied();
            let skip = prev.map(no_space_around).unwrap_or(true)
                || next.map(no_space_around).unwrap_or(true);
            if !skip {
                minified.push(' ');
            }
        } else {
            minified.push(c);
            i += 1;
        }
    }

    // Remove last semicolon before closing brace: `; }` → `}`
    let semi_brace_re = Regex::new(r";\s*\}").unwrap();
    let minified = semi_brace_re.replace_all(&minified, "}").into_owned();
    let minified = minified.trim().to_string();

    let minified_size = minified.len();
    let saved = original_size.saturating_sub(minified_size);
    let ratio = if original_size > 0 {
        (1.0 - minified_size as f64 / original_size as f64) * 100.0
    } else {
        0.0
    };

    let mut out = format!("CSS Minify Result\n{}\n\n", "=".repeat(60));
    out += &format!("Original size:  {} bytes\n", original_size);
    out += &format!("Minified size:  {} bytes\n", minified_size);
    out += &format!(
        "Saved:          {} bytes ({:.1}% reduction)\n",
        saved, ratio
    );
    out += "\n";
    out += &"─".repeat(60);
    out += "\n";
    out += &minified;
    out += "\n";

    Ok(out)
}
