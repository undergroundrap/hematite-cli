use serde_json::Value;

pub async fn execute(args: &Value) -> Result<String, String> {
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("convert");
    match action {
        "convert" => convert_action(args),
        "parse" => parse_action(args),
        "format" => format_action(args),
        "compare" => compare_action(args),
        "bandwidth" => bandwidth_action(args),
        other => Err(format!(
            "size_tools: unknown action '{other}'. Valid: convert, parse, format, compare, bandwidth"
        )),
    }
}

// Parse a size string like "1.5 GB", "2048MB", "512 KiB", "1073741824", "4k"
fn parse_bytes(s: &str) -> Result<f64, String> {
    let s = s.trim();
    // Find split point between number and unit
    let split = s.find(|c: char| c.is_alphabetic()).unwrap_or(s.len());
    let num_str = s[..split].trim();
    let unit_str = s[split..].trim().to_lowercase();

    let num: f64 = num_str
        .parse()
        .map_err(|_| format!("size_tools: cannot parse number from '{s}'"))?;

    let multiplier: f64 = match unit_str.as_str() {
        "" | "b" | "byte" | "bytes" => 1.0,
        "k" | "kb" | "kbyte" | "kbytes" => 1_000.0,
        "m" | "mb" | "mbyte" | "mbytes" => 1_000_000.0,
        "g" | "gb" | "gbyte" | "gbytes" => 1_000_000_000.0,
        "t" | "tb" | "tbyte" | "tbytes" => 1_000_000_000_000.0,
        "p" | "pb" | "pbyte" | "pbytes" => 1_000_000_000_000_000.0,
        "e" | "eb" | "ebyte" | "ebytes" => 1_000_000_000_000_000_000.0,
        // IEC binary prefixes
        "ki" | "kib" | "kibibyte" | "kibibytes" => 1_024.0,
        "mi" | "mib" | "mebibyte" | "mebibytes" => 1_048_576.0,
        "gi" | "gib" | "gibibyte" | "gibibytes" => 1_073_741_824.0,
        "ti" | "tib" | "tebibyte" | "tebibytes" => 1_099_511_627_776.0,
        "pi" | "pib" | "pebibyte" | "pebibytes" => 1_125_899_906_842_624.0,
        "ei" | "eib" | "exbibyte" | "exbibytes" => 1_152_921_504_606_846_976.0,
        other => {
            return Err(format!(
                "size_tools: unknown unit '{other}'. Use B, KB, MB, GB, TB, PB, KiB, MiB, GiB, TiB"
            ))
        }
    };

    Ok(num * multiplier)
}

fn get_input(args: &Value) -> Result<String, String> {
    args.get("size")
        .or_else(|| args.get("input"))
        .or_else(|| args.get("value"))
        .and_then(|v| {
            v.as_str()
                .map(|s| s.to_string())
                .or_else(|| v.as_f64().map(|n| n.to_string()))
                .or_else(|| v.as_u64().map(|n| n.to_string()))
                .or_else(|| v.as_i64().map(|n| n.to_string()))
        })
        .ok_or("size_tools: 'size'/'input'/'value' required".to_string())
}

fn fmt_bytes_decimal(bytes: f64) -> String {
    let units = [
        (1e18, "EB"),
        (1e15, "PB"),
        (1e12, "TB"),
        (1e9, "GB"),
        (1e6, "MB"),
        (1e3, "KB"),
        (1.0, "B"),
    ];
    for (factor, unit) in &units {
        if bytes.abs() >= *factor {
            let v = bytes / factor;
            return if v.fract() < 0.005 {
                format!("{:.0} {unit}", v)
            } else {
                format!("{:.2} {unit}", v)
            };
        }
    }
    format!("{bytes} B")
}

