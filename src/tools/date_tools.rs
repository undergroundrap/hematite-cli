use chrono::{DateTime, Datelike, Duration, Local, NaiveDate, TimeZone, Timelike, Utc};

pub async fn execute(args: &serde_json::Value) -> Result<String, String> {
    let action = args.get("action").and_then(|v| v.as_str()).unwrap_or("now");

    match action {
        "now" => now(args),
        "parse" => parse(args),
        "format" => format_date(args),
        "add" => add(args),
        "diff" => diff(args),
        "timestamp" => timestamp(args),
        "from-timestamp" => from_timestamp(args),
        "relative" => relative(args),
        "weekday" => weekday(args),
        other => Err(format!(
            "date_tools: unknown action '{other}'. Valid: now, parse, format, add, diff, timestamp, from-timestamp, relative, weekday"
        )),
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn parse_datetime(s: &str) -> Result<DateTime<Utc>, String> {
    let s = s.trim();

    // Try RFC3339 / ISO 8601 with timezone
    if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
        return Ok(dt.with_timezone(&Utc));
    }
    // Try RFC2822
    if let Ok(dt) = DateTime::parse_from_rfc2822(s) {
        return Ok(dt.with_timezone(&Utc));
    }
    // Try common formats
    let formats = [
        "%Y-%m-%d %H:%M:%S",
        "%Y-%m-%dT%H:%M:%S",
        "%Y-%m-%d %H:%M",
        "%Y-%m-%d",
        "%d/%m/%Y",
        "%m/%d/%Y",
        "%d-%m-%Y",
        "%B %d, %Y",
        "%b %d, %Y",
        "%d %B %Y",
        "%d %b %Y",
    ];
    for fmt in &formats {
        if let Ok(naive) = chrono::NaiveDateTime::parse_from_str(s, fmt) {
            return Ok(Utc.from_utc_datetime(&naive));
        }
        if let Ok(date) = NaiveDate::parse_from_str(s, fmt) {
            let naive = date.and_hms_opt(0, 0, 0).unwrap();
            return Ok(Utc.from_utc_datetime(&naive));
        }
    }
    Err(format!(
        "date_tools: cannot parse '{s}' — try ISO 8601 (e.g. '2024-06-15' or '2024-06-15T14:30:00Z')"
    ))
}

fn fmt_dt(dt: &DateTime<Utc>) -> String {
    dt.format("%Y-%m-%d %H:%M:%S UTC").to_string()
}

// ── Actions ───────────────────────────────────────────────────────────────────

fn now(args: &serde_json::Value) -> Result<String, String> {
    let utc = Utc::now();
    let local = Local::now();
    let fmt = args.get("format").and_then(|v| v.as_str());

    let mut out = format!("DATE NOW\n{}\n", "─".repeat(50));

    if let Some(f) = fmt {
        out.push_str(&utc.format(f).to_string());
        out.push('\n');
        return Ok(out);
    }

    out.push_str(&format!(
        "UTC   : {}\n",
        utc.format("%Y-%m-%d %H:%M:%S UTC")
    ));
    out.push_str(&format!(
        "Local : {}\n",
        local.format("%Y-%m-%d %H:%M:%S %Z")
    ));
    out.push_str(&format!("Unix  : {}\n", utc.timestamp()));
    out.push_str(&format!("ISO   : {}\n", utc.format("%Y-%m-%dT%H:%M:%SZ")));
    out.push_str(&format!(
        "Week  : Week {} of {}, {}\n",
        utc.iso_week().week(),
        utc.year(),
        utc.format("%A")
    ));
    Ok(out)
}

fn parse(args: &serde_json::Value) -> Result<String, String> {
    let input = args
        .get("input")
        .and_then(|v| v.as_str())
        .ok_or("date_tools parse: 'input' is required")?;

    let dt = parse_datetime(input)?;

    let mut out = format!("DATE PARSE\n{}\n", "─".repeat(50));
    out.push_str(&format!("Input      : {input}\n"));
    out.push_str(&format!("UTC        : {}\n", fmt_dt(&dt)));
    out.push_str(&format!(
        "ISO 8601   : {}\n",
        dt.format("%Y-%m-%dT%H:%M:%SZ")
    ));
    out.push_str(&format!("Unix epoch : {}\n", dt.timestamp()));
    out.push_str(&format!("Day of week: {}\n", dt.format("%A (%a)")));
    out.push_str(&format!("Day of year: {}\n", dt.ordinal()));
    out.push_str(&format!("Week number: {}\n", dt.iso_week().week()));
    Ok(out)
}

fn format_date(args: &serde_json::Value) -> Result<String, String> {
    let input = args
        .get("input")
        .and_then(|v| v.as_str())
        .ok_or("date_tools format: 'input' is required")?;
    let fmt = args
        .get("format")
        .and_then(|v| v.as_str())
        .unwrap_or("%Y-%m-%d");

    let dt = parse_datetime(input)?;
    let formatted = dt.format(fmt).to_string();

    Ok(format!(
        "DATE FORMAT\n{}\nInput  : {input}\nFormat : {fmt}\nResult : {formatted}",
        "─".repeat(50)
    ))
}

fn add(args: &serde_json::Value) -> Result<String, String> {
    let input = args
        .get("input")
        .and_then(|v| v.as_str())
        .ok_or("date_tools add: 'input' is required")?;

    let dt = parse_datetime(input)?;

    let days = args.get("days").and_then(|v| v.as_i64()).unwrap_or(0);
    let hours = args.get("hours").and_then(|v| v.as_i64()).unwrap_or(0);
    let minutes = args.get("minutes").and_then(|v| v.as_i64()).unwrap_or(0);
    let weeks = args.get("weeks").and_then(|v| v.as_i64()).unwrap_or(0);
    let months = args.get("months").and_then(|v| v.as_i64()).unwrap_or(0);
    let years = args.get("years").and_then(|v| v.as_i64()).unwrap_or(0);

    let mut result =
        dt + Duration::days(days + weeks * 7) + Duration::hours(hours) + Duration::minutes(minutes);

    // Handle months and years by shifting the naive date
    if months != 0 || years != 0 {
        let mut y = result.year() + years as i32;
        let mut m = result.month() as i64 + months;
        while m > 12 {
            m -= 12;
            y += 1;
        }
        while m < 1 {
            m += 12;
            y -= 1;
        }
        let day = result.day().min(days_in_month(y, m as u32));
        if let Some(naive) = NaiveDate::from_ymd_opt(y, m as u32, day) {
            let naive_dt = naive
                .and_hms_opt(result.hour(), result.minute(), result.second())
                .unwrap();
            result = Utc.from_utc_datetime(&naive_dt);
        }
    }

    let mut out = format!("DATE ADD\n{}\n", "─".repeat(50));
    out.push_str(&format!("Input  : {}\n", fmt_dt(&dt)));
    let mut parts = Vec::new();
    if years != 0 {
        parts.push(format!("{years}y"));
    }
    if months != 0 {
        parts.push(format!("{months}mo"));
    }
    if weeks != 0 {
        parts.push(format!("{weeks}w"));
    }
    if days != 0 {
        parts.push(format!("{days}d"));
    }
    if hours != 0 {
        parts.push(format!("{hours}h"));
    }
    if minutes != 0 {
        parts.push(format!("{minutes}m"));
    }
    if parts.is_empty() {
        parts.push("0d".to_string());
    }
    out.push_str(&format!("Add    : {}\n", parts.join(" ")));
    out.push_str(&format!("Result : {}\n", fmt_dt(&result)));
    out.push_str(&format!("Unix   : {}\n", result.timestamp()));
    Ok(out)
}

fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if year % 4 == 0 && (year % 100 != 0 || year % 400 == 0) {
                29
            } else {
                28
            }
        }
        _ => 30,
    }
}

