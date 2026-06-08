pub async fn execute(args: &serde_json::Value) -> Result<String, String> {
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("parse");

    match action {
        "parse" => parse_action(args),
        "compare" => compare(args),
        "bump" => bump(args),
        "validate" => validate(args),
        "satisfies" => satisfies(args),
        "sort" => sort_action(args),
        other => Err(format!(
            "semver_tools: unknown action '{other}'. Valid: parse, compare, bump, validate, satisfies, sort"
        )),
    }
}

// ── SemVer parsing ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
struct SemVer {
    major: u64,
    minor: u64,
    patch: u64,
    pre: Option<String>,
    build: Option<String>,
}

impl SemVer {
    fn parse(s: &str) -> Result<Self, String> {
        let s = s.trim().trim_start_matches('v').trim_start_matches('V');

        // Split off build metadata
        let (version_and_pre, build) = if let Some(idx) = s.find('+') {
            (&s[..idx], Some(s[idx + 1..].to_string()))
        } else {
            (s, None)
        };

        // Split off pre-release
        let (version, pre) = if let Some(idx) = version_and_pre.find('-') {
            (
                &version_and_pre[..idx],
                Some(version_and_pre[idx + 1..].to_string()),
            )
        } else {
            (version_and_pre, None)
        };

        let parts: Vec<&str> = version.split('.').collect();
        if parts.len() != 3 {
            return Err(format!(
                "semver_tools: '{}' is not valid semver — expected MAJOR.MINOR.PATCH",
                s
            ));
        }

        let parse_num = |p: &str, field: &str| -> Result<u64, String> {
            if p.starts_with('0') && p.len() > 1 {
                return Err(format!("semver_tools: {field} '{p}' has leading zero"));
            }
            p.parse::<u64>()
                .map_err(|_| format!("semver_tools: {field} '{p}' is not a valid integer"))
        };

        Ok(SemVer {
            major: parse_num(parts[0], "major")?,
            minor: parse_num(parts[1], "minor")?,
            patch: parse_num(parts[2], "patch")?,
            pre,
            build,
        })
    }

    fn core(&self) -> String {
        format!("{}.{}.{}", self.major, self.minor, self.patch)
    }

    fn full(&self) -> String {
        let mut s = self.core();
        if let Some(p) = &self.pre {
            s.push('-');
            s.push_str(p);
        }
        if let Some(b) = &self.build {
            s.push('+');
            s.push_str(b);
        }
        s
    }
}

impl PartialOrd for SemVer {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for SemVer {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Compare major, minor, patch
        match self.major.cmp(&other.major) {
            std::cmp::Ordering::Equal => {}
            o => return o,
        }
        match self.minor.cmp(&other.minor) {
            std::cmp::Ordering::Equal => {}
            o => return o,
        }
        match self.patch.cmp(&other.patch) {
            std::cmp::Ordering::Equal => {}
            o => return o,
        }
        // Pre-release: presence of pre-release lowers precedence
        match (&self.pre, &other.pre) {
            (None, None) => std::cmp::Ordering::Equal,
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (Some(a), Some(b)) => compare_pre(a, b),
        }
    }
}

fn compare_pre(a: &str, b: &str) -> std::cmp::Ordering {
    // Split by dots, compare each identifier
    let a_parts: Vec<&str> = a.split('.').collect();
    let b_parts: Vec<&str> = b.split('.').collect();
    for (ai, bi) in a_parts.iter().zip(b_parts.iter()) {
        let ord = match (ai.parse::<u64>(), bi.parse::<u64>()) {
            (Ok(an), Ok(bn)) => an.cmp(&bn),
            (Ok(_), Err(_)) => std::cmp::Ordering::Less,
            (Err(_), Ok(_)) => std::cmp::Ordering::Greater,
            (Err(_), Err(_)) => ai.cmp(bi),
        };
        if ord != std::cmp::Ordering::Equal {
            return ord;
        }
    }
    a_parts.len().cmp(&b_parts.len())
}

// ── Range checking ────────────────────────────────────────────────────────────

fn version_satisfies(v: &SemVer, range: &str) -> Result<bool, String> {
    let range = range.trim();

    // Multiple ranges separated by || (OR)
    if range.contains("||") {
        for part in range.split("||") {
            if version_satisfies(v, part.trim())? {
                return Ok(true);
            }
        }
        return Ok(false);
    }

    // Space-separated constraints are AND
    let constraints: Vec<&str> = range.split_whitespace().collect();
    for constraint in constraints {
        if !single_constraint(v, constraint)? {
            return Ok(false);
        }
    }
    Ok(true)
}

