use serde_json::{json, Value};

pub fn interval_tools_schema() -> Value {
    json!({
        "name": "interval_tools",
        "description": "Date interval operations without external utilities: check overlaps, containment, compute unions/intersections, generate schedules, and calculate durations between date ranges. Actions: overlap (do two intervals overlap?), contains (is a date within an interval?), union (merge overlapping intervals from a list), intersect (find the intersection of two intervals), duration (time between two dates or within an interval), schedule (generate N dates at a regular interval from a start date). Accepts ISO 8601 dates (YYYY-MM-DD) and datetimes (YYYY-MM-DDTHH:MM:SS).",
        "parameters": {
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["overlap", "contains", "union", "intersect", "duration", "schedule"],
                    "description": "Operation to perform (default: overlap)"
                },
                "start": {
                    "type": "string",
                    "description": "Start of first interval (ISO 8601 date or datetime)"
                },
                "end": {
                    "type": "string",
                    "description": "End of first interval (ISO 8601 date or datetime)"
                },
                "start2": {
                    "type": "string",
                    "description": "Start of second interval for overlap/intersect (ISO 8601)"
                },
                "end2": {
                    "type": "string",
                    "description": "End of second interval for overlap/intersect (ISO 8601)"
                },
                "date": {
                    "type": "string",
                    "description": "Point-in-time date for 'contains' action (ISO 8601)"
                },
                "intervals": {
                    "type": "array",
                    "description": "Array of {start, end} objects for 'union' action",
                    "items": {
                        "type": "object"
                    }
                },
                "step": {
                    "type": "string",
                    "description": "Step interval for 'schedule' action: '1d' (1 day), '2w' (2 weeks), '1m' (1 month), '1y' (1 year), '6h' (6 hours), '30min' (30 minutes)"
                },
                "count": {
                    "type": "integer",
                    "description": "Number of dates to generate for 'schedule' (default: 10, max: 365)"
                },
                "inclusive": {
                    "type": "boolean",
                    "description": "Treat interval boundaries as inclusive (default: true)"
                }
            },
            "required": []
        }
    })
}

// ── date parsing ──────────────────────────────────────────────────────────────

/// Parses ISO 8601 date or datetime to (year, month, day, hour, minute, second).
/// Accepts: YYYY-MM-DD, YYYY-MM-DDTHH:MM:SS, YYYY-MM-DD HH:MM:SS
fn parse_datetime(s: &str) -> Result<(i32, u32, u32, u32, u32, u32), String> {
    let s = s.trim();
    // Strip trailing Z or timezone offset
    let s = if let Some(idx) = s.find('+').or_else(|| s.rfind('-').filter(|&i| i > 7)) {
        &s[..idx]
    } else {
        s
    };
    let s = s.trim_end_matches('Z');

    if s.len() == 10 {
        // YYYY-MM-DD
        let parts: Vec<&str> = s.split('-').collect();
        if parts.len() != 3 {
            return Err(format!("Invalid date: '{s}'"));
        }
        let y = parts[0]
            .parse::<i32>()
            .map_err(|_| format!("Invalid year in '{s}'"))?;
        let mo = parts[1]
            .parse::<u32>()
            .map_err(|_| format!("Invalid month in '{s}'"))?;
        let d = parts[2]
            .parse::<u32>()
            .map_err(|_| format!("Invalid day in '{s}'"))?;
        return Ok((y, mo, d, 0, 0, 0));
    }

    // YYYY-MM-DDTHH:MM:SS or YYYY-MM-DD HH:MM:SS
    let sep_pos = s.find('T').or_else(|| s.find(' '));
    let (date_part, time_part) = match sep_pos {
        Some(i) => (&s[..i], &s[i + 1..]),
        None => return Err(format!("Cannot parse datetime: '{s}'")),
    };

    let dp: Vec<&str> = date_part.split('-').collect();
    if dp.len() != 3 {
        return Err(format!("Invalid date part: '{date_part}'"));
    }
    let y = dp[0]
        .parse::<i32>()
        .map_err(|_| format!("Invalid year in '{s}'"))?;
    let mo = dp[1]
        .parse::<u32>()
        .map_err(|_| format!("Invalid month in '{s}'"))?;
    let d = dp[2]
        .parse::<u32>()
        .map_err(|_| format!("Invalid day in '{s}'"))?;

    let tp: Vec<&str> = time_part.split(':').collect();
    let h = tp.first().and_then(|v| v.parse::<u32>().ok()).unwrap_or(0);
    let mi = tp.get(1).and_then(|v| v.parse::<u32>().ok()).unwrap_or(0);
    let sec = tp.get(2).and_then(|v| v.parse::<u32>().ok()).unwrap_or(0);

    Ok((y, mo, d, h, mi, sec))
}

