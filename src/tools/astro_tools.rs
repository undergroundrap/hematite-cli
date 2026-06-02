use serde_json::{json, Value};
use std::f64::consts::PI;

pub fn astro_tools_schema() -> Value {
    json!({
        "name": "astro_tools",
        "description": "Astronomy calculations: planet positions, rise/set times, angular separation, magnitude, distance conversions, and constellation lookup. All calculations are offline, no API required.",
        "input_schema": {
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["planet", "rise_set", "separation", "magnitude", "distance", "constellation", "moon_phase", "julian"],
                    "description": "Action to perform (default: planet)"
                },
                "body": {"type": "string", "description": "Planet or solar system body name"},
                "ra": {"type": "number", "description": "Right Ascension in decimal degrees"},
                "dec": {"type": "number", "description": "Declination in decimal degrees"},
                "ra2": {"type": "number", "description": "Second RA in decimal degrees (for separation)"},
                "dec2": {"type": "number", "description": "Second Dec in decimal degrees (for separation)"},
                "lat": {"type": "number", "description": "Observer latitude in decimal degrees"},
                "lon": {"type": "number", "description": "Observer longitude in decimal degrees (negative = west)"},
                "date": {"type": "string", "description": "Date as YYYY-MM-DD (defaults to today)"},
                "value": {"type": "number", "description": "Numeric value for distance conversion"},
                "from_unit": {"type": "string", "description": "Source unit: au/ly/pc/km/m"},
                "to_unit": {"type": "string", "description": "Target unit: au/ly/pc/km/m"},
                "flux": {"type": "number", "description": "Flux ratio for magnitude calculation"},
                "mag1": {"type": "number", "description": "First apparent magnitude"},
                "mag2": {"type": "number", "description": "Second apparent magnitude"},
                "jd": {"type": "number", "description": "Julian Date for conversion"},
                "query": {"type": "string", "description": "Constellation name or abbreviation"}
            }
        }
    })
}

// ── date helpers ────────────────────────────────────────────────────────────

fn parse_date(s: &str) -> Option<(i32, u32, u32)> {
    let parts: Vec<&str> = s.splitn(3, '-').collect();
    if parts.len() != 3 {
        return None;
    }
    let y: i32 = parts[0].parse().ok()?;
    let m: u32 = parts[1].parse().ok()?;
    let d: u32 = parts[2].parse().ok()?;
    Some((y, m, d))
}

fn calendar_to_jd(y: i32, m: u32, d: u32) -> f64 {
    let (y, m) = if m <= 2 { (y - 1, m + 12) } else { (y, m) };
    let a = (y as f64 / 100.0).floor() as i32;
    let b = 2 - a + (a / 4);
    (365.25 * (y + 4716) as f64).floor() + (30.6001 * (m + 1) as f64).floor() + d as f64 + b as f64
        - 1524.5
}

fn jd_to_calendar(jd: f64) -> (i32, u32, u32) {
    let z = (jd + 0.5).floor() as i64;
    let f = jd + 0.5 - z as f64;
    let alpha = ((z as f64 - 1867216.25) / 36524.25).floor() as i64;
    let a = z + 1 + alpha - alpha / 4;
    let b = a + 1524;
    let c = ((b as f64 - 122.1) / 365.25).floor() as i64;
    let d = (365.25 * c as f64).floor() as i64;
    let e = ((b - d) as f64 / 30.6001).floor() as i64;
    let day = (b - d - (30.6001 * e as f64).floor() as i64) as u32 + f as u32;
    let month = if e < 14 { e - 1 } else { e - 13 } as u32;
    let year = if month > 2 { c - 4716 } else { c - 4715 } as i32;
    (year, month, day)
}

fn today_jd() -> f64 {
    // Approximate current JD from system time
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    2440587.5 + secs as f64 / 86400.0
}

fn julian_day_number(date_str: &str) -> f64 {
    if let Some((y, m, d)) = parse_date(date_str) {
        calendar_to_jd(y, m, d)
    } else {
        today_jd()
    }
}

// ── angular helpers ─────────────────────────────────────────────────────────

fn deg_to_rad(d: f64) -> f64 {
    d * PI / 180.0
}
fn rad_to_deg(r: f64) -> f64 {
    r * 180.0 / PI
}
fn norm360(x: f64) -> f64 {
    ((x % 360.0) + 360.0) % 360.0
}
fn norm_pi(x: f64) -> f64 {
    let mut v = x;
    while v > PI {
        v -= 2.0 * PI;
    }
    while v < -PI {
        v += 2.0 * PI;
    }
    v
}

fn hms(degrees: f64) -> String {
    let total_sec = (degrees / 15.0 * 3600.0).abs();
    let h = (total_sec / 3600.0) as u32;
    let m = ((total_sec % 3600.0) / 60.0) as u32;
    let s = total_sec % 60.0;
    format!("{:02}h {:02}m {:05.2}s", h, m, s)
}

