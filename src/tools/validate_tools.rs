use serde_json::Value;

pub async fn execute(args: &Value) -> Result<String, String> {
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("auto");
    match action {
        "email" => email_action(args),
        "ipv4" => ipv4_action(args),
        "ipv6" => ipv6_action(args),
        "cidr" => cidr_action(args),
        "mac" => mac_action(args),
        "url" => url_action(args),
        "credit_card" | "creditcard" | "luhn" => credit_card_action(args),
        "isbn" => isbn_action(args),
        "uuid" => uuid_action(args),
        "phone" => phone_action(args),
        "semver" => semver_action(args),
        "hex_color" | "color" => hex_color_action(args),
        "auto" => auto_action(args),
        other => Err(format!(
            "validate_tools: unknown action '{other}'. Valid: email, ipv4, ipv6, cidr, mac, \
             url, credit_card, isbn, uuid, phone, semver, hex_color, auto"
        )),
    }
}

fn get_input(args: &Value) -> Result<String, String> {
    args.get("value")
        .or_else(|| args.get("input"))
        .or_else(|| args.get("text"))
        .and_then(|v| v.as_str())
        .ok_or("validate_tools: 'value'/'input'/'text' required".to_string())
        .map(|s| s.to_string())
}

fn verdict(valid: bool, details: &str) -> String {
    let mark = if valid { "VALID" } else { "INVALID" };
    format!("{mark}  {details}\n")
}

// ── Email ────────────────────────────────────────────────────────────────────

fn is_valid_email(s: &str) -> (bool, &'static str) {
    let s = s.trim();
    let parts: Vec<&str> = s.splitn(2, '@').collect();
    if parts.len() != 2 {
        return (false, "missing '@'");
    }
    let local = parts[0];
    let domain = parts[1];
    if local.is_empty() {
        return (false, "local part is empty");
    }
    if local.len() > 64 {
        return (false, "local part exceeds 64 chars");
    }
    if domain.is_empty() {
        return (false, "domain is empty");
    }
    if !domain.contains('.') {
        return (false, "domain has no dot");
    }
    if domain.starts_with('.') || domain.ends_with('.') {
        return (false, "domain starts or ends with dot");
    }
    if domain.len() > 255 {
        return (false, "domain exceeds 255 chars");
    }
    // Check domain labels
    for label in domain.split('.') {
        if label.is_empty() {
            return (false, "domain has empty label (double dot)");
        }
        if label.starts_with('-') || label.ends_with('-') {
            return (false, "domain label starts or ends with hyphen");
        }
        if !label.chars().all(|c| c.is_alphanumeric() || c == '-') {
            return (false, "domain label contains invalid character");
        }
    }
    // Check TLD is at least 2 chars
    let tld = domain.rsplit('.').next().unwrap_or("");
    if tld.len() < 2 {
        return (false, "TLD must be at least 2 characters");
    }
    // Check local part allowed chars (simplified RFC 5321)
    let allowed_local = |c: char| c.is_alphanumeric() || "!#$%&'*+/=?^_`{|}~.-".contains(c);
    if !local.chars().all(allowed_local) {
        return (false, "local part contains invalid character");
    }
    (true, "well-formed email address")
}

fn email_action(args: &Value) -> Result<String, String> {
    let input = get_input(args)?;
    let (valid, detail) = is_valid_email(&input);
    let mut out = format!("Email: {input}\n\n");
    out.push_str(&verdict(valid, detail));
    if valid {
        let parts: Vec<&str> = input.splitn(2, '@').collect();
        out.push_str(&format!("  Local:   {}\n", parts[0]));
        out.push_str(&format!("  Domain:  {}\n", parts[1]));
    }
    Ok(out)
}

// ── IPv4 ─────────────────────────────────────────────────────────────────────

