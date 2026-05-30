use serde_json::Value;
use std::f64::consts::PI;

pub async fn execute(args: &Value) -> Result<String, String> {
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("area");
    match action {
        "area" => action_area(args),
        "volume" => action_volume(args),
        "perimeter" => action_perimeter(args),
        "triangle" => action_triangle(args),
        "circle" => action_circle(args),
        other => Err(format!(
            "Unknown action '{other}'. Use: area, volume, perimeter, triangle, circle"
        )),
    }
}

fn get_f64(args: &Value, key: &str) -> Option<f64> {
    args.get(key).and_then(|v| v.as_f64())
}

fn req_f64(args: &Value, key: &str) -> Result<f64, String> {
    get_f64(args, key).ok_or_else(|| format!("Missing '{key}'"))
}

fn fmt(v: f64) -> String {
    if v.abs() < 1e-10 {
        return "0".to_string();
    }
    let s = format!("{:.6}", v);
    let s = s.trim_end_matches('0').trim_end_matches('.');
    s.to_string()
}

fn action_area(args: &Value) -> Result<String, String> {
    let shape = args
        .get("shape")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'shape'. Options: rectangle, square, circle, ellipse, triangle, trapezoid, parallelogram, regular_polygon, rhombus, sector")?;

    let mut out = format!("geometry_tools — area — {shape}\n\n");

    let (area, notes) = match shape {
        "rectangle" | "rect" => {
            let w = req_f64(args, "width")?;
            let h = req_f64(args, "height")?;
            out.push_str(&format!("Width:  {}\nHeight: {}\n\n", fmt(w), fmt(h)));
            (w * h, format!("Formula: width × height = {} × {} = {}", fmt(w), fmt(h), fmt(w * h)))
        }
        "square" => {
            let s = req_f64(args, "side")?;
            out.push_str(&format!("Side: {}\n\n", fmt(s)));
            (s * s, format!("Formula: side² = {}² = {}", fmt(s), fmt(s * s)))
        }
        "circle" => {
            let r = req_f64(args, "radius")?;
            let a = PI * r * r;
            out.push_str(&format!("Radius: {}\n\n", fmt(r)));
            (a, format!("Formula: π × r² = π × {}² = {}", fmt(r), fmt(a)))
        }
        "ellipse" => {
            let a_val = req_f64(args, "a")?;
            let b_val = req_f64(args, "b")?;
            let area_val = PI * a_val * b_val;
            out.push_str(&format!("Semi-major (a): {}\nSemi-minor (b): {}\n\n", fmt(a_val), fmt(b_val)));
            (area_val, format!("Formula: π × a × b = π × {} × {} = {}", fmt(a_val), fmt(b_val), fmt(area_val)))
        }
        "triangle" => {
            if let (Some(base), Some(height)) = (get_f64(args, "base"), get_f64(args, "height")) {
                let a = 0.5 * base * height;
                out.push_str(&format!("Base:   {}\nHeight: {}\n\n", fmt(base), fmt(height)));
                (a, format!("Formula: ½ × base × height = ½ × {} × {} = {}", fmt(base), fmt(height), fmt(a)))
            } else {
                let a_s = req_f64(args, "a")?;
                let b_s = req_f64(args, "b")?;
                let c_s = req_f64(args, "c")?;
                let s = (a_s + b_s + c_s) / 2.0;
                let disc = s * (s - a_s) * (s - b_s) * (s - c_s);
                if disc < 0.0 {
                    return Err("Invalid triangle: sides do not satisfy triangle inequality".to_string());
                }
                let area_val = disc.sqrt();
                out.push_str(&format!("Sides: a={}, b={}, c={}\n\n", fmt(a_s), fmt(b_s), fmt(c_s)));
                (area_val, format!("Formula: Heron's (s={}) → √(s(s-a)(s-b)(s-c)) = {}", fmt(s), fmt(area_val)))
            }
        }
        "trapezoid" | "trapezium" => {
            let a_val = req_f64(args, "a")?;
            let b_val = req_f64(args, "b")?;
            let h = req_f64(args, "height")?;
            let area_val = 0.5 * (a_val + b_val) * h;
            out.push_str(&format!("Parallel sides: a={}, b={}\nHeight: {}\n\n", fmt(a_val), fmt(b_val), fmt(h)));
            (area_val, format!("Formula: ½ × (a+b) × h = ½ × {} × {} = {}", fmt(a_val + b_val), fmt(h), fmt(area_val)))
        }
        "parallelogram" => {
            let b = req_f64(args, "base")?;
            let h = req_f64(args, "height")?;
            let a = b * h;
            out.push_str(&format!("Base:   {}\nHeight: {}\n\n", fmt(b), fmt(h)));
            (a, format!("Formula: base × height = {} × {} = {}", fmt(b), fmt(h), fmt(a)))
        }
        "rhombus" => {
            let d1 = req_f64(args, "d1")?;
            let d2 = req_f64(args, "d2")?;
            let a = 0.5 * d1 * d2;
            out.push_str(&format!("Diagonal 1: {}\nDiagonal 2: {}\n\n", fmt(d1), fmt(d2)));
            (a, format!("Formula: ½ × d₁ × d₂ = ½ × {} × {} = {}", fmt(d1), fmt(d2), fmt(a)))
        }
        "regular_polygon" | "polygon" => {
            let n = args.get("sides").and_then(|v| v.as_f64()).ok_or("Missing 'sides' (number of sides)")? as i64;
            if n < 3 {
                return Err("Polygon needs at least 3 sides".to_string());
            }
            let s = req_f64(args, "side_length")?;
            let nf = n as f64;
            let a = (nf * s * s) / (4.0 * (PI / nf).tan());
            out.push_str(&format!("Sides:       {n}\nSide length: {}\n\n", fmt(s)));
            (a, format!("Formula: (n×s²) / (4×tan(π/n)) = {}", fmt(a)))
        }
        "sector" => {
            let r = req_f64(args, "radius")?;
            let angle_deg = req_f64(args, "angle")?;
            let theta = angle_deg.to_radians();
            let a = 0.5 * r * r * theta;
            out.push_str(&format!("Radius: {}\nAngle:  {}°\n\n", fmt(r), fmt(angle_deg)));
            (a, format!("Formula: ½ × r² × θ(rad) = ½ × {}² × {:.6} = {}", fmt(r), theta, fmt(a)))
        }
        other => return Err(format!("Unknown shape '{other}'. Options: rectangle, square, circle, ellipse, triangle, trapezoid, parallelogram, rhombus, regular_polygon, sector")),
    };

    out.push_str(&format!("Area: {}\n", fmt(area)));
    out.push('\n');
    out.push_str(&notes);
    out.push('\n');
    Ok(out)
}