fn dms(degrees: f64) -> String {
    let sign = if degrees < 0.0 { "-" } else { "+" };
    let abs = degrees.abs();
    let d = abs.floor() as u32;
    let m = ((abs - d as f64) * 60.0).floor() as u32;
    let s = ((abs - d as f64) * 60.0 - m as f64) * 60.0;
    format!("{}{:02}° {:02}′ {:05.2}″", sign, d, m, s)
}

// ── planet data (mean orbital elements at J2000, simplified VSOP) ───────────

#[allow(dead_code)]
struct Planet {
    name: &'static str,
    symbol: &'static str,
    a: f64,  // semi-major axis AU
    e: f64,  // eccentricity
    i: f64,  // inclination deg
    l: f64,  // mean longitude deg at J2000
    lp: f64, // longitude of perihelion deg
    om: f64, // longitude of ascending node deg
    // rates per century
    da: f64,
    de: f64,
    di: f64,
    dl: f64,
    dlp: f64,
    dom: f64,
    mag_a: f64,
    mag_b: f64, // magnitude formula: mag_a + 5*log10(r*delta) + mag_b*(phase_angle/100)
}

static PLANETS: &[Planet] = &[
    Planet {
        name: "Mercury",
        symbol: "☿",
        a: 0.38709927,
        e: 0.20563593,
        i: 7.00497902,
        l: 252.25032350,
        lp: 77.45779628,
        om: 48.33076593,
        da: 0.00000037,
        de: 0.00001906,
        di: -0.00594749,
        dl: 149472.67411175,
        dlp: 0.16047689,
        dom: -0.12534081,
        mag_a: -0.60,
        mag_b: 0.04,
    },
    Planet {
        name: "Venus",
        symbol: "♀",
        a: 0.72333566,
        e: 0.00677672,
        i: 3.39467605,
        l: 181.97909950,
        lp: 131.60246718,
        om: 76.67984255,
        da: 0.00000390,
        de: -0.00004107,
        di: -0.00078890,
        dl: 58517.81538729,
        dlp: 0.00268329,
        dom: -0.27769418,
        mag_a: -4.40,
        mag_b: 0.01,
    },
    Planet {
        name: "Earth",
        symbol: "⊕",
        a: 1.00000261,
        e: 0.01671123,
        i: -0.00001531,
        l: 100.46457166,
        lp: 102.93768193,
        om: 0.0,
        da: 0.00000562,
        de: -0.00004392,
        di: -0.01294668,
        dl: 35999.37244981,
        dlp: 0.32327364,
        dom: 0.0,
        mag_a: -3.86,
        mag_b: 0.0,
    },
    Planet {
        name: "Mars",
        symbol: "♂",
        a: 1.52371034,
        e: 0.09339410,
        i: 1.84969142,
        l: -4.55343205,
        lp: -23.94362959,
        om: 49.55953891,
        da: 0.00001847,
        de: 0.00007882,
        di: -0.00813131,
        dl: 19140.30268499,
        dlp: 0.44441088,
        dom: -0.29257343,
        mag_a: -1.52,
        mag_b: 0.02,
    },
    Planet {
        name: "Jupiter",
        symbol: "♃",
        a: 5.20288700,
        e: 0.04838624,
        i: 1.30439695,
        l: 34.39644051,
        lp: 14.72847983,
        om: 100.47390909,
        da: -0.00011607,
        de: -0.00013253,
        di: -0.00183714,
        dl: 3034.74612775,
        dlp: 0.21252668,
        dom: 0.20469106,
        mag_a: -9.40,
        mag_b: 0.01,
    },
    Planet {
        name: "Saturn",
        symbol: "♄",
        a: 9.53667594,
        e: 0.05386179,
        i: 2.48599187,
        l: 49.95424423,
        lp: 92.59887831,
        om: 113.66242448,
        da: -0.00125060,
        de: -0.00050991,
        di: 0.00193609,
        dl: 1222.49362201,
        dlp: -0.41897216,
        dom: -0.28867794,
        mag_a: -8.88,
        mag_b: 0.04,
    },
    Planet {
        name: "Uranus",
        symbol: "⛢",
        a: 19.18916464,
        e: 0.04725744,
        i: 0.77263783,
        l: 313.23810451,
        lp: 170.95427630,
        om: 74.01692503,
        da: -0.00196176,
        de: -0.00004397,
        di: -0.00242939,
        dl: 428.48202785,
        dlp: 0.40805281,
        dom: 0.04240589,
        mag_a: -7.19,
        mag_b: 0.0,
    },
    Planet {
        name: "Neptune",
        symbol: "♆",
        a: 30.06992276,
        e: 0.00859048,
        i: 1.77004347,
        l: -55.12002969,
        lp: 44.96476227,
        om: 131.78422574,
        da: 0.00026291,
        de: 0.00005105,
        di: 0.00035372,
        dl: 218.45945325,
        dlp: -0.32241464,
        dom: -0.00508664,
        mag_a: -6.87,
        mag_b: 0.0,
    },
];