fn single_constraint(v: &SemVer, constraint: &str) -> Result<bool, String> {
    let c = constraint.trim();

    if c == "*" || c.is_empty() {
        return Ok(true);
    }

    // Caret: ^1.2.3 — compatible with 1.x.x
    if let Some(ver_str) = c.strip_prefix('^') {
        let req = SemVer::parse(ver_str)?;
        let upper = SemVer {
            major: req.major + 1,
            minor: 0,
            patch: 0,
            pre: None,
            build: None,
        };
        return Ok(*v >= req && *v < upper);
    }

    // Tilde: ~1.2.3 — compatible with 1.2.x
    if let Some(ver_str) = c.strip_prefix('~') {
        let req = SemVer::parse(ver_str)?;
        let upper = SemVer {
            major: req.major,
            minor: req.minor + 1,
            patch: 0,
            pre: None,
            build: None,
        };
        return Ok(*v >= req && *v < upper);
    }

    // >= <= > < =
    if let Some(ver_str) = c.strip_prefix(">=") {
        let req = SemVer::parse(ver_str.trim())?;
        return Ok(*v >= req);
    }
    if let Some(ver_str) = c.strip_prefix("<=") {
        let req = SemVer::parse(ver_str.trim())?;
        return Ok(*v <= req);
    }
    if let Some(ver_str) = c.strip_prefix('>') {
        let req = SemVer::parse(ver_str.trim())?;
        return Ok(*v > req);
    }
    if let Some(ver_str) = c.strip_prefix('<') {
        let req = SemVer::parse(ver_str.trim())?;
        return Ok(*v < req);
    }
    if let Some(ver_str) = c.strip_prefix('=') {
        let req = SemVer::parse(ver_str.trim())?;
        return Ok(*v == req);
    }

    // Plain version — exact match
    let req = SemVer::parse(c)?;
    Ok(*v == req)
}

// ── Actions ───────────────────────────────────────────────────────────────────

fn parse_action(args: &serde_json::Value) -> Result<String, String> {
    let input = args
        .get("input")
        .or_else(|| args.get("version"))
        .and_then(|v| v.as_str())
        .ok_or("semver_tools parse: 'input' is required")?;

    let v = SemVer::parse(input)?;

    let mut out = format!("SEMVER PARSE\n{}\n", "─".repeat(50));
    out.push_str(&format!("Input       : {input}\n"));
    out.push_str(&format!("Major       : {}\n", v.major));
    out.push_str(&format!("Minor       : {}\n", v.minor));
    out.push_str(&format!("Patch       : {}\n", v.patch));
    out.push_str(&format!(
        "Pre-release : {}\n",
        v.pre.as_deref().unwrap_or("(none)")
    ));
    out.push_str(&format!(
        "Build meta  : {}\n",
        v.build.as_deref().unwrap_or("(none)")
    ));
    out.push_str(&format!("Stable      : {}\n", v.pre.is_none()));
    out.push_str(&format!("Full string : {}\n", v.full()));
    out.push_str(&format!("Core        : {}\n", v.core()));
    Ok(out)
}

fn compare(args: &serde_json::Value) -> Result<String, String> {
    let a_str = args
        .get("a")
        .or_else(|| args.get("version1"))
        .and_then(|v| v.as_str())
        .ok_or("semver_tools compare: 'a' is required")?;
    let b_str = args
        .get("b")
        .or_else(|| args.get("version2"))
        .and_then(|v| v.as_str())
        .ok_or("semver_tools compare: 'b' is required")?;

    let a = SemVer::parse(a_str)?;
    let b = SemVer::parse(b_str)?;

    let (symbol, label) = match a.cmp(&b) {
        std::cmp::Ordering::Less => ("<", "A is older"),
        std::cmp::Ordering::Equal => ("=", "equal"),
        std::cmp::Ordering::Greater => (">", "A is newer"),
    };

    let mut out = format!("SEMVER COMPARE\n{}\n", "─".repeat(50));
    out.push_str(&format!("A      : {}\n", a.full()));
    out.push_str(&format!("B      : {}\n", b.full()));
    out.push_str(&format!("Result : A {symbol} B  ({label})\n"));
    Ok(out)
}

