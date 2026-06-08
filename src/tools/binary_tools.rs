use serde_json::Value;

pub async fn execute(args: &Value) -> Result<String, String> {
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("info");
    match action {
        "info" => action_info(args),
        "flags" => action_flags(args),
        "pack" => action_pack(args),
        "unpack" => action_unpack(args),
        "ops" => action_ops(args),
        other => Err(format!(
            "binary_tools: unknown action '{other}'. Valid: info, flags, pack, unpack, ops"
        )),
    }
}

fn get_number(args: &Value) -> Result<u64, String> {
    if let Some(v) = args
        .get("value")
        .or_else(|| args.get("n").or_else(|| args.get("input")))
    {
        if let Some(s) = v.as_str() {
            let s = s.trim();
            if s.starts_with("0x") || s.starts_with("0X") {
                return u64::from_str_radix(&s[2..], 16)
                    .map_err(|_| format!("Cannot parse hex number: {s}"));
            } else if s.starts_with("0b") || s.starts_with("0B") {
                return u64::from_str_radix(&s[2..], 2)
                    .map_err(|_| format!("Cannot parse binary number: {s}"));
            } else if s.starts_with("0o") || s.starts_with("0O") {
                return u64::from_str_radix(&s[2..], 8)
                    .map_err(|_| format!("Cannot parse octal number: {s}"));
            } else {
                return s
                    .parse::<u64>()
                    .map_err(|_| format!("Cannot parse number: {s}"));
            }
        }
        if let Some(n) = v.as_u64() {
            return Ok(n);
        }
        if let Some(n) = v.as_i64() {
            return Ok(n as u64);
        }
        if let Some(f) = v.as_f64() {
            return Ok(f as u64);
        }
    }
    Err("binary_tools: 'value' (integer or 0x/0b/0o string) is required".to_string())
}

fn get_width(args: &Value) -> u32 {
    args.get("width")
        .and_then(|v| v.as_u64())
        .map(|n| n.clamp(1, 64) as u32)
        .unwrap_or(0)
}

fn auto_width(n: u64) -> u32 {
    if n == 0 {
        8
    } else {
        let bits = 64 - n.leading_zeros();
        // round up to nearest byte boundary
        bits.div_ceil(8) * 8
    }
}

fn bin_str(n: u64, bits: u32) -> String {
    (0..bits)
        .rev()
        .map(|i| if (n >> i) & 1 == 1 { '1' } else { '0' })
        .collect()
}

fn nibble_str(n: u64, bits: u32) -> String {
    let s = bin_str(n, bits);
    s.as_bytes()
        .chunks(4)
        .map(|c| std::str::from_utf8(c).unwrap())
        .collect::<Vec<_>>()
        .join("_")
}

// ── action: info ─────────────────────────────────────────────────────────────

fn action_info(args: &Value) -> Result<String, String> {
    let n = get_number(args)?;
    let w = {
        let req = get_width(args);
        if req == 0 {
            auto_width(n)
        } else {
            req
        }
    };

    let popcount = n.count_ones();
    let trailing = n.trailing_zeros();
    let leading = if w <= 64 {
        w - (64 - n.leading_zeros()).min(w)
    } else {
        0
    };
    let parity = if popcount % 2 == 0 { "even" } else { "odd" };
    let gray = n ^ (n >> 1);

    let mut out = "binary_tools — info\n\n".to_string();
    out.push_str(&format!("  Decimal  : {}\n", n));
    out.push_str(&format!("  Hex      : 0x{:X} (0x{:x})\n", n, n));
    out.push_str(&format!("  Octal    : 0o{:o}\n", n));
    out.push_str(&format!("  Binary   : {}\n", nibble_str(n, w)));
    out.push_str(&format!("  Width    : {} bits ({} bytes)\n", w, w / 8));
    out.push('\n');
    out.push_str(&format!("  Popcount : {} bit(s) set\n", popcount));
    out.push_str(&format!("  Parity   : {}\n", parity));
    out.push_str(&format!("  Leading  : {} zero bit(s)\n", leading));
    out.push_str(&format!("  Trailing : {} zero bit(s)\n", trailing));
    out.push_str(&format!("  MSB      : bit {}\n", 63 - n.leading_zeros()));
    out.push_str("  LSB      : bit 0\n");
    out.push('\n');
    out.push_str(&format!("  Gray code: {} (0x{:X})\n", gray, gray));
    out.push_str(&format!("  NOT      : 0x{:X}\n", !n & ((1u64 << w) - 1)));

    // IEEE 754 float view for 32-bit values
    if w <= 32 {
        let f = f32::from_bits(n as u32);
        if f.is_finite() {
            out.push_str(&format!("  As f32   : {}\n", f));
        }
    }
    // IEEE 754 float view for 64-bit values
    if w == 64 {
        let f = f64::from_bits(n);
        if f.is_finite() {
            out.push_str(&format!("  As f64   : {}\n", f));
        }
    }

    Ok(out)
}

// ── action: flags ─────────────────────────────────────────────────────────────