fn is_valid_ipv4(s: &str) -> (bool, String) {
    let octets: Vec<&str> = s.trim().split('.').collect();
    if octets.len() != 4 {
        return (
            false,
            "must have exactly 4 octets separated by '.'".to_string(),
        );
    }
    let mut values = Vec::new();
    for oct in &octets {
        match oct.parse::<u8>() {
            Ok(v) => values.push(v),
            Err(_) => return (false, format!("octet '{}' is not 0–255", oct)),
        }
    }
    let class = match values[0] {
        0 => "This network (0.x.x.x)",
        10 => "Private (Class A, RFC1918)",
        127 => "Loopback",
        169 => {
            if values[1] == 254 {
                "Link-local (APIPA)"
            } else {
                "Public"
            }
        }
        172 => {
            if (16..=31).contains(&values[1]) {
                "Private (Class B, RFC1918)"
            } else {
                "Public"
            }
        }
        192 => {
            if values[1] == 168 {
                "Private (Class C, RFC1918)"
            } else {
                "Public"
            }
        }
        224..=239 => "Multicast",
        240..=254 => "Reserved (IETF)",
        255 => "Broadcast",
        _ => "Public",
    };
    let is_private = class.starts_with("Private") || class == "Loopback";
    (true, format!("{class}; private={is_private}"))
}

fn ipv4_action(args: &Value) -> Result<String, String> {
    let input = get_input(args)?;
    let (valid, detail) = is_valid_ipv4(&input);
    let mut out = format!("IPv4: {input}\n\n");
    out.push_str(&verdict(valid, &detail));
    Ok(out)
}

// ── IPv6 ─────────────────────────────────────────────────────────────────────

fn is_valid_ipv6(s: &str) -> (bool, &'static str) {
    let s = s.trim();
    // Use Rust's std parser
    match s.parse::<std::net::Ipv6Addr>() {
        Ok(addr) => {
            let kind = if addr.is_loopback() {
                "loopback (::1)"
            } else if addr.is_unspecified() {
                "unspecified (::)"
            } else if addr.to_string().starts_with("fe80") {
                "link-local"
            } else if addr.to_string().starts_with("fc") || addr.to_string().starts_with("fd") {
                "unique local (ULA)"
            } else {
                "global unicast"
            };
            // We store kind in a static-like way via a helper
            let _ = kind;
            (true, "well-formed IPv6 address")
        }
        Err(_) => (false, "not a valid IPv6 address"),
    }
}

fn ipv6_action(args: &Value) -> Result<String, String> {
    let input = get_input(args)?;
    let (valid, detail) = is_valid_ipv6(&input);
    let mut out = format!("IPv6: {input}\n\n");
    out.push_str(&verdict(valid, detail));
    if valid {
        if let Ok(addr) = input.parse::<std::net::Ipv6Addr>() {
            out.push_str(&format!("  Expanded:  {addr}\n"));
            out.push_str(&format!(
                "  Loopback:  {}\n",
                if addr.is_loopback() { "yes" } else { "no" }
            ));
        }
    }
    Ok(out)
}

// ── CIDR ─────────────────────────────────────────────────────────────────────

fn cidr_action(args: &Value) -> Result<String, String> {
    let input = get_input(args)?;
    let parts: Vec<&str> = input.trim().splitn(2, '/').collect();
    if parts.len() != 2 {
        return Ok(format!(
            "CIDR: {input}\n\nINVALID  missing '/' prefix length\n"
        ));
    }
    let ip_str = parts[0];
    let prefix_len: u8 = match parts[1].parse() {
        Ok(n) => n,
        Err(_) => {
            return Ok(format!(
                "CIDR: {input}\n\nINVALID  prefix length '{}' is not a number\n",
                parts[1]
            ))
        }
    };

    // Try IPv4
    if let Ok(ip) = ip_str.parse::<std::net::Ipv4Addr>() {
        if prefix_len > 32 {
            return Ok(format!(
                "CIDR: {input}\n\nINVALID  prefix length {prefix_len} > 32 for IPv4\n"
            ));
        }
        let ip_u32 = u32::from(ip);
        let mask = if prefix_len == 0 {
            0u32
        } else {
            !0u32 << (32 - prefix_len)
        };
        let network = ip_u32 & mask;
        let broadcast = network | !mask;
        let host_count: u64 = if prefix_len >= 32 {
            1
        } else {
            1u64 << (32 - prefix_len)
        };
        let usable = if prefix_len >= 31 {
            host_count
        } else {
            host_count.saturating_sub(2)
        };

        let mut out = format!("CIDR: {input}\n\nVALID  IPv4 network\n\n");
        out.push_str(&format!("  Network:    {}\n", format_ipv4(network)));
        out.push_str(&format!("  Broadcast:  {}\n", format_ipv4(broadcast)));
        out.push_str(&format!("  Subnet mask:{}\n", format_ipv4(mask)));
        out.push_str(&format!("  First host: {}\n", format_ipv4(network + 1)));
        out.push_str(&format!("  Last host:  {}\n", format_ipv4(broadcast - 1)));
        out.push_str(&format!("  Host count: {} ({usable} usable)\n", host_count));
        return Ok(out);
    }

    // Try IPv6
    if ip_str.parse::<std::net::Ipv6Addr>().is_ok() {
        if prefix_len > 128 {
            return Ok(format!(
                "CIDR: {input}\n\nINVALID  prefix length {prefix_len} > 128 for IPv6\n"
            ));
        }
        return Ok(format!(
            "CIDR: {input}\n\nVALID  IPv6 network (/{prefix_len})\n"
        ));
    }

    Ok(format!(
        "CIDR: {input}\n\nINVALID  '{ip_str}' is not a valid IP address\n"
    ))
}