fn planet_helio(p: &Planet, t: f64) -> (f64, f64, f64) {
    // t = centuries since J2000
    let a = p.a + p.da * t;
    let e = p.e + p.de * t;
    let _i = p.i + p.di * t;
    let l = norm360(p.l + p.dl * t);
    let lp = norm360(p.lp + p.dlp * t);
    let _om = norm360(p.om + p.dom * t);

    // Mean anomaly
    let m = deg_to_rad(norm360(l - lp));
    // Eccentric anomaly (Newton's method)
    let mut ea = m;
    for _ in 0..10 {
        ea -= (ea - e * ea.sin() - m) / (1.0 - e * ea.cos());
    }
    // True anomaly
    let nu = 2.0 * ((((1.0 + e) / (1.0 - e)).sqrt()) * (ea / 2.0).tan()).atan();
    let r = a * (1.0 - e * ea.cos());
    let lon = rad_to_deg(nu) + lp;
    (r, norm360(lon), _i)
}

fn planet_geo(p: &Planet, t: f64) -> (f64, f64, f64) {
    let earth = &PLANETS[2];
    let (re, le, _) = planet_helio(earth, t);
    let (rp, lp_l, _) = planet_helio(p, t);

    let lp_r = deg_to_rad(lp_l);
    let le_r = deg_to_rad(le);

    let xp = rp * lp_r.cos();
    let yp = rp * lp_r.sin();
    let xe = re * le_r.cos();
    let ye = re * le_r.sin();

    let dx = xp - xe;
    let dy = yp - ye;
    let delta = (dx * dx + dy * dy).sqrt();
    let ra = norm360(rad_to_deg(dy.atan2(dx)));
    let dec = 0.0; // simplified: ecliptic coords → RA/Dec conversion omitted for brevity
    (ra, dec, delta)
}

fn action_planet(args: &Value) -> String {
    let date_str = args["date"].as_str().unwrap_or("");
    let jd = if date_str.is_empty() {
        today_jd()
    } else {
        julian_day_number(date_str)
    };
    let t = (jd - 2451545.0) / 36525.0;

    let body_filter = args["body"].as_str().unwrap_or("").to_lowercase();

    let (y, m, d) = jd_to_calendar(jd);
    let mut out = format!(
        "Planetary Positions — {}-{:02}-{:02} (JD {:.1})\n",
        y, m, d, jd
    );
    out.push_str(&format!("T = {:.6} centuries since J2000\n\n", t));
    out.push_str(&format!(
        "{:<10} {:<6} {:>12} {:>12} {:>10} {:>8}\n",
        "Planet", "Sym", "Helio Long", "Helio Dist", "Geo Dist", "Est Mag"
    ));
    out.push_str(&"─".repeat(64));
    out.push('\n');

    for p in PLANETS
        .iter()
        .filter(|p| body_filter.is_empty() || p.name.to_lowercase().contains(&body_filter))
    {
        if p.name == "Earth" {
            continue;
        }
        let (r, lon, _) = planet_helio(p, t);
        let (_, _, delta) = planet_geo(p, t);
        let mag = p.mag_a + 5.0 * (r * delta).log10();
        out.push_str(&format!(
            "{:<10} {:<6} {:>12.4}° {:>10.4} AU {:>8.4} AU {:>8.2}\n",
            p.name, p.symbol, lon, r, delta, mag
        ));
    }

    // Append note about RA/Dec
    out.push_str("\nNote: Positions use simplified mean orbital elements (VSOP-lite).\n");
    out.push_str("For precise RA/Dec, use a full VSOP87 implementation.\n");
    out
}

// ── rise/set ─────────────────────────────────────────────────────────────────

