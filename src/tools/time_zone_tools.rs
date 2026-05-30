use serde_json::Value;

pub async fn execute(args: &Value) -> Result<String, String> {
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("convert");
    match action {
        "convert" => action_convert(args),
        "list" => action_list(args),
        "offset" => action_offset(args),
        "world_clock" | "world-clock" => action_world_clock(args),
        other => Err(format!(
            "time_zone_tools: unknown action '{other}'. Valid: convert, list, offset, world_clock"
        )),
    }
}

// ── Timezone database ─────────────────────────────────────────────────────────

/// Returns offset in minutes from UTC. DST not applied (uses standard offset).
fn tz_offset_minutes(tz: &str) -> Option<i32> {
    let tz = tz.trim().to_uppercase();
    // Try numeric offsets first: "+05:30", "-07:00", "+0530"
    if tz.starts_with('+') || tz.starts_with('-') {
        let sign = if tz.starts_with('-') { -1 } else { 1 };
        let s = &tz[1..];
        // Try HH:MM
        if let Some((h, m)) = s.split_once(':') {
            if let (Ok(h), Ok(m)) = (h.parse::<i32>(), m.parse::<i32>()) {
                return Some(sign * (h * 60 + m));
            }
        }
        // Try HHMM
        if s.len() == 4 {
            if let (Ok(h), Ok(m)) = (s[..2].parse::<i32>(), s[2..].parse::<i32>()) {
                return Some(sign * (h * 60 + m));
            }
        }
        // Try HH
        if let Ok(h) = s.parse::<i32>() {
            return Some(sign * h * 60);
        }
        return None;
    }

    // Named timezone table (standard offsets, no DST)
    let offset = match tz.as_str() {
        // UTC family
        "UTC" | "GMT" | "Z" | "WET" => 0,
        // Americas
        "NT" | "NST" => -210, // Newfoundland -3:30
        "AST" | "ADT" => -240,
        "EST" => -300,
        "EDT" | "CST" => -360,
        "CDT" | "MST" => -420,
        "MDT" | "PST" | "PSD" => -480,
        "PDT" | "AKST" => -540,
        "AKDT" | "HST" => -600,
        "SST" | "HAST" => -660,
        "HADT" => -540,
        // South America
        "BRT" | "BRS" => -180,
        "ART" => -180,
        "COT" => -300,
        "PET" => -300,
        "CLT" => -240,
        "VET" => -270,
        "BOT" => -240,
        "PYT" => -240,
        "GYT" | "SRT" => -240,
        "ECT" => -300,
        // Europe / Africa
        "CET" | "MET" | "WAT" => 60,
        "CEST" | "BST" | "IST" | "WEST" => 60, // approximation
        "EET" | "CAT" => 120,
        "EEST" | "FET" | "SAST" => 120,
        "MSK" => 180,
        "EAT" => 180,
        "TRT" | "AST_T" => 180, // Turkey / Arabia
        "SAMT" | "AZT" => 240,
        "GST_G" | "SCT" | "MUT" | "RET" => 240, // Gulf
        // Middle East / Asia
        "IRST" => 210,  // Iran +3:30
        "IST_I" => 330, // India +5:30 (conflict alias)
        "NPT" => 345,   // Nepal +5:45
        "PKT" => 300,
        "UZT" => 300,
        "AFT" => 270,           // Afghanistan +4:30
        "MMT" => 390,           // Myanmar +6:30
        "CCT" | "IOT" => 360,   // Cocos/Indian Ocean
        "BST_B" | "BDT" => 360, // Bangladesh +6
        "ICT" => 420,
        "WIB" => 420,
        "PHT" | "CST_C" | "AWST" | "MYT" | "HKT" | "SGT" | "WITA" => 480,
        "JST" | "KST" | "WIT" | "TLT" => 540,
        "AEST" | "PGT" | "CHST" => 600,
        "AEDT" | "NFT" | "SBT" => 660,
        "NZST" | "FJT" | "TVT" | "WAKT" | "MHT" | "GILT" => 720,
        "NZDT" | "PHOT" | "TOT" | "LINT" => 780,
        // Pacific
        "CHAST" => 765, // Chatham +12:45
        "TOST" => 840,  // Tonga Summer +14
        "LINT_S" => 840,
        // Special
        "IST" => 330,  // When used alone, assume India
        "CST" => -360, // When used alone, assume US Central (EST-1)
        "GST" => 240,  // Gulf Standard Time
        _ => return None,
    };
    Some(offset)
}