fn format_ipv4(n: u32) -> String {
    format!(
        "{}.{}.{}.{}",
        (n >> 24) & 0xFF,
        (n >> 16) & 0xFF,
        (n >> 8) & 0xFF,
        n & 0xFF
    )
}

// ── MAC address ───────────────────────────────────────────────────────────────

fn is_valid_mac(s: &str) -> (bool, String) {
    let s = s.trim();
    // Accept : or - separators, or no separator (12 hex chars)
    let sep = if s.contains(':') {
        Some(':')
    } else if s.contains('-') {
        Some('-')
    } else {
        None
    };

    let hex_str: String = match sep {
        Some(c) => {
            let parts: Vec<&str> = s.split(c).collect();
            if parts.len() != 6 {
                return (
                    false,
                    format!("expected 6 groups separated by '{c}', got {}", parts.len()),
                );
            }
            for part in &parts {
                if part.len() != 2 {
                    return (
                        false,
                        format!("each group must be 2 hex digits, got '{part}'"),
                    );
                }
                if !part.chars().all(|c| c.is_ascii_hexdigit()) {
                    return (false, format!("'{part}' is not a valid hex pair"));
                }
            }
            parts.join("")
        }
        None => {
            if s.len() != 12 {
                return (
                    false,
                    format!("without separators, must be 12 hex digits, got {}", s.len()),
                );
            }
            s.to_string()
        }
    };

    if !hex_str.chars().all(|c| c.is_ascii_hexdigit()) {
        return (false, "contains non-hex characters".to_string());
    }

    let first_byte = u8::from_str_radix(&hex_str[..2], 16).unwrap_or(0);
    let is_multicast = first_byte & 0x01 != 0;
    let is_locally_administered = first_byte & 0x02 != 0;
    let detail =
        format!("multicast={is_multicast}; locally-administered={is_locally_administered}");
    (true, detail)
}

fn mac_action(args: &Value) -> Result<String, String> {
    let input = get_input(args)?;
    let (valid, detail) = is_valid_mac(&input);
    let mut out = format!("MAC: {input}\n\n");
    out.push_str(&verdict(valid, &detail));
    if valid {
        // Normalize to XX:XX:XX:XX:XX:XX
        let clean: String = input.chars().filter(|c| c.is_ascii_hexdigit()).collect();
        if clean.len() == 12 {
            let normalized: String = clean
                .as_bytes()
                .chunks(2)
                .map(|b| String::from_utf8_lossy(b).to_uppercase())
                .collect::<Vec<_>>()
                .join(":");
            out.push_str(&format!("  Normalized: {normalized}\n"));
        }
    }
    Ok(out)
}

// ── URL ───────────────────────────────────────────────────────────────────────