fn action_rise_set(args: &Value) -> String {
    let ra = match args["ra"].as_f64() {
        Some(v) => v,
        None => return "Error: 'ra' (Right Ascension in degrees) is required.".into(),
    };
    let dec = match args["dec"].as_f64() {
        Some(v) => v,
        None => return "Error: 'dec' (Declination in degrees) is required.".into(),
    };
    let lat = match args["lat"].as_f64() {
        Some(v) => v,
        None => return "Error: 'lat' (observer latitude in degrees) is required.".into(),
    };
    let lon = args["lon"].as_f64().unwrap_or(0.0);
    let date_str = args["date"].as_str().unwrap_or("");
    let jd0 = if date_str.is_empty() {
        today_jd().floor()
    } else {
        julian_day_number(date_str).floor()
    };
    let (y, m, d) = jd_to_calendar(jd0);

    // Hour angle at rise/set: cos(H) = (sin(h0) - sin(lat)*sin(dec)) / (cos(lat)*cos(dec))
    // h0 = -0.5667° for stars (refraction), -0.8333° for sun
    let h0 = -0.5667_f64;
    let cos_h = (deg_to_rad(h0).sin() - deg_to_rad(lat).sin() * deg_to_rad(dec).sin())
        / (deg_to_rad(lat).cos() * deg_to_rad(dec).cos());

    if cos_h < -1.0 {
        return format!("Object is circumpolar at lat {:.2}° — never sets.", lat);
    }
    if cos_h > 1.0 {
        return format!("Object never rises at lat {:.2}°.", lat);
    }

    let h = rad_to_deg(cos_h.acos());
    // GMST at J2000
    let t = (jd0 - 2451545.0) / 36525.0;
    let gmst0 = norm360(100.4606184 + 36000.77004 * t + 0.000387933 * t * t);
    // LST at transit
    let lst_transit = norm360(ra - lon);
    let transit_ut = norm_pi(deg_to_rad(lst_transit - gmst0)) / (2.0 * PI) * 24.0;
    let transit_h = ((transit_ut % 24.0) + 24.0) % 24.0;
    let rise_h = ((transit_h - h / 15.0) % 24.0 + 24.0) % 24.0;
    let set_h = ((transit_h + h / 15.0) % 24.0 + 24.0) % 24.0;

    fn hm(h: f64) -> String {
        let hi = h as u32;
        let mi = ((h - hi as f64) * 60.0).round() as u32;
        format!("{:02}:{:02} UT", hi % 24, mi % 60)
    }

    let mut out = format!("Rise/Set for RA={} Dec={}\n", hms(ra), dms(dec));
    out.push_str(&format!(
        "Date: {}-{:02}-{:02}  Observer: lat {:.4}°, lon {:.4}°\n\n",
        y, m, d, lat, lon
    ));
    out.push_str(&format!("  Rise:    {}\n", hm(rise_h)));
    out.push_str(&format!("  Transit: {}\n", hm(transit_h)));
    out.push_str(&format!("  Set:     {}\n", hm(set_h)));
    out.push_str(&format!(
        "\nHour angle at rise/set: {:.4}° ({:.4} h)\n",
        h,
        h / 15.0
    ));
    out.push_str("Note: times are approximate UT; add timezone offset for local time.\n");
    out
}

// ── angular separation ───────────────────────────────────────────────────────

fn action_separation(args: &Value) -> String {
    let ra1 = match args["ra"].as_f64() {
        Some(v) => v,
        None => return "Error: 'ra' (first RA in degrees) is required.".into(),
    };
    let dec1 = match args["dec"].as_f64() {
        Some(v) => v,
        None => return "Error: 'dec' (first Dec in degrees) is required.".into(),
    };
    let ra2 = match args["ra2"].as_f64() {
        Some(v) => v,
        None => return "Error: 'ra2' (second RA in degrees) is required.".into(),
    };
    let dec2 = match args["dec2"].as_f64() {
        Some(v) => v,
        None => return "Error: 'dec2' (second Dec in degrees) is required.".into(),
    };

    // Haversine formula on sphere
    let d_ra = deg_to_rad(ra2 - ra1);
    let d_dec = deg_to_rad(dec2 - dec1);
    let a = (d_dec / 2.0).sin().powi(2)
        + deg_to_rad(dec1).cos() * deg_to_rad(dec2).cos() * (d_ra / 2.0).sin().powi(2);
    let sep_rad = 2.0 * a.sqrt().asin();
    let sep_deg = rad_to_deg(sep_rad);
    let sep_min = sep_deg * 60.0;
    let sep_sec = sep_deg * 3600.0;

    let mut out = String::from("Angular Separation\n\n");
    out.push_str(&format!("  Object 1: RA {} Dec {}\n", hms(ra1), dms(dec1)));
    out.push_str(&format!(
        "  Object 2: RA {} Dec {}\n\n",
        hms(ra2),
        dms(dec2)
    ));
    out.push_str(&format!("  Separation: {:.6}°\n", sep_deg));
    out.push_str(&format!("            = {:.4}′\n", sep_min));
    out.push_str(&format!("            = {:.2}″\n", sep_sec));
    if sep_deg > 1.0 {
        out.push_str(&format!(
            "            ≈ {} arcminutes\n",
            sep_min.round() as u32
        ));
    }
    out
}

// ── magnitude ────────────────────────────────────────────────────────────────