fn action_flags(args: &Value) -> Result<String, String> {
    let n = get_number(args)?;
    let w = {
        let req = get_width(args);
        if req == 0 {
            auto_width(n)
        } else {
            req
        }
    };

    // Optional: named flags
    let names: Vec<String> = args
        .get("names")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .map(|v| v.as_str().unwrap_or("").to_string())
                .collect()
        })
        .unwrap_or_default();

    let mut out = "binary_tools — flags\n\n".to_string();
    out.push_str(&format!("  Value: {} (0x{:X})\n\n", n, n));
    out.push_str(&format!(
        "  {:>4}  {:>12}  {}\n",
        "Bit", "Hex mask", "State"
    ));
    out.push_str(&format!("  {:-<4}  {:-<12}  {:-<20}\n", "", "", ""));

    for i in (0..w).rev() {
        let set = (n >> i) & 1 == 1;
        let mask = 1u64 << i;
        let name = names
            .get((w - 1 - i) as usize)
            .map(|s| s.as_str())
            .unwrap_or("");
        let state = if set { "SET  ✓" } else { "clear" };
        if name.is_empty() {
            out.push_str(&format!("  {:>4}  0x{:0>10X}  {}\n", i, mask, state));
        } else {
            out.push_str(&format!(
                "  {:>4}  0x{:0>10X}  {}  ({})\n",
                i, mask, state, name
            ));
        }
    }

    let set_bits: Vec<u32> = (0..w).filter(|&i| (n >> i) & 1 == 1).rev().collect();
    out.push_str(&format!(
        "\n  Set bits: {:?}  ({} of {} set)\n",
        set_bits,
        set_bits.len(),
        w
    ));

    Ok(out)
}

// ── action: pack ─────────────────────────────────────────────────────────────

fn action_pack(args: &Value) -> Result<String, String> {
    // fields: array of {value, bits} objects
    let fields = args
        .get("fields")
        .and_then(|v| v.as_array())
        .ok_or("binary_tools pack: 'fields' array is required. Each entry: {value, bits}")?;

    let mut result: u64 = 0;
    let mut total_bits: u32 = 0;
    let mut out = "binary_tools — pack\n\n".to_string();
    out.push_str(&format!(
        "  {:<20}  {:>6}  {:>10}  {}\n",
        "Field", "Bits", "Value", "Binary"
    ));
    out.push_str(&format!(
        "  {:-<20}  {:-<6}  {:-<10}  {:-<20}\n",
        "", "", "", ""
    ));

    for (i, field) in fields.iter().enumerate() {
        let bits = field
            .get("bits")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| format!("Field {i}: missing 'bits'"))? as u32;
        let val = field
            .get("value")
            .or_else(|| field.get("v"))
            .and_then(|v| v.as_u64())
            .ok_or_else(|| format!("Field {i}: missing 'value'"))?;
        let default_name = format!("field{i}");
        let name = field
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or(&default_name);
        let mask = if bits >= 64 {
            u64::MAX
        } else {
            (1u64 << bits) - 1
        };
        let masked = val & mask;
        result = (result << bits) | masked;
        total_bits += bits;
        out.push_str(&format!(
            "  {:<20}  {:>6}  {:>10}  {}\n",
            name,
            bits,
            masked,
            bin_str(masked, bits)
        ));
    }

    out.push_str(&format!("\n  Total bits : {}\n", total_bits));
    out.push_str(&format!("  Packed u64 : {}\n", result));
    out.push_str(&format!("  Hex        : 0x{:X}\n", result));
    out.push_str(&format!(
        "  Binary     : {}\n",
        nibble_str(result, total_bits)
    ));

    Ok(out)
}

// ── action: unpack ────────────────────────────────────────────────────────────

fn action_unpack(args: &Value) -> Result<String, String> {
    let n = get_number(args)?;

    // layout: array of {bits, name?} objects (MSB first)
    let layout = args
        .get("layout")
        .and_then(|v| v.as_array())
        .ok_or("binary_tools unpack: 'layout' array is required. Each entry: {bits[, name]}")?;

    let total_bits: u32 = layout
        .iter()
        .map(|f| f.get("bits").and_then(|v| v.as_u64()).unwrap_or(0) as u32)
        .sum();

    let mut out = "binary_tools — unpack\n\n".to_string();
    out.push_str(&format!("  Input : {} (0x{:X})\n", n, n));
    out.push_str(&format!("  Layout: {} bits\n\n", total_bits));
    out.push_str(&format!(
        "  {:<20}  {:>6}  {:>12}  {:>10}  {}\n",
        "Field", "Bits", "Hex", "Decimal", "Binary"
    ));
    out.push_str(&format!(
        "  {:-<20}  {:-<6}  {:-<12}  {:-<10}  {:-<20}\n",
        "", "", "", "", ""
    ));

    let mut shift = total_bits;
    for (i, field) in layout.iter().enumerate() {
        let bits = field
            .get("bits")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| format!("Layout entry {i}: missing 'bits'"))? as u32;
        let default_name = format!("field{i}");
        let name = field
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or(&default_name);
        shift -= bits;
        let mask = if bits >= 64 {
            u64::MAX
        } else {
            (1u64 << bits) - 1
        };
        let val = (n >> shift) & mask;
        out.push_str(&format!(
            "  {:<20}  {:>6}  0x{:<10X}  {:>10}  {}\n",
            name,
            bits,
            val,
            val,
            bin_str(val, bits)
        ));
    }

    Ok(out)
}

