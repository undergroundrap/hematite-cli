pub async fn execute(args: &serde_json::Value) -> Result<String, String> {
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("parse");
    match action {
        "parse" => parse_action(args),
        "humanize" => humanize_action(args),
        "convert" => convert_action(args),
        "add" => add_action(args),
        other => Err(format!(
            "duration_tools: unknown action '{other}'. Valid: parse, humanize, convert, add"
        )),
    }
}

// ── Input helper ──────────────────────────────────────────────────────────────

fn get_input(args: &serde_json::Value) -> Result<&str, String> {
    args.get("duration")
        .or_else(|| args.get("input"))
        .or_else(|| args.get("text"))
        .or_else(|| args.get("value"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| "duration_tools: 'duration' is required".to_string())
}

// ── Parser ────────────────────────────────────────────────────────────────────

fn parse_duration_str(s: &str) -> Result<i64, String> {
    let s = s.trim();

    // Plain integer or float → seconds
    if let Ok(n) = s.parse::<i64>() {
        return Ok(n);
    }
    if let Ok(f) = s.parse::<f64>() {
        return Ok(f as i64);
    }

    // HH:MM:SS or MM:SS (no alphabetic chars)
    if s.contains(':') && !s.chars().any(|c| c.is_alphabetic()) {
        return parse_colon_format(s);
    }

    // ISO 8601 duration: PT1H30M45S, P2DT4H, etc.
    if s.starts_with('P') || s.starts_with('p') {
        return parse_iso8601(&s.to_uppercase());
    }

    // Human-readable: "1h 30m 45s", "2 days 4 hours", "90 minutes"
    parse_human(s)
}

fn parse_colon_format(s: &str) -> Result<i64, String> {
    let parts: Vec<&str> = s.split(':').collect();
    match parts.len() {
        2 => {
            let m: i64 = parts[0]
                .trim()
                .parse()
                .map_err(|_| format!("duration_tools: invalid minutes '{}'", parts[0]))?;
            let sec: i64 = parts[1]
                .trim()
                .parse()
                .map_err(|_| format!("duration_tools: invalid seconds '{}'", parts[1]))?;
            Ok(m * 60 + sec)
        }
        3 => {
            let h: i64 = parts[0]
                .trim()
                .parse()
                .map_err(|_| format!("duration_tools: invalid hours '{}'", parts[0]))?;
            let m: i64 = parts[1]
                .trim()
                .parse()
                .map_err(|_| format!("duration_tools: invalid minutes '{}'", parts[1]))?;
            let sec: i64 = parts[2]
                .trim()
                .parse()
                .map_err(|_| format!("duration_tools: invalid seconds '{}'", parts[2]))?;
            Ok(h * 3600 + m * 60 + sec)
        }
        _ => Err(format!(
            "duration_tools: expected MM:SS or HH:MM:SS, got '{s}'"
        )),
    }
}

fn parse_iso8601_segment(seg: &str, is_time: bool) -> i64 {
    let mut acc = 0i64;
    let mut num_buf = String::new();
    for ch in seg.chars() {
        if ch.is_ascii_digit() || ch == '.' {
            num_buf.push(ch);
        } else {
            let n: f64 = num_buf.parse().unwrap_or(0.0);
            num_buf.clear();
            acc += if is_time {
                match ch {
                    'H' => n as i64 * 3600,
                    'M' => n as i64 * 60,
                    'S' => n as i64,
                    _ => 0,
                }
            } else {
                match ch {
                    'Y' => n as i64 * 365 * 86400,
                    'M' => n as i64 * 30 * 86400,
                    'W' => n as i64 * 7 * 86400,
                    'D' => n as i64 * 86400,
                    _ => 0,
                }
            };
        }
    }
    acc
}

fn parse_iso8601(s: &str) -> Result<i64, String> {
    // P[nY][nM][nW][nD][T[nH][nM][nS]]
    let s = s.trim_start_matches('P');
    let (date_part, time_part) = if let Some(t_pos) = s.find('T') {
        (&s[..t_pos], &s[t_pos + 1..])
    } else {
        (s, "")
    };
    Ok(parse_iso8601_segment(date_part, false) + parse_iso8601_segment(time_part, true))
}

fn unit_to_seconds(unit: &str) -> Result<i64, String> {
    match unit {
        "s" | "sec" | "secs" | "second" | "seconds" => Ok(1),
        "m" | "min" | "mins" | "minute" | "minutes" => Ok(60),
        "h" | "hr" | "hrs" | "hour" | "hours" => Ok(3600),
        "d" | "day" | "days" => Ok(86400),
        "w" | "wk" | "wks" | "week" | "weeks" => Ok(604_800),
        "mo" | "mos" | "month" | "months" => Ok(2_592_000),
        "y" | "yr" | "yrs" | "year" | "years" => Ok(31_536_000),
        "ms" | "millisecond" | "milliseconds" | "millis" => Ok(0),
        other => Err(format!("duration_tools: unknown unit '{other}'")),
    }
}

fn parse_human(s: &str) -> Result<i64, String> {
    let lower = s.to_lowercase();
    let mut total = 0i64;
    let mut found_any = false;

    let mut chars = lower.chars().peekable();
    loop {
        // Skip non-digits (whitespace, commas, "and", etc.)
        while chars.peek().map(|c| !c.is_ascii_digit()).unwrap_or(false) {
            chars.next();
        }
        if chars.peek().is_none() {
            break;
        }

        // Read number (digits + optional decimal point)
        let mut num_str = String::new();
        while chars
            .peek()
            .map(|c| c.is_ascii_digit() || *c == '.')
            .unwrap_or(false)
        {
            num_str.push(chars.next().unwrap());
        }
        if num_str.is_empty() {
            break;
        }

        let n: f64 = num_str
            .parse()
            .map_err(|_| format!("duration_tools: invalid number '{num_str}'"))?;

        // Skip whitespace between number and unit
        while chars.peek().map(|c| c.is_whitespace()).unwrap_or(false) {
            chars.next();
        }

        // Read unit (alphabetic chars)
        let mut unit = String::new();
        while chars.peek().map(|c| c.is_alphabetic()).unwrap_or(false) {
            unit.push(chars.next().unwrap());
        }

        let mult = if unit.is_empty() {
            1i64 // no unit → treat as seconds
        } else {
            unit_to_seconds(&unit)?
        };

        total += (n * mult as f64) as i64;
        found_any = true;
    }

    if found_any {
        Ok(total)
    } else {
        Err(format!("duration_tools: cannot parse duration '{s}'"))
    }
}

// ── Formatters ────────────────────────────────────────────────────────────────

fn breakdown(secs: i64) -> (bool, i64, i64, i64, i64, i64) {
    let neg = secs < 0;
    let abs = secs.unsigned_abs() as i64;
    let years = abs / 31_536_000;
    let rem = abs % 31_536_000;
    let days = rem / 86400;
    let rem = rem % 86400;
    let hours = rem / 3600;
    let rem = rem % 3600;
    let minutes = rem / 60;
    let seconds = rem % 60;
    (neg, years, days, hours, minutes, seconds)
}

fn humanize(secs: i64) -> String {
    if secs == 0 {
        return "0 seconds".to_string();
    }
    let (neg, years, days, hours, minutes, seconds) = breakdown(secs);
    let mut parts = Vec::new();
    if years > 0 {
        parts.push(format!(
            "{years} {}",
            if years == 1 { "year" } else { "years" }
        ));
    }
    if days > 0 {
        parts.push(format!("{days} {}", if days == 1 { "day" } else { "days" }));
    }
    if hours > 0 {
        parts.push(format!(
            "{hours} {}",
            if hours == 1 { "hour" } else { "hours" }
        ));
    }
    if minutes > 0 {
        parts.push(format!(
            "{minutes} {}",
            if minutes == 1 { "minute" } else { "minutes" }
        ));
    }
    if seconds > 0 {
        parts.push(format!(
            "{seconds} {}",
            if seconds == 1 { "second" } else { "seconds" }
        ));
    }

    let result = match parts.len() {
        0 => "0 seconds".to_string(),
        1 => parts[0].clone(),
        2 => format!("{} and {}", parts[0], parts[1]),
        _ => {
            let last = parts.pop().unwrap();
            format!("{}, and {last}", parts.join(", "))
        }
    };
    if neg {
        format!("-{result}")
    } else {
        result
    }
}

fn compact(secs: i64) -> String {
    if secs == 0 {
        return "0s".to_string();
    }
    let (neg, years, days, hours, minutes, seconds) = breakdown(secs);
    let mut parts = Vec::new();
    if years > 0 {
        parts.push(format!("{years}y"));
    }
    if days > 0 {
        parts.push(format!("{days}d"));
    }
    if hours > 0 {
        parts.push(format!("{hours}h"));
    }
    if minutes > 0 {
        parts.push(format!("{minutes}m"));
    }
    if seconds > 0 {
        parts.push(format!("{seconds}s"));
    }
    let result = parts.join(" ");
    if neg {
        format!("-{result}")
    } else {
        result
    }
}

// ── Actions ───────────────────────────────────────────────────────────────────

fn parse_action(args: &serde_json::Value) -> Result<String, String> {
    let input = get_input(args)?;
    let total_secs = parse_duration_str(input)?;
    let (neg, years, days, hours, minutes, seconds) = breakdown(total_secs);

    let mut out = format!("DURATION PARSE\n{}\n", "─".repeat(50));
    out.push_str(&format!("Input      : {input}\n"));
    if neg {
        out.push_str("Sign       : negative\n");
    }
    out.push_str(&format!("Total secs : {total_secs}\n\n"));
    out.push_str(&format!("Years      : {years}\n"));
    out.push_str(&format!("Days       : {days}\n"));
    out.push_str(&format!("Hours      : {hours}\n"));
    out.push_str(&format!("Minutes    : {minutes}\n"));
    out.push_str(&format!("Seconds    : {seconds}\n\n"));
    out.push_str(&format!("Compact    : {}\n", compact(total_secs)));
    out.push_str(&format!("Human      : {}\n", humanize(total_secs)));
    Ok(out)
}

fn humanize_action(args: &serde_json::Value) -> Result<String, String> {
    let input = get_input(args)?;
    let total_secs = parse_duration_str(input)?;
    let style = args.get("style").and_then(|v| v.as_str()).unwrap_or("long");

    let result = if style == "compact" || style == "short" {
        compact(total_secs)
    } else {
        humanize(total_secs)
    };

    let mut out = format!("DURATION HUMANIZE\n{}\n", "─".repeat(50));
    out.push_str(&format!("Input      : {input}\n"));
    out.push_str(&format!("Total secs : {total_secs}\n"));
    out.push_str(&format!("Result     : {result}\n"));
    Ok(out)
}

fn convert_action(args: &serde_json::Value) -> Result<String, String> {
    let input = get_input(args)?;
    let total_secs = parse_duration_str(input)?;
    let to = args.get("to").and_then(|v| v.as_str());

    let mut out = format!("DURATION CONVERT\n{}\n", "─".repeat(50));
    out.push_str(&format!("Input   : {input}\n"));
    out.push_str(&format!("Seconds : {total_secs}\n"));

    if let Some(unit) = to {
        let (label, divisor) = match unit.to_lowercase().as_str() {
            "seconds" | "second" | "s" | "sec" => ("Seconds", 1.0f64),
            "minutes" | "minute" | "m" | "min" => ("Minutes", 60.0),
            "hours" | "hour" | "h" | "hr" => ("Hours", 3600.0),
            "days" | "day" | "d" => ("Days", 86400.0),
            "weeks" | "week" | "w" | "wk" => ("Weeks", 604_800.0),
            other => {
                return Err(format!(
                    "duration_tools convert: unknown unit '{other}'. Valid: seconds, minutes, hours, days, weeks"
                ))
            }
        };
        out.push_str(&format!("{label:<8}: {:.4}\n", total_secs as f64 / divisor));
    } else {
        out.push_str(&format!("Minutes : {:.4}\n", total_secs as f64 / 60.0));
        out.push_str(&format!("Hours   : {:.4}\n", total_secs as f64 / 3600.0));
        out.push_str(&format!("Days    : {:.4}\n", total_secs as f64 / 86400.0));
        out.push_str(&format!("Weeks   : {:.4}\n", total_secs as f64 / 604_800.0));
    }
    Ok(out)
}

fn add_action(args: &serde_json::Value) -> Result<String, String> {
    let total_secs = if let Some(arr) = args.get("durations").and_then(|v| v.as_array()) {
        let mut sum = 0i64;
        for item in arr {
            let s = item
                .as_str()
                .ok_or("duration_tools add: each 'durations' item must be a string")?;
            sum += parse_duration_str(s)?;
        }
        sum
    } else {
        let a_str = args
            .get("a")
            .and_then(|v| v.as_str())
            .ok_or("duration_tools add: 'a' and 'b' (or 'durations' array) are required")?;
        let b_str = args
            .get("b")
            .and_then(|v| v.as_str())
            .ok_or("duration_tools add: 'b' is required")?;
        parse_duration_str(a_str)? + parse_duration_str(b_str)?
    };

    let mut out = format!("DURATION ADD\n{}\n", "─".repeat(50));
    out.push_str(&format!("Total secs : {total_secs}\n"));
    out.push_str(&format!("Compact    : {}\n", compact(total_secs)));
    out.push_str(&format!("Human      : {}\n", humanize(total_secs)));
    Ok(out)
}