fn action_magnitude(args: &Value) -> String {
    let mut out = String::from("Astronomical Magnitude\n\n");

    if let (Some(mag1), Some(mag2)) = (args["mag1"].as_f64(), args["mag2"].as_f64()) {
        let ratio = 10_f64.powf((mag2 - mag1) / 2.5);
        out.push_str(&format!("mag1 = {:.2}  mag2 = {:.2}\n", mag1, mag2));
        out.push_str(&format!("  Δmag = {:.4}\n", (mag2 - mag1).abs()));
        out.push_str(&format!(
            "  Flux ratio = {:.6} (object 1 is {:.2}× {})\n",
            ratio,
            if ratio >= 1.0 { ratio } else { 1.0 / ratio },
            if mag1 < mag2 { "brighter" } else { "fainter" }
        ));
    } else if let Some(flux) = args["flux"].as_f64() {
        let delta_mag = -2.5 * flux.abs().log10();
        out.push_str(&format!("Flux ratio: {:.6}\n", flux));
        out.push_str(&format!("  Δmag = {:.4}\n", delta_mag));
        out.push_str(&format!(
            "  If reference mag = 0: result = {:.4}\n",
            delta_mag
        ));
    } else {
        // Reference table of famous objects
        out.push_str("Apparent magnitude scale (lower = brighter):\n\n");
        let objects = &[
            ("Sun", -26.74),
            ("Full Moon", -12.74),
            ("Venus (max)", -4.89),
            ("Jupiter (max)", -2.94),
            ("Mars (max)", -2.91),
            ("Sirius", -1.46),
            ("Canopus", -0.74),
            ("Alpha Centauri A", -0.01),
            ("Vega", 0.03),
            ("Naked-eye limit", 6.5),
            ("Binocular limit", 10.0),
            ("Hubble limit", 31.5),
        ];
        for (name, mag) in objects {
            out.push_str(&format!("  {:<25} {:>7.2}\n", name, mag));
        }
    }
    out
}

// ── distance conversion ──────────────────────────────────────────────────────

const AU_M: f64 = 1.495978707e11;
const LY_M: f64 = 9.4607304725808e15;
const PC_M: f64 = 3.085677581491367e16;
const PC_LY: f64 = PC_M / LY_M;
const PC_AU: f64 = PC_M / AU_M;

fn to_meters(val: f64, unit: &str) -> Option<f64> {
    match unit.to_lowercase().trim_end_matches('s') {
        "au" | "astronomical unit" => Some(val * AU_M),
        "ly" | "light-year" | "lightyear" => Some(val * LY_M),
        "pc" | "parsec" => Some(val * PC_M),
        "km" | "kilometer" | "kilometre" => Some(val * 1e3),
        "m" | "meter" | "metre" => Some(val),
        _ => None,
    }
}

fn from_meters(meters: f64, unit: &str) -> Option<f64> {
    match unit.to_lowercase().trim_end_matches('s') {
        "au" => Some(meters / AU_M),
        "ly" => Some(meters / LY_M),
        "pc" => Some(meters / PC_M),
        "km" => Some(meters / 1e3),
        "m" => Some(meters),
        _ => None,
    }
}

fn action_distance(args: &Value) -> String {
    let val = match args["value"].as_f64() {
        Some(v) => v,
        None => {
            // Show conversion table
            let mut out = String::from("Astronomical Distance Units\n\n");
            out.push_str(&format!("1 AU  = {:>20.6e} m\n", AU_M));
            out.push_str(&format!("      = {:>20.6} km\n", AU_M / 1e3));
            out.push_str(&format!("      = {:>20.9} ly\n", AU_M / LY_M));
            out.push_str(&format!("      = {:>20.12} pc\n\n", AU_M / PC_M));
            out.push_str(&format!("1 ly  = {:>20.6e} m\n", LY_M));
            out.push_str(&format!("      = {:>20.6} AU\n", LY_M / AU_M));
            out.push_str(&format!("      = {:>20.9} pc\n\n", LY_M / PC_M));
            out.push_str(&format!("1 pc  = {:>20.6e} m\n", PC_M));
            out.push_str(&format!("      = {:>20.6} AU\n", PC_AU));
            out.push_str(&format!("      = {:>20.6} ly\n\n", PC_LY));
            out.push_str("Common distances:\n");
            out.push_str("  Earth–Moon         = 384,400 km = 0.00257 AU\n");
            out.push_str("  Earth–Sun          = 1 AU = 8.317 light-minutes\n");
            out.push_str("  Sun–Proxima Cen    = 1.295 pc = 4.246 ly\n");
            out.push_str("  Milky Way diameter ≈ 30 kpc = 100,000 ly\n");
            return out;
        }
    };

    let from = args["from_unit"]
        .as_str()
        .or_else(|| args["from"].as_str())
        .unwrap_or("au");
    let to = args["to_unit"]
        .as_str()
        .or_else(|| args["to"].as_str())
        .unwrap_or("");

    let meters = match to_meters(val, from) {
        Some(m) => m,
        None => return format!("Error: unknown unit '{}'.", from),
    };

    if to.is_empty() {
        let mut out = format!("Distance: {:.6e} {} =\n\n", val, from);
        for (u, label) in &[
            ("au", "AU"),
            ("ly", "ly"),
            ("pc", "pc"),
            ("km", "km"),
            ("m", "m"),
        ] {
            if let Some(v) = from_meters(meters, u) {
                out.push_str(&format!("  {:>20.6e}  {}\n", v, label));
            }
        }
        out
    } else {
        match from_meters(meters, to) {
            Some(result) => format!("{:.6e} {} = {:.6e} {}", val, from, result, to),
            None => format!("Error: unknown target unit '{}'.", to),
        }
    }
}

