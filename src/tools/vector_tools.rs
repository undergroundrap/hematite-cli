use serde_json::{json, Value};

pub fn vector_tools_schema() -> Value {
    json!({
        "name": "vector_tools",
        "description": "2D/3D (and nD) vector math without external utilities. Actions: info (magnitude, unit vector, direction), add (a+b), subtract (a-b), scale (v × scalar), dot (dot product + angle), cross (3D cross product), magnitude (|v| and |v|²), normalize (unit vector), angle (angle between two vectors in degrees/radians), project (scalar and vector projection of a onto b), reflect (reflect v over normal n).",
        "parameters": {
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["info", "add", "subtract", "sub", "scale", "dot", "cross", "magnitude", "length", "norm", "normalize", "unit", "angle", "project", "projection", "reflect"],
                    "description": "Action to perform (default: info)"
                },
                "v": {
                    "type": ["array", "string"],
                    "description": "Primary vector as JSON array [1,2,3] or comma-separated string '1,2,3'"
                },
                "vector": {
                    "type": ["array", "string"],
                    "description": "Alias for 'v'"
                },
                "a": {
                    "type": ["array", "string"],
                    "description": "First vector for binary operations (alias for 'v')"
                },
                "b": {
                    "type": ["array", "string"],
                    "description": "Second vector for add/subtract/dot/cross/angle/project"
                },
                "scalar": {
                    "type": "number",
                    "description": "Scalar multiplier for 'scale' action"
                },
                "n": {
                    "type": ["array", "string"],
                    "description": "Normal vector for 'reflect' action"
                }
            }
        }
    })
}

pub async fn execute(args: &Value) -> Result<String, String> {
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("info");
    match action {
        "add" => action_add(args),
        "subtract" | "sub" => action_subtract(args),
        "scale" => action_scale(args),
        "dot" => action_dot(args),
        "cross" => action_cross(args),
        "magnitude" | "length" | "norm" => action_magnitude(args),
        "normalize" | "unit" => action_normalize(args),
        "angle" => action_angle(args),
        "project" | "projection" => action_project(args),
        "reflect" => action_reflect(args),
        _ => action_info(args),
    }
}

fn parse_vec(v: &Value) -> Result<Vec<f64>, String> {
    if let Some(arr) = v.as_array() {
        arr.iter()
            .map(|x| {
                x.as_f64()
                    .ok_or_else(|| "non-numeric component".to_string())
            })
            .collect()
    } else if let Some(s) = v.as_str() {
        let s = s.trim().trim_start_matches('[').trim_end_matches(']');
        s.split(',')
            .map(|x| {
                x.trim()
                    .parse::<f64>()
                    .map_err(|_| format!("cannot parse '{}'", x.trim()))
            })
            .collect()
    } else {
        Err("expected array or comma-separated string".to_string())
    }
}

fn fmt_vec(v: &[f64]) -> String {
    let comps: Vec<String> = v.iter().map(|x| fmt_f(*x)).collect();
    format!("[{}]", comps.join(", "))
}

fn fmt_f(x: f64) -> String {
    if x.fract() == 0.0 && x.abs() < 1e12 {
        format!("{}", x as i64)
    } else {
        format!("{:.6}", x)
            .trim_end_matches('0')
            .trim_end_matches('.')
            .to_string()
    }
}

fn magnitude(v: &[f64]) -> f64 {
    v.iter().map(|x| x * x).sum::<f64>().sqrt()
}

fn dot(a: &[f64], b: &[f64]) -> Result<f64, String> {
    if a.len() != b.len() {
        return Err(format!("dimension mismatch ({} vs {})", a.len(), b.len()));
    }
    Ok(a.iter().zip(b.iter()).map(|(x, y)| x * y).sum())
}

fn add_vecs(a: &[f64], b: &[f64]) -> Result<Vec<f64>, String> {
    if a.len() != b.len() {
        return Err(format!("dimension mismatch ({} vs {})", a.len(), b.len()));
    }
    Ok(a.iter().zip(b.iter()).map(|(x, y)| x + y).collect())
}

fn sub_vecs(a: &[f64], b: &[f64]) -> Result<Vec<f64>, String> {
    if a.len() != b.len() {
        return Err(format!("dimension mismatch ({} vs {})", a.len(), b.len()));
    }
    Ok(a.iter().zip(b.iter()).map(|(x, y)| x - y).collect())
}

