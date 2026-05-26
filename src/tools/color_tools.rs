pub async fn execute(args: &serde_json::Value) -> Result<String, String> {
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("info");

    match action {
        "info" => info(args),
        "convert" => convert(args),
        "mix" => mix(args),
        "lighten" => adjust_lightness(args, true),
        "darken" => adjust_lightness(args, false),
        "contrast" => contrast(args),
        "palette" => palette(args),
        other => Err(format!(
            "color_tools: unknown action '{other}'. Valid: info, convert, mix, lighten, darken, contrast, palette"
        )),
    }
}

// ── Color types ───────────────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug)]
struct Rgb {
    r: u8,
    g: u8,
    b: u8,
}

#[derive(Clone, Copy, Debug)]
struct Hsl {
    h: f64, // 0–360
    s: f64, // 0–100
    l: f64, // 0–100
}

impl Rgb {
    fn to_hsl(self) -> Hsl {
        let r = self.r as f64 / 255.0;
        let g = self.g as f64 / 255.0;
        let b = self.b as f64 / 255.0;
        let max = r.max(g).max(b);
        let min = r.min(g).min(b);
        let delta = max - min;
        let l = (max + min) / 2.0;
        if delta < 1e-10 {
            return Hsl {
                h: 0.0,
                s: 0.0,
                l: l * 100.0,
            };
        }
        let s = if l > 0.5 {
            delta / (2.0 - max - min)
        } else {
            delta / (max + min)
        };
        let h = if (max - r).abs() < 1e-10 {
            ((g - b) / delta).rem_euclid(6.0) / 6.0
        } else if (max - g).abs() < 1e-10 {
            ((b - r) / delta + 2.0) / 6.0
        } else {
            ((r - g) / delta + 4.0) / 6.0
        };
        Hsl {
            h: h * 360.0,
            s: s * 100.0,
            l: l * 100.0,
        }
    }

    fn luminance(self) -> f64 {
        fn linearize(v: f64) -> f64 {
            if v <= 0.04045 {
                v / 12.92
            } else {
                ((v + 0.055) / 1.055).powf(2.4)
            }
        }
        let r = linearize(self.r as f64 / 255.0);
        let g = linearize(self.g as f64 / 255.0);
        let b = linearize(self.b as f64 / 255.0);
        0.2126 * r + 0.7152 * g + 0.0722 * b
    }
}

impl Hsl {
    fn to_rgb(self) -> Rgb {
        let h = self.h / 360.0;
        let s = self.s / 100.0;
        let l = self.l / 100.0;
        if s < 1e-10 {
            let v = (l * 255.0).round() as u8;
            return Rgb { r: v, g: v, b: v };
        }
        let q = if l < 0.5 {
            l * (1.0 + s)
        } else {
            l + s - l * s
        };
        let p = 2.0 * l - q;
        let r = hue_to_rgb(p, q, h + 1.0 / 3.0);
        let g = hue_to_rgb(p, q, h);
        let b = hue_to_rgb(p, q, h - 1.0 / 3.0);
        Rgb {
            r: (r * 255.0).round() as u8,
            g: (g * 255.0).round() as u8,
            b: (b * 255.0).round() as u8,
        }
    }
}

fn hue_to_rgb(p: f64, q: f64, mut t: f64) -> f64 {
    if t < 0.0 {
        t += 1.0;
    }
    if t > 1.0 {
        t -= 1.0;
    }
    if t < 1.0 / 6.0 {
        return p + (q - p) * 6.0 * t;
    }
    if t < 1.0 / 2.0 {
        return q;
    }
    if t < 2.0 / 3.0 {
        return p + (q - p) * (2.0 / 3.0 - t) * 6.0;
    }
    p
}

// ── Parsing ───────────────────────────────────────────────────────────────────