// ── action: ops ───────────────────────────────────────────────────────────────

fn action_ops(args: &Value) -> Result<String, String> {
    let a = get_number(args)?;
    let b = args.get("b").and_then(|v| {
        v.as_u64().or_else(|| {
            v.as_str().and_then(|s| {
                let s = s.trim();
                if s.starts_with("0x") || s.starts_with("0X") {
                    u64::from_str_radix(&s[2..], 16).ok()
                } else if s.starts_with("0b") || s.starts_with("0B") {
                    u64::from_str_radix(&s[2..], 2).ok()
                } else {
                    s.parse::<u64>().ok()
                }
            })
        })
    });

    let shift = args
        .get("shift")
        .and_then(|v| v.as_u64())
        .map(|n| n as u32)
        .unwrap_or(1);

    let w = {
        let req = get_width(args);
        if req == 0 {
            auto_width(a)
        } else {
            req
        }
    };
    let mask_w = if w >= 64 { u64::MAX } else { (1u64 << w) - 1 };

    let mut out = "binary_tools — ops\n\n".to_string();
    out.push_str(&format!("  A = {} (0x{:X})\n", a, a));
    if let Some(b_val) = b {
        out.push_str(&format!("  B = {} (0x{:X})\n", b_val, b_val));
    }
    out.push_str(&format!("  Width = {} bits, Shift = {}\n\n", w, shift));

    out.push_str("  Unary on A:\n");
    out.push_str(&format!(
        "    NOT A           : 0x{:X}  ({})\n",
        !a & mask_w,
        !a & mask_w
    ));
    out.push_str(&format!(
        "    A << {}          : 0x{:X}  ({})\n",
        shift,
        (a << shift) & mask_w,
        (a << shift) & mask_w
    ));
    out.push_str(&format!(
        "    A >> {}          : 0x{:X}  ({})\n",
        shift,
        a >> shift,
        a >> shift
    ));
    out.push_str(&format!(
        "    A >>> {} (rot_l) : 0x{:X}  ({})\n",
        shift,
        a.rotate_left(shift % 64),
        a.rotate_left(shift % 64)
    ));
    out.push_str(&format!(
        "    A <<< {} (rot_r) : 0x{:X}  ({})\n",
        shift,
        a.rotate_right(shift % 64),
        a.rotate_right(shift % 64)
    ));
    out.push_str(&format!(
        "    POPCOUNT(A)     : {} bit(s)\n",
        a.count_ones()
    ));
    out.push_str(&format!(
        "    PARITY(A)       : {} (bit {})\n",
        a.count_ones() % 2,
        if a.count_ones() % 2 == 0 {
            "even"
        } else {
            "odd"
        }
    ));
    out.push_str(&format!("    GRAY(A)         : 0x{:X}\n", a ^ (a >> 1)));

    if let Some(b_val) = b {
        out.push_str("\n  Binary on A, B:\n");
        out.push_str(&format!(
            "    A AND B         : 0x{:X}  ({})\n",
            a & b_val,
            a & b_val
        ));
        out.push_str(&format!(
            "    A OR  B         : 0x{:X}  ({})\n",
            a | b_val,
            a | b_val
        ));
        out.push_str(&format!(
            "    A XOR B         : 0x{:X}  ({})\n",
            a ^ b_val,
            a ^ b_val
        ));
        out.push_str(&format!(
            "    A NAND B        : 0x{:X}  ({})\n",
            !(a & b_val) & mask_w,
            !(a & b_val) & mask_w
        ));
        out.push_str(&format!(
            "    A NOR  B        : 0x{:X}  ({})\n",
            !(a | b_val) & mask_w,
            !(a | b_val) & mask_w
        ));
        out.push_str(&format!(
            "    A XNOR B        : 0x{:X}  ({})\n",
            !(a ^ b_val) & mask_w,
            !(a ^ b_val) & mask_w
        ));
        // test / set / clear / toggle bits
        out.push_str(&format!(
            "    A & B mask      : 0x{:X}  (bits in A that match B)\n",
            a & b_val
        ));
        out.push_str(&format!(
            "    A | B set       : 0x{:X}  (A with B bits set)\n",
            a | b_val
        ));
        out.push_str(&format!(
            "    A &~B clear     : 0x{:X}  (A with B bits cleared)\n",
            a & !b_val
        ));
        out.push_str(&format!(
            "    A ^ B toggle    : 0x{:X}  (A with B bits toggled)\n",
            a ^ b_val
        ));
    }

    Ok(out)
}