fn url_action(args: &Value) -> Result<String, String> {
    let input = get_input(args)?;
    let s = input.trim();

    // Simple structural check — no external deps
    let has_scheme = s.contains("://");
    if !has_scheme {
        let out = format!("URL: {s}\n\nINVALID  missing scheme (e.g. https://)\n");
        return Ok(out);
    }

    let scheme_end = s.find("://").unwrap();
    let scheme = &s[..scheme_end];
    let rest = &s[scheme_end + 3..];

    let allowed_scheme = scheme
        .chars()
        .all(|c| c.is_alphanumeric() || c == '+' || c == '-' || c == '.');
    if !allowed_scheme || scheme.is_empty() {
        let out = format!("URL: {s}\n\nINVALID  scheme '{scheme}' contains invalid characters\n");
        return Ok(out);
    }

    if rest.is_empty() {
        let out = format!("URL: {s}\n\nINVALID  no host after scheme\n");
        return Ok(out);
    }

    // Extract host (up to first / ? #)
    let host_end = rest.find(['/', '?', '#']).unwrap_or(rest.len());
    let host = &rest[..host_end];

    let is_localhost = host == "localhost" || host.starts_with("localhost:");
    let is_https = scheme == "https";
    let is_http = scheme == "http";

    let mut out = format!("URL: {s}\n\nVALID  ");
    if is_http && !is_localhost {
        out.push_str("WARNING: plain HTTP (not encrypted)\n");
    } else {
        out.push_str("well-formed URL\n");
    }
    out.push_str(&format!("  Scheme:  {scheme}\n"));
    out.push_str(&format!("  Host:    {host}\n"));
    out.push_str(&format!(
        "  HTTPS:   {}\n",
        if is_https { "yes" } else { "no" }
    ));
    Ok(out)
}

// ── Credit card / Luhn ───────────────────────────────────────────────────────

fn luhn_check(s: &str) -> bool {
    let digits: Vec<u32> = s
        .chars()
        .filter(|c| c.is_ascii_digit())
        .map(|c| c.to_digit(10).unwrap())
        .collect();
    if digits.len() < 12 {
        return false;
    }
    let sum: u32 = digits
        .iter()
        .rev()
        .enumerate()
        .map(|(i, &d)| {
            if i % 2 == 1 {
                let doubled = d * 2;
                if doubled > 9 {
                    doubled - 9
                } else {
                    doubled
                }
            } else {
                d
            }
        })
        .sum();
    sum.is_multiple_of(10)
}

fn card_network(digits: &str) -> &'static str {
    if digits.starts_with('4') && (digits.len() == 13 || digits.len() == 16) {
        "Visa"
    } else if digits.len() == 16 && {
        let prefix: u32 = digits[..4].parse().unwrap_or(0);
        (5100..=5599).contains(&prefix)
    } {
        "Mastercard"
    } else if digits.starts_with("34") || digits.starts_with("37") {
        "American Express"
    } else if digits.starts_with("6011") || digits.starts_with("65") {
        "Discover"
    } else {
        "Unknown network"
    }
}

fn credit_card_action(args: &Value) -> Result<String, String> {
    let input = get_input(args)?;
    let digits: String = input.chars().filter(|c| c.is_ascii_digit()).collect();
    let valid = luhn_check(&digits);
    let mut out = format!("Credit Card: {input}\n\n");
    if valid {
        let network = card_network(&digits);
        let masked = format!("{}xxxx{}", &digits[..4], &digits[digits.len() - 4..]);
        out.push_str(&verdict(true, &format!("Luhn check passed; {network}")));
        out.push_str(&format!("  Masked:   {masked}\n"));
        out.push_str(&format!("  Length:   {} digits\n", digits.len()));
    } else {
        out.push_str(&verdict(false, "Luhn check failed"));
    }
    Ok(out)
}

// ── ISBN ──────────────────────────────────────────────────────────────────────

fn isbn10_check(s: &str) -> bool {
    let digits: Vec<u32> = s
        .chars()
        .filter(|c| c.is_ascii_digit() || *c == 'X' || *c == 'x')
        .collect::<Vec<char>>()
        .iter()
        .map(|&c| {
            if c == 'X' || c == 'x' {
                10
            } else {
                c.to_digit(10).unwrap()
            }
        })
        .collect();
    if digits.len() != 10 {
        return false;
    }
    let sum: u32 = digits
        .iter()
        .enumerate()
        .map(|(i, &d)| (10 - i as u32) * d)
        .sum();
    sum.is_multiple_of(11)
}