fn parse_color(s: &str) -> Result<Rgb, String> {
    let s = s.trim();

    // Named colors
    if let Some(rgb) = named_color(s) {
        return Ok(rgb);
    }

    // Hex: #RRGGBB or #RGB
    let hex = s.trim_start_matches('#');
    match hex.len() {
        6 => {
            let r = u8::from_str_radix(&hex[0..2], 16)
                .map_err(|_| format!("color_tools: invalid hex '{s}'"))?;
            let g = u8::from_str_radix(&hex[2..4], 16)
                .map_err(|_| format!("color_tools: invalid hex '{s}'"))?;
            let b = u8::from_str_radix(&hex[4..6], 16)
                .map_err(|_| format!("color_tools: invalid hex '{s}'"))?;
            return Ok(Rgb { r, g, b });
        }
        3 => {
            let r = u8::from_str_radix(&hex[0..1].repeat(2), 16)
                .map_err(|_| format!("color_tools: invalid hex '{s}'"))?;
            let g = u8::from_str_radix(&hex[1..2].repeat(2), 16)
                .map_err(|_| format!("color_tools: invalid hex '{s}'"))?;
            let b = u8::from_str_radix(&hex[2..3].repeat(2), 16)
                .map_err(|_| format!("color_tools: invalid hex '{s}'"))?;
            return Ok(Rgb { r, g, b });
        }
        _ => {}
    }

    // rgb(R, G, B)
    let lower = s.to_lowercase();
    if lower.starts_with("rgb(") && lower.ends_with(')') {
        let inner = &lower[4..lower.len() - 1];
        let parts: Vec<&str> = inner.split(',').collect();
        if parts.len() == 3 {
            let parse_chan = |p: &str| -> Result<u8, String> {
                let v: f64 = p
                    .trim()
                    .trim_end_matches('%')
                    .parse()
                    .map_err(|_| format!("color_tools: invalid rgb value '{p}'"))?;
                if p.trim().ends_with('%') {
                    Ok((v / 100.0 * 255.0).round().min(255.0) as u8)
                } else {
                    Ok(v.round().min(255.0) as u8)
                }
            };
            return Ok(Rgb {
                r: parse_chan(parts[0])?,
                g: parse_chan(parts[1])?,
                b: parse_chan(parts[2])?,
            });
        }
    }

    // hsl(H, S%, L%)
    if lower.starts_with("hsl(") && lower.ends_with(')') {
        let inner = &lower[4..lower.len() - 1];
        let parts: Vec<&str> = inner.split(',').collect();
        if parts.len() == 3 {
            let h: f64 = parts[0]
                .trim()
                .parse()
                .map_err(|_| "invalid hue".to_string())?;
            let s: f64 = parts[1]
                .trim()
                .trim_end_matches('%')
                .parse()
                .map_err(|_| "invalid saturation".to_string())?;
            let l: f64 = parts[2]
                .trim()
                .trim_end_matches('%')
                .parse()
                .map_err(|_| "invalid lightness".to_string())?;
            return Ok(Hsl { h, s, l }.to_rgb());
        }
    }

    Err(format!(
        "color_tools: cannot parse color '{s}'. Use #RRGGBB, #RGB, rgb(R,G,B), hsl(H,S%,L%), or a CSS color name."
    ))
}

fn to_hex(rgb: Rgb) -> String {
    format!("#{:02X}{:02X}{:02X}", rgb.r, rgb.g, rgb.b)
}

