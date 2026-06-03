use serde_json::{json, Value};
use std::collections::HashMap;

pub fn make_schema() -> Value {
    json!({
        "name": "cvss_tools",
        "description": "Decode, score, and compare CVSS v3.x vulnerability vectors without external utilities.",
        "parameters": {
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["decode", "score", "compare", "severity"],
                    "description": "decode=all metrics with human labels (default), score=base score calculation with formula, compare=two vectors side-by-side, severity=rating only"
                },
                "vector": { "type": "string", "description": "CVSS v3.x vector e.g. CVSS:3.1/AV:N/AC:L/PR:N/UI:N/S:U/C:H/I:H/A:H" },
                "vector_a": { "type": "string", "description": "First vector for compare action" },
                "vector_b": { "type": "string", "description": "Second vector for compare action" }
            }
        }
    })
}

struct Metrics {
    version: String,
    av: char,
    ac: char,
    pr: char,
    ui: char,
    s: char,
    c: char,
    i: char,
    a: char,
}

fn parse_vector(v: &str) -> Result<Metrics, String> {
    let v = v.trim();
    let (version, rest) = if v.starts_with("CVSS:") {
        let slash = v.find('/').ok_or("Invalid CVSS vector: missing '/'")?;
        (v[5..slash].to_string(), &v[slash + 1..])
    } else {
        ("3.1".to_string(), v)
    };

    let mut map: HashMap<&str, char> = HashMap::new();
    for part in rest.split('/') {
        if part.is_empty() {
            continue;
        }
        let colon = part
            .find(':')
            .ok_or(format!("Invalid metric segment '{}'", part))?;
        let key = &part[..colon];
        let val = part[colon + 1..]
            .chars()
            .next()
            .ok_or(format!("Empty value for metric '{}'", key))?;
        map.insert(key, val);
    }

    let get = |k: &str| -> Result<char, String> {
        map.get(k)
            .copied()
            .ok_or(format!("Missing required metric '{}'", k))
    };

    Ok(Metrics {
        version,
        av: get("AV")?,
        ac: get("AC")?,
        pr: get("PR")?,
        ui: get("UI")?,
        s: get("S")?,
        c: get("C")?,
        i: get("I")?,
        a: get("A")?,
    })
}

fn av_label(c: char) -> (&'static str, &'static str) {
    match c {
        'N' => (
            "Network",
            "Exploitable remotely with no physical/adjacent access.",
        ),
        'A' => (
            "Adjacent",
            "Exploitable from the same network segment or LAN.",
        ),
        'L' => ("Local", "Attacker must have local (logged-in) access."),
        'P' => ("Physical", "Requires physical hardware access."),
        _ => ("Unknown", ""),
    }
}
fn ac_label(c: char) -> (&'static str, &'static str) {
    match c {
        'L' => ("Low", "No special conditions required."),
        'H' => ("High", "Requires specific conditions or race conditions."),
        _ => ("Unknown", ""),
    }
}
fn pr_label(c: char) -> (&'static str, &'static str) {
    match c {
        'N' => ("None", "No authentication or privileges required."),
        'L' => ("Low", "Basic user-level privileges required."),
        'H' => ("High", "Administrator or elevated privileges required."),
        _ => ("Unknown", ""),
    }
}
fn ui_label(c: char) -> (&'static str, &'static str) {
    match c {
        'N' => ("None", "No user interaction needed."),
        'R' => (
            "Required",
            "A user must take some action to trigger the exploit.",
        ),
        _ => ("Unknown", ""),
    }
}
fn scope_label(c: char) -> (&'static str, &'static str) {
    match c {
        'U' => (
            "Unchanged",
            "Exploit impact stays within the vulnerable component.",
        ),
        'C' => (
            "Changed",
            "Exploit can impact resources beyond the vulnerable component.",
        ),
        _ => ("Unknown", ""),
    }
}
fn impact_label(c: char) -> (&'static str, &'static str) {
    match c {
        'N' => ("None", "No impact."),
        'L' => (
            "Low",
            "Some limited impact on confidentiality, integrity, or availability.",
        ),
        'H' => (
            "High",
            "Total or severe impact — complete loss or compromise.",
        ),
        _ => ("Unknown", ""),
    }
}

fn av_score(c: char) -> f64 {
    match c {
        'N' => 0.85,
        'A' => 0.62,
        'L' => 0.55,
        'P' => 0.2,
        _ => 0.0,
    }
}
fn ac_score(c: char) -> f64 {
    match c {
        'L' => 0.77,
        'H' => 0.44,
        _ => 0.0,
    }
}
fn pr_score(c: char, scope: char) -> f64 {
    if scope == 'C' {
        match c {
            'N' => 0.85,
            'L' => 0.68,
            'H' => 0.5,
            _ => 0.0,
        }
    } else {
        match c {
            'N' => 0.85,
            'L' => 0.62,
            'H' => 0.27,
            _ => 0.0,
        }
    }
}
fn ui_score(c: char) -> f64 {
    match c {
        'N' => 0.85,
        'R' => 0.62,
        _ => 0.0,
    }
}
fn impact_score(c: char) -> f64 {
    match c {
        'N' => 0.0,
        'L' => 0.22,
        'H' => 0.56,
        _ => 0.0,
    }
}