fn diff(args: &serde_json::Value) -> Result<String, String> {
    let a_str = args
        .get("from")
        .and_then(|v| v.as_str())
        .ok_or("date_tools diff: 'from' is required")?;
    let b_str = args
        .get("to")
        .and_then(|v| v.as_str())
        .ok_or("date_tools diff: 'to' is required")?;

    let a = parse_datetime(a_str)?;
    let b = parse_datetime(b_str)?;

    let total_secs = (b - a).num_seconds().abs();
    let sign = if b >= a { "" } else { "-" };

    let days = total_secs / 86400;
    let hours = (total_secs % 86400) / 3600;
    let minutes = (total_secs % 3600) / 60;
    let seconds = total_secs % 60;
    let weeks = days / 7;
    let rem_days = days % 7;

    // Approximate months/years
    let total_days = (b - a).num_days().abs();
    let approx_months = total_days / 30;
    let approx_years = total_days / 365;

    let mut out = format!("DATE DIFF\n{}\n", "─".repeat(50));
    out.push_str(&format!("From   : {}\n", fmt_dt(&a)));
    out.push_str(&format!("To     : {}\n", fmt_dt(&b)));
    out.push_str(&format!(
        "Result : {sign}{weeks}w {rem_days}d {hours}h {minutes}m {seconds}s\n"
    ));
    out.push_str(&format!("       = {sign}{days} day(s)\n"));
    out.push_str(&format!("       ≈ {sign}{approx_months} month(s)\n"));
    if approx_years > 0 {
        let rem_m = approx_months % 12;
        out.push_str(&format!(
            "       ≈ {sign}{approx_years} year(s) {rem_m} month(s)\n"
        ));
    }
    out.push_str(&format!("Seconds: {sign}{total_secs}\n"));
    Ok(out)
}