fn named_color(s: &str) -> Option<Rgb> {
    match s.to_lowercase().as_str() {
        "black" => Some(Rgb { r: 0, g: 0, b: 0 }),
        "white" => Some(Rgb {
            r: 255,
            g: 255,
            b: 255,
        }),
        "red" => Some(Rgb { r: 255, g: 0, b: 0 }),
        "green" => Some(Rgb { r: 0, g: 128, b: 0 }),
        "blue" => Some(Rgb { r: 0, g: 0, b: 255 }),
        "yellow" => Some(Rgb {
            r: 255,
            g: 255,
            b: 0,
        }),
        "orange" => Some(Rgb {
            r: 255,
            g: 165,
            b: 0,
        }),
        "purple" => Some(Rgb {
            r: 128,
            g: 0,
            b: 128,
        }),
        "pink" => Some(Rgb {
            r: 255,
            g: 192,
            b: 203,
        }),
        "cyan" => Some(Rgb {
            r: 0,
            g: 255,
            b: 255,
        }),
        "magenta" => Some(Rgb {
            r: 255,
            g: 0,
            b: 255,
        }),
        "lime" => Some(Rgb { r: 0, g: 255, b: 0 }),
        "maroon" => Some(Rgb { r: 128, g: 0, b: 0 }),
        "navy" => Some(Rgb { r: 0, g: 0, b: 128 }),
        "teal" => Some(Rgb {
            r: 0,
            g: 128,
            b: 128,
        }),
        "silver" => Some(Rgb {
            r: 192,
            g: 192,
            b: 192,
        }),
        "gray" | "grey" => Some(Rgb {
            r: 128,
            g: 128,
            b: 128,
        }),
        "coral" => Some(Rgb {
            r: 255,
            g: 127,
            b: 80,
        }),
        "salmon" => Some(Rgb {
            r: 250,
            g: 128,
            b: 114,
        }),
        "gold" => Some(Rgb {
            r: 255,
            g: 215,
            b: 0,
        }),
        "indigo" => Some(Rgb {
            r: 75,
            g: 0,
            b: 130,
        }),
        "violet" => Some(Rgb {
            r: 238,
            g: 130,
            b: 238,
        }),
        "brown" => Some(Rgb {
            r: 165,
            g: 42,
            b: 42,
        }),
        "crimson" => Some(Rgb {
            r: 220,
            g: 20,
            b: 60,
        }),
        "khaki" => Some(Rgb {
            r: 240,
            g: 230,
            b: 140,
        }),
        "lavender" => Some(Rgb {
            r: 230,
            g: 230,
            b: 250,
        }),
        "turquoise" => Some(Rgb {
            r: 64,
            g: 224,
            b: 208,
        }),
        "tomato" => Some(Rgb {
            r: 255,
            g: 99,
            b: 71,
        }),
        "aqua" => Some(Rgb {
            r: 0,
            g: 255,
            b: 255,
        }),
        "fuchsia" => Some(Rgb {
            r: 255,
            g: 0,
            b: 255,
        }),
        "olive" => Some(Rgb {
            r: 128,
            g: 128,
            b: 0,
        }),
        _ => None,
    }
}

fn wcag_grade(ratio: f64) -> &'static str {
    if ratio >= 7.0 {
        "AAA (excellent)"
    } else if ratio >= 4.5 {
        "AA (passes normal text)"
    } else if ratio >= 3.0 {
        "AA Large (passes large text only)"
    } else {
        "FAIL (insufficient contrast)"
    }
}

// ── Actions ───────────────────────────────────────────────────────────────────

fn info(args: &serde_json::Value) -> Result<String, String> {
    let input = args
        .get("input")
        .and_then(|v| v.as_str())
        .ok_or("color_tools info: 'input' is required")?;

    let rgb = parse_color(input)?;
    let hsl = rgb.to_hsl();
    let lum = rgb.luminance();
    let is_dark = lum < 0.179;

    let mut out = format!("COLOR INFO\n{}\n", "─".repeat(50));
    out.push_str(&format!("Input      : {input}\n"));
    out.push_str(&format!("Hex        : {}\n", to_hex(rgb)));
    out.push_str(&format!(
        "RGB        : rgb({}, {}, {})\n",
        rgb.r, rgb.g, rgb.b
    ));
    out.push_str(&format!(
        "HSL        : hsl({:.0}, {:.1}%, {:.1}%)\n",
        hsl.h, hsl.s, hsl.l
    ));
    out.push_str(&format!("Luminance  : {:.4}\n", lum));
    out.push_str(&format!(
        "Perceived  : {}\n",
        if is_dark { "dark" } else { "light" }
    ));
    out.push_str(&format!(
        "On white   : contrast {:.2}:1 — {}\n",
        contrast_ratio(
            rgb,
            Rgb {
                r: 255,
                g: 255,
                b: 255
            }
        ),
        wcag_grade(contrast_ratio(
            rgb,
            Rgb {
                r: 255,
                g: 255,
                b: 255
            }
        ))
    ));
    out.push_str(&format!(
        "On black   : contrast {:.2}:1 — {}\n",
        contrast_ratio(rgb, Rgb { r: 0, g: 0, b: 0 }),
        wcag_grade(contrast_ratio(rgb, Rgb { r: 0, g: 0, b: 0 }))
    ));
    Ok(out)
}