/// Days in a given month (1-based), handling leap years.
fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if is_leap(year) {
                29
            } else {
                28
            }
        }
        _ => 30,
    }
}

fn is_leap(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

/// Convert (year, month, day, hour, minute, second) to a Unix-like seconds value
/// (days since 0001-01-01 * 86400 + time).
fn to_epoch(y: i32, mo: u32, d: u32, h: u32, mi: u32, s: u32) -> i64 {
    // Days from year 1 to start of year y
    let yy = y as i64 - 1;
    let days_before_year = yy * 365 + yy / 4 - yy / 100 + yy / 400;

    // Days in months 1..mo-1 of year y
    let mut days_in_months = 0i64;
    for m in 1..mo {
        days_in_months += days_in_month(y, m) as i64;
    }

    let total_days = days_before_year + days_in_months + d as i64 - 1;
    total_days * 86400 + h as i64 * 3600 + mi as i64 * 60 + s as i64
}

fn epoch_to_dt(mut e: i64) -> (i32, u32, u32, u32, u32, u32) {
    let neg = e < 0;
    if neg {
        e = -e;
    }
    let sec = (e % 60) as u32;
    e /= 60;
    let min = (e % 60) as u32;
    e /= 60;
    let hour = (e % 24) as u32;
    let mut day_num = e / 24; // 0-based day count from epoch

    if neg {
        // For negative epochs (very old dates), just return a placeholder
        return (0, 1, 1, 0, 0, 0);
    }

    // Convert day_num back to year/month/day
    let mut year = 1i32;
    loop {
        let days_in_year = if is_leap(year) { 366 } else { 365 };
        if day_num < days_in_year as i64 {
            break;
        }
        day_num -= days_in_year as i64;
        year += 1;
    }
    let mut month = 1u32;
    loop {
        let dim = days_in_month(year, month) as i64;
        if day_num < dim {
            break;
        }
        day_num -= dim;
        month += 1;
    }
    let day = day_num as u32 + 1;
    (year, month, day, hour, min, sec)
}

fn parse_to_epoch(s: &str) -> Result<i64, String> {
    let (y, mo, d, h, mi, s) = parse_datetime(s)?;
    Ok(to_epoch(y, mo, d, h, mi, s))
}

fn format_dt(e: i64, has_time: bool) -> String {
    let (y, mo, d, h, mi, s) = epoch_to_dt(e);
    if has_time {
        format!("{y:04}-{mo:02}-{d:02}T{h:02}:{mi:02}:{s:02}")
    } else {
        format!("{y:04}-{mo:02}-{d:02}")
    }
}

fn has_time_component(s: &str) -> bool {
    s.contains('T') || (s.len() > 10 && s.contains(' '))
}

// ── step parsing ──────────────────────────────────────────────────────────────

#[derive(Clone, Copy)]
enum Step {
    Seconds(i64),
    Months(i32),
    Years(i32),
}

fn parse_step(s: &str) -> Result<Step, String> {
    let s = s.trim().to_lowercase();
    // Patterns: Nd, Nw, Nm, Ny, Nh, Nmin, Ns
    let (num_str, unit) = s
        .find(|c: char| c.is_alphabetic())
        .map(|i| (&s[..i], &s[i..]))
        .ok_or_else(|| format!("Cannot parse step '{s}' — expected e.g. '1d', '2w', '1m', '6h'"))?;
    let n: i64 = num_str
        .parse()
        .map_err(|_| format!("Invalid step number in '{s}'"))?;
    match unit {
        "s" | "sec" | "second" | "seconds" => Ok(Step::Seconds(n)),
        "min" | "minute" | "minutes" => Ok(Step::Seconds(n * 60)),
        "h" | "hour" | "hours" => Ok(Step::Seconds(n * 3600)),
        "d" | "day" | "days" => Ok(Step::Seconds(n * 86400)),
        "w" | "week" | "weeks" => Ok(Step::Seconds(n * 7 * 86400)),
        "m" | "month" | "months" => Ok(Step::Months(n as i32)),
        "y" | "year" | "years" => Ok(Step::Years(n as i32)),
        _ => Err(format!("Unknown step unit '{unit}' — use s/min/h/d/w/m/y")),
    }
}

fn apply_step(e: i64, step: Step) -> i64 {
    match step {
        Step::Seconds(s) => e + s,
        Step::Months(mo) => {
            let (y, mut m, d, h, mi, s) = epoch_to_dt(e);
            let total_months = y as i64 * 12 + m as i64 - 1 + mo as i64;
            let new_y = (total_months / 12) as i32;
            m = (total_months % 12) as u32 + 1;
            let capped_d = d.min(days_in_month(new_y, m));
            to_epoch(new_y, m, capped_d, h, mi, s)
        }
        Step::Years(yr) => {
            let (y, mo, d, h, mi, s) = epoch_to_dt(e);
            let new_y = y + yr;
            let capped_d = d.min(days_in_month(new_y, mo));
            to_epoch(new_y, mo, capped_d, h, mi, s)
        }
    }
}

// ── duration formatting ───────────────────────────────────────────────────────

fn format_duration(secs: i64) -> String {
    let abs = secs.unsigned_abs();
    let sign = if secs < 0 { "-" } else { "" };
    let years = abs / (365 * 86400);
    let rem = abs % (365 * 86400);
    let days = rem / 86400;
    let rem2 = rem % 86400;
    let hours = rem2 / 3600;
    let minutes = (rem2 % 3600) / 60;
    let seconds = rem2 % 60;

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
    if seconds > 0 || parts.is_empty() {
        parts.push(format!("{seconds}s"));
    }
    format!("{sign}{}", parts.join(" "))
}

fn format_duration_verbose(secs: i64) -> String {
    let abs = secs.unsigned_abs();
    let total_days = abs / 86400;
    let weeks = total_days / 7;
    let days = total_days % 7;
    let rem = abs % 86400;
    let hours = rem / 3600;
    let minutes = (rem % 3600) / 60;
    let seconds = rem % 60;

    let mut lines = Vec::new();
    lines.push(format!("Total seconds:  {}", abs));
    lines.push(format!("Total minutes:  {}", abs / 60));
    lines.push(format!("Total hours:    {}", abs / 3600));
    lines.push(format!("Total days:     {total_days}"));
    lines.push(format!("Weeks:          {weeks}"));
    lines.push(format!(
        "Breakdown:      {}w {}d {}h {}m {}s",
        weeks, days, hours, minutes, seconds
    ));
    lines.join("\n")
}

// ── actions ───────────────────────────────────────────────────────────────────

fn action_overlap(args: &Value) -> Result<String, String> {
    let s1 = args
        .get("start")
        .and_then(|v| v.as_str())
        .ok_or("'start' is required")?;
    let e1 = args
        .get("end")
        .and_then(|v| v.as_str())
        .ok_or("'end' is required")?;
    let s2 = args
        .get("start2")
        .and_then(|v| v.as_str())
        .ok_or("'start2' is required")?;
    let e2 = args
        .get("end2")
        .and_then(|v| v.as_str())
        .ok_or("'end2' is required")?;
    let inclusive = args
        .get("inclusive")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);

    let a0 = parse_to_epoch(s1)?;
    let a1 = parse_to_epoch(e1)?;
    let b0 = parse_to_epoch(s2)?;
    let b1 = parse_to_epoch(e2)?;

    if a0 > a1 {
        return Err(format!("'start' ({s1}) must be ≤ 'end' ({e1})"));
    }
    if b0 > b1 {
        return Err(format!("'start2' ({s2}) must be ≤ 'end2' ({e2})"));
    }

    let has_time = has_time_component(s1) || has_time_component(s2);

    let overlaps = if inclusive {
        a0 <= b1 && b0 <= a1
    } else {
        a0 < b1 && b0 < a1
    };

    let mut out = format!("Interval A:  {} → {}\n", s1, e1);
    out.push_str(&format!("Interval B:  {} → {}\n", s2, e2));
    out.push_str(&format!(
        "Boundaries:  {}\n",
        if inclusive { "inclusive" } else { "exclusive" }
    ));
    out.push_str(&"─".repeat(50));
    out.push('\n');

    if overlaps {
        let overlap_start = a0.max(b0);
        let overlap_end = a1.min(b1);
        let duration = overlap_end - overlap_start;
        out.push_str("Result:      OVERLAPPING ✓\n");
        out.push_str(&format!(
            "Overlap:     {} → {}\n",
            format_dt(overlap_start, has_time),
            format_dt(overlap_end, has_time)
        ));
        out.push_str(&format!("Duration:    {}\n", format_duration(duration)));
    } else {
        out.push_str("Result:      NO OVERLAP ✗\n");
        // Show gap
        let (gap_start, gap_end) = if a1 < b0 { (a1, b0) } else { (b1, a0) };
        let gap = (gap_end - gap_start).abs();
        out.push_str(&format!(
            "Gap:         {} → {} ({})\n",
            format_dt(gap_start, has_time),
            format_dt(gap_end, has_time),
            format_duration(gap)
        ));
    }
    Ok(out)
}