fn action_volume(args: &Value) -> Result<String, String> {
    let shape = args
        .get("shape")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'shape'. Options: cube, rectangular_prism, sphere, hemisphere, cylinder, cone, pyramid, torus")?;

    let mut out = format!("geometry_tools — volume — {shape}\n\n");

    let (volume, surface_area, notes) = match shape {
        "cube" => {
            let s = req_f64(args, "side")?;
            out.push_str(&format!("Side: {}\n\n", fmt(s)));
            let v = s * s * s;
            let sa = 6.0 * s * s;
            (v, sa, format!("V = s³ = {}³ = {}  |  SA = 6s² = {}", fmt(s), fmt(v), fmt(sa)))
        }
        "rectangular_prism" | "box" | "cuboid" => {
            let w = req_f64(args, "width")?;
            let h = req_f64(args, "height")?;
            let d = req_f64(args, "depth")?;
            out.push_str(&format!("Width:  {}\nHeight: {}\nDepth:  {}\n\n", fmt(w), fmt(h), fmt(d)));
            let v = w * h * d;
            let sa = 2.0 * (w * h + h * d + d * w);
            (v, sa, format!("V = w×h×d = {}  |  SA = 2(wh+hd+dw) = {}", fmt(v), fmt(sa)))
        }
        "sphere" => {
            let r = req_f64(args, "radius")?;
            out.push_str(&format!("Radius: {}\n\n", fmt(r)));
            let v = (4.0 / 3.0) * PI * r * r * r;
            let sa = 4.0 * PI * r * r;
            (v, sa, format!("V = (4/3)πr³ = {}  |  SA = 4πr² = {}", fmt(v), fmt(sa)))
        }
        "hemisphere" => {
            let r = req_f64(args, "radius")?;
            out.push_str(&format!("Radius: {}\n\n", fmt(r)));
            let v = (2.0 / 3.0) * PI * r * r * r;
            let sa = 3.0 * PI * r * r;
            (v, sa, format!("V = (2/3)πr³ = {}  |  SA = 3πr² (curved+flat) = {}", fmt(v), fmt(sa)))
        }
        "cylinder" => {
            let r = req_f64(args, "radius")?;
            let h = req_f64(args, "height")?;
            out.push_str(&format!("Radius: {}\nHeight: {}\n\n", fmt(r), fmt(h)));
            let v = PI * r * r * h;
            let sa = 2.0 * PI * r * (r + h);
            (v, sa, format!("V = πr²h = {}  |  SA = 2πr(r+h) = {}", fmt(v), fmt(sa)))
        }
        "cone" => {
            let r = req_f64(args, "radius")?;
            let h = req_f64(args, "height")?;
            out.push_str(&format!("Radius: {}\nHeight: {}\n\n", fmt(r), fmt(h)));
            let slant = (r * r + h * h).sqrt();
            let v = (1.0 / 3.0) * PI * r * r * h;
            let sa = PI * r * (r + slant);
            (v, sa, format!("V = (1/3)πr²h = {}  |  SA = πr(r+l) slant l={} → {}", fmt(v), fmt(slant), fmt(sa)))
        }
        "pyramid" => {
            let base_area = req_f64(args, "base_area")?;
            let h = req_f64(args, "height")?;
            out.push_str(&format!("Base area: {}\nHeight:    {}\n\n", fmt(base_area), fmt(h)));
            let v = (1.0 / 3.0) * base_area * h;
            let sa = f64::NAN; // surface area depends on pyramid type; skip
            (v, sa, format!("V = (1/3) × base_area × h = (1/3) × {} × {} = {}", fmt(base_area), fmt(h), fmt(v)))
        }
        "torus" => {
            let big_r = req_f64(args, "major_radius")?;
            let small_r = req_f64(args, "minor_radius")?;
            out.push_str(&format!("Major radius (R): {}\nMinor radius (r): {}\n\n", fmt(big_r), fmt(small_r)));
            let v = 2.0 * PI * PI * big_r * small_r * small_r;
            let sa = 4.0 * PI * PI * big_r * small_r;
            (v, sa, format!("V = 2π²Rr² = {}  |  SA = 4π²Rr = {}", fmt(v), fmt(sa)))
        }
        other => return Err(format!("Unknown shape '{other}'. Options: cube, rectangular_prism, sphere, hemisphere, cylinder, cone, pyramid, torus")),
    };

    out.push_str(&format!("Volume: {}\n", fmt(volume)));
    if !surface_area.is_nan() {
        out.push_str(&format!("Surface area: {}\n", fmt(surface_area)));
    }
    out.push('\n');
    out.push_str(&notes);
    out.push('\n');
    Ok(out)
}