fn fmt_bytes_binary(bytes: f64) -> String {
    let units = [
        (1_152_921_504_606_846_976.0f64, "EiB"),
        (1_125_899_906_842_624.0, "PiB"),
        (1_099_511_627_776.0, "TiB"),
        (1_073_741_824.0, "GiB"),
        (1_048_576.0, "MiB"),
        (1_024.0, "KiB"),
        (1.0, "B"),
    ];
    for (factor, unit) in &units {
        if bytes.abs() >= *factor {
            let v = bytes / factor;
            return if v.fract() < 0.005 {
                format!("{:.0} {unit}", v)
            } else {
                format!("{:.2} {unit}", v)
            };
        }
    }
    format!("{bytes} B")
}

fn add_commas(n: u64) -> String {
    let s = n.to_string();
    let mut result = String::new();
    for (i, ch) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.push(',');
        }
        result.push(ch);
    }
    result.chars().rev().collect()
}

fn convert_action(args: &Value) -> Result<String, String> {
    let input = get_input(args)?;
    let bytes = parse_bytes(&input)?;
    let bytes_u = bytes.round() as u64;

    let to = args.get("to").and_then(|v| v.as_str());

    if let Some(unit) = to {
        let unit_lower = unit.to_lowercase();
        let (factor, label) = match unit_lower.as_str() {
            "b" | "bytes" => (1.0, "B"),
            "kb" => (1_000.0, "KB"),
            "mb" => (1_000_000.0, "MB"),
            "gb" => (1_000_000_000.0, "GB"),
            "tb" => (1_000_000_000_000.0, "TB"),
            "pb" => (1_000_000_000_000_000.0, "PB"),
            "kib" => (1_024.0, "KiB"),
            "mib" => (1_048_576.0, "MiB"),
            "gib" => (1_073_741_824.0, "GiB"),
            "tib" => (1_099_511_627_776.0, "TiB"),
            "pib" => (1_125_899_906_842_624.0, "PiB"),
            other => {
                return Err(format!(
                "size_tools: unknown target unit '{other}'. Use KB, MB, GB, TB, KiB, MiB, GiB, TiB"
            ))
            }
        };
        let result = bytes / factor;
        return Ok(format!("{result:.4} {label}\n"));
    }

    // Show all conversions
    let mut out = format!("Size: {input}\n\n");
    out.push_str(&format!(
        "  Bytes (raw):      {} bytes\n",
        add_commas(bytes_u)
    ));
    out.push_str("\n  Decimal (SI):\n");
    out.push_str(&format!("    Kilobytes:      {:.4} KB\n", bytes / 1e3));
    out.push_str(&format!("    Megabytes:      {:.4} MB\n", bytes / 1e6));
    out.push_str(&format!("    Gigabytes:      {:.6} GB\n", bytes / 1e9));
    out.push_str(&format!("    Terabytes:      {:.9} TB\n", bytes / 1e12));
    out.push_str("\n  Binary (IEC):\n");
    out.push_str(&format!("    Kibibytes:      {:.4} KiB\n", bytes / 1_024.0));
    out.push_str(&format!(
        "    Mebibytes:      {:.4} MiB\n",
        bytes / 1_048_576.0
    ));
    out.push_str(&format!(
        "    Gibibytes:      {:.6} GiB\n",
        bytes / 1_073_741_824.0
    ));
    out.push_str(&format!(
        "    Tebibytes:      {:.9} TiB\n",
        bytes / 1_099_511_627_776.0
    ));
    out.push_str(&format!(
        "\n  Human-readable:   {} (SI)  /  {} (IEC)\n",
        fmt_bytes_decimal(bytes),
        fmt_bytes_binary(bytes)
    ));
    Ok(out)
}

fn parse_action(args: &Value) -> Result<String, String> {
    let input = get_input(args)?;
    let bytes = parse_bytes(&input)?;
    let bytes_u = bytes.round() as u64;
    let mut out = format!("Parsed: {input}\n\n");
    out.push_str(&format!("  Exact bytes:  {}\n", add_commas(bytes_u)));
    out.push_str(&format!("  Decimal:      {}\n", fmt_bytes_decimal(bytes)));
    out.push_str(&format!("  Binary:       {}\n", fmt_bytes_binary(bytes)));
    Ok(out)
}

