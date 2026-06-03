use serde_json::{json, Value};

pub fn make_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "action": {
                "type": "string",
                "enum": ["info", "parse", "validate", "list", "check"],
                "description": "info (default): look up a license ID; parse: parse SPDX expression tree; validate: check expression validity; list: list all licenses; check: check OSI/copyleft/FSF properties"
            },
            "license": { "type": "string", "description": "SPDX license identifier to look up (e.g. MIT, Apache-2.0)" },
            "expression": { "type": "string", "description": "SPDX license expression to parse/validate/check (e.g. 'MIT AND Apache-2.0')" },
            "category": { "type": "string", "description": "Filter list by: permissive, copyleft, weak-copyleft, network-copyleft, public-domain, deprecated" }
        }
    })
}

struct SpdxEntry {
    id: &'static str,
    name: &'static str,
    osi: bool,
    fsf: bool,
    copyleft: &'static str, // none, weak, strong, network
    deprecated: bool,
}

static LICENSES: &[SpdxEntry] = &[
    SpdxEntry { id: "MIT", name: "MIT License", osi: true, fsf: true, copyleft: "none", deprecated: false },
    SpdxEntry { id: "MIT-0", name: "MIT No Attribution", osi: true, fsf: false, copyleft: "none", deprecated: false },
    SpdxEntry { id: "Apache-2.0", name: "Apache License 2.0", osi: true, fsf: true, copyleft: "none", deprecated: false },
    SpdxEntry { id: "GPL-2.0-only", name: "GNU General Public License v2.0 only", osi: true, fsf: true, copyleft: "strong", deprecated: false },
    SpdxEntry { id: "GPL-2.0-or-later", name: "GNU General Public License v2.0 or later", osi: true, fsf: true, copyleft: "strong", deprecated: false },
    SpdxEntry { id: "GPL-3.0-only", name: "GNU General Public License v3.0 only", osi: true, fsf: true, copyleft: "strong", deprecated: false },
    SpdxEntry { id: "GPL-3.0-or-later", name: "GNU General Public License v3.0 or later", osi: true, fsf: true, copyleft: "strong", deprecated: false },
    SpdxEntry { id: "LGPL-2.0-only", name: "GNU Library General Public License v2 only", osi: true, fsf: true, copyleft: "weak", deprecated: false },
    SpdxEntry { id: "LGPL-2.0-or-later", name: "GNU Library General Public License v2 or later", osi: true, fsf: true, copyleft: "weak", deprecated: false },
    SpdxEntry { id: "LGPL-2.1-only", name: "GNU Lesser General Public License v2.1 only", osi: true, fsf: true, copyleft: "weak", deprecated: false },
    SpdxEntry { id: "LGPL-2.1-or-later", name: "GNU Lesser General Public License v2.1 or later", osi: true, fsf: true, copyleft: "weak", deprecated: false },
    SpdxEntry { id: "LGPL-3.0-only", name: "GNU Lesser General Public License v3.0 only", osi: true, fsf: true, copyleft: "weak", deprecated: false },
    SpdxEntry { id: "LGPL-3.0-or-later", name: "GNU Lesser General Public License v3.0 or later", osi: true, fsf: true, copyleft: "weak", deprecated: false },
    SpdxEntry { id: "AGPL-3.0-only", name: "GNU Affero General Public License v3.0 only", osi: true, fsf: true, copyleft: "network", deprecated: false },
    SpdxEntry { id: "AGPL-3.0-or-later", name: "GNU Affero General Public License v3.0 or later", osi: true, fsf: true, copyleft: "network", deprecated: false },
    SpdxEntry { id: "MPL-2.0", name: "Mozilla Public License 2.0", osi: true, fsf: true, copyleft: "weak", deprecated: false },
    SpdxEntry { id: "MPL-2.0-no-copyleft-exception", name: "Mozilla Public License 2.0 (no copyleft exception)", osi: true, fsf: false, copyleft: "weak", deprecated: false },
    SpdxEntry { id: "BSD-2-Clause", name: "BSD 2-Clause \"Simplified\" License", osi: true, fsf: true, copyleft: "none", deprecated: false },
    SpdxEntry { id: "BSD-3-Clause", name: "BSD 3-Clause \"New\" or \"Revised\" License", osi: true, fsf: true, copyleft: "none", deprecated: false },
    SpdxEntry { id: "BSD-4-Clause", name: "BSD 4-Clause \"Original\" License", osi: false, fsf: false, copyleft: "none", deprecated: false },
    SpdxEntry { id: "ISC", name: "ISC License", osi: true, fsf: true, copyleft: "none", deprecated: false },
    SpdxEntry { id: "0BSD", name: "BSD Zero Clause License", osi: true, fsf: false, copyleft: "none", deprecated: false },
    SpdxEntry { id: "CC0-1.0", name: "Creative Commons Zero v1.0 Universal", osi: false, fsf: true, copyleft: "none", deprecated: false },
    SpdxEntry { id: "Unlicense", name: "The Unlicense", osi: true, fsf: true, copyleft: "none", deprecated: false },
    SpdxEntry { id: "WTFPL", name: "Do What The F*ck You Want To Public License", osi: false, fsf: true, copyleft: "none", deprecated: false },
    SpdxEntry { id: "Zlib", name: "zlib License", osi: true, fsf: true, copyleft: "none", deprecated: false },
    SpdxEntry { id: "BSL-1.0", name: "Boost Software License 1.0", osi: true, fsf: true, copyleft: "none", deprecated: false },
    SpdxEntry { id: "OFL-1.1", name: "SIL Open Font License 1.1", osi: true, fsf: true, copyleft: "weak", deprecated: false },
    SpdxEntry { id: "PostgreSQL", name: "PostgreSQL License", osi: true, fsf: true, copyleft: "none", deprecated: false },
    SpdxEntry { id: "Python-2.0", name: "Python License 2.0", osi: true, fsf: true, copyleft: "none", deprecated: false },
    SpdxEntry { id: "Artistic-2.0", name: "Artistic License 2.0", osi: true, fsf: true, copyleft: "weak", deprecated: false },
    SpdxEntry { id: "EUPL-1.2", name: "European Union Public License 1.2", osi: true, fsf: true, copyleft: "strong", deprecated: false },
    SpdxEntry { id: "CDDL-1.0", name: "Common Development and Distribution License 1.0", osi: true, fsf: false, copyleft: "weak", deprecated: false },
    SpdxEntry { id: "EPL-1.0", name: "Eclipse Public License 1.0", osi: true, fsf: false, copyleft: "weak", deprecated: false },
    SpdxEntry { id: "EPL-2.0", name: "Eclipse Public License 2.0", osi: true, fsf: true, copyleft: "weak", deprecated: false },
    SpdxEntry { id: "OSL-3.0", name: "Open Software License 3.0", osi: true, fsf: true, copyleft: "network", deprecated: false },
    SpdxEntry { id: "CPAL-1.0", name: "Common Public Attribution License 1.0", osi: true, fsf: false, copyleft: "network", deprecated: false },
    SpdxEntry { id: "CC-BY-4.0", name: "Creative Commons Attribution 4.0 International", osi: false, fsf: false, copyleft: "none", deprecated: false },
    SpdxEntry { id: "CC-BY-SA-4.0", name: "Creative Commons Attribution Share Alike 4.0 International", osi: false, fsf: true, copyleft: "strong", deprecated: false },
    SpdxEntry { id: "CC-BY-NC-4.0", name: "Creative Commons Attribution Non Commercial 4.0 International", osi: false, fsf: false, copyleft: "none", deprecated: false },
    SpdxEntry { id: "SSPL-1.0", name: "Server Side Public License v1", osi: false, fsf: false, copyleft: "network", deprecated: false },
    SpdxEntry { id: "BUSL-1.1", name: "Business Source License 1.1", osi: false, fsf: false, copyleft: "none", deprecated: false },
    SpdxEntry { id: "GPL-2.0", name: "GNU General Public License v2.0 only", osi: true, fsf: true, copyleft: "strong", deprecated: true },
    SpdxEntry { id: "GPL-3.0", name: "GNU General Public License v3.0 only", osi: true, fsf: true, copyleft: "strong", deprecated: true },
    SpdxEntry { id: "LGPL-2.1", name: "GNU Lesser General Public License v2.1 only", osi: true, fsf: true, copyleft: "weak", deprecated: true },
    SpdxEntry { id: "LGPL-3.0", name: "GNU Lesser General Public License v3.0 only", osi: true, fsf: true, copyleft: "weak", deprecated: true },
    SpdxEntry { id: "AGPL-3.0", name: "GNU Affero General Public License v3.0", osi: true, fsf: true, copyleft: "network", deprecated: true },
    SpdxEntry { id: "eCos-2.0", name: "eCos license version 2.0", osi: true, fsf: true, copyleft: "weak", deprecated: false },
    SpdxEntry { id: "LPPL-1.3c", name: "LaTeX Project Public License v1.3c", osi: true, fsf: true, copyleft: "weak", deprecated: false },
    SpdxEntry { id: "MPL-1.1", name: "Mozilla Public License 1.1", osi: true, fsf: false, copyleft: "weak", deprecated: false },
    SpdxEntry { id: "RPL-1.5", name: "Reciprocal Public License 1.5", osi: true, fsf: false, copyleft: "strong", deprecated: false },
    SpdxEntry { id: "SimPL-2.0", name: "Simple Public License 2.0", osi: true, fsf: false, copyleft: "strong", deprecated: false },
    SpdxEntry { id: "MS-PL", name: "Microsoft Public License", osi: true, fsf: false, copyleft: "none", deprecated: false },
    SpdxEntry { id: "MS-RL", name: "Microsoft Reciprocal License", osi: true, fsf: false, copyleft: "weak", deprecated: false },
];