fn action_perimeter(args: &Value) -> Result<String, String> {
    let shape = args
        .get("shape")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'shape'. Options: rectangle, square, circle, ellipse, triangle, trapezoid, parallelogram, rhombus, regular_polygon")?;

    let mut out = format!("geometry_tools — perimeter — {shape}\n\n");

    let (perimeter, notes) = match shape {
        "rectangle" | "rect" => {
            let w = req_f64(args, "width")?;
            let h = req_f64(args, "height")?;
            let p = 2.0 * (w + h);
            out.push_str(&format!("Width:  {}\nHeight: {}\n\n", fmt(w), fmt(h)));
            (p, format!("Formula: 2(w+h) = 2({} + {}) = {}", fmt(w), fmt(h), fmt(p)))
        }
        "square" => {
            let s = req_f64(args, "side")?;
            let p = 4.0 * s;
            out.push_str(&format!("Side: {}\n\n", fmt(s)));
            (p, format!("Formula: 4s = 4×{} = {}", fmt(s), fmt(p)))
        }
        "circle" => {
            let r = req_f64(args, "radius")?;
            let p = 2.0 * PI * r;
            out.push_str(&format!("Radius: {}\n\n", fmt(r)));
            (p, format!("Formula: 2πr = 2π×{} = {} (circumference)", fmt(r), fmt(p)))
        }
        "ellipse" => {
            let a_val = req_f64(args, "a")?;
            let b_val = req_f64(args, "b")?;
            // Ramanujan's approximation
            let h = ((a_val - b_val) * (a_val - b_val)) / ((a_val + b_val) * (a_val + b_val));
            let p = PI * (a_val + b_val) * (1.0 + (3.0 * h) / (10.0 + (4.0 - 3.0 * h).sqrt()));
            out.push_str(&format!("Semi-major (a): {}\nSemi-minor (b): {}\n\n", fmt(a_val), fmt(b_val)));
            (p, format!("Formula: Ramanujan approximation ≈ {}", fmt(p)))
        }
        "triangle" => {
            let a_s = req_f64(args, "a")?;
            let b_s = req_f64(args, "b")?;
            let c_s = req_f64(args, "c")?;
            let p = a_s + b_s + c_s;
            out.push_str(&format!("Sides: a={}, b={}, c={}\n\n", fmt(a_s), fmt(b_s), fmt(c_s)));
            (p, format!("Formula: a+b+c = {}+{}+{} = {}", fmt(a_s), fmt(b_s), fmt(c_s), fmt(p)))
        }
        "trapezoid" | "trapezium" => {
            let a_val = req_f64(args, "a")?;
            let b_val = req_f64(args, "b")?;
            let c_val = req_f64(args, "c")?;
            let d_val = req_f64(args, "d")?;
            let p = a_val + b_val + c_val + d_val;
            out.push_str(&format!("Sides: a={}, b={}, c={}, d={}\n\n", fmt(a_val), fmt(b_val), fmt(c_val), fmt(d_val)));
            (p, format!("Formula: a+b+c+d = {}", fmt(p)))
        }
        "parallelogram" | "rhombus" => {
            let a_s = req_f64(args, "a")?;
            let b_s = req_f64(args, "b")?;
            let p = 2.0 * (a_s + b_s);
            out.push_str(&format!("Sides: a={}, b={}\n\n", fmt(a_s), fmt(b_s)));
            (p, format!("Formula: 2(a+b) = 2({} + {}) = {}", fmt(a_s), fmt(b_s), fmt(p)))
        }
        "regular_polygon" | "polygon" => {
            let n = args.get("sides").and_then(|v| v.as_f64()).ok_or("Missing 'sides'")? as i64;
            let s = req_f64(args, "side_length")?;
            let p = n as f64 * s;
            out.push_str(&format!("Sides:       {n}\nSide length: {}\n\n", fmt(s)));
            (p, format!("Formula: n×s = {}×{} = {}", n, fmt(s), fmt(p)))
        }
        other => return Err(format!("Unknown shape '{other}'. Options: rectangle, square, circle, ellipse, triangle, trapezoid, parallelogram, rhombus, regular_polygon")),
    };

    out.push_str(&format!("Perimeter: {}\n", fmt(perimeter)));
    out.push('\n');
    out.push_str(&notes);
    out.push('\n');
    Ok(out)
}

