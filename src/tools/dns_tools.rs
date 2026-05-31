use serde_json::Value;

pub async fn execute(args: &Value) -> Result<String, String> {
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("parse");
    match action {
        "parse" | "list" => action_parse(args),
        "records" => action_records(args),
        "validate" => action_validate(args),
        "explain" => action_explain(args),
        other => Err(format!(
            "dns_tools: unknown action '{other}'. Valid: parse, records, validate, explain"
        )),
    }
}

// ── I/O ───────────────────────────────────────────────────────────────────────

fn get_zone_text(args: &Value) -> Result<String, String> {
    if let Some(s) = args
        .get("text")
        .or_else(|| args.get("zone"))
        .or_else(|| args.get("input"))
        .and_then(|v| v.as_str())
    {
        return Ok(s.to_string());
    }
    if let Some(path) = args.get("file").and_then(|v| v.as_str()) {
        return std::fs::read_to_string(path)
            .map_err(|e| format!("dns_tools: cannot read '{path}': {e}"));
    }
    Err("dns_tools: 'text'/'zone' (inline zone content) or 'file' (path) is required".to_string())
}

// ── Data structures ───────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
struct DnsRecord {
    name: String,
    ttl: Option<u32>,
    #[allow(dead_code)]
    class: String,
    rtype: String,
    rdata: String,
}

#[derive(Debug, Default)]
struct ZoneFile {
    origin: Option<String>,
    default_ttl: Option<u32>,
    records: Vec<DnsRecord>,
}

// ── Parser ────────────────────────────────────────────────────────────────────

/// Join continuation lines (parenthesised multi-line records into one logical line).
fn join_continuations(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut depth = 0usize;
    for ch in text.chars() {
        match ch {
            '(' => {
                depth += 1;
                out.push(' ');
            }
            ')' => {
                depth = depth.saturating_sub(1);
                out.push(' ');
            }
            '\n' | '\r' => {
                if depth > 0 {
                    out.push(' ');
                } else {
                    out.push('\n');
                }
            }
            _ => out.push(ch),
        }
    }
    out
}

/// Strip an inline or trailing comment (everything after `;` not inside a quoted string).
fn strip_comment(s: &str) -> &str {
    let mut in_quotes = false;
    for (i, ch) in s.char_indices() {
        match ch {
            '"' => in_quotes = !in_quotes,
            ';' if !in_quotes => return &s[..i],
            _ => {}
        }
    }
    s
}

fn parse_zone(text: &str) -> ZoneFile {
    let joined = join_continuations(text);
    let mut zone = ZoneFile::default();
    let mut last_name = "@".to_string();

    for raw_line in joined.lines() {
        let line = strip_comment(raw_line).trim();
        if line.is_empty() {
            continue;
        }

        // $ORIGIN directive
        if let Some(rest) = line.strip_prefix("$ORIGIN") {
            zone.origin = Some(rest.trim().to_string());
            continue;
        }

        // $TTL directive
        if let Some(rest) = line.strip_prefix("$TTL") {
            if let Ok(v) = rest.trim().parse::<u32>() {
                zone.default_ttl = Some(v);
            }
            continue;
        }

        // Skip other $ directives
        if line.starts_with('$') {
            continue;
        }

        // Parse a resource record line
        if let Some(rec) = parse_record_line(line, &last_name) {
            if rec.name != "@" && !rec.name.is_empty() {
                last_name = rec.name.clone();
            }
            zone.records.push(rec);
        }
    }

    zone
}