fn action_contains(args: &Value) -> Result<String, String> {
    let s = args
        .get("start")
        .and_then(|v| v.as_str())
        .ok_or("'start' is required")?;
    let e = args
        .get("end")
        .and_then(|v| v.as_str())
        .ok_or("'end' is required")?;
    let pt = args
        .get("date")
        .and_then(|v| v.as_str())
        .ok_or("'date' is required")?;
    let inclusive = args
        .get("inclusive")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);

    let es = parse_to_epoch(s)?;
    let ee = parse_to_epoch(e)?;
    let ep = parse_to_epoch(pt)?;

    if es > ee {
        return Err(format!("'start' ({s}) must be ≤ 'end' ({e})"));
    }

    let has_time = has_time_component(s) || has_time_component(pt);

    let inside = if inclusive {
        ep >= es && ep <= ee
    } else {
        ep > es && ep < ee
    };

    let mut out = format!("Interval:  {} → {}\nDate:      {}\n", s, e, pt);
    out.push_str(&format!(
        "Boundary:  {}\n",
        if inclusive { "inclusive" } else { "exclusive" }
    ));
    out.push_str(&"─".repeat(50));
    out.push('\n');
    out.push_str(&format!(
        "Result:    {} {}\n",
        if inside { "INSIDE ✓" } else { "OUTSIDE ✗" },
        ""
    ));

    if !inside {
        if ep < es {
            let before = es - ep;
            out.push_str(&format!(
                "           {} before interval start\n",
                format_duration(before)
            ));
        } else {
            let after = ep - ee;
            out.push_str(&format!(
                "           {} after interval end\n",
                format_duration(after)
            ));
        }
    } else {
        let from_start = ep - es;
        let to_end = ee - ep;
        out.push_str(&format!(
            "           {} from start, {} to end\n",
            format_duration(from_start),
            format_duration(to_end)
        ));
        let _ = has_time;
    }
    Ok(out)
}