fn cross3(a: &[f64], b: &[f64]) -> Result<Vec<f64>, String> {
    if a.len() != 3 || b.len() != 3 {
        return Err("cross product requires 3D vectors".to_string());
    }
    Ok(vec![
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ])
}

fn normalize(v: &[f64]) -> Result<Vec<f64>, String> {
    let mag = magnitude(v);
    if mag == 0.0 {
        return Err("cannot normalize zero vector".to_string());
    }
    Ok(v.iter().map(|x| x / mag).collect())
}

fn get_v<'a>(args: &'a Value) -> &'a Value {
    if !args["v"].is_null() {
        &args["v"]
    } else {
        &args["vector"]
    }
}

fn get_a<'a>(args: &'a Value) -> &'a Value {
    if !args["a"].is_null() {
        &args["a"]
    } else {
        get_v(args)
    }
}

fn action_info(args: &Value) -> Result<String, String> {
    let v_val = get_v(args);
    if v_val.is_null() {
        return Ok(
            "Vector tools: 2D/3D (and nD) vector math.\n\
             Actions: info, add, subtract, scale, dot, cross, magnitude, normalize, angle, project, reflect\n\
             Pass 'v' (or 'vector') as JSON array or comma-separated string.\n\
             For binary ops also pass 'b' (second vector). For scale pass 'scalar'."
                .to_string(),
        );
    }
    let v = parse_vec(v_val)?;
    let mag = magnitude(&v);
    let unit = normalize(&v);
    let mut lines = Vec::new();
    lines.push(format!("Vector:    {}", fmt_vec(&v)));
    lines.push(format!("Dimension: {}D", v.len()));
    lines.push(format!("Magnitude: {}", fmt_f(mag)));
    if let Ok(u) = unit {
        lines.push(format!("Unit:      {}", fmt_vec(&u)));
    }
    if v.len() == 2 {
        let angle_deg = v[1].atan2(v[0]).to_degrees();
        lines.push(format!("Angle (from +x): {:.4}°", angle_deg));
    }
    if v.len() == 3 {
        lines.push(format!(
            "Components: x={}, y={}, z={}",
            fmt_f(v[0]),
            fmt_f(v[1]),
            fmt_f(v[2])
        ));
    }
    Ok(lines.join("\n"))
}

fn action_add(args: &Value) -> Result<String, String> {
    let a = parse_vec(get_a(args))?;
    let b = parse_vec(&args["b"])?;
    let r = add_vecs(&a, &b)?;
    Ok(format!(
        "{} + {} = {}",
        fmt_vec(&a),
        fmt_vec(&b),
        fmt_vec(&r)
    ))
}

fn action_subtract(args: &Value) -> Result<String, String> {
    let a = parse_vec(get_a(args))?;
    let b = parse_vec(&args["b"])?;
    let r = sub_vecs(&a, &b)?;
    Ok(format!(
        "{} - {} = {}",
        fmt_vec(&a),
        fmt_vec(&b),
        fmt_vec(&r)
    ))
}

fn action_scale(args: &Value) -> Result<String, String> {
    let v = parse_vec(get_v(args))?;
    let s = args["scalar"]
        .as_f64()
        .or_else(|| args["s"].as_f64())
        .ok_or("pass 'scalar' (number)")?;
    let r: Vec<f64> = v.iter().map(|x| x * s).collect();
    Ok(format!("{} × {} = {}", fmt_f(s), fmt_vec(&v), fmt_vec(&r)))
}

fn action_dot(args: &Value) -> Result<String, String> {
    let a = parse_vec(get_a(args))?;
    let b = parse_vec(&args["b"])?;
    let d = dot(&a, &b)?;
    let mag_a = magnitude(&a);
    let mag_b = magnitude(&b);
    let mut lines = vec![format!("{} · {} = {}", fmt_vec(&a), fmt_vec(&b), fmt_f(d))];
    if mag_a > 0.0 && mag_b > 0.0 {
        let cos_theta = (d / (mag_a * mag_b)).clamp(-1.0, 1.0);
        let angle = cos_theta.acos().to_degrees();
        lines.push(format!("Angle between: {:.4}°", angle));
        if d.abs() < 1e-10 {
            lines.push("Vectors are perpendicular (orthogonal)".to_string());
        }
    }
    Ok(lines.join("\n"))
}