// ── constellation lookup ─────────────────────────────────────────────────────

static CONSTELLATIONS: &[(&str, &str, &str, &str)] = &[
    ("And", "Andromeda", "Princess of Ethiopia", "NQ1"),
    ("Ant", "Antlia", "Air Pump", "SQ2"),
    ("Aps", "Apus", "Bird of Paradise", "SQ3"),
    ("Aql", "Aquila", "Eagle", "NQ3"),
    ("Aqr", "Aquarius", "Water Bearer", "SQ4"),
    ("Ara", "Ara", "Altar", "SQ3"),
    ("Ari", "Aries", "Ram", "NQ1"),
    ("Aur", "Auriga", "Charioteer", "NQ1"),
    ("Boo", "Boötes", "Herdsman", "NQ3"),
    ("CMa", "Canis Major", "Greater Dog", "SQ1"),
    ("CMi", "Canis Minor", "Lesser Dog", "NQ1"),
    ("CVn", "Canes Venatici", "Hunting Dogs", "NQ2"),
    ("Cap", "Capricornus", "Sea Goat", "SQ4"),
    ("Car", "Carina", "Ship's Keel", "SQ2"),
    ("Cas", "Cassiopeia", "Queen of Ethiopia", "NQ1"),
    ("Cen", "Centaurus", "Centaur", "SQ3"),
    ("Cep", "Cepheus", "King of Ethiopia", "NQ4"),
    ("Cet", "Cetus", "Sea Monster/Whale", "SQ1"),
    ("Col", "Columba", "Dove", "SQ1"),
    ("Com", "Coma Berenices", "Berenice's Hair", "NQ2"),
    ("CrA", "Corona Australis", "Southern Crown", "SQ4"),
    ("CrB", "Corona Borealis", "Northern Crown", "NQ3"),
    ("Crt", "Crater", "Cup", "SQ2"),
    ("Crv", "Corvus", "Crow", "SQ2"),
    ("Cru", "Crux", "Southern Cross", "SQ3"),
    ("Cyg", "Cygnus", "Swan", "NQ4"),
    ("Del", "Delphinus", "Dolphin", "NQ4"),
    ("Dor", "Dorado", "Swordfish", "SQ1"),
    ("Dra", "Draco", "Dragon", "NQ3"),
    ("Equ", "Equuleus", "Little Horse", "NQ4"),
    ("Eri", "Eridanus", "River", "SQ1"),
    ("For", "Fornax", "Furnace", "SQ1"),
    ("Gem", "Gemini", "Twins", "NQ1"),
    ("Gru", "Grus", "Crane", "SQ4"),
    ("Her", "Hercules", "Hercules", "NQ3"),
    ("Hor", "Horologium", "Clock", "SQ1"),
    ("Hya", "Hydra", "Water Snake", "SQ2"),
    ("Hyi", "Hydrus", "Lesser Water Snake", "SQ1"),
    ("Ind", "Indus", "Indian", "SQ4"),
    ("Lac", "Lacerta", "Lizard", "NQ4"),
    ("Leo", "Leo", "Lion", "NQ2"),
    ("LMi", "Leo Minor", "Lesser Lion", "NQ2"),
    ("Lep", "Lepus", "Hare", "SQ1"),
    ("Lib", "Libra", "Scales", "SQ3"),
    ("Lup", "Lupus", "Wolf", "SQ3"),
    ("Lyn", "Lynx", "Lynx", "NQ2"),
    ("Lyr", "Lyra", "Lyre", "NQ4"),
    ("Men", "Mensa", "Table Mountain", "SQ1"),
    ("Mic", "Microscopium", "Microscope", "SQ4"),
    ("Mon", "Monoceros", "Unicorn", "NQ1"),
    ("Mus", "Musca", "Fly", "SQ3"),
    ("Nor", "Norma", "Carpenter's Square", "SQ3"),
    ("Oct", "Octans", "Octant", "SQ4"),
    ("Oph", "Ophiuchus", "Serpent Bearer", "SQ3"),
    ("Ori", "Orion", "Hunter", "SQ1"),
    ("Pav", "Pavo", "Peacock", "SQ4"),
    ("Peg", "Pegasus", "Winged Horse", "NQ4"),
    ("Per", "Perseus", "Hero", "NQ1"),
    ("Phe", "Phoenix", "Phoenix", "SQ1"),
    ("Pic", "Pictor", "Painter's Easel", "SQ1"),
    ("PsA", "Piscis Austrinus", "Southern Fish", "SQ4"),
    ("Psc", "Pisces", "Fish", "NQ1"),
    ("Pup", "Puppis", "Ship's Stern", "SQ2"),
    ("Pyx", "Pyxis", "Compass", "SQ2"),
    ("Ret", "Reticulum", "Reticle", "SQ1"),
    ("Scl", "Sculptor", "Sculptor", "SQ1"),
    ("Sco", "Scorpius", "Scorpion", "SQ3"),
    ("Sct", "Scutum", "Shield", "SQ4"),
    ("Ser", "Serpens", "Serpent", "NQ3"),
    ("Sex", "Sextans", "Sextant", "SQ2"),
    ("Sge", "Sagitta", "Arrow", "NQ4"),
    ("Sgr", "Sagittarius", "Archer", "SQ4"),
    ("Tau", "Taurus", "Bull", "NQ1"),
    ("Tel", "Telescopium", "Telescope", "SQ3"),
    ("TrA", "Triangulum Australe", "Southern Triangle", "SQ3"),
    ("Tri", "Triangulum", "Triangle", "NQ1"),
    ("Tuc", "Tucana", "Toucan", "SQ4"),
    ("UMa", "Ursa Major", "Great Bear", "NQ2"),
    ("UMi", "Ursa Minor", "Little Bear", "NQ3"),
    ("Vel", "Vela", "Ship's Sails", "SQ2"),
    ("Vir", "Virgo", "Virgin", "SQ3"),
    ("Vol", "Volans", "Flying Fish", "SQ2"),
    ("Vul", "Vulpecula", "Little Fox", "NQ4"),
];