fn action_union(args: &Value) -> Result<String, String> {
    let arr = args
        .get("intervals")
        .and_then(|v| v.as_array())
        .ok_or("'intervals' array of {start, end} objects is required")?;

    if arr.is_empty() {
        return Ok("No intervals provided.\n".to_string());
    }

    let mut parsed: Vec<(i64, i64, String, String)> = Vec::new();
    for (i, item) in arr.iter().enumerate() {
        let s = item
            .get("start")
            .and_then(|v| v.as_str())
            .ok_or_else(|| format!("intervals[{i}] missing 'start'"))?;
        let e = item
            .get("end")
            .and_then(|v| v.as_str())
            .ok_or_else(|| format!("intervals[{i}] missing 'end'"))?;
        let es = parse_to_epoch(s)?;
        let ee = parse_to_epoch(e)?;
        if es > ee {
            return Err(format!("intervals[{i}]: start ({s}) must be ≤ end ({e})"));
        }
        parsed.push((es, ee, s.to_string(), e.to_string()));
    }

    let has_time = parsed
        .iter()
        .any(|(_, _, s, e)| has_time_component(s) || has_time_component(e));

    // Sort by start
    parsed.sort_by_key(|(s, _, _, _)| *s);

    // Merge overlapping intervals
    let mut merged: Vec<(i64, i64)> = Vec::new();
    for (s, e, _, _) in &parsed {
        if let Some(last) = merged.last_mut() {
            if *s <= last.1 {
                last.1 = last.1.max(*e);
                continue;
            }
        }
        merged.push((*s, *e));
    }

    let mut out = format!("Input intervals:  {}\n", parsed.len());
    out.push_str(&format!("Merged result:    {}\n", merged.len()));
    out.push_str(&"─".repeat(50));
    out.push('\n');
    for (i, (s, e)) in merged.iter().enumerate() {
        let dur = e - s;
        out.push_str(&format!(
            "  [{}]  {} → {}  ({})\n",
            i + 1,
            format_dt(*s, has_time),
            format_dt(*e, has_time),
            format_duration(dur)
        ));
    }
    if parsed.len() > merged.len() {
        out.push_str(&format!(
            "\n{} overlapping interval(s) merged.\n",
            parsed.len() - merged.len()
        ));
    }
    Ok(out)
}

