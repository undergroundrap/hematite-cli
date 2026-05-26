use chrono::{Datelike, Duration, NaiveDateTime, Timelike, Utc};

pub async fn execute(args: &serde_json::Value) -> Result<String, String> {
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("explain");

    match action {
        "explain" => explain(args),
        "validate" => validate(args),
        "next" => next(args),
        "describe" => describe(args),
        other => Err(format!(
            "cron_tools: unknown action '{other}'. Valid: explain, validate, next, describe"
        )),
    }
}

// ── Cron field ────────────────────────────────────────────────────────────────

#[derive(Debug)]
enum CronValue {
    Any,
    Specific(Vec<u32>),
}

impl CronValue {
    fn matches(&self, val: u32) -> bool {
        match self {
            CronValue::Any => true,
            CronValue::Specific(vals) => vals.contains(&val),
        }
    }

    fn values(&self, min: u32, max: u32) -> Vec<u32> {
        match self {
            CronValue::Any => (min..=max).collect(),
            CronValue::Specific(v) => v.clone(),
        }
    }
}

struct CronExpr {
    minute: CronValue,
    hour: CronValue,
    dom: CronValue,
    month: CronValue,
    dow: CronValue,
}

fn parse_field(s: &str, min: u32, max: u32, names: Option<&[&str]>) -> Result<CronValue, String> {
    if s == "*" {
        return Ok(CronValue::Any);
    }

    let mut values: Vec<u32> = Vec::new();

    for part in s.split(',') {
        if part.contains('/') {
            // Step: */5 or 1-30/5
            let mut iter_parts = part.splitn(2, '/');
            let range_part = iter_parts.next().unwrap();
            let step: u32 = iter_parts
                .next()
                .and_then(|s| s.parse().ok())
                .ok_or_else(|| format!("cron_tools: invalid step in '{part}'"))?;
            if step == 0 {
                return Err("cron_tools: step cannot be zero".into());
            }
            let (lo, hi) = if range_part == "*" {
                (min, max)
            } else if range_part.contains('-') {
                parse_range(range_part, min, max, names)?
            } else {
                let v = parse_value(range_part, min, max, names)?;
                (v, max)
            };
            let mut v = lo;
            while v <= hi {
                values.push(v);
                v += step;
            }
        } else if part.contains('-') {
            let (lo, hi) = parse_range(part, min, max, names)?;
            for v in lo..=hi {
                values.push(v);
            }
        } else {
            values.push(parse_value(part, min, max, names)?);
        }
    }

    values.sort_unstable();
    values.dedup();
    Ok(CronValue::Specific(values))
}

fn parse_range(s: &str, min: u32, max: u32, names: Option<&[&str]>) -> Result<(u32, u32), String> {
    let mut parts = s.splitn(2, '-');
    let lo = parse_value(parts.next().unwrap(), min, max, names)?;
    let hi = parse_value(
        parts.next().ok_or("cron_tools: invalid range")?,
        min,
        max,
        names,
    )?;
    Ok((lo, hi))
}

fn parse_value(s: &str, min: u32, max: u32, names: Option<&[&str]>) -> Result<u32, String> {
    // Try numeric first
    if let Ok(n) = s.parse::<u32>() {
        if n < min || n > max {
            return Err(format!("cron_tools: value {n} out of range [{min}, {max}]"));
        }
        return Ok(n);
    }
    // Try named
    if let Some(name_list) = names {
        let lower = s.to_lowercase();
        for (i, name) in name_list.iter().enumerate() {
            if name.to_lowercase() == lower
                || name[..3].to_lowercase() == lower[..lower.len().min(3)]
            {
                let val = (i as u32) + min;
                return Ok(val);
            }
        }
    }
    Err(format!("cron_tools: cannot parse '{s}'"))
}

const MONTH_NAMES: &[&str] = &[
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];

const DOW_NAMES: &[&str] = &[
    "Sunday",
    "Monday",
    "Tuesday",
    "Wednesday",
    "Thursday",
    "Friday",
    "Saturday",
];

fn parse_cron(expr: &str) -> Result<CronExpr, String> {
    let parts: Vec<&str> = expr.split_whitespace().collect();
    if parts.len() != 5 {
        return Err(format!(
            "cron_tools: expected 5 fields, got {}. Format: <minute> <hour> <day> <month> <weekday>",
            parts.len()
        ));
    }
    Ok(CronExpr {
        minute: parse_field(parts[0], 0, 59, None)?,
        hour: parse_field(parts[1], 0, 23, None)?,
        dom: parse_field(parts[2], 1, 31, None)?,
        month: parse_field(parts[3], 1, 12, Some(MONTH_NAMES))?,
        dow: parse_field(parts[4], 0, 7, Some(DOW_NAMES))?,
    })
}

fn cron_matches(expr: &CronExpr, dt: &NaiveDateTime) -> bool {
    let dow = dt.weekday().num_days_from_sunday();
    expr.minute.matches(dt.minute())
        && expr.hour.matches(dt.hour())
        && expr.dom.matches(dt.day())
        && expr.month.matches(dt.month())
        && (expr.dow.matches(dow) || expr.dow.matches(dow % 7 + if dow == 0 { 7 } else { 0 }))
}