fn lookup(id: &str) -> Option<&'static SpdxEntry> {
    LICENSES.iter().find(|e| e.id.eq_ignore_ascii_case(id))
}

fn copyleft_label(c: &str) -> &'static str {
    match c {
        "strong" => "Strong copyleft",
        "weak" => "Weak copyleft",
        "network" => "Network copyleft (AGPL-style)",
        _ => "Permissive",
    }
}

fn action_info(args: &Value) -> Result<String, String> {
    let id = args.get("license")
        .or_else(|| args.get("expression"))
        .and_then(|v| v.as_str())
        .ok_or("Provide 'license' with an SPDX identifier (e.g. MIT, Apache-2.0, GPL-3.0-or-later).")?;

    if let Some(e) = lookup(id) {
        let osi = if e.osi { "✓ OSI Approved" } else { "✗ Not OSI Approved" };
        let fsf = if e.fsf { "✓ FSF Free" } else { "✗ Not FSF Free" };
        let dep = if e.deprecated { "  ⚠ DEPRECATED — prefer versioned successor\n" } else { "" };
        let mut out = format!("## {}\n\n", e.id);
        out.push_str(&format!("  Name:      {}\n", e.name));
        out.push_str(&format!("  Type:      {}\n", copyleft_label(e.copyleft)));
        out.push_str(&format!("  OSI:       {}\n", osi));
        out.push_str(&format!("  FSF:       {}\n", fsf));
        out.push_str(dep);
        out.push_str(&format!("\n  SPDX URL:  https://spdx.org/licenses/{}.html\n", e.id));
        return Ok(out);
    }

    // Fuzzy suggestions
    let lower = id.to_lowercase();
    let suggestions: Vec<&str> = LICENSES
        .iter()
        .filter(|e| {
            e.id.to_lowercase().contains(&lower)
                || e.name.to_lowercase().contains(&lower)
        })
        .map(|e| e.id)
        .take(5)
        .collect();

    if suggestions.is_empty() {
        Err(format!("'{}' is not a known SPDX identifier. Use action='list' to browse all licenses.", id))
    } else {
        Ok(format!(
            "'{}' not found. Did you mean one of:\n{}\n\nUse action='list' to browse all {} licenses.",
            id,
            suggestions.iter().map(|s| format!("  - {}", s)).collect::<Vec<_>>().join("\n"),
            LICENSES.len()
        ))
    }
}