fn action_triangle(args: &Value) -> Result<String, String> {
    // Supports SSS (3 sides) or SAS (2 sides + included angle in degrees)
    let mut out = String::from("geometry_tools — triangle\n\n");

    let (a, b, c) = if let (Some(a_v), Some(b_v), Some(c_v)) =
        (get_f64(args, "a"), get_f64(args, "b"), get_f64(args, "c"))
    {
        // SSS
        (a_v, b_v, c_v)
    } else if let (Some(a_v), Some(b_v), Some(angle_c)) = (
        get_f64(args, "a"),
        get_f64(args, "b"),
        get_f64(args, "angle_c"),
    ) {
        // SAS: sides a, b and included angle C
        let theta = angle_c.to_radians();
        let c_v = (a_v * a_v + b_v * b_v - 2.0 * a_v * b_v * theta.cos()).sqrt();
        (a_v, b_v, c_v)
    } else {
        return Err(
            "Provide either three sides (a, b, c) for SSS, or two sides (a, b) and included angle (angle_c) for SAS".to_string()
        );
    };

    // Validate triangle inequality
    if a + b <= c || a + c <= b || b + c <= a {
        return Err(format!(
            "Invalid triangle: sides {}, {}, {} violate the triangle inequality",
            fmt(a),
            fmt(b),
            fmt(c)
        ));
    }

    // Angles via law of cosines
    let angle_a = ((b * b + c * c - a * a) / (2.0 * b * c))
        .acos()
        .to_degrees();
    let angle_b = ((a * a + c * c - b * b) / (2.0 * a * c))
        .acos()
        .to_degrees();
    let angle_c_deg = 180.0 - angle_a - angle_b;

    let perimeter = a + b + c;
    let s = perimeter / 2.0;
    let area = (s * (s - a) * (s - b) * (s - c)).sqrt();
    let inradius = area / s;
    let circumradius = (a * b * c) / (4.0 * area);

    // Triangle type
    let angle_type = if angle_a > 90.0 || angle_b > 90.0 || angle_c_deg > 90.0 {
        "Obtuse"
    } else if (angle_a - 90.0).abs() < 1e-6
        || (angle_b - 90.0).abs() < 1e-6
        || (angle_c_deg - 90.0).abs() < 1e-6
    {
        "Right"
    } else {
        "Acute"
    };
    let side_type = if (a - b).abs() < 1e-9 && (b - c).abs() < 1e-9 {
        "Equilateral"
    } else if (a - b).abs() < 1e-9 || (b - c).abs() < 1e-9 || (a - c).abs() < 1e-9 {
        "Isosceles"
    } else {
        "Scalene"
    };

    out.push_str(&format!(
        "Sides:\n  a = {}  b = {}  c = {}\n\n",
        fmt(a),
        fmt(b),
        fmt(c)
    ));
    out.push_str(&format!(
        "Angles:\n  A = {:.4}°  B = {:.4}°  C = {:.4}°\n\n",
        angle_a, angle_b, angle_c_deg
    ));
    out.push_str(&format!("Perimeter:    {}\n", fmt(perimeter)));
    out.push_str(&format!("Area:         {}\n", fmt(area)));
    out.push_str(&format!("Inradius:     {}\n", fmt(inradius)));
    out.push_str(&format!("Circumradius: {}\n", fmt(circumradius)));
    out.push_str(&format!("Type:         {} {}\n", angle_type, side_type));
    Ok(out)
}