fn next_n_runs(expr: &CronExpr, from: NaiveDateTime, n: usize) -> Vec<NaiveDateTime> {
    let mut results = Vec::with_capacity(n);
    // Advance to next minute boundary
    let mut dt = from + Duration::minutes(1);
    dt = dt.with_second(0).unwrap().with_nanosecond(0).unwrap();
    // Cap at 2 years of searching (roughly 1M minutes)
    let max_steps = 1_051_200usize;
    for _ in 0..max_steps {
        if cron_matches(expr, &dt) {
            results.push(dt);
            if results.len() == n {
                break;
            }
        }
        dt += Duration::minutes(1);
    }
    results
}

// ── Field description helpers ─────────────────────────────────────────────────

fn describe_field(
    val: &CronValue,
    unit: &str,
    names: Option<&[&str]>,
    min: u32,
    max: u32,
) -> String {
    match val {
        CronValue::Any => format!("every {unit}"),
        CronValue::Specific(vals) => {
            if vals.len() == 1 {
                let v = vals[0];
                if let Some(ns) = names {
                    let idx = (v as usize).saturating_sub(min as usize);
                    if idx < ns.len() {
                        return format!("{} ({})", ns[idx], v);
                    }
                }
                format!("{unit} {v}")
            } else {
                // Detect step pattern
                let _full_range: Vec<u32> = (min..=max).collect();
                if vals.len() > 1 {
                    let step = vals[1] - vals[0];
                    let is_step = vals.windows(2).all(|w| w[1] - w[0] == step);
                    if is_step && vals[0] == min && *vals.last().unwrap() + step > max {
                        return format!("every {step} {unit}(s)");
                    }
                    if is_step && step > 1 {
                        return format!("every {step} {unit}(s) starting at {}", vals[0]);
                    }
                }
                // Range
                if vals.len() > 2 && *vals.last().unwrap() - vals[0] + 1 == vals.len() as u32 {
                    if let Some(ns) = names {
                        let lo = (vals[0] as usize).saturating_sub(min as usize);
                        let hi = (*vals.last().unwrap() as usize).saturating_sub(min as usize);
                        if lo < ns.len() && hi < ns.len() {
                            return format!("{} through {}", ns[lo], ns[hi]);
                        }
                    }
                    return format!("{unit} {} through {}", vals[0], vals.last().unwrap());
                }
                // List
                let labels: Vec<String> = vals
                    .iter()
                    .map(|&v| {
                        if let Some(ns) = names {
                            let idx = (v as usize).saturating_sub(min as usize);
                            if idx < ns.len() {
                                return ns[idx].to_string();
                            }
                        }
                        v.to_string()
                    })
                    .collect();
                format!("{}: {}", unit, labels.join(", ")).to_string()
            }
        }
    }
}

// ── Actions ───────────────────────────────────────────────────────────────────

fn explain(args: &serde_json::Value) -> Result<String, String> {
    let expr = args
        .get("expression")
        .or_else(|| args.get("input"))
        .and_then(|v| v.as_str())
        .ok_or("cron_tools explain: 'expression' is required")?;

    let parsed = parse_cron(expr)?;

    let minute_desc = describe_field(&parsed.minute, "minute", None, 0, 59);
    let hour_desc = describe_field(&parsed.hour, "hour", None, 0, 23);
    let dom_desc = describe_field(&parsed.dom, "day-of-month", None, 1, 31);
    let month_desc = describe_field(&parsed.month, "month", Some(MONTH_NAMES), 1, 12);
    let dow_desc = describe_field(&parsed.dow, "weekday", Some(DOW_NAMES), 0, 6);

    let parts: Vec<&str> = expr.split_whitespace().collect();
    let mut out = format!("CRON EXPLAIN\n{}\n", "─".repeat(50));
    out.push_str(&format!("Expression : {expr}\n\n"));
    out.push_str(&format!(
        "  Field 1 (minute)     : {:5} → {}\n",
        parts[0], minute_desc
    ));
    out.push_str(&format!(
        "  Field 2 (hour)       : {:5} → {}\n",
        parts[1], hour_desc
    ));
    out.push_str(&format!(
        "  Field 3 (day/month)  : {:5} → {}\n",
        parts[2], dom_desc
    ));
    out.push_str(&format!(
        "  Field 4 (month)      : {:5} → {}\n",
        parts[3], month_desc
    ));
    out.push_str(&format!(
        "  Field 5 (weekday)    : {:5} → {}\n\n",
        parts[4], dow_desc
    ));

    // One-line summary
    out.push_str(&format!("Summary    : {}\n", one_line(&parsed)));
    Ok(out)
}

fn validate(args: &serde_json::Value) -> Result<String, String> {
    let expr = args
        .get("expression")
        .or_else(|| args.get("input"))
        .and_then(|v| v.as_str())
        .ok_or("cron_tools validate: 'expression' is required")?;

    match parse_cron(expr) {
        Ok(_) => Ok(format!(
            "CRON VALIDATE\n{}\nExpression : {expr}\nValid      : YES\n",
            "─".repeat(50)
        )),
        Err(e) => Ok(format!(
            "CRON VALIDATE\n{}\nExpression : {expr}\nValid      : NO\nError      : {e}\n",
            "─".repeat(50)
        )),
    }
}