fn tz_name_for_display(tz: &str) -> String {
    tz.trim().to_string()
}

fn format_offset(minutes: i32) -> String {
    let sign = if minutes >= 0 { '+' } else { '-' };
    let abs = minutes.abs();
    let h = abs / 60;
    let m = abs % 60;
    if m == 0 {
        format!("UTC{}{:02}", sign, h)
    } else {
        format!("UTC{}{}:{:02}", sign, h, m)
    }
}

/// Parse a datetime string into (year, month, day, hour, minute, second).
/// Supports: "2024-06-15 14:30:00", "2024-06-15T14:30:00", "2024-06-15 14:30"
fn parse_datetime(s: &str) -> Option<(i32, u32, u32, u32, u32, u32)> {
    let s = s.trim().replace('T', " ");
    let parts: Vec<&str> = s.splitn(2, ' ').collect();
    if parts.is_empty() {
        return None;
    }
    let date_parts: Vec<u32> = parts[0].split('-').filter_map(|p| p.parse().ok()).collect();
    if date_parts.len() < 3 {
        return None;
    }
    let (y, mo, d) = (date_parts[0] as i32, date_parts[1], date_parts[2]);

    let (h, mi, s) = if parts.len() > 1 {
        let time_parts: Vec<u32> = parts[1].split(':').filter_map(|p| p.parse().ok()).collect();
        (
            *time_parts.first().unwrap_or(&0),
            *time_parts.get(1).unwrap_or(&0),
            *time_parts.get(2).unwrap_or(&0),
        )
    } else {
        (0, 0, 0)
    };

    Some((y, mo, d, h, mi, s))
}

/// Convert a datetime by adjusting the offset difference.
/// Returns the adjusted (y, mo, d, h, mi) with correct date rollover.
fn adjust_datetime(
    y: i32,
    mo: u32,
    d: u32,
    h: u32,
    mi: u32,
    delta_minutes: i32,
) -> (i32, u32, u32, u32, u32) {
    let total = h as i32 * 60 + mi as i32 + delta_minutes;
    let (new_h, new_mi) = {
        let min = total.rem_euclid(1440);
        (min / 60, min % 60)
    };
    let day_delta = (total.div_euclid(1440)).min(1).max(-1);
    let (new_y, new_mo, new_d) = adjust_date(y, mo, d, day_delta);
    (new_y, new_mo, new_d, new_h as u32, new_mi as u32)
}

fn days_in_month(y: i32, m: u32) -> u32 {
    match m {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if (y % 4 == 0 && y % 100 != 0) || y % 400 == 0 {
                29
            } else {
                28
            }
        }
        _ => 30,
    }
}

fn adjust_date(y: i32, m: u32, d: u32, delta_days: i32) -> (i32, u32, u32) {
    if delta_days == 0 {
        return (y, m, d);
    }
    let d_new = d as i32 + delta_days;
    if d_new < 1 {
        // go to previous month
        let (prev_m, prev_y) = if m == 1 { (12u32, y - 1) } else { (m - 1, y) };
        let days = days_in_month(prev_y, prev_m) as i32 + d_new;
        (prev_y, prev_m, days as u32)
    } else {
        let max = days_in_month(y, m) as i32;
        if d_new > max {
            let (next_m, next_y) = if m == 12 { (1u32, y + 1) } else { (m + 1, y) };
            (next_y, next_m, (d_new - max) as u32)
        } else {
            (y, m, d_new as u32)
        }
    }
}

const MONTH_NAMES: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

const DAY_NAMES: [&str; 7] = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];

/// Zeller's congruence for day-of-week (0=Mon..6=Sun).
fn day_of_week(y: i32, m: u32, d: u32) -> usize {
    let (m, y) = if m <= 2 { (m + 12, y - 1) } else { (m, y) };
    let k = y % 100;
    let j = y / 100;
    let f = (d as i32) + (13 * (m as i32 + 1)) / 5 + k + k / 4 + j / 4 - 2 * j;
    ((f - 2).rem_euclid(7)) as usize
}