fn bump(args: &serde_json::Value) -> Result<String, String> {
    let input = args
        .get("input")
        .or_else(|| args.get("version"))
        .and_then(|v| v.as_str())
        .ok_or("semver_tools bump: 'input' is required")?;
    let part = args.get("part").and_then(|v| v.as_str()).unwrap_or("patch");

    let v = SemVer::parse(input)?;

    let bumped = match part {
        "major" => SemVer { major: v.major + 1, minor: 0, patch: 0, pre: None, build: None },
        "minor" => SemVer { major: v.major, minor: v.minor + 1, patch: 0, pre: None, build: None },
        "patch" => SemVer { major: v.major, minor: v.minor, patch: v.patch + 1, pre: None, build: None },
        "premajor" => SemVer { major: v.major + 1, minor: 0, patch: 0, pre: Some("0".to_string()), build: None },
        "preminor" => SemVer { major: v.major, minor: v.minor + 1, patch: 0, pre: Some("0".to_string()), build: None },
        "prepatch" => SemVer { major: v.major, minor: v.minor, patch: v.patch + 1, pre: Some("0".to_string()), build: None },
        other => return Err(format!("semver_tools bump: unknown part '{other}'. Use: major, minor, patch, premajor, preminor, prepatch")),
    };

    let mut out = format!("SEMVER BUMP\n{}\n", "─".repeat(50));
    out.push_str(&format!("Input  : {}\n", v.full()));
    out.push_str(&format!("Part   : {part}\n"));
    out.push_str(&format!("Result : {}\n", bumped.full()));
    Ok(out)
}

fn validate(args: &serde_json::Value) -> Result<String, String> {
    let input = args
        .get("input")
        .or_else(|| args.get("version"))
        .and_then(|v| v.as_str())
        .ok_or("semver_tools validate: 'input' is required")?;

    match SemVer::parse(input) {
        Ok(v) => Ok(format!(
            "SEMVER VALIDATE\n{}\nInput : {input}\nValid : YES ({})\n",
            "─".repeat(50),
            v.full()
        )),
        Err(e) => Ok(format!(
            "SEMVER VALIDATE\n{}\nInput : {input}\nValid : NO\nError : {e}\n",
            "─".repeat(50)
        )),
    }
}

fn satisfies(args: &serde_json::Value) -> Result<String, String> {
    let version_str = args
        .get("version")
        .or_else(|| args.get("input"))
        .and_then(|v| v.as_str())
        .ok_or("semver_tools satisfies: 'version' is required")?;
    let range = args.get("range").and_then(|v| v.as_str()).ok_or(
        "semver_tools satisfies: 'range' is required (e.g. '^1.2.3', '>=2.0.0 <3.0.0', '~1.0')",
    )?;

    let v = SemVer::parse(version_str)?;
    let result = version_satisfies(&v, range)?;

    let mut out = format!("SEMVER SATISFIES\n{}\n", "─".repeat(50));
    out.push_str(&format!("Version : {}\n", v.full()));
    out.push_str(&format!("Range   : {range}\n"));
    out.push_str(&format!(
        "Result  : {}\n",
        if result {
            "YES — version satisfies the range"
        } else {
            "NO — version does not satisfy the range"
        }
    ));
    Ok(out)
}

fn sort_action(args: &serde_json::Value) -> Result<String, String> {
    let versions = args
        .get("versions")
        .and_then(|v| v.as_array())
        .ok_or("semver_tools sort: 'versions' must be an array of version strings")?;

    let order = args.get("order").and_then(|v| v.as_str()).unwrap_or("asc");

    let mut parsed: Vec<(String, SemVer)> = versions
        .iter()
        .filter_map(|v| v.as_str())
        .map(|s| SemVer::parse(s).map(|v| (s.to_string(), v)))
        .collect::<Result<_, _>>()?;

    match order {
        "desc" => parsed.sort_by(|a, b| b.1.cmp(&a.1)),
        _ => parsed.sort_by(|a, b| a.1.cmp(&b.1)),
    }

    let mut out = format!("SEMVER SORT ({order})\n{}\n", "─".repeat(50));
    for (i, (original, _v)) in parsed.iter().enumerate() {
        out.push_str(&format!("  {:2}. {original}\n", i + 1));
    }
    Ok(out)
}