fn timestamp(args: &serde_json::Value) -> Result<String, String> {
    let input = args.get("input").and_then(|v| v.as_str());

    let dt = if let Some(s) = input {
        parse_datetime(s)?
    } else {
        Utc::now()
    };

    Ok(format!(
        "DATE TIMESTAMP\n{}\nDatetime : {}\nUnix     : {}\nMillis   : {}",
        "─".repeat(50),
        fmt_dt(&dt),
        dt.timestamp(),
        dt.timestamp_millis()
    ))
}

fn from_timestamp(args: &serde_json::Value) -> Result<String, String> {
    let ts = args
        .get("input")
        .and_then(|v| v.as_i64())
        .or_else(|| {
            args.get("input")
                .and_then(|v| v.as_str())
                .and_then(|s| s.trim().parse::<i64>().ok())
        })
        .ok_or("date_tools from-timestamp: 'input' must be a Unix timestamp (seconds)")?;

    // Auto-detect milliseconds (> year 3000 in seconds = 32503680000)
    let (secs, millis_str) = if ts > 32_503_680_000 {
        (
            ts / 1000,
            format!(" (detected as milliseconds → {}s)", ts / 1000),
        )
    } else {
        (ts, String::new())
    };

    let dt = DateTime::from_timestamp(secs, 0)
        .ok_or_else(|| format!("date_tools: timestamp {ts} out of range"))?;

    let mut out = format!("DATE FROM TIMESTAMP\n{}\n", "─".repeat(50));
    out.push_str(&format!("Input  : {ts}{millis_str}\n"));
    out.push_str(&format!("UTC    : {}\n", fmt_dt(&dt)));
    out.push_str(&format!("ISO    : {}\n", dt.format("%Y-%m-%dT%H:%M:%SZ")));
    out.push_str(&format!("Day    : {}\n", dt.format("%A, %B %d, %Y")));
    Ok(out)
}

fn relative(args: &serde_json::Value) -> Result<String, String> {
    let input = args
        .get("input")
        .and_then(|v| v.as_str())
        .ok_or("date_tools relative: 'input' is required")?;

    let dt = parse_datetime(input)?;
    let now = Utc::now();
    let delta = now - dt;
    let secs = delta.num_seconds();
    let abs_secs = secs.unsigned_abs();

    let (past, label) = if secs >= 0 {
        (true, describe_duration(abs_secs))
    } else {
        (false, describe_duration(abs_secs))
    };

    let rel = if past {
        format!("{label} ago")
    } else {
        format!("in {label}")
    };

    Ok(format!(
        "DATE RELATIVE\n{}\nInput    : {input}\nDatetime : {}\nNow      : {}\nRelative : {rel}",
        "─".repeat(50),
        fmt_dt(&dt),
        fmt_dt(&now),
    ))
}

fn describe_duration(secs: u64) -> String {
    if secs < 60 {
        format!("{secs}s")
    } else if secs < 3600 {
        let m = secs / 60;
        let s = secs % 60;
        if s == 0 {
            format!("{m} minute(s)")
        } else {
            format!("{m}m {s}s")
        }
    } else if secs < 86400 {
        let h = secs / 3600;
        let m = (secs % 3600) / 60;
        if m == 0 {
            format!("{h} hour(s)")
        } else {
            format!("{h}h {m}m")
        }
    } else if secs < 86400 * 30 {
        let d = secs / 86400;
        format!("{d} day(s)")
    } else if secs < 86400 * 365 {
        let m = secs / (86400 * 30);
        format!("{m} month(s)")
    } else {
        let y = secs / (86400 * 365);
        let m = (secs % (86400 * 365)) / (86400 * 30);
        if m == 0 {
            format!("{y} year(s)")
        } else {
            format!("{y}y {m}mo")
        }
    }
}

fn weekday(args: &serde_json::Value) -> Result<String, String> {
    let input = args
        .get("input")
        .and_then(|v| v.as_str())
        .ok_or("date_tools weekday: 'input' is required")?;

    let dt = parse_datetime(input)?;

    Ok(format!(
        "DATE WEEKDAY\n{}\nInput   : {input}\nDate    : {}\nWeekday : {}\nWeek    : Week {} of {}",
        "─".repeat(50),
        dt.format("%Y-%m-%d"),
        dt.format("%A (%a)"),
        dt.iso_week().week(),
        dt.year(),
    ))
}