// ── action: convert ───────────────────────────────────────────────────────────

fn action_convert(args: &Value) -> Result<String, String> {
    let datetime_str = args
        .get("datetime")
        .or_else(|| args.get("time"))
        .or_else(|| args.get("dt"))
        .or_else(|| args.get("input"))
        .and_then(|v| v.as_str())
        .ok_or("time_zone_tools: 'datetime' is required (e.g. '2024-06-15 14:30:00')")?;

    let from_tz = args
        .get("from")
        .or_else(|| args.get("from_tz"))
        .and_then(|v| v.as_str())
        .unwrap_or("UTC");

    let to_tz = args
        .get("to")
        .or_else(|| args.get("to_tz"))
        .and_then(|v| v.as_str())
        .ok_or("time_zone_tools: 'to' timezone is required (e.g. 'EST', '+05:30')")?;

    let from_offset = tz_offset_minutes(from_tz).ok_or_else(|| {
        format!(
            "Unknown timezone: '{}'. Use UTC offset like '+05:30' or a standard abbreviation.",
            from_tz
        )
    })?;
    let to_offset = tz_offset_minutes(to_tz).ok_or_else(|| {
        format!(
            "Unknown timezone: '{}'. Use UTC offset like '+05:30' or a standard abbreviation.",
            to_tz
        )
    })?;

    let (y, mo, d, h, mi, _sec) = parse_datetime(datetime_str).ok_or_else(|| {
        format!(
            "Cannot parse datetime: '{}'. Use 'YYYY-MM-DD HH:MM:SS'",
            datetime_str
        )
    })?;

    let delta = to_offset - from_offset;
    let (ny, nmo, nd, nh, nmi) = adjust_datetime(y, mo, d, h, mi, delta);

    let dow_from = day_of_week(y, mo, d);
    let dow_to = day_of_week(ny, nmo, nd);

    let mut out = format!("time_zone_tools — convert\n\n");
    out.push_str(&format!(
        "  From : {:04}-{:02}-{:02} {:02}:{:02}  {}  ({})\n",
        y,
        mo,
        d,
        h,
        mi,
        DAY_NAMES[dow_from],
        format_offset(from_offset)
    ));
    out.push_str(&format!(
        "  To   : {:04}-{:02}-{:02} {:02}:{:02}  {}  ({})\n",
        ny,
        nmo,
        nd,
        nh,
        nmi,
        DAY_NAMES[dow_to],
        format_offset(to_offset)
    ));
    out.push_str(&format!(
        "\n  {} → {}: {:+} hours\n",
        tz_name_for_display(from_tz),
        tz_name_for_display(to_tz),
        delta as f64 / 60.0
    ));
    if nd != d || ny != y {
        out.push_str(&format!(
            "  Note: date changes from {} {} to {} {}\n",
            d,
            MONTH_NAMES[(mo - 1) as usize],
            nd,
            MONTH_NAMES[(nmo - 1) as usize]
        ));
    }
    Ok(out)
}

// ── action: offset ────────────────────────────────────────────────────────────

fn action_offset(args: &Value) -> Result<String, String> {
    let tz = args
        .get("tz")
        .or_else(|| args.get("timezone"))
        .or_else(|| args.get("zone"))
        .or_else(|| args.get("input"))
        .and_then(|v| v.as_str())
        .ok_or("time_zone_tools: 'tz' is required")?;

    let offset = tz_offset_minutes(tz).ok_or_else(|| {
        format!(
            "Unknown timezone: '{}'. Use UTC offset like '+05:30' or a standard abbreviation.",
            tz
        )
    })?;

    let mut out = format!("time_zone_tools — offset\n\n");
    out.push_str(&format!("  Timezone : {}\n", tz));
    out.push_str(&format!("  Offset   : {}\n", format_offset(offset)));
    out.push_str(&format!("  Minutes  : {:+}\n", offset));
    out.push_str(&format!("  Hours    : {:+.2}\n", offset as f64 / 60.0));
    Ok(out)
}