fn isbn13_check(s: &str) -> bool {
    let digits: Vec<u32> = s
        .chars()
        .filter(|c| c.is_ascii_digit())
        .map(|c| c.to_digit(10).unwrap())
        .collect();
    if digits.len() != 13 {
        return false;
    }
    let sum: u32 = digits
        .iter()
        .enumerate()
        .map(|(i, &d)| if i % 2 == 0 { d } else { d * 3 })
        .sum();
    sum.is_multiple_of(10)
}

fn isbn_action(args: &Value) -> Result<String, String> {
    let input = get_input(args)?;
    let stripped: String = input
        .chars()
        .filter(|c| c.is_ascii_digit() || *c == 'X' || *c == 'x')
        .collect();
    let mut out = format!("ISBN: {input}\n\n");
    match stripped.len() {
        10 => {
            let valid = isbn10_check(&stripped);
            out.push_str(&verdict(
                valid,
                if valid {
                    "ISBN-10 check digit valid"
                } else {
                    "ISBN-10 check digit invalid"
                },
            ));
        }
        13 => {
            let valid = isbn13_check(&stripped);
            out.push_str(&verdict(
                valid,
                if valid {
                    "ISBN-13 check digit valid"
                } else {
                    "ISBN-13 check digit invalid"
                },
            ));
        }
        _ => {
            out.push_str(&verdict(
                false,
                &format!("expected 10 or 13 digits, got {}", stripped.len()),
            ));
        }
    }
    Ok(out)
}

// ── UUID ──────────────────────────────────────────────────────────────────────

fn uuid_action(args: &Value) -> Result<String, String> {
    let input = get_input(args)?;
    let s = input.trim();
    let hex_only: String = s.chars().filter(|c| c.is_ascii_hexdigit()).collect();

    let mut out = format!("UUID: {s}\n\n");
    if hex_only.len() != 32 {
        out.push_str(&verdict(false, "must contain exactly 32 hex digits"));
        return Ok(out);
    }

    // Standard format: 8-4-4-4-12
    let parts: Vec<&str> = s.split('-').collect();
    let correct_format = parts.len() == 5
        && parts[0].len() == 8
        && parts[1].len() == 4
        && parts[2].len() == 4
        && parts[3].len() == 4
        && parts[4].len() == 12;

    if !correct_format {
        out.push_str(&verdict(false, "not in standard 8-4-4-4-12 format"));
        return Ok(out);
    }

    // Extract version and variant
    let version_char = parts[2].chars().next().unwrap_or('0');
    let version = version_char.to_digit(16).unwrap_or(0);
    let variant_char = parts[3].chars().next().unwrap_or('0');
    let variant_nibble = u8::from_str_radix(&variant_char.to_string(), 16).unwrap_or(0);
    let variant = if variant_nibble & 0xC == 0xC {
        "Microsoft"
    } else if variant_nibble & 0x8 != 0 {
        "RFC 4122"
    } else {
        "NCS"
    };

    out.push_str(&verdict(
        true,
        &format!("version {version}, {variant} variant"),
    ));
    out.push_str(&format!("  Version: {version}\n"));
    out.push_str(&format!("  Variant: {variant}\n"));
    Ok(out)
}

// ── Phone ─────────────────────────────────────────────────────────────────────