/// Parse one (already-flattened) resource record line.
fn parse_record_line(line: &str, last_name: &str) -> Option<DnsRecord> {
    let tokens: Vec<&str> = line.split_whitespace().collect();
    if tokens.is_empty() {
        return None;
    }

    // Determine if the first token is a name/owner or if the line starts with whitespace
    // (meaning the name is inherited from the previous record).
    let starts_with_ws = line.starts_with(' ') || line.starts_with('\t');

    let mut idx = 0usize;

    // Name field — present only when line does not start with whitespace
    let name = if starts_with_ws {
        last_name.to_string()
    } else {
        let n = tokens[idx].to_string();
        idx += 1;
        n
    };

    if idx >= tokens.len() {
        return None;
    }

    // Optional TTL (numeric)
    let mut ttl: Option<u32> = None;
    if let Ok(v) = tokens[idx].parse::<u32>() {
        ttl = Some(v);
        idx += 1;
    }

    if idx >= tokens.len() {
        return None;
    }

    // Optional class (IN / CH / HS)
    let mut class = "IN".to_string();
    let upper = tokens[idx].to_uppercase();
    if matches!(upper.as_str(), "IN" | "CH" | "HS" | "ANY") {
        class = upper;
        idx += 1;
    }

    if idx >= tokens.len() {
        return None;
    }

    // If we haven't seen a TTL yet, check again (class then ttl ordering is rare but valid)
    if ttl.is_none() {
        if let Ok(v) = tokens[idx].parse::<u32>() {
            ttl = Some(v);
            idx += 1;
        }
    }

    if idx >= tokens.len() {
        return None;
    }

    let rtype = tokens[idx].to_uppercase();
    idx += 1;

    let rdata = tokens[idx..].join(" ");

    Some(DnsRecord {
        name,
        ttl,
        class,
        rtype,
        rdata,
    })
}

// ── action_parse ──────────────────────────────────────────────────────────────

fn action_parse(args: &Value) -> Result<String, String> {
    let text = get_zone_text(args)?;
    let zone = parse_zone(&text);

    let mut out = String::from("DNS Zone File\n");
    out += &"=".repeat(60);
    out += "\n\n";

    if let Some(o) = &zone.origin {
        out += &format!("$ORIGIN  {}\n", o);
    }
    if let Some(t) = zone.default_ttl {
        out += &format!("$TTL     {}\n", t);
    }
    if zone.origin.is_some() || zone.default_ttl.is_some() {
        out += "\n";
    }

    if zone.records.is_empty() {
        out += "No records parsed.\n";
        return Ok(out);
    }

    // Group by record type
    let mut type_order: Vec<String> = Vec::new();
    let mut by_type: std::collections::BTreeMap<String, Vec<&DnsRecord>> =
        std::collections::BTreeMap::new();
    for rec in &zone.records {
        if !type_order.contains(&rec.rtype) {
            type_order.push(rec.rtype.clone());
        }
        by_type.entry(rec.rtype.clone()).or_default().push(rec);
    }

    for rtype in &type_order {
        let recs = &by_type[rtype];
        out += &format!(
            "── {} Records ({}) ──────────────────────────────────────\n",
            rtype,
            recs.len()
        );
        out += &format!("{:<30} {:>8}  {:<4}  {}\n", "NAME", "TTL", "TYPE", "DATA");
        out += &"─".repeat(80);
        out += "\n";
        for r in recs.iter() {
            let ttl_s = r
                .ttl
                .map(|v| v.to_string())
                .unwrap_or_else(|| "-".to_string());
            out += &format!(
                "{:<30} {:>8}  {:<4}  {}\n",
                r.name,
                ttl_s,
                r.rtype,
                truncate_rdata(&r.rdata, 60)
            );
        }
        out += "\n";
    }

    // Summary
    out += &"─".repeat(60);
    out += "\n";
    out += &format!("Total records: {}\n", zone.records.len());
    let mut counts: Vec<(String, usize)> =
        by_type.iter().map(|(k, v)| (k.clone(), v.len())).collect();
    counts.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    for (rtype, n) in &counts {
        out += &format!("  {:<8} {}\n", rtype, n);
    }

    Ok(out)
}

fn truncate_rdata(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}…", &s[..max])
    }
}

// ── action_records ────────────────────────────────────────────────────────────