fn next(args: &serde_json::Value) -> Result<String, String> {
    let expr = args
        .get("expression")
        .or_else(|| args.get("input"))
        .and_then(|v| v.as_str())
        .ok_or("cron_tools next: 'expression' is required")?;

    let n = args.get("n").and_then(|v| v.as_u64()).unwrap_or(5).min(20) as usize;

    let parsed = parse_cron(expr)?;
    let now = Utc::now().naive_utc();
    let runs = next_n_runs(&parsed, now, n);

    let mut out = format!("CRON NEXT ({n} runs)\n{}\n", "─".repeat(50));
    out.push_str(&format!("Expression : {expr}\n"));
    out.push_str(&format!(
        "From       : {}\n\n",
        now.format("%Y-%m-%d %H:%M UTC")
    ));

    if runs.is_empty() {
        out.push_str("  (no matching times found in the next 2 years)\n");
    } else {
        for (i, dt) in runs.iter().enumerate() {
            let delta = *dt - now;
            let hrs = delta.num_hours();
            let label = if hrs < 1 {
                format!("{}m from now", delta.num_minutes())
            } else if hrs < 48 {
                format!("{hrs}h {}m from now", delta.num_minutes() % 60)
            } else {
                format!("{}d {}h from now", delta.num_days(), hrs % 24)
            };
            out.push_str(&format!(
                "  {:2}. {}  ({})\n",
                i + 1,
                dt.format("%Y-%m-%d %H:%M UTC"),
                label
            ));
        }
    }
    Ok(out)
}

fn describe(args: &serde_json::Value) -> Result<String, String> {
    let expr = args
        .get("expression")
        .or_else(|| args.get("input"))
        .and_then(|v| v.as_str())
        .ok_or("cron_tools describe: 'expression' is required")?;

    let parsed = parse_cron(expr)?;
    Ok(format!(
        "CRON DESCRIBE\n{}\n{}\n",
        "─".repeat(50),
        one_line(&parsed)
    ))
}

fn one_line(expr: &CronExpr) -> String {
    // Check common patterns
    let min_vals = expr.minute.values(0, 59);
    let hr_vals = expr.hour.values(0, 23);

    let minute_any = matches!(expr.minute, CronValue::Any);
    let hour_any = matches!(expr.hour, CronValue::Any);
    let dom_any = matches!(expr.dom, CronValue::Any);
    let month_any = matches!(expr.month, CronValue::Any);
    let dow_any = matches!(expr.dow, CronValue::Any);

    if minute_any && hour_any && dom_any && month_any && dow_any {
        return "Runs every minute".to_string();
    }
    if !minute_any && hour_any && dom_any && month_any && dow_any {
        if min_vals.len() == 1 {
            return format!("Runs at minute {} of every hour", min_vals[0]);
        }
        if min_vals.len() > 1 {
            let step = if min_vals.len() > 1 {
                min_vals[1] - min_vals[0]
            } else {
                0
            };
            let is_step = min_vals.windows(2).all(|w| w[1] - w[0] == step);
            if is_step && step > 0 {
                return format!("Runs every {step} minutes");
            }
        }
    }
    if !minute_any && !hour_any && dom_any && month_any && dow_any {
        if min_vals.len() == 1 && hr_vals.len() == 1 {
            return format!("Runs daily at {:02}:{:02} UTC", hr_vals[0], min_vals[0]);
        }
    }
    if !minute_any && !hour_any && dom_any && month_any && !dow_any {
        if min_vals.len() == 1 && hr_vals.len() == 1 {
            let dow_desc = describe_field(&expr.dow, "weekday", Some(DOW_NAMES), 0, 6);
            return format!(
                "Runs weekly on {} at {:02}:{:02} UTC",
                dow_desc, hr_vals[0], min_vals[0]
            );
        }
    }
    if !minute_any && !hour_any && !dom_any && month_any && dow_any {
        if min_vals.len() == 1 && hr_vals.len() == 1 {
            let dom_desc = describe_field(&expr.dom, "day", None, 1, 31);
            return format!(
                "Runs monthly on {} at {:02}:{:02} UTC",
                dom_desc, hr_vals[0], min_vals[0]
            );
        }
    }

    // Generic fallback
    let minute_s = describe_field(&expr.minute, "minute", None, 0, 59);
    let hour_s = describe_field(&expr.hour, "hour", None, 0, 23);
    let dom_s = describe_field(&expr.dom, "day-of-month", None, 1, 31);
    let month_s = describe_field(&expr.month, "month", Some(MONTH_NAMES), 1, 12);
    let dow_s = describe_field(&expr.dow, "weekday", Some(DOW_NAMES), 0, 6);
    format!("At {minute_s} of {hour_s}, {dom_s}, {month_s}, {dow_s}")
}
