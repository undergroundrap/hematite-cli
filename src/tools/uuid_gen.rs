use rand::Rng;

pub async fn execute(args: &serde_json::Value) -> Result<String, String> {
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("generate");

    match action {
        "generate" => generate(args),
        "validate" => validate(args),
        "nil" => nil(),
        "bulk" => bulk(args),
        other => Err(format!(
            "uuid_gen: unknown action '{other}'. Valid: generate, validate, nil, bulk"
        )),
    }
}

// ── Core ──────────────────────────────────────────────────────────────────────

fn gen_v4() -> String {
    let mut rng = rand::thread_rng();
    let mut bytes = [0u8; 16];
    rng.fill(&mut bytes);
    // Set version 4
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    // Set variant bits (10xx)
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    format!(
        "{:08x}-{:04x}-{:04x}-{:04x}-{:012x}",
        u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
        u16::from_be_bytes([bytes[4], bytes[5]]),
        u16::from_be_bytes([bytes[6], bytes[7]]),
        u16::from_be_bytes([bytes[8], bytes[9]]),
        {
            let mut tail = 0u64;
            for b in &bytes[10..16] {
                tail = (tail << 8) | (*b as u64);
            }
            tail
        }
    )
}

fn parse_uuid(s: &str) -> Option<[u8; 16]> {
    let clean: String = s.chars().filter(|c| *c != '-').collect();
    if clean.len() != 32 {
        return None;
    }
    let mut bytes = [0u8; 16];
    for (i, chunk) in clean.as_bytes().chunks(2).enumerate() {
        let hex = std::str::from_utf8(chunk).ok()?;
        bytes[i] = u8::from_str_radix(hex, 16).ok()?;
    }
    Some(bytes)
}

// ── Actions ───────────────────────────────────────────────────────────────────

fn generate(args: &serde_json::Value) -> Result<String, String> {
    let upper = args.get("upper").and_then(|v| v.as_bool()).unwrap_or(false);
    let uuid = gen_v4();
    let uuid = if upper { uuid.to_uppercase() } else { uuid };

    let mut out = format!("UUID GENERATE\n{}\n", "─".repeat(50));
    out.push_str(&format!("UUID v4 : {uuid}\n"));
    out.push_str("Version : 4 (random)\n");
    out.push_str("Variant : RFC 4122\n");
    Ok(out)
}

fn validate(args: &serde_json::Value) -> Result<String, String> {
    let input = args
        .get("input")
        .and_then(|v| v.as_str())
        .ok_or("uuid_gen validate: 'input' is required")?;

    let trimmed = input.trim();
    let mut out = format!("UUID VALIDATE\n{}\n", "─".repeat(50));
    out.push_str(&format!("Input : {trimmed}\n"));

    match parse_uuid(trimmed) {
        None => {
            out.push_str("Valid   : NO — not a valid UUID format\n");
        }
        Some(bytes) => {
            let version = (bytes[6] >> 4) & 0x0f;
            let variant_byte = bytes[8] >> 4;
            let variant = if variant_byte & 0b1100 == 0b1000 {
                "RFC 4122"
            } else if variant_byte & 0b1110 == 0b1100 {
                "Microsoft"
            } else if variant_byte & 0b1000 == 0 {
                "NCS"
            } else {
                "Reserved"
            };
            let is_nil = bytes.iter().all(|b| *b == 0);
            out.push_str("Valid   : YES\n");
            if is_nil {
                out.push_str("Type    : Nil UUID (all zeros)\n");
            } else {
                out.push_str(&format!("Version : {version}\n"));
                out.push_str(&format!("Variant : {variant}\n"));
            }
        }
    }
    Ok(out)
}

fn nil() -> Result<String, String> {
    Ok(format!(
        "UUID NIL\n{}\n00000000-0000-0000-0000-000000000000\n",
        "─".repeat(50)
    ))
}

fn bulk(args: &serde_json::Value) -> Result<String, String> {
    let n = args.get("n").and_then(|v| v.as_u64()).unwrap_or(5).min(100) as usize;

    let upper = args.get("upper").and_then(|v| v.as_bool()).unwrap_or(false);

    let mut out = format!("UUID BULK ({n})\n{}\n", "─".repeat(50));
    for _ in 0..n {
        let uuid = gen_v4();
        let uuid = if upper { uuid.to_uppercase() } else { uuid };
        out.push_str(&format!("{uuid}\n"));
    }
    Ok(out)
}