fn action_records(args: &Value) -> Result<String, String> {
    let text = get_zone_text(args)?;
    let zone = parse_zone(&text);

    let filter = args
        .get("type")
        .or_else(|| args.get("rtype"))
        .or_else(|| args.get("record_type"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_uppercase());

    let filter_ref = filter.as_deref().unwrap_or("");

    let filtered: Vec<&DnsRecord> = zone
        .records
        .iter()
        .filter(|r| filter_ref.is_empty() || r.rtype == filter_ref)
        .collect();

    if filtered.is_empty() {
        return Ok(format!(
            "No {} records found in zone file.\n",
            if filter_ref.is_empty() {
                "any".to_string()
            } else {
                filter_ref.to_string()
            }
        ));
    }

    let label = if filter_ref.is_empty() {
        "All Records".to_string()
    } else {
        format!("{} Records", filter_ref)
    };

    let mut out = format!("{}\n{}\n\n", label, "=".repeat(60));
    out += &format!("{:<30} {:>8}  {:<4}  {}\n", "NAME", "TTL", "TYPE", "DATA");
    out += &"─".repeat(80);
    out += "\n";
    for r in &filtered {
        let ttl_s = r
            .ttl
            .map(|v| v.to_string())
            .unwrap_or_else(|| "-".to_string());
        out += &format!("{:<30} {:>8}  {:<4}  {}\n", r.name, ttl_s, r.rtype, r.rdata);
    }
    out += &format!("\n{} record(s) shown.\n", filtered.len());
    Ok(out)
}

// ── action_validate ───────────────────────────────────────────────────────────

fn action_validate(args: &Value) -> Result<String, String> {
    let text = get_zone_text(args)?;
    let zone = parse_zone(&text);
    let mut warnings: Vec<String> = Vec::new();

    let has_soa = zone.records.iter().any(|r| r.rtype == "SOA");
    if !has_soa {
        warnings.push(
            "Missing SOA record — every authoritative zone requires exactly one SOA.".to_string(),
        );
    }

    let ns_count = zone.records.iter().filter(|r| r.rtype == "NS").count();
    if ns_count == 0 {
        warnings.push("No NS records found — zone has no name servers.".to_string());
    } else if ns_count == 1 {
        warnings.push(
            "Only one NS record — at least two name servers are recommended for redundancy."
                .to_string(),
        );
    }

    // CNAME at zone apex (name is "@" or matches origin)
    let origin = zone.origin.as_deref().unwrap_or("@");
    for rec in zone.records.iter().filter(|r| r.rtype == "CNAME") {
        let n = rec.name.trim_end_matches('.');
        let o = origin.trim_end_matches('.');
        if rec.name == "@" || n == o {
            warnings.push(format!(
                "CNAME at zone apex ('{}') is prohibited by RFC 1034.",
                rec.name
            ));
        }
    }

    // MX pointing to a CNAME target
    let cname_targets: std::collections::HashSet<String> = zone
        .records
        .iter()
        .filter(|r| r.rtype == "CNAME")
        .map(|r| r.name.to_lowercase())
        .collect();
    for rec in zone.records.iter().filter(|r| r.rtype == "MX") {
        // MX rdata: "<priority> <hostname>"
        let host = rec
            .rdata
            .split_whitespace()
            .nth(1)
            .unwrap_or("")
            .to_lowercase();
        let host = host.trim_end_matches('.');
        if cname_targets.contains(host) {
            warnings.push(format!(
                "MX record '{}' points to '{}' which appears to be a CNAME target — not recommended (RFC 2181).",
                rec.name, host
            ));
        }
    }

    // Duplicate A/AAAA records for the same name
    let mut a_names: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for rec in zone
        .records
        .iter()
        .filter(|r| r.rtype == "A" || r.rtype == "AAAA")
    {
        let key = format!("{}|{}", rec.name.to_lowercase(), rec.rtype);
        *a_names.entry(key).or_insert(0) += 1;
    }
    for (key, n) in &a_names {
        if *n > 1 {
            let parts: Vec<&str> = key.splitn(2, '|').collect();
            warnings.push(format!(
                "Duplicate {} record for '{}' ({} entries) — may be intentional (round-robin) but verify.",
                parts.get(1).copied().unwrap_or("A/AAAA"),
                parts.first().copied().unwrap_or("?"),
                n
            ));
        }
    }

    // TXT record string length > 255 chars
    for rec in zone.records.iter().filter(|r| r.rtype == "TXT") {
        let content = rec.rdata.trim_matches('"');
        if content.len() > 255 {
            warnings.push(format!(
                "TXT record for '{}' contains a string longer than 255 characters ({} chars) — exceeds single-string DNS limit.",
                rec.name,
                content.len()
            ));
        }
    }

    // Multiple SPF TXT records for the same name
    let mut spf_owners: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for rec in zone.records.iter().filter(|r| r.rtype == "TXT") {
        if rec.rdata.contains("v=spf1") {
            *spf_owners.entry(rec.name.to_lowercase()).or_insert(0) += 1;
        }
    }
    for (owner, n) in &spf_owners {
        if *n > 1 {
            warnings.push(format!(
                "Multiple SPF TXT records for '{}' ({} found) — only one SPF record is allowed per name (RFC 7208).",
                owner, n
            ));
        }
    }

    let verdict = if warnings.is_empty() {
        "VALID — no issues found"
    } else {
        "WARNINGS FOUND"
    };

    let mut out = format!("DNS Zone Validation\n{}\n\n", "=".repeat(60));
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

// ── action_explain ────────────────────────────────────────────────────────────

fn action_explain(args: &Value) -> Result<String, String> {
    let text = get_zone_text(args)?;
    let zone = parse_zone(&text);

    let mut out = format!("DNS Zone Explanation\n{}\n\n", "=".repeat(60));

    if let Some(o) = &zone.origin {
        out += &format!("Origin domain: {}\n", o);
    }
    if let Some(t) = zone.default_ttl {
        out += &format!("Default TTL:   {} seconds ({} minutes)\n", t, t / 60);
    }
    if zone.origin.is_some() || zone.default_ttl.is_some() {
        out += "\n";
    }

    if zone.records.is_empty() {
        out += "No records to explain.\n";
        return Ok(out);
    }

    // Collect unique record types
    let mut seen_types: Vec<String> = Vec::new();
    for r in &zone.records {
        if !seen_types.contains(&r.rtype) {
            seen_types.push(r.rtype.clone());
        }
    }

    let count_of =
        |rtype: &str| -> usize { zone.records.iter().filter(|r| r.rtype == rtype).count() };

    for rtype in &seen_types {
        let n = count_of(rtype);
        out += &format!(
            "── {} ({} record{})\n",
            rtype,
            n,
            if n == 1 { "" } else { "s" }
        );
        out += &format!("   {}\n", record_type_description(rtype));

        match rtype.as_str() {
            "MX" => {
                out += "   Mail servers (priority → hostname):\n";
                for rec in zone.records.iter().filter(|r| r.rtype == "MX") {
                    let mut parts = rec.rdata.splitn(2, ' ');
                    let prio = parts.next().unwrap_or("?");
                    let host = parts.next().unwrap_or("?");
                    out += &format!("     {} priority {} → {}\n", rec.name, prio, host);
                }
            }
            "TXT" => {
                explain_txt_records(&zone.records, &mut out);
            }
            "SOA" => {
                if let Some(soa) = zone.records.iter().find(|r| r.rtype == "SOA") {
                    let parts: Vec<&str> = soa.rdata.split_whitespace().collect();
                    // SOA rdata: mname rname serial refresh retry expire minimum
                    if parts.len() >= 7 {
                        out += &format!("   Primary NS:     {}\n", parts[0]);
                        out += &format!(
                            "   Admin email:    {} (dots before @ are escaped)\n",
                            parts[1]
                        );
                        out += &format!("   Serial:         {}\n", parts[2]);
                        out += &format!("   Refresh:        {}s\n", parts[3]);
                        out += &format!("   Retry:          {}s\n", parts[4]);
                        out += &format!("   Expire:         {}s\n", parts[5]);
                        out += &format!("   Minimum TTL:    {}s\n", parts[6]);
                    } else {
                        out += &format!("   RDATA: {}\n", soa.rdata);
                    }
                }
            }
            "CAA" => {
                out += "   Certification Authority Authorization — restricts which CAs may issue certificates:\n";
                for rec in zone.records.iter().filter(|r| r.rtype == "CAA") {
                    // CAA rdata: <flags> <tag> <value>
                    let parts: Vec<&str> = rec.rdata.splitn(3, ' ').collect();
                    if parts.len() >= 3 {
                        let tag = parts[1];
                        let val = parts[2].trim_matches('"');
                        match tag {
                            "issue" => {
                                out += &format!(
                                    "     {} — allows {} to issue certificates\n",
                                    rec.name, val
                                )
                            }
                            "issuewild" => {
                                out += &format!(
                                    "     {} — allows {} to issue wildcard certificates\n",
                                    rec.name, val
                                )
                            }
                            "iodef" => {
                                out +=
                                    &format!("     {} — violation reports to: {}\n", rec.name, val)
                            }
                            _ => out += &format!("     {} — {} {}\n", rec.name, tag, val),
                        }
                    }
                }
            }
            "NS" => {
                out += "   Name servers:\n";
                for rec in zone.records.iter().filter(|r| r.rtype == "NS") {
                    out += &format!("     {} → {}\n", rec.name, rec.rdata);
                }
            }
            "SRV" => {
                out += "   Service records (priority weight port target):\n";
                for rec in zone.records.iter().filter(|r| r.rtype == "SRV") {
                    out += &format!("     {} → {}\n", rec.name, rec.rdata);
                }
            }
            _ => {}
        }
        out += "\n";
    }

    Ok(out)
}

fn record_type_description(rtype: &str) -> &'static str {
    match rtype {
        "A"     => "Maps a hostname to an IPv4 address.",
        "AAAA"  => "Maps a hostname to an IPv6 address.",
        "MX"    => "Identifies mail servers responsible for accepting email for the domain.",
        "TXT"   => "Stores arbitrary text; used for SPF, DKIM, DMARC, ownership verification, and more.",
        "CNAME" => "Alias — maps one hostname to another (canonical) hostname.",
        "NS"    => "Specifies the authoritative name servers for the zone.",
        "SOA"   => "Start of Authority — defines zone parameters: primary NS, admin email, serial, and timing values.",
        "PTR"   => "Reverse DNS lookup — maps an IP address back to a hostname.",
        "SRV"   => "Service locator — specifies host and port for a given service (e.g. _sip._tcp).",
        "CAA"   => "Certification Authority Authorization — limits which CAs may issue TLS certificates.",
        "DNAME" => "Delegation Name — maps an entire subtree of the DNS namespace to another domain.",
        _       => "Resource record of this type.",
    }
}

fn explain_txt_records(records: &[DnsRecord], out: &mut String) {
    let txt_records: Vec<&DnsRecord> = records.iter().filter(|r| r.rtype == "TXT").collect();

    // SPF
    let spf: Vec<&DnsRecord> = txt_records
        .iter()
        .copied()
        .filter(|r| r.rdata.contains("v=spf1"))
        .collect();
    for rec in &spf {
        out.push_str(&format!("   SPF record for '{}':\n", rec.name));
        explain_spf(&rec.rdata, out);
    }

    // DKIM
    let dkim: Vec<&DnsRecord> = txt_records
        .iter()
        .copied()
        .filter(|r| r.rdata.contains("v=DKIM1"))
        .collect();
    if !dkim.is_empty() {
        out.push_str(&format!(
            "   {} DKIM public key record(s) — used to verify email signatures.\n",
            dkim.len()
        ));
    }

    // DMARC
    let dmarc: Vec<&DnsRecord> = txt_records
        .iter()
        .copied()
        .filter(|r| r.rdata.contains("v=DMARC1"))
        .collect();
    for rec in &dmarc {
        out.push_str(&format!("   DMARC record for '{}':\n", rec.name));
        explain_dmarc(&rec.rdata, out);
    }

    // Other TXT
    let other_count = txt_records.len() - spf.len() - dkim.len() - dmarc.len();
    if other_count > 0 {
        out.push_str(&format!(
            "   {} other TXT record(s) — may be domain verification, DKIM selectors, or custom data.\n",
            other_count
        ));
    }
}

fn explain_spf(rdata: &str, out: &mut String) {
    let content = rdata.trim_matches('"');
    for token in content.split_whitespace() {
        let desc = match token {
            "v=spf1" => "SPF version 1",
            "+all" | "all" => "ALLOW all senders not matched above (permissive — not recommended)",
            "~all" => "SOFTFAIL non-matching senders (mark as suspicious but deliver)",
            "-all" => "FAIL non-matching senders (reject)",
            "?all" => "NEUTRAL — no explicit policy for unmatched senders",
            _ if token.starts_with("include:") => {
                let domain = &token["include:".len()..];
                return out.push_str(&format!(
                    "     include:{}  — also accept senders authorized by {}\n",
                    domain, domain
                ));
            }
            _ if token.starts_with("ip4:") => {
                return out.push_str(&format!(
                    "     {}  — allow this IPv4 address/range\n",
                    token
                ));
            }
            _ if token.starts_with("ip6:") => {
                return out.push_str(&format!(
                    "     {}  — allow this IPv6 address/range\n",
                    token
                ));
            }
            _ if token.starts_with("redirect=") => {
                let domain = &token["redirect=".len()..];
                return out.push_str(&format!(
                    "     redirect={}  — use {}'s SPF policy instead\n",
                    domain, domain
                ));
            }
            _ if token.starts_with("a:") || token == "a" => {
                "allow senders matching the A record of this domain"
            }
            _ if token.starts_with("mx:") || token == "mx" => {
                "allow senders matching the MX record(s) of this domain"
            }
            _ if token.starts_with("ptr:") || token == "ptr" => {
                "allow senders with matching PTR (reverse DNS) records"
            }
            _ if token.starts_with("exists:") => {
                "conditional mechanism — sender authorized if named host resolves"
            }
            _ => "SPF mechanism",
        };
        out.push_str(&format!("     {}  — {}\n", token, desc));
    }
}

fn explain_dmarc(rdata: &str, out: &mut String) {
    let content = rdata.trim_matches('"');
    for tag in content.split(';') {
        let tag = tag.trim();
        if tag.is_empty() {
            continue;
        }
        let (key, val) = if let Some(pos) = tag.find('=') {
            (&tag[..pos], &tag[pos + 1..])
        } else {
            (tag, "")
        };
        let desc = match key.trim() {
            "v" => format!("DMARC version: {}", val),
            "p" => match val {
                "none" => "Policy: none — monitor only, no action taken".to_string(),
                "quarantine" => "Policy: quarantine — mark failing messages as spam".to_string(),
                "reject" => "Policy: reject — refuse messages that fail DMARC checks".to_string(),
                _ => format!("Policy: {}", val),
            },
            "sp" => format!("Subdomain policy: {}", val),
            "rua" => format!("Aggregate reports sent to: {}", val),
            "ruf" => format!("Forensic reports sent to: {}", val),
            "pct" => format!("Policy applies to {}% of failing messages", val),
            "adkim" => format!(
                "DKIM alignment: {}",
                if val == "s" { "strict" } else { "relaxed" }
            ),
            "aspf" => format!(
                "SPF alignment: {}",
                if val == "s" { "strict" } else { "relaxed" }
            ),
            "fo" => format!("Failure reporting options: {}", val),
            "ri" => format!("Report interval: {} seconds", val),
            _ => format!("{} = {}", key.trim(), val),
        };
        out.push_str(&format!("     {}\n", desc));
    }
}