fn action_circle(args: &Value) -> Result<String, String> {
    // Accept any one known quantity and compute the rest
    let (radius, source) = if let Some(r) = get_f64(args, "radius") {
        (r, "radius")
    } else if let Some(d) = get_f64(args, "diameter") {
        (d / 2.0, "diameter")
    } else if let Some(c) = get_f64(args, "circumference") {
        (c / (2.0 * PI), "circumference")
    } else if let Some(a) = get_f64(args, "area") {
        ((a / PI).sqrt(), "area")
    } else {
        return Err("Provide one of: radius, diameter, circumference, area".to_string());
    };

    if radius <= 0.0 {
        return Err("Radius must be positive".to_string());
    }

    let diameter = 2.0 * radius;
    let circumference = 2.0 * PI * radius;
    let area = PI * radius * radius;

    let mut out = String::from("geometry_tools — circle\n\n");
    out.push_str(&format!("Given: {source}\n\n"));
    out.push_str(&format!("Radius:        {}\n", fmt(radius)));
    out.push_str(&format!("Diameter:      {}\n", fmt(diameter)));
    out.push_str(&format!("Circumference: {}\n", fmt(circumference)));
    out.push_str(&format!("Area:          {}\n", fmt(area)));

    // Optional: arc length and sector area for a given central angle
    if let Some(angle_deg) = get_f64(args, "angle") {
        let theta = angle_deg.to_radians();
        let arc_len = radius * theta;
        let sector_area = 0.5 * radius * radius * theta;
        let chord_len = 2.0 * radius * (theta / 2.0).sin();
        out.push_str(&format!("\nAngle: {}°\n", fmt(angle_deg)));
        out.push_str(&format!("Arc length:   {}\n", fmt(arc_len)));
        out.push_str(&format!("Sector area:  {}\n", fmt(sector_area)));
        out.push_str(&format!("Chord length: {}\n", fmt(chord_len)));
    }

    Ok(out)
}