fn phone_action(args: &Value) -> Result<String, String> {
    let input = get_input(args)?;
    let digits: String = input.chars().filter(|c| c.is_ascii_digit()).collect();
    let s = input.trim();

    let mut out = format!("Phone: {s}\n\n");
    let has_plus = s.starts_with('+');

    match digits.len() {
        10 if !has_plus => {
            // US/CA NANP
            let area = &digits[..3];
            let exchange = &digits[3..6];
            let subscriber = &digits[6..];
            let valid = area
                .chars()
                .next()
                .is_some_and(|c| ('2'..='9').contains(&c))
                && exchange
                    .chars()
                    .next()
                    .is_some_and(|c| ('2'..='9').contains(&c));
            out.push_str(&verdict(
                valid,
                if valid {
                    "US/CA NANP format"
                } else {
                    "invalid NANP area or exchange code"
                },
            ));
            if valid {
                out.push_str(&format!("  Formatted: ({area}) {exchange}-{subscriber}\n"));
                out.push_str(&format!("  E.164:     +1{digits}\n"));
            }
        }
        11 if digits.starts_with('1') => {
            let rest = &digits[1..];
            let area = &rest[..3];
            let exchange = &rest[3..6];
            let subscriber = &rest[6..];
            let valid = area
                .chars()
                .next()
                .is_some_and(|c| ('2'..='9').contains(&c));
            out.push_str(&verdict(
                valid,
                if valid {
                    "US/CA with country code 1"
                } else {
                    "invalid area code"
                },
            ));
            if valid {
                out.push_str(&format!(
                    "  Formatted: +1 ({area}) {exchange}-{subscriber}\n"
                ));
            }
        }
        7..=15 if has_plus => {
            out.push_str(&verdict(
                true,
                "E.164 international format (structural check only)",
            ));
            out.push_str(&format!("  Digits:  {digits}\n"));
            out.push_str("  Note: country-code validation not performed\n");
        }
        _ => {
            out.push_str(&verdict(
                false,
                &format!(
                    "{} digits — expected 10 (US/CA) or 7–15 with '+' (international)",
                    digits.len()
                ),
            ));
        }
    }
    Ok(out)
}

// ── SemVer ────────────────────────────────────────────────────────────────────

fn semver_action(args: &Value) -> Result<String, String> {
    let input = get_input(args)?;
    let s = input.trim().trim_start_matches('v');
    let mut out = format!("SemVer: {s}\n\n");

    // Split on '-' for pre-release and '+' for build metadata
    let (core, pre_and_build) = if let Some(idx) = s.find('-') {
        (&s[..idx], &s[idx + 1..])
    } else if let Some(idx) = s.find('+') {
        (&s[..idx], &s[idx..])
    } else {
        (s, "")
    };

    let parts: Vec<&str> = core.split('.').collect();
    if parts.len() != 3 {
        out.push_str(&verdict(
            false,
            "core version must have exactly 3 dot-separated components (MAJOR.MINOR.PATCH)",
        ));
        return Ok(out);
    }

    let mut nums = Vec::new();
    for (i, part) in parts.iter().enumerate() {
        match part.parse::<u64>() {
            Ok(n) => nums.push(n),
            Err(_) => {
                let name = ["MAJOR", "MINOR", "PATCH"][i];
                out.push_str(&verdict(
                    false,
                    &format!("{name} '{part}' is not a non-negative integer"),
                ));
                return Ok(out);
            }
        }
    }

    let (pre, build) = if let Some(build_rest) = pre_and_build.strip_prefix('+') {
        ("", build_rest)
    } else if let Some(plus_idx) = pre_and_build.find('+') {
        (&pre_and_build[..plus_idx], &pre_and_build[plus_idx + 1..])
    } else {
        (pre_and_build, "")
    };

    let stable = pre.is_empty();
    out.push_str(&verdict(
        true,
        if stable {
            "stable release"
        } else {
            "pre-release"
        },
    ));
    out.push_str(&format!("  Major:     {}\n", nums[0]));
    out.push_str(&format!("  Minor:     {}\n", nums[1]));
    out.push_str(&format!("  Patch:     {}\n", nums[2]));
    if !pre.is_empty() {
        out.push_str(&format!("  Pre-release: {pre}\n"));
    }
    if !build.is_empty() {
        out.push_str(&format!("  Build meta:  {build}\n"));
    }
    Ok(out)
}

// ── Hex color ─────────────────────────────────────────────────────────────────