fn action_constellation(args: &Value) -> String {
    let q = args["query"]
        .as_str()
        .or_else(|| args["body"].as_str())
        .unwrap_or("")
        .to_lowercase();

    if q.is_empty() {
        let mut out = format!("All 88 IAU Constellations\n\n");
        out.push_str(&format!(
            "{:<6} {:<26} {:<26} {}\n",
            "Abbr", "Name", "Meaning", "Region"
        ));
        out.push_str(&"─".repeat(72));
        out.push('\n');
        for (abbr, name, meaning, region) in CONSTELLATIONS {
            out.push_str(&format!(
                "{:<6} {:<26} {:<26} {}\n",
                abbr, name, meaning, region
            ));
        }
        out.push_str(&format!(
            "\n({} entries shown — 88 IAU constellations total)\n",
            CONSTELLATIONS.len()
        ));
        return out;
    }

    let matches: Vec<_> = CONSTELLATIONS
        .iter()
        .filter(|(abbr, name, meaning, _)| {
            abbr.to_lowercase().contains(&q)
                || name.to_lowercase().contains(&q)
                || meaning.to_lowercase().contains(&q)
        })
        .collect();

    if matches.is_empty() {
        return format!("No constellation found matching '{}'.", q);
    }

    let mut out = format!("Constellation results for '{}'\n\n", q);
    for (abbr, name, meaning, region) in &matches {
        out.push_str(&format!("  {abbr} — {name}\n"));
        out.push_str(&format!("    Meaning: {meaning}\n"));
        let (hem, quad) = match &region[..2] {
            "NQ" => ("Northern", &region[2..]),
            "SQ" => ("Southern", &region[2..]),
            _ => ("Unknown", ""),
        };
        out.push_str(&format!("    Hemisphere: {hem}, Quadrant {quad}\n\n"));
    }
    out
}

// ── moon phase ───────────────────────────────────────────────────────────────