// ── action: list ──────────────────────────────────────────────────────────────

fn action_list(args: &Value) -> Result<String, String> {
    let filter = args
        .get("filter")
        .or_else(|| args.get("region"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_uppercase();

    // Curated list of commonly used timezone abbreviations with descriptions
    let zones: &[(&str, i32, &str)] = &[
        ("SST", -660, "Samoa Standard (Pacific/Pago_Pago)"),
        ("HST", -600, "Hawaii Standard"),
        ("AKST", -540, "Alaska Standard"),
        ("PST", -480, "Pacific Standard (US/Canada)"),
        ("PDT", -420, "Pacific Daylight"),
        ("MST", -420, "Mountain Standard (US/Canada)"),
        ("MDT", -360, "Mountain Daylight"),
        ("CST", -360, "Central Standard (US/Canada)"),
        ("CDT", -300, "Central Daylight"),
        ("EST", -300, "Eastern Standard (US/Canada)"),
        ("EDT", -240, "Eastern Daylight"),
        ("AST", -240, "Atlantic Standard"),
        ("NST", -210, "Newfoundland Standard"),
        ("BRT", -180, "Brazil Time (Brasília)"),
        ("ART", -180, "Argentina Time"),
        ("FKST", -180, "Falkland Islands Summer"),
        ("UTC", 0, "Coordinated Universal Time"),
        ("GMT", 0, "Greenwich Mean Time"),
        ("WET", 0, "Western European Time"),
        ("BST", 60, "British Summer Time"),
        ("CET", 60, "Central European Time"),
        ("WAT", 60, "West Africa Time"),
        ("CEST", 120, "Central European Summer Time"),
        ("EET", 120, "Eastern European Time"),
        ("CAT", 120, "Central Africa Time"),
        ("SAST", 120, "South Africa Standard"),
        ("MSK", 180, "Moscow Standard Time"),
        ("EAT", 180, "East Africa Time"),
        ("TRT", 180, "Turkey Time"),
        ("AFT", 270, "Afghanistan Time (+4:30)"),
        ("GST", 240, "Gulf Standard Time"),
        ("AZT", 240, "Azerbaijan Time"),
        ("PKT", 300, "Pakistan Standard"),
        ("IST", 330, "India Standard (+5:30)"),
        ("NPT", 345, "Nepal Time (+5:45)"),
        ("BST_B", 360, "Bangladesh Standard (+6)"),
        ("MMT", 390, "Myanmar Time (+6:30)"),
        ("ICT", 420, "Indochina Time"),
        ("WIB", 420, "Western Indonesia"),
        ("AWST", 480, "Australian Western Standard"),
        ("HKT", 480, "Hong Kong Time"),
        ("SGT", 480, "Singapore Time"),
        ("CST_C", 480, "China Standard"),
        ("JST", 540, "Japan Standard"),
        ("KST", 540, "Korea Standard"),
        ("AEST", 600, "Australian Eastern Standard"),
        ("AEDT", 660, "Australian Eastern Daylight"),
        ("NZST", 720, "New Zealand Standard"),
        ("NZDT", 780, "New Zealand Daylight"),
    ];

    let mut out = format!("time_zone_tools — list\n\n");
    out.push_str(&format!(
        "  {:<12}  {:>8}  {}\n",
        "Abbrev", "UTC", "Description"
    ));
    out.push_str(&format!("  {:-<12}  {:-<8}  {:-<45}\n", "", "", ""));

    for (abbr, off, desc) in zones {
        if !filter.is_empty()
            && !abbr.contains(filter.as_str())
            && !desc.to_uppercase().contains(filter.as_str())
        {
            continue;
        }
        out.push_str(&format!(
            "  {:<12}  {:>8}  {}\n",
            abbr,
            format_offset(*off),
            desc
        ));
    }

    out.push_str(&format!(
        "\n  Use 'offset' action to look up a specific timezone.\n"
    ));
    Ok(out)
}

// ── action: world_clock ───────────────────────────────────────────────────────

fn utc_now() -> (i32, u32, u32, u32, u32) {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    // Convert Unix timestamp to (y, mo, d, h, mi)
    let s = secs as i64;
    let days = s / 86400;
    let time = s % 86400;
    let h = (time / 3600) as u32;
    let mi = ((time % 3600) / 60) as u32;
    // Civil date from days since epoch (2000-03-01 base)
    let z = days + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let mo = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let y = if mo <= 2 { y + 1 } else { y } as i32;
    (y, mo, d, h, mi)
}

fn action_world_clock(args: &Value) -> Result<String, String> {
    // datetime is optional — default to current UTC
    let (y, mo, d, h, mi) = if let Some(utc_str) = args
        .get("utc")
        .or_else(|| args.get("time"))
        .or_else(|| args.get("datetime"))
        .and_then(|v| v.as_str())
    {
        let (y, mo, d, h, mi, _sec) = parse_datetime(utc_str).ok_or_else(|| {
            format!(
                "Cannot parse datetime: '{}'. Use 'YYYY-MM-DD HH:MM'",
                utc_str
            )
        })?;
        (y, mo, d, h, mi)
    } else {
        utc_now()
    };

    // Default cities with their timezone offsets
    let default_cities: &[(&str, i32)] = &[
        ("London (GMT/BST)", 0),
        ("Paris / Berlin (CET)", 60),
        ("Moscow (MSK)", 180),
        ("Dubai (GST)", 240),
        ("Mumbai (IST)", 330),
        ("Singapore / HK", 480),
        ("Tokyo / Seoul", 540),
        ("Sydney (AEST)", 600),
        ("New York (EST)", -300),
        ("Chicago (CST)", -360),
        ("Denver (MST)", -420),
        ("Los Angeles (PST)", -480),
        ("Honolulu (HST)", -600),
    ];

    // Allow custom list of timezones — accept both string array and object array
    let custom: Option<Vec<(String, i32)>> =
        args.get("zones").and_then(|v| v.as_array()).map(|arr| {
            arr.iter()
                .filter_map(|z| {
                    // String form: "JST"
                    if let Some(s) = z.as_str() {
                        return tz_offset_minutes(s).map(|off| (s.to_string(), off));
                    }
                    // Object form: {name: "Tokyo", tz: "JST"}
                    let name = z
                        .get("name")
                        .or_else(|| z.get("tz"))
                        .and_then(|v| v.as_str());
                    let tz = z
                        .get("tz")
                        .or_else(|| z.get("offset"))
                        .and_then(|v| v.as_str());
                    if let (Some(name), Some(tz)) = (name, tz) {
                        tz_offset_minutes(tz).map(|off| (name.to_string(), off))
                    } else {
                        None
                    }
                })
                .collect()
        });

    let mut out = format!("time_zone_tools — world clock\n\n");
    out.push_str(&format!(
        "  UTC : {:04}-{:02}-{:02} {:02}:{:02}\n\n",
        y, mo, d, h, mi
    ));
    out.push_str(&format!("  {:<30}  {}\n", "City / Zone", "Local Time"));
    out.push_str(&format!("  {:-<30}  {:-<20}\n", "", ""));

    if let Some(custom_zones) = custom {
        for (name, off) in &custom_zones {
            let (ny, nmo, nd, nh, nmi) = adjust_datetime(y, mo, d, h, mi, *off);
            let dow = day_of_week(ny, nmo, nd);
            out.push_str(&format!(
                "  {:<30}  {:02}:{:02}  {:04}-{:02}-{:02} ({})\n",
                name, nh, nmi, ny, nmo, nd, DAY_NAMES[dow]
            ));
        }
    } else {
        for (name, off) in default_cities {
            let (ny, nmo, nd, nh, nmi) = adjust_datetime(y, mo, d, h, mi, *off);
            let dow = day_of_week(ny, nmo, nd);
            let date_note = if nd != d || ny != y {
                format!(" {:02} {}", nd, MONTH_NAMES[(nmo - 1) as usize])
            } else {
                String::new()
            };
            out.push_str(&format!(
                "  {:<30}  {:02}:{:02}{}  ({})\n",
                name, nh, nmi, date_note, DAY_NAMES[dow]
            ));
        }
    }

    Ok(out)
}