fn roundup(val: f64) -> f64 {
    // CVSS roundup: round to 1 decimal, always up
    let rounded = (val * 10.0).ceil() / 10.0;
    (rounded * 10.0).round() / 10.0
}

fn compute_score(m: &Metrics) -> f64 {
    let ic = impact_score(m.c);
    let ii = impact_score(m.i);
    let ia = impact_score(m.a);

    let iss = 1.0 - (1.0 - ic) * (1.0 - ii) * (1.0 - ia);

    if iss <= 0.0 {
        return 0.0;
    }

    let isc_base = if m.s == 'C' {
        7.52 * (iss - 0.029) - 3.25 * (iss - 0.02_f64).powf(15.0)
    } else {
        6.42 * iss
    };

    if isc_base <= 0.0 {
        return 0.0;
    }

    let exp = 8.22 * av_score(m.av) * ac_score(m.ac) * pr_score(m.pr, m.s) * ui_score(m.ui);

    if m.s == 'C' {
        roundup(f64::min(1.08 * (isc_base + exp), 10.0))
    } else {
        roundup(f64::min(isc_base + exp, 10.0))
    }
}

fn severity(score: f64) -> &'static str {
    if score == 0.0 {
        "None"
    } else if score < 4.0 {
        "Low"
    } else if score < 7.0 {
        "Medium"
    } else if score < 9.0 {
        "High"
    } else {
        "Critical"
    }
}

pub async fn execute(args: &Value) -> Result<String, String> {
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("decode");

    if action == "compare" {
        let va = args
            .get("vector_a")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim();
        let vb = args
            .get("vector_b")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim();
        if va.is_empty() || vb.is_empty() {
            return Err("'compare' requires both 'vector_a' and 'vector_b'.".to_string());
        }
        let ma = parse_vector(va)?;
        let mb = parse_vector(vb)?;
        return action_compare(va, &ma, vb, &mb);
    }

    let vector = args
        .get("vector")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim();
    if vector.is_empty() {
        return Err(
            "Provide 'vector' (CVSS v3.x string, e.g. CVSS:3.1/AV:N/AC:L/PR:N/UI:N/S:U/C:H/I:H/A:H)."
                .to_string(),
        );
    }

    let m = parse_vector(vector)?;

    match action {
        "score" => action_score(vector, &m),
        "severity" => action_severity(vector, &m),
        _ => action_decode(vector, &m),
    }
}

fn action_decode(vector: &str, m: &Metrics) -> Result<String, String> {
    let score = compute_score(m);
    let sev = severity(score);
    let mut out = String::new();
    out.push_str(&format!(
        "## CVSS {} — {} ({}) — {}\n\n",
        m.version, score, sev, vector
    ));

    let fmt = |label: &str, name: &str, desc: &str| format!("{:<30} {} — {}\n", label, name, desc);

    let (av_n, av_d) = av_label(m.av);
    let (ac_n, ac_d) = ac_label(m.ac);
    let (pr_n, pr_d) = pr_label(m.pr);
    let (ui_n, ui_d) = ui_label(m.ui);
    let (s_n, s_d) = scope_label(m.s);
    let (c_n, c_d) = impact_label(m.c);
    let (i_n, i_d) = impact_label(m.i);
    let (a_n, a_d) = impact_label(m.a);

    out.push_str("### Base Metrics\n");
    out.push_str(&fmt(&format!("Attack Vector (AV:{})", m.av), av_n, av_d));
    out.push_str(&fmt(
        &format!("Attack Complexity (AC:{})", m.ac),
        ac_n,
        ac_d,
    ));
    out.push_str(&fmt(
        &format!("Privileges Required (PR:{})", m.pr),
        pr_n,
        pr_d,
    ));
    out.push_str(&fmt(&format!("User Interaction (UI:{})", m.ui), ui_n, ui_d));
    out.push_str(&fmt(&format!("Scope (S:{})", m.s), s_n, s_d));
    out.push('\n');
    out.push_str("### Impact Metrics\n");
    out.push_str(&fmt(&format!("Confidentiality (C:{})", m.c), c_n, c_d));
    out.push_str(&fmt(&format!("Integrity (I:{})", m.i), i_n, i_d));
    out.push_str(&fmt(&format!("Availability (A:{})", m.a), a_n, a_d));
    out.push('\n');
    out.push_str(&format!("**Base Score:** {} **{}**\n", score, sev));
    out.push_str(&format!("\n{}\n", score_bar(score)));

    Ok(out)
}