fn action_cross(args: &Value) -> Result<String, String> {
    let a = parse_vec(get_a(args))?;
    let b = parse_vec(&args["b"])?;
    let r = cross3(&a, &b)?;
    let d = dot(&a, &b)?;
    let mut lines = vec![format!(
        "{} × {} = {}",
        fmt_vec(&a),
        fmt_vec(&b),
        fmt_vec(&r)
    )];
    lines.push(format!("|cross product| = {}", fmt_f(magnitude(&r))));
    if d.abs() < 1e-10 && magnitude(&a) > 0.0 && magnitude(&b) > 0.0 {
        lines.push("Vectors are parallel (cross product ≈ zero)".to_string());
    }
    Ok(lines.join("\n"))
}

fn action_magnitude(args: &Value) -> Result<String, String> {
    let v = parse_vec(get_v(args))?;
    let mag = magnitude(&v);
    let mag_sq: f64 = v.iter().map(|x| x * x).sum();
    Ok(format!(
        "Vector:     {}\nMagnitude:  {}\n|v|²:       {}",
        fmt_vec(&v),
        fmt_f(mag),
        fmt_f(mag_sq)
    ))
}

fn action_normalize(args: &Value) -> Result<String, String> {
    let v = parse_vec(get_v(args))?;
    let u = normalize(&v)?;
    let check = magnitude(&u);
    Ok(format!(
        "v:    {}\nunit: {}\n|unit| = {} (verification)",
        fmt_vec(&v),
        fmt_vec(&u),
        fmt_f(check)
    ))
}

fn action_angle(args: &Value) -> Result<String, String> {
    let a = parse_vec(get_a(args))?;
    let b = parse_vec(&args["b"])?;
    if a.len() != b.len() {
        return Err(format!("dimension mismatch ({} vs {})", a.len(), b.len()));
    }
    let d = dot(&a, &b)?;
    let mag_a = magnitude(&a);
    let mag_b = magnitude(&b);
    if mag_a == 0.0 || mag_b == 0.0 {
        return Err("angle undefined for zero vector".to_string());
    }
    let cos_theta = (d / (mag_a * mag_b)).clamp(-1.0, 1.0);
    let angle_rad = cos_theta.acos();
    let angle_deg = angle_rad.to_degrees();
    let mut lines = vec![
        format!("a:        {}", fmt_vec(&a)),
        format!("b:        {}", fmt_vec(&b)),
        format!("Angle:    {:.6}° ({:.6} rad)", angle_deg, angle_rad),
    ];
    if (angle_deg - 90.0).abs() < 1e-6 {
        lines.push("Vectors are perpendicular".to_string());
    } else if angle_deg < 1e-6 {
        lines.push("Vectors are parallel (same direction)".to_string());
    } else if (angle_deg - 180.0).abs() < 1e-6 {
        lines.push("Vectors are antiparallel (opposite directions)".to_string());
    }
    Ok(lines.join("\n"))
}

fn action_project(args: &Value) -> Result<String, String> {
    let a = parse_vec(get_a(args))?;
    let b = parse_vec(&args["b"])?;
    if a.len() != b.len() {
        return Err(format!("dimension mismatch ({} vs {})", a.len(), b.len()));
    }
    let d = dot(&a, &b)?;
    let mag_b_sq: f64 = b.iter().map(|x| x * x).sum();
    if mag_b_sq == 0.0 {
        return Err("cannot project onto zero vector".to_string());
    }
    let scalar_proj = d / magnitude(&b);
    let proj: Vec<f64> = b.iter().map(|x| x * d / mag_b_sq).collect();
    let perp = sub_vecs(&a, &proj)?;
    Ok(format!(
        "Project a = {} onto b = {}\nScalar projection:  {}\nVector projection:  {}\nPerpendicular part: {}",
        fmt_vec(&a),
        fmt_vec(&b),
        fmt_f(scalar_proj),
        fmt_vec(&proj),
        fmt_vec(&perp)
    ))
}

fn action_reflect(args: &Value) -> Result<String, String> {
    let v = parse_vec(get_v(args))?;
    let n_raw = parse_vec(&args["n"])?;
    if v.len() != n_raw.len() {
        return Err(format!(
            "dimension mismatch ({} vs {})",
            v.len(),
            n_raw.len()
        ));
    }
    let n = normalize(&n_raw)?;
    let d = dot(&v, &n)?;
    let reflected: Vec<f64> = v
        .iter()
        .zip(n.iter())
        .map(|(vi, ni)| vi - 2.0 * d * ni)
        .collect();
    Ok(format!(
        "v:         {}\nnormal:    {} (normalized)\nreflected: {}",
        fmt_vec(&v),
        fmt_vec(&n),
        fmt_vec(&reflected)
    ))
}