fn action_intersect(args: &Value) -> Result<String, String> {
    let s1 = args
        .get("start")
        .and_then(|v| v.as_str())
        .ok_or("'start' is required")?;
    let e1 = args
        .get("end")
        .and_then(|v| v.as_str())
        .ok_or("'end' is required")?;
    let s2 = args
        .get("start2")
        .and_then(|v| v.as_str())
        .ok_or("'start2' is required")?;
    let e2 = args
        .get("end2")
        .and_then(|v| v.as_str())
        .ok_or("'end2' is required")?;

    let a0 = parse_to_epoch(s1)?;
    let a1 = parse_to_epoch(e1)?;
    let b0 = parse_to_epoch(s2)?;
    let b1 = parse_to_epoch(e2)?;

    if a0 > a1 {
        return Err(format!("'start' ({s1}) must be ≤ 'end' ({e1})"));
    }
    if b0 > b1 {
        return Err(format!("'start2' ({s2}) must be ≤ 'end2' ({e2})"));
    }

    let has_time = has_time_component(s1) || has_time_component(s2);

    let mut out = format!("Interval A:  {} → {}\n", s1, e1);
    out.push_str(&format!("Interval B:  {} → {}\n", s2, e2));
    out.push_str(&"─".repeat(50));
    out.push('\n');

    let i_start = a0.max(b0);
    let i_end = a1.min(b1);

    if i_start <= i_end {
        let dur = i_end - i_start;
        out.push_str("Result:      INTERSECTION EXISTS ✓\n");
        out.push_str(&format!(
            "Intersection: {} → {}\n",
            format_dt(i_start, has_time),
            format_dt(i_end, has_time)
        ));
        out.push_str(&format!("Duration:    {}\n", format_duration(dur)));
    } else {
        out.push_str("Result:      NO INTERSECTION ✗\n");
        let gap = i_start - i_end;
        out.push_str(&format!("Gap:         {}\n", format_duration(gap)));
    }
    Ok(out)
}

fn action_duration(args: &Value) -> Result<String, String> {
    // Can take start+end or just two dates
    let s = args
        .get("start")
        .and_then(|v| v.as_str())
        .ok_or("'start' is required")?;
    let e = args
        .get("end")
        .and_then(|v| v.as_str())
        .ok_or("'end' is required")?;

    let es = parse_to_epoch(s)?;
    let ee = parse_to_epoch(e)?;
    let delta = ee - es;

    let mut out = format!("From:  {}\nTo:    {}\n", s, e);
    out.push_str(&"─".repeat(50));
    out.push('\n');
    out.push_str(&format!("Duration:  {}\n\n", format_duration(delta)));
    out.push_str(&format_duration_verbose(delta));
    out.push('\n');
    if delta < 0 {
        out.push_str("\n(end is before start — negative duration)\n");
    }
    Ok(out)
}

fn action_schedule(args: &Value) -> Result<String, String> {
    let s = args
        .get("start")
        .and_then(|v| v.as_str())
        .ok_or("'start' is required")?;
    let step_str = args.get("step").and_then(|v| v.as_str()).unwrap_or("1d");
    let count = args
        .get("count")
        .and_then(|v| v.as_u64())
        .unwrap_or(10)
        .min(365) as usize;

    let step = parse_step(step_str)?;
    let has_time = has_time_component(s);

    let mut current = parse_to_epoch(s)?;

    let mut out = format!("Start:  {}\nStep:   {}\nCount:  {}\n", s, step_str, count);
    out.push_str(&"─".repeat(50));
    out.push('\n');

    for i in 0..count {
        out.push_str(&format!(
            "  {:>3}.  {}\n",
            i + 1,
            format_dt(current, has_time)
        ));
        current = apply_step(current, step);
    }
    Ok(out)
}

// ── entry point ───────────────────────────────────────────────────────────────

pub async fn execute(args: &Value) -> Result<String, String> {
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("overlap");

    match action {
        "overlap" => action_overlap(args),
        "contains" => action_contains(args),
        "union" => action_union(args),
        "intersect" => action_intersect(args),
        "duration" => action_duration(args),
        "schedule" => action_schedule(args),
        _ => Err(format!(
            "Unknown action '{action}'. Valid: overlap, contains, union, intersect, duration, schedule"
        )),
    }
}