fn convert(args: &serde_json::Value) -> Result<String, String> {
    let input = args
        .get("input")
        .and_then(|v| v.as_str())
        .ok_or("color_tools convert: 'input' is required")?;

    let rgb = parse_color(input)?;
    let hsl = rgb.to_hsl();

    let mut out = format!("COLOR CONVERT\n{}\n", "─".repeat(50));
    out.push_str(&format!("Input : {input}\n\n"));
    out.push_str(&format!("Hex   : {}\n", to_hex(rgb)));
    out.push_str(&format!("RGB   : rgb({}, {}, {})\n", rgb.r, rgb.g, rgb.b));
    out.push_str(&format!(
        "HSL   : hsl({:.1}, {:.1}%, {:.1}%)\n",
        hsl.h, hsl.s, hsl.l
    ));
    out.push_str(&format!("CSS   : color: {}\n", to_hex(rgb).to_lowercase()));
    Ok(out)
}

fn contrast_ratio(a: Rgb, b: Rgb) -> f64 {
    let l1 = a.luminance();
    let l2 = b.luminance();
    let (lighter, darker) = if l1 > l2 { (l1, l2) } else { (l2, l1) };
    (lighter + 0.05) / (darker + 0.05)
}

fn contrast(args: &serde_json::Value) -> Result<String, String> {
    let a_str = args
        .get("color1")
        .or_else(|| args.get("a"))
        .and_then(|v| v.as_str())
        .ok_or("color_tools contrast: 'color1' is required")?;
    let b_str = args
        .get("color2")
        .or_else(|| args.get("b"))
        .and_then(|v| v.as_str())
        .ok_or("color_tools contrast: 'color2' is required")?;

    let a = parse_color(a_str)?;
    let b = parse_color(b_str)?;
    let ratio = contrast_ratio(a, b);

    let mut out = format!("COLOR CONTRAST\n{}\n", "─".repeat(50));
    out.push_str(&format!("Color 1  : {} → {}\n", a_str, to_hex(a)));
    out.push_str(&format!("Color 2  : {} → {}\n", b_str, to_hex(b)));
    out.push_str(&format!("Ratio    : {ratio:.2}:1\n"));
    out.push_str(&format!("WCAG     : {}\n", wcag_grade(ratio)));
    Ok(out)
}

fn mix(args: &serde_json::Value) -> Result<String, String> {
    let a_str = args
        .get("color1")
        .or_else(|| args.get("a"))
        .and_then(|v| v.as_str())
        .ok_or("color_tools mix: 'color1' is required")?;
    let b_str = args
        .get("color2")
        .or_else(|| args.get("b"))
        .and_then(|v| v.as_str())
        .ok_or("color_tools mix: 'color2' is required")?;
    let ratio = args
        .get("ratio")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.5)
        .clamp(0.0, 1.0);

    let a = parse_color(a_str)?;
    let b = parse_color(b_str)?;

    let blend =
        |x: u8, y: u8| -> u8 { (x as f64 * (1.0 - ratio) + y as f64 * ratio).round() as u8 };
    let mixed = Rgb {
        r: blend(a.r, b.r),
        g: blend(a.g, b.g),
        b: blend(a.b, b.b),
    };
    let hsl = mixed.to_hsl();

    let mut out = format!("COLOR MIX\n{}\n", "─".repeat(50));
    out.push_str(&format!("Color 1 : {} → {}\n", a_str, to_hex(a)));
    out.push_str(&format!("Color 2 : {} → {}\n", b_str, to_hex(b)));
    out.push_str(&format!(
        "Ratio   : {:.0}% / {:.0}%\n",
        (1.0 - ratio) * 100.0,
        ratio * 100.0
    ));
    out.push_str(&format!("Result  : {}\n", to_hex(mixed)));
    out.push_str(&format!(
        "RGB     : rgb({}, {}, {})\n",
        mixed.r, mixed.g, mixed.b
    ));
    out.push_str(&format!(
        "HSL     : hsl({:.0}, {:.1}%, {:.1}%)\n",
        hsl.h, hsl.s, hsl.l
    ));
    Ok(out)
}