fn format_action(args: &Value) -> Result<String, String> {
    let input = get_input(args)?;
    let bytes = parse_bytes(&input)?;
    let style = args.get("style").and_then(|v| v.as_str()).unwrap_or("auto");
    let result = match style {
        "decimal" | "si" => fmt_bytes_decimal(bytes),
        "binary" | "iec" => fmt_bytes_binary(bytes),
        _ => {
            // auto: use binary if input was likely binary (power of 2 boundary), else decimal
            if bytes >= 1024.0 && bytes.log2().fract() < 0.001 {
                fmt_bytes_binary(bytes)
            } else {
                fmt_bytes_decimal(bytes)
            }
        }
    };
    Ok(format!("{result}\n"))
}

fn compare_action(args: &Value) -> Result<String, String> {
    let a_str = args
        .get("a")
        .and_then(|v| {
            v.as_str()
                .map(|s| s.to_string())
                .or_else(|| v.as_f64().map(|n| n.to_string()))
        })
        .ok_or("size_tools compare: 'a' required")?;
    let b_str = args
        .get("b")
        .and_then(|v| {
            v.as_str()
                .map(|s| s.to_string())
                .or_else(|| v.as_f64().map(|n| n.to_string()))
        })
        .ok_or("size_tools compare: 'b' required")?;

    let a = parse_bytes(&a_str)?;
    let b = parse_bytes(&b_str)?;

    let cmp = if (a - b).abs() < 1.0 {
        "A == B"
    } else if a > b {
        "A > B"
    } else {
        "A < B"
    };

    let diff = (a - b).abs();
    let ratio = if b != 0.0 { a / b } else { f64::INFINITY };

    let mut out = "Size Comparison\n\n".to_string();
    out.push_str(&format!("  A: {a_str:<20}  =  {}\n", fmt_bytes_decimal(a)));
    out.push_str(&format!("  B: {b_str:<20}  =  {}\n", fmt_bytes_decimal(b)));
    out.push_str(&format!("\n  {cmp}\n"));
    out.push_str(&format!("  Difference: {}\n", fmt_bytes_decimal(diff)));
    if b != 0.0 && !ratio.is_infinite() {
        out.push_str(&format!("  Ratio A/B:  {ratio:.4}x\n"));
    }
    Ok(out)
}

fn bandwidth_action(args: &Value) -> Result<String, String> {
    // Calculate transfer time given size and speed (or throughput given size and time)
    let size_str = get_input(args)?;
    let size_bytes = parse_bytes(&size_str)?;

    if let Some(speed_str) = args
        .get("speed")
        .or_else(|| args.get("rate"))
        .and_then(|v| v.as_str())
    {
        // Parse speed — may be like "100 Mbps", "1 Gbps", "50 MBps"
        let speed_lower = speed_str.to_lowercase();
        let (bits_per_sec, label) = parse_bandwidth_speed(speed_str, &speed_lower)?;
        let seconds = size_bytes * 8.0 / bits_per_sec;
        let mut out = "Bandwidth Estimate\n\n".to_string();
        out.push_str(&format!(
            "  File size:   {}\n",
            fmt_bytes_decimal(size_bytes)
        ));
        out.push_str(&format!("  Speed:       {label}\n"));
        out.push_str(&format!("  Time:        {}\n", fmt_duration_secs(seconds)));
        return Ok(out);
    }

    if let Some(time_str) = args
        .get("time")
        .or_else(|| args.get("duration"))
        .and_then(|v| v.as_str())
    {
        let secs = parse_duration_secs(time_str)?;
        if secs == 0.0 {
            return Err("size_tools bandwidth: 'time' must be > 0".to_string());
        }
        let bits_per_sec = size_bytes * 8.0 / secs;
        let mut out = "Bandwidth Estimate\n\n".to_string();
        out.push_str(&format!(
            "  File size:   {}\n",
            fmt_bytes_decimal(size_bytes)
        ));
        out.push_str(&format!("  Time:        {time_str}\n"));
        out.push_str(&format!(
            "  Speed:       {:.2} Mbps  /  {:.2} MB/s\n",
            bits_per_sec / 1e6,
            size_bytes / secs / 1e6
        ));
        return Ok(out);
    }

    // Default: show transfer time at common speeds
    let mut out = format!("Transfer Time for {}\n\n", fmt_bytes_decimal(size_bytes));
    let speeds: &[(&str, f64)] = &[
        ("56 Kbps  (dialup)", 56_000.0),
        ("1 Mbps   (ADSL)", 1_000_000.0),
        ("10 Mbps  (cable/fibre)", 10_000_000.0),
        ("100 Mbps (fast LAN)", 100_000_000.0),
        ("1 Gbps   (gigabit LAN)", 1_000_000_000.0),
        ("10 Gbps  (10G)", 10_000_000_000.0),
    ];
    for (label, bps) in speeds {
        let secs = size_bytes * 8.0 / bps;
        out.push_str(&format!("  {label:<26}  {}\n", fmt_duration_secs(secs)));
    }
    Ok(out)
}