fn action_moon_phase(args: &Value) -> String {
    let date_str = args["date"].as_str().unwrap_or("");
    let jd = if date_str.is_empty() {
        today_jd()
    } else {
        julian_day_number(date_str)
    };
    let (y, m, d) = jd_to_calendar(jd);

    // Moon's mean elongation from Sun (simplified)
    let t = (jd - 2451545.0) / 36525.0;
    let d_moon = norm360(297.85036 + 445267.111480 * t);
    let m_sun = norm360(357.52772 + 35999.050340 * t);
    let m_moon = norm360(134.96298 + 477198.867398 * t);
    let f = norm360(93.27191 + 483202.017538 * t);

    // Equation of centre approximation
    let phase_angle = norm360(
        d_moon + 6.29 * deg_to_rad(m_moon).sin() - 1.27 * deg_to_rad(m_moon - 2.0 * d_moon).sin()
            + 0.43 * deg_to_rad(2.0 * m_moon).sin()
            + 0.21 * deg_to_rad(2.0 * f).sin()
            - 0.20 * deg_to_rad(m_sun).sin(),
    );

    let illum = (1.0 - deg_to_rad(phase_angle).cos()) / 2.0 * 100.0;
    let phase_name = match (phase_angle as u32 / 45) % 8 {
        0 => "New Moon",
        1 => "Waxing Crescent",
        2 => "First Quarter",
        3 => "Waxing Gibbous",
        4 => "Full Moon",
        5 => "Waning Gibbous",
        6 => "Last Quarter",
        _ => "Waning Crescent",
    };

    // Synodic period = 29.53059 days; find days to next new moon
    let synodic = 29.53059_f64;
    let days_since_new = (phase_angle / 360.0) * synodic;
    let days_to_next = synodic - days_since_new;

    let bar_len = 20_usize;
    let filled = (illum / 100.0 * bar_len as f64).round() as usize;
    let bar = format!("[{}{}]", "█".repeat(filled), "░".repeat(bar_len - filled));

    let mut out = format!("Moon Phase — {}-{:02}-{:02}\n\n", y, m, d);
    out.push_str(&format!("  Phase:        {}\n", phase_name));
    out.push_str(&format!("  Illumination: {:.1}%  {}\n", illum, bar));
    out.push_str(&format!("  Phase angle:  {:.2}°\n", phase_angle));
    out.push_str(&format!(
        "  Age:          {:.1} days since New Moon\n",
        days_since_new
    ));
    out.push_str(&format!("  Next New Moon: in {:.1} days\n", days_to_next));
    out.push_str("\nNote: simplified calculation; ±1° accuracy.\n");
    out
}

// ── Julian Date conversion ───────────────────────────────────────────────────

fn action_julian(args: &Value) -> String {
    let mut out = String::from("Julian Date Conversion\n\n");

    if let Some(jd) = args["jd"].as_f64() {
        let (y, m, d) = jd_to_calendar(jd);
        out.push_str(&format!(
            "JD {:.5} → {}-{:02}-{:02} ({:.1} UT)\n",
            jd,
            y,
            m,
            d,
            (jd + 0.5) % 1.0 * 24.0
        ));
        out.push_str(&format!("  MJD (Modified JD): {:.5}\n", jd - 2400000.5));
        out.push_str(&format!("  J2000 offset: {:.5} days\n", jd - 2451545.0));
    } else if let Some(date_str) = args["date"].as_str() {
        let jd = julian_day_number(date_str);
        let (y, m, d) = jd_to_calendar(jd);
        out.push_str(&format!("{}-{:02}-{:02} → JD {:.5}\n", y, m, d, jd));
        out.push_str(&format!("  MJD (Modified JD): {:.5}\n", jd - 2400000.5));
        out.push_str(&format!("  J2000 offset: {:.5} days\n", jd - 2451545.0));
    } else {
        let jd = today_jd();
        let (y, m, d) = jd_to_calendar(jd);
        out.push_str(&format!("Today ({}-{:02}-{:02}):\n", y, m, d));
        out.push_str(&format!("  JD  = {:.5}\n", jd));
        out.push_str(&format!("  MJD = {:.5}\n", jd - 2400000.5));
        out.push_str(&format!("  J2000 offset = {:.5} days\n", jd - 2451545.0));
        out.push_str("\nHistorical epochs:\n");
        out.push_str("  J2000.0   = JD 2451545.0  (2000 Jan 1.5)\n");
        out.push_str("  B1950.0   = JD 2433282.4  (1950 Jan 0.923)\n");
        out.push_str("  MJD epoch = JD 2400000.5  (1858 Nov 17)\n");
    }
    out
}

// ── dispatch ─────────────────────────────────────────────────────────────────

pub async fn execute(args: &Value) -> Result<String, String> {
    let action = args["action"].as_str().unwrap_or("planet");
    let result = match action {
        "planet"        => action_planet(args),
        "rise_set"      => action_rise_set(args),
        "separation"    => action_separation(args),
        "magnitude"     => action_magnitude(args),
        "distance"      => action_distance(args),
        "constellation" => action_constellation(args),
        "moon_phase"    => action_moon_phase(args),
        "julian"        => action_julian(args),
        other           => return Err(format!("Unknown action '{}'. Valid: planet, rise_set, separation, magnitude, distance, constellation, moon_phase, julian.", other)),
    };
    Ok(result)
}