fn hex_color_action(args: &Value) -> Result<String, String> {
    let input = get_input(args)?;
    let s = input.trim().trim_start_matches('#');
    let mut out = format!("Hex Color: #{s}\n\n");

    let (r, g, b, a) = match s.len() {
        3 => {
            let chars: Vec<char> = s.chars().collect();
            let expand = |c: char| u8::from_str_radix(&format!("{c}{c}"), 16).unwrap_or(0);
            (expand(chars[0]), expand(chars[1]), expand(chars[2]), 255u8)
        }
        4 => {
            let chars: Vec<char> = s.chars().collect();
            let expand = |c: char| u8::from_str_radix(&format!("{c}{c}"), 16).unwrap_or(0);
            (
                expand(chars[0]),
                expand(chars[1]),
                expand(chars[2]),
                expand(chars[3]),
            )
        }
        6 => {
            let r =
                u8::from_str_radix(&s[0..2], 16).map_err(|_| "size_tools: invalid hex in color")?;
            let g =
                u8::from_str_radix(&s[2..4], 16).map_err(|_| "size_tools: invalid hex in color")?;
            let b =
                u8::from_str_radix(&s[4..6], 16).map_err(|_| "size_tools: invalid hex in color")?;
            (r, g, b, 255u8)
        }
        8 => {
            let r =
                u8::from_str_radix(&s[0..2], 16).map_err(|_| "size_tools: invalid hex in color")?;
            let g =
                u8::from_str_radix(&s[2..4], 16).map_err(|_| "size_tools: invalid hex in color")?;
            let b =
                u8::from_str_radix(&s[4..6], 16).map_err(|_| "size_tools: invalid hex in color")?;
            let a =
                u8::from_str_radix(&s[6..8], 16).map_err(|_| "size_tools: invalid hex in color")?;
            (r, g, b, a)
        }
        _ => {
            out.push_str(&verdict(
                false,
                "must be 3, 4, 6, or 8 hex digits (with optional leading #)",
            ));
            return Ok(out);
        }
    };

    if !s.chars().all(|c| c.is_ascii_hexdigit()) {
        out.push_str(&verdict(false, "contains non-hex characters"));
        return Ok(out);
    }

    let luminance = 0.299 * r as f64 + 0.587 * g as f64 + 0.114 * b as f64;
    let brightness = if luminance > 127.5 { "light" } else { "dark" };

    out.push_str(&verdict(true, "valid hex color"));
    out.push_str(&format!("  RGB:        rgb({r}, {g}, {b})\n"));
    if a != 255 {
        out.push_str(&format!(
            "  Alpha:      {} ({:.1}%)\n",
            a,
            a as f64 / 255.0 * 100.0
        ));
    }
    out.push_str(&format!("  Normalized: #{:02X}{:02X}{:02X}\n", r, g, b));
    out.push_str(&format!(
        "  Brightness: {brightness} (luminance {luminance:.0})\n"
    ));
    Ok(out)
}

// ── Auto-detect ───────────────────────────────────────────────────────────────

fn auto_action(args: &Value) -> Result<String, String> {
    let input = get_input(args)?;
    let s = input.trim();

    // Try to detect the type automatically
    if s.contains('@') {
        return email_action(args);
    }
    if s.starts_with('#')
        || (s.len() == 6 || s.len() == 3) && s.chars().all(|c| c.is_ascii_hexdigit())
    {
        return hex_color_action(args);
    }
    if s.contains('/') && (s.contains('.') || s.contains(':')) {
        return cidr_action(args);
    }
    if s.starts_with("http://") || s.starts_with("https://") || s.contains("://") {
        return url_action(args);
    }
    if s.contains(':') && s.split(':').count() >= 6 {
        // Could be IPv6 or MAC
        if s.split(':').count() == 6 && s.split(':').all(|p| p.len() <= 2) {
            return mac_action(args);
        }
        return ipv6_action(args);
    }
    if s.contains('.') && s.split('.').count() == 4 {
        return ipv4_action(args);
    }
    if s.contains('-') && s.split('-').count() == 5 {
        return uuid_action(args);
    }
    if s.starts_with('+') || s.chars().filter(|c| c.is_ascii_digit()).count() >= 10 {
        return phone_action(args);
    }
    // SemVer heuristic
    let v = s.trim_start_matches('v');
    if v.split('.').count() == 3 {
        return semver_action(args);
    }
    // ISBN
    let digits_only: String = s
        .chars()
        .filter(|c| c.is_ascii_digit() || *c == 'X')
        .collect();
    if digits_only.len() == 10 || digits_only.len() == 13 {
        return isbn_action(args);
    }

    Err(format!(
        "validate_tools auto: could not detect type for '{s}'. \
         Specify action: email, ipv4, ipv6, cidr, mac, url, credit_card, isbn, uuid, phone, semver, hex_color"
    ))
}