fn parse_bandwidth_speed(raw: &str, lower: &str) -> Result<(f64, String), String> {
    let split = raw.find(|c: char| c.is_alphabetic()).unwrap_or(raw.len());
    let num: f64 = raw[..split]
        .trim()
        .parse()
        .map_err(|_| format!("size_tools: cannot parse speed number from '{raw}'"))?;
    let unit = lower[split..].trim();

    let (bits_per_sec, label) = match unit {
        "kbps" | "kilobits/s" => (num * 1_000.0, format!("{num} Kbps")),
        "mbps" | "megabits/s" => (num * 1_000_000.0, format!("{num} Mbps")),
        "gbps" | "gigabits/s" => (num * 1_000_000_000.0, format!("{num} Gbps")),
        "kb/s" | "kbytes/s" | "kbs" => (num * 8_000.0, format!("{num} KB/s")),
        "mb/s" | "mbytes/s" | "mbs" => (num * 8_000_000.0, format!("{num} MB/s")),
        "gb/s" | "gbytes/s" | "gbs" => (num * 8_000_000_000.0, format!("{num} GB/s")),
        _ => {
            return Err(format!(
                "size_tools: unknown speed unit '{unit}'. Use Mbps, Gbps, MB/s, etc."
            ))
        }
    };
    Ok((bits_per_sec, label))
}

fn parse_duration_secs(s: &str) -> Result<f64, String> {
    let lower = s.to_lowercase();
    if let Ok(n) = s.parse::<f64>() {
        return Ok(n);
    }
    let split = s.find(|c: char| c.is_alphabetic()).unwrap_or(s.len());
    let num: f64 = s[..split]
        .trim()
        .parse()
        .map_err(|_| format!("size_tools: cannot parse duration from '{s}'"))?;
    let unit = lower[split..].trim();
    match unit {
        "s" | "sec" | "secs" | "second" | "seconds" => Ok(num),
        "m" | "min" | "mins" | "minute" | "minutes" => Ok(num * 60.0),
        "h" | "hr" | "hrs" | "hour" | "hours" => Ok(num * 3600.0),
        _ => Err(format!("size_tools: unknown time unit '{unit}'")),
    }
}

fn fmt_duration_secs(secs: f64) -> String {
    if secs < 1.0 {
        return format!("{:.2}ms", secs * 1000.0);
    }
    if secs < 60.0 {
        return format!("{:.1}s", secs);
    }
    let mins = (secs / 60.0).floor();
    let rem_secs = secs - mins * 60.0;
    if mins < 60.0 {
        return format!("{:.0}m {:.0}s", mins, rem_secs);
    }
    let hours = (mins / 60.0).floor();
    let rem_mins = mins - hours * 60.0;
    format!("{:.0}h {:.0}m {:.0}s", hours, rem_mins, rem_secs)
}