fn adjust_lightness(args: &serde_json::Value, lighten: bool) -> Result<String, String> {
    let action = if lighten { "lighten" } else { "darken" };
    let input = args
        .get("input")
        .and_then(|v| v.as_str())
        .ok_or_else(|| format!("color_tools {action}: 'input' is required"))?;
    let amount = args
        .get("amount")
        .and_then(|v| v.as_f64())
        .unwrap_or(10.0)
        .abs()
        .min(100.0);

    let rgb = parse_color(input)?;
    let mut hsl = rgb.to_hsl();
    if lighten {
        hsl.l = (hsl.l + amount).min(100.0);
    } else {
        hsl.l = (hsl.l - amount).max(0.0);
    }
    let adjusted = hsl.to_rgb();

    let verb = if lighten { "Lightened" } else { "Darkened" };
    let mut out = format!("COLOR {}\n{}\n", verb.to_uppercase(), "─".repeat(50));
    out.push_str(&format!("Input    : {} → {}\n", input, to_hex(rgb)));
    out.push_str(&format!("Amount   : {amount:.0}%\n"));
    out.push_str(&format!("Result   : {}\n", to_hex(adjusted)));
    out.push_str(&format!(
        "RGB      : rgb({}, {}, {})\n",
        adjusted.r, adjusted.g, adjusted.b
    ));
    let adj_hsl = adjusted.to_hsl();
    out.push_str(&format!(
        "HSL      : hsl({:.0}, {:.1}%, {:.1}%)\n",
        adj_hsl.h, adj_hsl.s, adj_hsl.l
    ));
    Ok(out)
}

fn palette(args: &serde_json::Value) -> Result<String, String> {
    let input = args
        .get("input")
        .and_then(|v| v.as_str())
        .ok_or("color_tools palette: 'input' is required")?;

    let rgb = parse_color(input)?;
    let hsl = rgb.to_hsl();

    // Complementary: rotate 180°
    let comp = Hsl {
        h: (hsl.h + 180.0) % 360.0,
        s: hsl.s,
        l: hsl.l,
    }
    .to_rgb();
    // Triadic: ±120°
    let tri1 = Hsl {
        h: (hsl.h + 120.0) % 360.0,
        s: hsl.s,
        l: hsl.l,
    }
    .to_rgb();
    let tri2 = Hsl {
        h: (hsl.h + 240.0) % 360.0,
        s: hsl.s,
        l: hsl.l,
    }
    .to_rgb();
    // Analogous: ±30°
    let ana1 = Hsl {
        h: (hsl.h + 30.0) % 360.0,
        s: hsl.s,
        l: hsl.l,
    }
    .to_rgb();
    let ana2 = Hsl {
        h: ((hsl.h - 30.0) + 360.0) % 360.0,
        s: hsl.s,
        l: hsl.l,
    }
    .to_rgb();
    // Shades
    let lighter = Hsl {
        h: hsl.h,
        s: hsl.s,
        l: (hsl.l + 20.0).min(100.0),
    }
    .to_rgb();
    let darker = Hsl {
        h: hsl.h,
        s: hsl.s,
        l: (hsl.l - 20.0).max(0.0),
    }
    .to_rgb();

    let mut out = format!("COLOR PALETTE\n{}\n", "─".repeat(50));
    out.push_str(&format!(
        "Base         : {} — rgb({}, {}, {})\n",
        to_hex(rgb),
        rgb.r,
        rgb.g,
        rgb.b
    ));
    out.push_str(&format!("Lighter      : {}\n", to_hex(lighter)));
    out.push_str(&format!("Darker       : {}\n", to_hex(darker)));
    out.push_str(&format!("Complementary: {}\n", to_hex(comp)));
    out.push_str(&format!("Triadic 1    : {}\n", to_hex(tri1)));
    out.push_str(&format!("Triadic 2    : {}\n", to_hex(tri2)));
    out.push_str(&format!("Analogous +  : {}\n", to_hex(ana1)));
    out.push_str(&format!("Analogous −  : {}\n", to_hex(ana2)));
    Ok(out)
}