// ── SPDX expression parser ────────────────────────────────────────────────────

#[derive(Debug, Clone)]
enum Token {
    Id(String),
    And,
    Or,
    With,
    LParen,
    RParen,
    Eof,
}

fn tokenize(expr: &str) -> Vec<Token> {
    let mut tokens = Vec::new();
    let mut chars = expr.chars().peekable();
    while let Some(&c) = chars.peek() {
        match c {
            ' ' | '\t' | '\n' => { chars.next(); }
            '(' => { chars.next(); tokens.push(Token::LParen); }
            ')' => { chars.next(); tokens.push(Token::RParen); }
            _ => {
                let mut word = String::new();
                while let Some(&ch) = chars.peek() {
                    if ch == ' ' || ch == '\t' || ch == '(' || ch == ')' { break; }
                    word.push(ch);
                    chars.next();
                }
                match word.to_uppercase().as_str() {
                    "AND" => tokens.push(Token::And),
                    "OR" => tokens.push(Token::Or),
                    "WITH" => tokens.push(Token::With),
                    _ => tokens.push(Token::Id(word)),
                }
            }
        }
    }
    tokens.push(Token::Eof);
    tokens
}

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0 }
    }

    fn peek(&self) -> &Token {
        &self.tokens[self.pos]
    }

    fn consume(&mut self) -> Token {
        let t = self.tokens[self.pos].clone();
        if self.pos + 1 < self.tokens.len() { self.pos += 1; }
        t
    }

    fn parse_expr(&mut self) -> Result<ExprNode, String> {
        self.parse_or()
    }

    fn parse_or(&mut self) -> Result<ExprNode, String> {
        let mut left = self.parse_and()?;
        while matches!(self.peek(), Token::Or) {
            self.consume();
            let right = self.parse_and()?;
            left = ExprNode::Or(Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn parse_and(&mut self) -> Result<ExprNode, String> {
        let mut left = self.parse_primary()?;
        while matches!(self.peek(), Token::And) {
            self.consume();
            let right = self.parse_primary()?;
            left = ExprNode::And(Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn parse_primary(&mut self) -> Result<ExprNode, String> {
        match self.peek().clone() {
            Token::LParen => {
                self.consume();
                let inner = self.parse_expr()?;
                if !matches!(self.peek(), Token::RParen) {
                    return Err("Missing closing ')'".to_string());
                }
                self.consume();
                Ok(inner)
            }
            Token::Id(id) => {
                self.consume();
                let or_later = id.ends_with('+');
                let base_id = if or_later { id.trim_end_matches('+').to_string() } else { id.clone() };
                // Check for WITH clause
                if matches!(self.peek(), Token::With) {
                    self.consume();
                    match self.peek().clone() {
                        Token::Id(exc) => {
                            self.consume();
                            Ok(ExprNode::With(base_id, exc, or_later))
                        }
                        _ => Err("Expected exception identifier after WITH".to_string()),
                    }
                } else {
                    Ok(ExprNode::License(base_id, or_later))
                }
            }
            Token::Eof => Err("Unexpected end of expression".to_string()),
            other => Err(format!("Unexpected token: {:?}", other)),
        }
    }
}

#[derive(Debug)]
enum ExprNode {
    License(String, bool),             // id, or_later
    With(String, String, bool),        // id, exception, or_later
    And(Box<ExprNode>, Box<ExprNode>),
    Or(Box<ExprNode>, Box<ExprNode>),
}

impl ExprNode {
    fn display(&self, indent: usize) -> String {
        let pad = "  ".repeat(indent);
        match self {
            ExprNode::License(id, or_later) => {
                let suffix = if *or_later { "+" } else { "" };
                let known = if lookup(id).is_some() { "" } else { " ⚠ unknown" };
                format!("{}License: {}{}{}\n", pad, id, suffix, known)
            }
            ExprNode::With(id, exc, or_later) => {
                let suffix = if *or_later { "+" } else { "" };
                format!("{}License: {}{} WITH {}\n", pad, id, suffix, exc)
            }
            ExprNode::And(l, r) => {
                format!("{}AND\n{}{}", pad, l.display(indent + 1), r.display(indent + 1))
            }
            ExprNode::Or(l, r) => {
                format!("{}OR\n{}{}", pad, l.display(indent + 1), r.display(indent + 1))
            }
        }
    }

    fn collect_ids(&self) -> Vec<String> {
        match self {
            ExprNode::License(id, _) | ExprNode::With(id, _, _) => vec![id.clone()],
            ExprNode::And(l, r) | ExprNode::Or(l, r) => {
                let mut v = l.collect_ids();
                v.extend(r.collect_ids());
                v
            }
        }
    }

    fn is_osi_compatible(&self) -> bool {
        match self {
            ExprNode::License(id, _) | ExprNode::With(id, _, _) => {
                lookup(id).map(|e| e.osi).unwrap_or(false)
            }
            ExprNode::And(l, r) => l.is_osi_compatible() && r.is_osi_compatible(),
            ExprNode::Or(l, r) => l.is_osi_compatible() || r.is_osi_compatible(),
        }
    }

    fn max_copyleft(&self) -> &'static str {
        match self {
            ExprNode::License(id, _) | ExprNode::With(id, _, _) => {
                lookup(id).map(|e| e.copyleft).unwrap_or("none")
            }
            ExprNode::And(l, r) | ExprNode::Or(l, r) => {
                let lc = l.max_copyleft();
                let rc = r.max_copyleft();
                // network > strong > weak > none
                for level in &["network", "strong", "weak"] {
                    if lc == *level || rc == *level { return level; }
                }
                "none"
            }
        }
    }
}

fn parse_expression(expr: &str) -> Result<ExprNode, String> {
    let tokens = tokenize(expr);
    let mut parser = Parser::new(tokens);
    let node = parser.parse_expr()?;
    if !matches!(parser.peek(), Token::Eof) {
        return Err("Unexpected tokens after expression end".to_string());
    }
    Ok(node)
}

fn action_parse(args: &Value) -> Result<String, String> {
    let expr = args.get("expression")
        .or_else(|| args.get("license"))
        .and_then(|v| v.as_str())
        .ok_or("Provide 'expression' with an SPDX license expression.")?;

    let node = parse_expression(expr).map_err(|e| format!("Parse error: {}", e))?;
    let ids = node.collect_ids();
    let unknown: Vec<&str> = ids.iter().filter(|id| lookup(id).is_none()).map(|s| s.as_str()).collect();

    let mut out = format!("## SPDX Expression Parse\n\n  {}\n\n## Tree\n\n", expr);
    out.push_str(&node.display(1));
    out.push('\n');

    if !unknown.is_empty() {
        out.push_str(&format!("  ⚠ Unknown identifiers: {}\n\n", unknown.join(", ")));
    }

    // Summary per leaf
    out.push_str("## Licenses\n\n");
    for id in &ids {
        if let Some(e) = lookup(id) {
            let flags = format!("OSI:{} FSF:{} {}",
                if e.osi { "✓" } else { "✗" },
                if e.fsf { "✓" } else { "✗" },
                copyleft_label(e.copyleft));
            out.push_str(&format!("  {:30} {}\n", e.id, flags));
        }
    }
    Ok(out)
}

fn action_validate(args: &Value) -> Result<String, String> {
    let expr = args.get("expression")
        .or_else(|| args.get("license"))
        .and_then(|v| v.as_str())
        .ok_or("Provide 'expression' with an SPDX license expression.")?;

    match parse_expression(expr) {
        Err(e) => Ok(format!("INVALID\n\n  Parse error: {}", e)),
        Ok(node) => {
            let ids = node.collect_ids();
            let unknown: Vec<&str> = ids.iter()
                .filter(|id| lookup(id).is_none() && !id.starts_with("LicenseRef-"))
                .map(|s| s.as_str())
                .collect();
            let deprecated: Vec<&str> = ids.iter()
                .filter_map(|id| lookup(id))
                .filter(|e| e.deprecated)
                .map(|e| e.id)
                .collect();

            if unknown.is_empty() && deprecated.is_empty() {
                Ok(format!("VALID\n\n  Expression parses correctly.\n  Identifiers: {}\n", ids.join(", ")))
            } else {
                let mut out = "VALID (with warnings)\n".to_string();
                if !unknown.is_empty() {
                    out.push_str(&format!("\n  ⚠ Unknown identifiers (may be valid LicenseRef- or newer SPDX): {}\n", unknown.join(", ")));
                }
                if !deprecated.is_empty() {
                    out.push_str(&format!("\n  ⚠ Deprecated identifiers (use versioned successor): {}\n", deprecated.join(", ")));
                }
                Ok(out)
            }
        }
    }
}

fn action_list(args: &Value) -> Result<String, String> {
    let category = args.get("category").and_then(|v| v.as_str()).unwrap_or("");

    let filtered: Vec<&SpdxEntry> = LICENSES.iter().filter(|e| {
        match category.to_lowercase().as_str() {
            "permissive" => e.copyleft == "none" && !e.deprecated,
            "copyleft" | "strong-copyleft" => e.copyleft == "strong" && !e.deprecated,
            "weak-copyleft" => e.copyleft == "weak" && !e.deprecated,
            "network-copyleft" => e.copyleft == "network" && !e.deprecated,
            "public-domain" => (e.id == "CC0-1.0" || e.id == "Unlicense" || e.id == "0BSD" || e.id == "WTFPL") && !e.deprecated,
            "deprecated" => e.deprecated,
            _ => !e.deprecated,
        }
    }).collect();

    let mut out = format!("{:<25} {:<5} {:<5} {}\n", "SPDX ID", "OSI", "FSF", "TYPE");
    out.push_str(&format!("{}\n", "-".repeat(70)));
    for e in &filtered {
        let osi = if e.osi { "✓" } else { "✗" };
        let fsf = if e.fsf { "✓" } else { "✗" };
        let typ = copyleft_label(e.copyleft);
        out.push_str(&format!("{:<25} {:<5} {:<5} {}\n", e.id, osi, fsf, typ));
    }
    out.push_str(&format!("\nTotal: {} license(s)\n", filtered.len()));
    if category.is_empty() {
        out.push_str("  Filter with category= : permissive / copyleft / weak-copyleft / network-copyleft / deprecated\n");
    }
    Ok(out)
}

fn action_check(args: &Value) -> Result<String, String> {
    let expr = args.get("expression")
        .or_else(|| args.get("license"))
        .and_then(|v| v.as_str())
        .ok_or("Provide 'expression' with an SPDX license expression to check.")?;

    let node = parse_expression(expr).map_err(|e| format!("Parse error: {}", e))?;
    let ids = node.collect_ids();
    let copyleft = node.max_copyleft();
    let osi = node.is_osi_compatible();

    let all_osi = ids.iter().all(|id| lookup(id).map(|e| e.osi).unwrap_or(false));
    let all_fsf = ids.iter().all(|id| lookup(id).map(|e| e.fsf).unwrap_or(false));
    let any_deprecated = ids.iter().any(|id| lookup(id).map(|e| e.deprecated).unwrap_or(false));

    let copyleft_label_str = copyleft_label(copyleft);

    let mut out = format!("## Expression Properties: {}\n\n", expr);
    out.push_str(&format!("  OSI compatible:    {} ({})\n",
        if osi { "✓ Yes" } else { "✗ No" },
        if all_osi { "all licenses" } else if osi { "at least one branch" } else { "no OSI-approved licenses" }));
    out.push_str(&format!("  FSF free:          {}\n", if all_fsf { "✓ All licenses" } else { "✗ Some licenses are not FSF free" }));
    out.push_str(&format!("  Copyleft:          {}\n", copyleft_label_str));
    if any_deprecated {
        out.push_str("  Deprecated IDs:    ⚠ Expression contains deprecated SPDX identifiers\n");
    }
    out.push_str(&format!("  License count:     {}\n", ids.len()));
    out.push_str(&format!("  Identifiers:       {}\n", ids.join(", ")));

    if copyleft == "strong" || copyleft == "network" {
        out.push_str("\n  Note: Strong/network copyleft licenses typically require derivative works\n");
        out.push_str("        to be distributed under the same terms.\n");
    }
    Ok(out)
}

pub async fn execute(args: &Value) -> Result<String, String> {
    let action = args.get("action").and_then(|v| v.as_str()).unwrap_or(
        if args.get("expression").is_some() && args.get("license").is_none() { "parse" }
        else { "info" }
    );
    match action {
        "parse" => action_parse(args),
        "validate" => action_validate(args),
        "list" => action_list(args),
        "check" => action_check(args),
        _ => action_info(args),
    }
}