fn action_score(vector: &str, m: &Metrics) -> Result<String, String> {
    let ic = impact_score(m.c);
    let ii = impact_score(m.i);
    let ia = impact_score(m.a);
    let iss = 1.0 - (1.0 - ic) * (1.0 - ii) * (1.0 - ia);
    let isc_base = if m.s == 'C' {
        7.52 * (iss - 0.029) - 3.25 * (iss - 0.02_f64).powf(15.0)
    } else {
        6.42 * iss
    };
    let exp = 8.22 * av_score(m.av) * ac_score(m.ac) * pr_score(m.pr, m.s) * ui_score(m.ui);
    let score = compute_score(m);
    let sev = severity(score);

    let mut out = String::new();
    out.push_str(&format!(
        "## CVSS {} Score Breakdown — {}\n\n",
        m.version, vector
    ));
    out.push_str("### CVSS v3.1 Base Score Formula\n\n");
    out.push_str(&format!(
        "ISS = 1 - [(1-C:{:.2}) × (1-I:{:.2}) × (1-A:{:.2})] = {:.4}\n",
        ic, ii, ia, iss
    ));
    if m.s == 'C' {
        out.push_str(&format!(
            "ISCBase = 7.52 × (ISS-0.029) − 3.25 × (ISS-0.02)^15 = {:.4}  (Scope: Changed)\n",
            isc_base
        ));
    } else {
        out.push_str(&format!(
            "ISCBase = 6.42 × ISS = {:.4}  (Scope: Unchanged)\n",
            isc_base
        ));
    }
    out.push_str(&format!(
        "Exploitability = 8.22 × AV:{:.2} × AC:{:.2} × PR:{:.2} × UI:{:.2} = {:.4}\n",
        av_score(m.av),
        ac_score(m.ac),
        pr_score(m.pr, m.s),
        ui_score(m.ui),
        exp
    ));
    if m.s == 'C' {
        out.push_str(&format!(
            "BaseScore = Roundup(min(1.08 × (ISCBase + Exploitability), 10)) = **{:.1}**\n",
            score
        ));
    } else {
        out.push_str(&format!(
            "BaseScore = Roundup(min(ISCBase + Exploitability, 10)) = **{:.1}**\n",
            score
        ));
    }
    out.push_str(&format!("\n**Severity: {}**  {}\n", sev, score_bar(score)));
    Ok(out)
}

fn action_severity(vector: &str, m: &Metrics) -> Result<String, String> {
    let score = compute_score(m);
    let sev = severity(score);
    Ok(format!(
        "## CVSS Severity\n\n**{}** ({}) — {}\n{}\n",
        sev,
        score,
        vector,
        score_bar(score)
    ))
}

fn action_compare(va: &str, ma: &Metrics, vb: &str, mb: &Metrics) -> Result<String, String> {
    let sa = compute_score(ma);
    let sb = compute_score(mb);

    let cmp_metric = |label: &str, a: char, b: char| -> String {
        let diff = if a != b { " ◄ differs" } else { "" };
        format!("{:<30} {}          {}{}\n", label, a, b, diff)
    };

    let mut out = String::new();
    out.push_str("## CVSS Vector Comparison\n\n");
    out.push_str(&format!("A: {}\n", va));
    out.push_str(&format!("B: {}\n\n", vb));
    out.push_str(&format!(
        "{:<30} {:10} {}\n",
        "Metric", "Vector A", "Vector B"
    ));
    out.push_str(&format!("{}\n", "─".repeat(55)));
    out.push_str(&cmp_metric("Attack Vector (AV)", ma.av, mb.av));
    out.push_str(&cmp_metric("Attack Complexity (AC)", ma.ac, mb.ac));
    out.push_str(&cmp_metric("Privileges Required (PR)", ma.pr, mb.pr));
    out.push_str(&cmp_metric("User Interaction (UI)", ma.ui, mb.ui));
    out.push_str(&cmp_metric("Scope (S)", ma.s, mb.s));
    out.push_str(&cmp_metric("Confidentiality (C)", ma.c, mb.c));
    out.push_str(&cmp_metric("Integrity (I)", ma.i, mb.i));
    out.push_str(&cmp_metric("Availability (A)", ma.a, mb.a));
    out.push('\n');
    out.push_str(&format!(
        "**Score A:** {} ({})    **Score B:** {} ({})\n",
        sa,
        severity(sa),
        sb,
        severity(sb)
    ));
    let higher = if sa > sb {
        "A is higher risk"
    } else if sb > sa {
        "B is higher risk"
    } else {
        "Equal score"
    };
    out.push_str(&format!("**Verdict:** {}\n", higher));
    Ok(out)
}

fn score_bar(score: f64) -> String {
    let pct = ((score / 10.0) * 30.0) as usize;
    let filled = pct.min(30);
    let bar = format!("{}{}", "█".repeat(filled), "░".repeat(30 - filled));
    format!("[{}] {:.1}/10", bar, score)
}
