use serde_json::{json, Value};

pub fn schema() -> Value {
    json!({
        "name": "materials_tools",
        "description": "Materials science calculations: properties lookup, stress/strain, thermal expansion, beam bending, hardness, buoyancy, safety factor, and crystal structures.",
        "parameters": {
            "type": "object",
            "properties": {
                "action": { "type": "string" },
                "material": { "type": "string" },
                "query": { "type": "string" },
                "stress": { "type": "number" },
                "strain": { "type": "number" },
                "force": { "type": "number" },
                "area": { "type": "number" },
                "delta_l": { "type": "number" },
                "l0": { "type": "number" },
                "modulus": { "type": "number" },
                "alpha": { "type": "number" },
                "delta_t": { "type": "number" },
                "length": { "type": "number" },
                "width": { "type": "number" },
                "height": { "type": "number" },
                "diameter": { "type": "number" },
                "moment": { "type": "number" },
                "distance": { "type": "number" },
                "section": { "type": "string" },
                "rho": { "type": "number" },
                "volume": { "type": "number" },
                "depth": { "type": "number" },
                "g": { "type": "number" },
                "failure_load": { "type": "number" },
                "working_load": { "type": "number" },
                "yield_strength": { "type": "number" },
                "applied_stress": { "type": "number" },
                "crystal": { "type": "string" },
                "lattice_a": { "type": "number" }
            },
            "required": []
        }
    })
}

pub async fn execute(args: &Value) -> Result<String, String> {
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("properties");
    match action {
        "properties" | "lookup" | "material" => action_properties(args),
        "stress" | "strain" | "elastic" => action_stress(args),
        "thermal" | "expansion" => action_thermal(args),
        "bending" | "beam" | "moment" => action_bending(args),
        "hardness" | "mohs" => action_hardness(args),
        "pressure" | "buoyancy" | "hydrostatic" => action_pressure(args),
        "safety" | "factor" | "fatigue" => action_safety(args),
        "crystal" | "unit_cell" => action_crystal(args),
        _ => Err(format!(
            "Unknown action '{action}'. Valid: properties, stress, thermal, bending, hardness, pressure, safety, crystal"
        )),
    }
}

// Material database: (name, density kg/m³, E GPa, ν Poisson, σ_y MPa, α 1e-6/°C, k W/mK)
fn material_table() -> Vec<(&'static str, f64, f64, f64, f64, f64, f64)> {
    vec![
        // name              ρ(kg/m³)  E(GPa)   ν       σ_y(MPa)  α(µ/°C)  k(W/mK)
        ("steel_mild", 7850.0, 200.0, 0.30, 250.0, 11.7, 50.0),
        ("steel_stainless", 8000.0, 193.0, 0.28, 215.0, 17.2, 16.0),
        ("aluminum_6061", 2700.0, 69.0, 0.33, 276.0, 23.6, 167.0),
        ("aluminum_pure", 2710.0, 70.0, 0.35, 15.0, 23.1, 237.0),
        ("copper", 8940.0, 117.0, 0.34, 70.0, 17.0, 401.0),
        ("brass", 8500.0, 100.0, 0.34, 200.0, 19.0, 120.0),
        ("titanium", 4500.0, 116.0, 0.34, 880.0, 8.6, 22.0),
        ("cast_iron", 7200.0, 100.0, 0.26, 250.0, 10.8, 55.0),
        ("concrete", 2300.0, 30.0, 0.20, 30.0, 12.0, 1.7),
        ("glass", 2500.0, 70.0, 0.23, 50.0, 8.5, 1.0),
        ("wood_pine", 530.0, 12.0, 0.40, 40.0, 5.0, 0.14),
        ("wood_oak", 700.0, 12.0, 0.40, 60.0, 5.0, 0.17),
        ("nylon", 1140.0, 3.0, 0.40, 60.0, 80.0, 0.25),
        ("polycarbonate", 1200.0, 2.3, 0.37, 60.0, 65.0, 0.20),
        ("rubber", 1200.0, 0.001, 0.49, 5.0, 160.0, 0.16),
        ("carbon_fiber", 1600.0, 230.0, 0.20, 3500.0, 0.5, 5.0),
        ("kevlar", 1440.0, 125.0, 0.36, 3600.0, -4.0, 0.04),
        ("bone_cortical", 1900.0, 17.0, 0.30, 130.0, 11.0, 0.56),
    ]
}

fn find_material(name: &str) -> Option<(&'static str, f64, f64, f64, f64, f64, f64)> {
    let lower = name.to_lowercase();
    material_table()
        .into_iter()
        .find(|(n, ..)| n.to_lowercase().contains(&lower) || lower.contains(&n.replace('_', " ")))
}

fn action_properties(args: &Value) -> Result<String, String> {
    let query = args
        .get("material")
        .or_else(|| args.get("query"))
        .or_else(|| args.get("name"))
        .and_then(|v| v.as_str());

    if let Some(q) = query {
        if let Some((name, rho, e, nu, sy, alpha, k)) = find_material(q) {
            let mut out = String::from("MATERIAL PROPERTIES\n");
            out.push_str("═══════════════════════════════════════\n");
            out.push_str(&format!(
                "  Material           {}\n",
                name.replace('_', " ")
            ));
            out.push_str(&format!("  Density (ρ)        {:.0} kg/m³\n", rho));
            out.push_str(&format!("  Young's modulus (E) {:.1} GPa\n", e));
            out.push_str(&format!("  Poisson's ratio (ν) {:.2}\n", nu));
            out.push_str(&format!(
                "  Shear modulus (G)  {:.1} GPa  [E/(2(1+ν))]\n",
                e / (2.0 * (1.0 + nu))
            ));
            out.push_str(&format!(
                "  Bulk modulus (K)   {:.1} GPa  [E/(3(1-2ν))]\n",
                e / (3.0 * (1.0 - 2.0 * nu))
            ));
            out.push_str(&format!("  Yield strength (σ_y) {:.0} MPa\n", sy));
            out.push_str(&format!("  Thermal expansion (α) {:.1} µm/m·°C\n", alpha));
            out.push_str(&format!("  Thermal conductivity (k) {:.2} W/m·K\n", k));
            return Ok(out);
        } else {
            // Fuzzy list
            let mut out = format!("No exact match for '{}'. Available materials:\n\n", q);
            for (name, ..) in material_table() {
                out.push_str(&format!("  {}\n", name.replace('_', " ")));
            }
            return Ok(out);
        }
    }

    // List all
    let mut out = String::from("MATERIALS DATABASE\n");
    out.push_str("═══════════════════════════════════════\n");
    out.push_str(&format!(
        "  {:<22}  {:>8}  {:>7}  {:>7}  {:>8}\n",
        "Material", "ρ kg/m³", "E GPa", "ν", "σ_y MPa"
    ));
    out.push_str("  ──────────────────────────────────────────────────────\n");
    for (name, rho, e, nu, sy, ..) in material_table() {
        out.push_str(&format!(
            "  {:<22}  {:>8.0}  {:>7.1}  {:>7.2}  {:>8.0}\n",
            name.replace('_', " "),
            rho,
            e,
            nu,
            sy
        ));
    }
    out.push_str("\nProvide 'material' to see full properties for one material.\n");
    Ok(out)
}

fn action_stress(args: &Value) -> Result<String, String> {
    // σ = F/A, ε = ΔL/L₀, E = σ/ε → solve for any missing
    let stress = args.get("stress").and_then(|v| v.as_f64());
    let strain = args.get("strain").and_then(|v| v.as_f64());
    let force = args.get("force").and_then(|v| v.as_f64());
    let area = args.get("area").and_then(|v| v.as_f64());
    let delta_l = args.get("delta_l").and_then(|v| v.as_f64());
    let l0 = args.get("l0").and_then(|v| v.as_f64());
    let modulus = args
        .get("modulus")
        .or_else(|| args.get("e"))
        .and_then(|v| v.as_f64());

    let mut out = String::from("STRESS / STRAIN / ELASTIC MODULUS\n");
    out.push_str("═══════════════════════════════════════\n");

    // Compute stress from force/area
    let sigma = match (stress, force, area) {
        (Some(s), _, _) => s,
        (None, Some(f), Some(a)) if a > 0.0 => {
            let s = f / a;
            out.push_str(&format!(
                "  σ = F/A = {:.4e} / {:.4e} = {:.4e} Pa\n",
                f, a, s
            ));
            s
        }
        (None, Some(_), None) => {
            return Err("provide 'area' (m²) when computing stress from force".into())
        }
        (None, None, Some(_)) => {
            return Err("provide 'force' (N) when computing stress from area".into())
        }
        _ => f64::NAN,
    };

    // Compute strain from delta_l/l0
    let eps = match (strain, delta_l, l0) {
        (Some(e), _, _) => e,
        (None, Some(dl), Some(l)) if l > 0.0 => {
            let e = dl / l;
            out.push_str(&format!(
                "  ε = ΔL/L₀ = {:.4e} / {:.4e} = {:.6}\n",
                dl, l, e
            ));
            e
        }
        _ => f64::NAN,
    };

    // Summary
    if !sigma.is_nan() {
        out.push_str(&format!(
            "  Stress (σ)          {:.4e} Pa  ({:.4} MPa)\n",
            sigma,
            sigma / 1e6
        ));
    }
    if !eps.is_nan() {
        out.push_str(&format!(
            "  Strain (ε)          {:.6}  (dimensionless)\n",
            eps
        ));
    }

    // Compute E if both known
    if !sigma.is_nan() && !eps.is_nan() && eps.abs() > 0.0 {
        let e_calc = sigma / eps;
        out.push_str(&format!(
            "  Young's modulus (E) {:.4e} Pa  ({:.2} GPa)  [= σ/ε]\n",
            e_calc,
            e_calc / 1e9
        ));
    } else if !sigma.is_nan() && modulus.is_some() {
        let e = modulus.unwrap();
        let e_calc = if e < 1e6 { e * 1e9 } else { e }; // accept GPa or Pa input
        let eps_calc = sigma / e_calc;
        out.push_str(&format!(
            "  Young's modulus (E) {:.2} GPa  (given)\n",
            e_calc / 1e9
        ));
        out.push_str(&format!("  Strain (ε = σ/E)    {:.6}\n", eps_calc));
        if let Some(l) = l0 {
            let dl = eps_calc * l;
            out.push_str(&format!(
                "  Elongation (ΔL)     {:.4e} m  (= {:.4} mm)\n",
                dl,
                dl * 1000.0
            ));
        }
    } else if !eps.is_nan() {
        if let Some(e) = modulus {
            let e_calc = if e < 1e6 { e * 1e9 } else { e };
            let sigma_calc = e_calc * eps;
            out.push_str(&format!(
                "  Young's modulus (E) {:.2} GPa  (given)\n",
                e_calc / 1e9
            ));
            out.push_str(&format!(
                "  Stress (σ = E·ε)    {:.4e} Pa  ({:.4} MPa)\n",
                sigma_calc,
                sigma_calc / 1e6
            ));
        }
    }

    if sigma.is_nan() && eps.is_nan() {
        out.push_str("Provide combinations of 'stress'/'force'+'area', 'strain'/'delta_l'+'l0', and 'modulus' (GPa or Pa).\n");
    }

    Ok(out)
}

fn action_thermal(args: &Value) -> Result<String, String> {
    // ΔL = α · L₀ · ΔT
    // Thermal stress: σ = E · α · ΔT  (fully constrained)
    let mat_name = args.get("material").and_then(|v| v.as_str());
    let alpha_in = args.get("alpha").and_then(|v| v.as_f64()); // µm/m·°C or /°C
    let delta_t = args
        .get("delta_t")
        .or_else(|| args.get("dt"))
        .and_then(|v| v.as_f64());
    let l0 = args
        .get("l0")
        .or_else(|| args.get("length"))
        .and_then(|v| v.as_f64());
    let modulus = args
        .get("modulus")
        .or_else(|| args.get("e"))
        .and_then(|v| v.as_f64());

    // Resolve alpha
    let (alpha, e_gpa): (f64, Option<f64>) = if let Some(a) = alpha_in {
        let a_si = if a > 1e-3 { a * 1e-6 } else { a }; // convert µm/m/°C to 1/°C
        (a_si, modulus)
    } else if let Some(name) = mat_name {
        match find_material(name) {
            Some((_, _, e, _, _, alpha, _)) => (alpha * 1e-6, modulus.or(Some(e * 1e9))),
            None => return Err(format!("Material '{}' not found in database", name)),
        }
    } else {
        return Err("provide 'alpha' (µm/m·°C) or 'material' name, and 'delta_t' (°C)".into());
    };

    let delta_t = match delta_t {
        Some(dt) => dt,
        None => return Err("provide 'delta_t' (temperature change in °C or K)".into()),
    };

    let mut out = String::from("THERMAL EXPANSION\n");
    out.push_str("═══════════════════════════════════════\n");
    if let Some(name) = mat_name {
        out.push_str(&format!("  Material            {}\n", name));
    }
    out.push_str(&format!(
        "  Thermal expansion α {:.2} µm/m·°C  = {:.4e} /°C\n",
        alpha * 1e6,
        alpha
    ));
    out.push_str(&format!("  Temperature change  {:+.2} °C\n", delta_t));

    if let Some(l) = l0 {
        let dl = alpha * l * delta_t;
        out.push_str(&format!("  Original length L₀  {:.4e} m\n", l));
        out.push_str("\n  ΔL = α · L₀ · ΔT\n");
        out.push_str(&format!(
            "  Elongation ΔL       {:.4e} m  ({:.4} mm)\n",
            dl,
            dl * 1000.0
        ));
        out.push_str(&format!("  New length L        {:.4e} m\n", l + dl));
    } else {
        out.push_str("\n  Provide 'l0' (metres) to compute elongation ΔL.\n");
    }

    // Thermal stress if constrained
    if let Some(e) = e_gpa {
        let e_si = if e < 1e6 { e * 1e9 } else { e };
        let sigma = e_si * alpha * delta_t.abs();
        out.push_str("\n  Thermal stress (fully constrained):\n");
        out.push_str(&format!("  E = {:.2} GPa\n", e_si / 1e9));
        out.push_str(&format!(
            "  σ_thermal = E·α·|ΔT| = {:.4e} Pa  ({:.2} MPa)\n",
            sigma,
            sigma / 1e6
        ));
        if delta_t < 0.0 {
            out.push_str("  (contraction → tensile stress if constrained)\n");
        } else {
            out.push_str("  (expansion → compressive stress if constrained)\n");
        }
    }

    // Common thermal expansion coefficients table (if no specific material)
    if mat_name.is_none() && alpha_in.is_some() {
        out.push_str("\n  Reference CTE values:\n");
        out.push_str("  Invar:          1.2 µm/m·°C\n");
        out.push_str("  Carbon steel:   11.7 µm/m·°C\n");
        out.push_str("  Stainless steel: 17.2 µm/m·°C\n");
        out.push_str("  Aluminum:       23.1 µm/m·°C\n");
        out.push_str("  Copper:         17.0 µm/m·°C\n");
        out.push_str("  Glass:           8.5 µm/m·°C\n");
        out.push_str("  Concrete:       12.0 µm/m·°C\n");
    }

    Ok(out)
}

fn action_bending(args: &Value) -> Result<String, String> {
    // σ_max = M·c / I   (bending stress)
    // Cross sections: rectangular, circular, hollow_circular, I_beam
    let section = args
        .get("section")
        .and_then(|v| v.as_str())
        .unwrap_or("rectangular");
    let moment = args.get("moment").and_then(|v| v.as_f64()); // N·m

    let mut out = String::from("BEAM BENDING\n");
    out.push_str("═══════════════════════════════════════\n");

    // Compute moment of inertia I and centroid distance c
    let (i, c, section_desc) = match section {
        "rectangular" | "rect" => {
            let width = match args.get("width").and_then(|v| v.as_f64()) {
                Some(w) if w > 0.0 => w,
                _ => return Err("provide 'width' (m) for rectangular section".into()),
            };
            let height = match args.get("height").and_then(|v| v.as_f64()) {
                Some(h) if h > 0.0 => h,
                _ => return Err("provide 'height' (m) for rectangular section".into()),
            };
            let i = width * height.powi(3) / 12.0;
            let c = height / 2.0;
            let desc = format!("Rectangular  b={:.4} m  h={:.4} m", width, height);
            (i, c, desc)
        }
        "circular" | "circle" => {
            let d = match args.get("diameter").and_then(|v| v.as_f64()) {
                Some(d) if d > 0.0 => d,
                _ => return Err("provide 'diameter' (m) for circular section".into()),
            };
            let i = std::f64::consts::PI * d.powi(4) / 64.0;
            let c = d / 2.0;
            let desc = format!("Circular  d={:.4} m", d);
            (i, c, desc)
        }
        "hollow_circular" | "hollow" | "tube" => {
            let d_outer = match args.get("diameter").and_then(|v| v.as_f64()) {
                Some(d) if d > 0.0 => d,
                _ => {
                    return Err(
                        "provide 'diameter' (outer, m) and 'inner_diameter' for hollow section"
                            .into(),
                    )
                }
            };
            let d_inner = match args
                .get("inner_diameter")
                .or_else(|| args.get("d_inner"))
                .and_then(|v| v.as_f64())
            {
                Some(d) if d > 0.0 && d < d_outer => d,
                _ => return Err("provide 'inner_diameter' (m) < outer diameter".into()),
            };
            let i = std::f64::consts::PI * (d_outer.powi(4) - d_inner.powi(4)) / 64.0;
            let c = d_outer / 2.0;
            let desc = format!("Hollow Circular  OD={:.4} m  ID={:.4} m", d_outer, d_inner);
            (i, c, desc)
        }
        _ => {
            return Err(format!(
                "Unknown section '{section}'. Valid: rectangular, circular, hollow_circular"
            ))
        }
    };

    out.push_str(&format!("  Section: {}\n", section_desc));
    out.push_str(&format!("  I (2nd moment of area) = {:.4e} m⁴\n", i));
    out.push_str(&format!("  c (neutral axis dist.) = {:.4e} m\n", c));
    out.push_str(&format!("  S = I/c (section mod.)  = {:.4e} m³\n", i / c));

    if let Some(m) = moment {
        let sigma_max = m * c / i;
        out.push_str(&format!("\n  Bending moment M        {:.4e} N·m\n", m));
        out.push_str("  σ_max = M·c / I\n");
        out.push_str(&format!(
            "  Max bending stress      {:.4e} Pa  ({:.4} MPa)\n",
            sigma_max,
            sigma_max / 1e6
        ));

        // Check against material yield if provided
        if let Some(sigma_y) = args.get("yield_strength").and_then(|v| v.as_f64()) {
            let sy_pa = if sigma_y < 1e3 {
                sigma_y * 1e6
            } else {
                sigma_y
            };
            let fs = sy_pa / sigma_max;
            out.push_str(&format!(
                "  Yield strength (σ_y)    {:.0} MPa  (given)\n",
                sy_pa / 1e6
            ));
            out.push_str(&format!("  Factor of safety        {:.3}\n", fs));
            if fs < 1.0 {
                out.push_str("  ⚠  Yield exceeded!\n");
            } else if fs < 1.5 {
                out.push_str("  ⚠  Low safety factor — review design\n");
            }
        }
    } else {
        out.push_str("\nProvide 'moment' (N·m) to compute bending stress.\n");
        out.push_str("Optional: 'yield_strength' (MPa) for safety factor.\n");
    }

    Ok(out)
}

fn action_hardness(args: &Value) -> Result<String, String> {
    let query = args
        .get("material")
        .or_else(|| args.get("query"))
        .and_then(|v| v.as_str());

    // Mohs hardness table
    let mohs: &[(&str, f64, &str)] = &[
        ("talc", 1.0, "fingernail scratches it easily"),
        ("gypsum", 2.0, "fingernail (2.2) scratches it"),
        ("calcite", 3.0, "copper coin (3.2) scratches it"),
        ("fluorite", 4.0, "steel knife scratches it"),
        ("apatite", 5.0, "glass (5.5) barely scratches it"),
        ("orthoclase feldspar", 6.0, "steel file scratches it"),
        ("quartz", 7.0, "scratches glass and steel"),
        ("topaz", 8.0, "harder than most steels"),
        ("corundum (sapphire/ruby)", 9.0, "only diamond scratches it"),
        ("diamond", 10.0, "hardest known natural material"),
    ];

    let engineering_hardness: &[(&str, f64, f64, f64)] = &[
        // (material, Mohs~, Vickers HV, Brinell HB)
        ("lead", 1.5, 4.0, 5.0),
        ("tin", 1.5, 5.0, 6.0),
        ("gold", 2.5, 25.0, 25.0),
        ("aluminum (pure)", 2.5, 15.0, 15.0),
        ("copper", 3.0, 70.0, 60.0),
        ("mild steel", 5.5, 120.0, 120.0),
        ("stainless steel", 5.5, 200.0, 180.0),
        ("titanium", 6.0, 250.0, 220.0),
        ("hardened steel", 6.5, 700.0, 600.0),
        ("tungsten carbide", 9.0, 2100.0, 0.0),
        ("diamond", 10.0, 10000.0, 0.0),
    ];

    let mut out = String::from("MATERIAL HARDNESS\n");
    out.push_str("═══════════════════════════════════════\n");

    if let Some(q) = query {
        let lower = q.to_lowercase();
        // Search Mohs table
        let mohs_match = mohs
            .iter()
            .find(|(name, ..)| name.contains(lower.as_str()) || lower.contains(name));
        let eng_match = engineering_hardness.iter().find(|(name, ..)| {
            name.to_lowercase().contains(&lower) || lower.contains(&name.to_lowercase())
        });
        if let Some((name, hardness, note)) = mohs_match {
            out.push_str(&format!("  {} — Mohs hardness: {:.1}\n", name, hardness));
            out.push_str(&format!("  Reference mineral: {}\n", name));
            out.push_str(&format!("  Scratch test note: {}\n", note));
        }
        if let Some((name, mohs_approx, hv, hb)) = eng_match {
            out.push_str(&format!("\n  {} — Engineering hardness:\n", name));
            out.push_str(&format!("    Mohs (approx):    {:.1}\n", mohs_approx));
            out.push_str(&format!("    Vickers (HV):     {:.0}\n", hv));
            if *hb > 0.0 {
                out.push_str(&format!("    Brinell (HB):     {:.0}\n", hb));
            }
        }
        if mohs_match.is_none() && eng_match.is_none() {
            out.push_str(&format!(
                "  No match for '{}'. See full table below.\n\n",
                q
            ));
        } else {
            return Ok(out);
        }
    }

    out.push_str("  Mohs Scale:\n");
    out.push_str("  ───────────────────────────────────────────────────\n");
    for (name, hardness, note) in mohs {
        out.push_str(&format!("  {:2}  {:<30}  {}\n", hardness, name, note));
    }
    out.push_str("\n  Engineering Hardness:\n");
    out.push_str("  ─────────────────────────────────────────────────\n");
    out.push_str(&format!(
        "  {:<22}  {:>7}  {:>7}  {:>7}\n",
        "Material", "Mohs~", "HV", "HB"
    ));
    for (name, mohs_approx, hv, hb) in engineering_hardness {
        let hb_str = if *hb > 0.0 {
            format!("{:.0}", hb)
        } else {
            "N/A".to_string()
        };
        out.push_str(&format!(
            "  {:<22}  {:>7.1}  {:>7.0}  {:>7}\n",
            name, mohs_approx, hv, hb_str
        ));
    }
    out.push_str("\nProvide 'material' to search.\n");
    Ok(out)
}

fn action_pressure(args: &Value) -> Result<String, String> {
    // Hydrostatic pressure P = ρ·g·h
    // Buoyancy F_b = ρ_fluid·g·V
    let rho = args
        .get("rho")
        .or_else(|| args.get("density"))
        .and_then(|v| v.as_f64());
    let g = args.get("g").and_then(|v| v.as_f64()).unwrap_or(9.81);
    let depth = args
        .get("depth")
        .or_else(|| args.get("h"))
        .and_then(|v| v.as_f64());
    let volume = args
        .get("volume")
        .or_else(|| args.get("v"))
        .and_then(|v| v.as_f64());

    let mut out = String::from("HYDROSTATIC PRESSURE & BUOYANCY\n");
    out.push_str("═══════════════════════════════════════\n");

    match (rho, depth) {
        (Some(r), Some(h)) if r > 0.0 && h > 0.0 => {
            let pressure = r * g * h;
            out.push_str(&format!("  Fluid density (ρ)   {:.2} kg/m³\n", r));
            out.push_str(&format!("  Depth (h)           {:.2} m\n", h));
            out.push_str(&format!("  g                   {:.4} m/s²\n", g));
            out.push_str("\n  P = ρ·g·h\n");
            out.push_str(&format!(
                "  Hydrostatic pressure {:.4e} Pa  ({:.4} kPa)\n",
                pressure,
                pressure / 1e3
            ));
            out.push_str(&format!(
                "                       {:.4} atm  ({:.4} bar)\n",
                pressure / 101325.0,
                pressure / 1e5
            ));
            out.push_str(&format!(
                "                       {:.4} psi\n",
                pressure / 6894.76
            ));

            if let Some(v) = volume {
                let fb = r * g * v;
                out.push_str(&format!("\n  Volume (V)          {:.4e} m³\n", v));
                out.push_str("  F_b = ρ·g·V  (Archimedes)\n");
                out.push_str(&format!(
                    "  Buoyant force       {:.4e} N  ({:.4} kN)\n",
                    fb,
                    fb / 1e3
                ));
            }
        }
        (Some(r), None) if r > 0.0 => {
            if let Some(v) = volume {
                let fb = r * g * v;
                out.push_str(&format!("  Fluid density (ρ)   {:.2} kg/m³\n", r));
                out.push_str(&format!("  Submerged volume    {:.4e} m³\n", v));
                out.push_str("\n  F_b = ρ·g·V\n");
                out.push_str(&format!("  Buoyant force       {:.4e} N\n", fb));
            } else {
                out.push_str(
                    "Provide 'depth' (m) for hydrostatic pressure or 'volume' (m³) for buoyancy.\n",
                );
            }
        }
        _ => {
            out.push_str("  Provide 'rho' (kg/m³) and one of:\n");
            out.push_str("    'depth' (m) for hydrostatic pressure P = ρ·g·h\n");
            out.push_str("    'volume' (m³) for buoyant force F_b = ρ·g·V\n\n");
            out.push_str("  Common fluid densities:\n");
            let fluids: &[(&str, f64)] = &[
                ("Fresh water (20°C)", 998.0),
                ("Salt water", 1025.0),
                ("Air (20°C)", 1.21),
                ("Ethanol", 789.0),
                ("Mercury", 13_534.0),
                ("Glycerin", 1261.0),
                ("Gasoline", 750.0),
            ];
            for (name, d) in fluids {
                out.push_str(&format!("    {:24}  {:.0} kg/m³\n", name, d));
            }
        }
    }

    Ok(out)
}

fn action_safety(args: &Value) -> Result<String, String> {
    // Factor of safety: FS = σ_failure / σ_applied
    let failure_load = args
        .get("failure_load")
        .or_else(|| args.get("ultimate"))
        .and_then(|v| v.as_f64());
    let working_load = args
        .get("working_load")
        .or_else(|| args.get("applied"))
        .and_then(|v| v.as_f64());
    let yield_strength = args
        .get("yield_strength")
        .or_else(|| args.get("yield"))
        .and_then(|v| v.as_f64());
    let applied_stress = args
        .get("applied_stress")
        .or_else(|| args.get("sigma"))
        .and_then(|v| v.as_f64());

    let mut out = String::from("FACTOR OF SAFETY\n");
    out.push_str("═══════════════════════════════════════\n");

    // Load-based FS
    if let (Some(fl), Some(wl)) = (failure_load, working_load) {
        if wl <= 0.0 {
            return Err("working_load must be positive".into());
        }
        let fs = fl / wl;
        out.push_str(&format!("  Failure load        {:.4e} N\n", fl));
        out.push_str(&format!("  Working load        {:.4e} N\n", wl));
        out.push_str("  FS = failure_load / working_load\n");
        out.push_str(&format!("  Factor of safety    {:.4}\n", fs));
        out.push_str(&format!("  Assessment          {}\n", fs_label(fs)));
        return Ok(out);
    }

    // Stress-based FS
    if let (Some(sy), Some(sa)) = (yield_strength, applied_stress) {
        if sa <= 0.0 {
            return Err("applied_stress must be positive".into());
        }
        // Accept MPa or Pa — if sy < 1e3 assume MPa
        let sy_pa = if sy < 1e3 { sy * 1e6 } else { sy };
        let sa_pa = if sa < 1e3 { sa * 1e6 } else { sa };
        let fs = sy_pa / sa_pa;
        out.push_str(&format!("  Yield strength σ_y  {:.2} MPa\n", sy_pa / 1e6));
        out.push_str(&format!("  Applied stress σ    {:.2} MPa\n", sa_pa / 1e6));
        out.push_str("  FS = σ_y / σ_applied\n");
        out.push_str(&format!("  Factor of safety    {:.4}\n", fs));
        out.push_str(&format!("  Assessment          {}\n", fs_label(fs)));
        return Ok(out);
    }

    // Reference table
    out.push_str("  Provide (failure_load + working_load) or (yield_strength + applied_stress).\n");
    out.push_str("  Stress values accepted in MPa or Pa.\n\n");
    out.push_str("  Typical safety factor guidelines:\n");
    out.push_str("  ───────────────────────────────────────────────────────\n");
    let guidelines: &[(&str, &str)] = &[
        ("< 1.0", "Failure — do not use"),
        ("1.0 – 1.5", "Marginal — well-known loads, ductile material"),
        ("1.5 – 2.5", "Typical for most engineering design"),
        ("2.5 – 4.0", "Used for unknown loads or impact loads"),
        (
            "4.0 – 8.0",
            "Safety-critical: lifting, pressure vessels, bridges",
        ),
        ("> 8.0", "Very conservative or brittle material"),
    ];
    for (fs_range, context) in guidelines {
        out.push_str(&format!("  {:10}  {}\n", fs_range, context));
    }
    Ok(out)
}

fn fs_label(fs: f64) -> &'static str {
    if fs < 1.0 {
        "FAILURE — structure will fail"
    } else if fs < 1.5 {
        "Marginal — review material and load assumptions"
    } else if fs < 2.5 {
        "Acceptable for standard engineering"
    } else if fs < 4.0 {
        "Good — suitable for uncertain or dynamic loads"
    } else if fs < 8.0 {
        "Conservative — safety-critical application"
    } else {
        "Very conservative"
    }
}

fn action_crystal(args: &Value) -> Result<String, String> {
    let crystal = args
        .get("crystal")
        .or_else(|| args.get("type"))
        .and_then(|v| v.as_str())
        .unwrap_or("list");
    let lattice_a = args
        .get("lattice_a")
        .or_else(|| args.get("a"))
        .and_then(|v| v.as_f64());

    let structures: &[(&str, &str, f64, usize, f64, f64)] = &[
        // (name, full_name, APF, coord_num, atoms_per_cell, packing fraction description)
        ("fcc", "Face-Centered Cubic (FCC)", 0.7405, 12, 4.0, 0.7405),
        ("bcc", "Body-Centered Cubic (BCC)", 0.6802, 8, 2.0, 0.6802),
        (
            "hcp",
            "Hexagonal Close-Packed (HCP)",
            0.7405,
            12,
            6.0,
            0.7405,
        ),
        (
            "sc",
            "Simple Cubic (SC)",
            std::f64::consts::FRAC_PI_6,
            6,
            1.0,
            std::f64::consts::FRAC_PI_6,
        ),
        ("diamond", "Diamond Cubic", 0.3401, 4, 8.0, 0.3401),
    ];

    let crystal_lower = crystal.to_lowercase();

    if crystal_lower == "list" {
        let mut out = String::from("CRYSTAL STRUCTURES\n");
        out.push_str("═══════════════════════════════════════\n");
        out.push_str(&format!(
            "  {:<10}  {:<30}  {:>6}  {:>8}  {:>12}\n",
            "Type", "Full Name", "APF", "Coord#", "Atoms/Cell"
        ));
        out.push_str("  ──────────────────────────────────────────────────────────────\n");
        for (abbr, name, apf, cn, atoms, _) in structures {
            out.push_str(&format!(
                "  {:<10}  {:<30}  {:>6.4}  {:>8}  {:>12.1}\n",
                abbr, name, apf, cn, atoms
            ));
        }
        out.push_str(
            "\nProvide 'crystal' (fcc/bcc/hcp/sc/diamond) and optionally 'lattice_a' (nm).\n",
        );
        out.push_str("\n  FCC examples: Cu, Al, Ni, Pb, Au, Ag\n");
        out.push_str("  BCC examples: Fe (α), W, Mo, Cr, V, Ta\n");
        out.push_str("  HCP examples: Mg, Zn, Ti (α), Co, Cd\n");
        out.push_str("  Diamond:      C (diamond), Si, Ge\n");
        return Ok(out);
    }

    let found = structures.iter().find(|(abbr, ..)| crystal_lower == *abbr);

    match found {
        None => Err(format!(
            "Unknown crystal '{crystal}'. Valid: fcc, bcc, hcp, sc, diamond"
        )),
        Some((abbr, name, apf, cn, atoms, _)) => {
            let mut out = String::from("CRYSTAL STRUCTURE\n");
            out.push_str("═══════════════════════════════════════\n");
            out.push_str(&format!(
                "  Structure           {} ({})\n",
                abbr.to_uppercase(),
                name
            ));
            out.push_str(&format!("  Atoms per unit cell {:.1}\n", atoms));
            out.push_str(&format!("  Coordination number {}\n", cn));
            out.push_str(&format!(
                "  Atomic packing factor (APF) = {:.4}  ({:.2}%)\n",
                apf,
                apf * 100.0
            ));

            // Relationship between atomic radius r and lattice parameter a
            let r_over_a = match *abbr {
                "fcc" => {
                    out.push_str("  r = a·√2/4  →  a = 2√2·r\n");
                    2.0_f64.sqrt() / 4.0
                }
                "bcc" => {
                    out.push_str("  r = a·√3/4  →  a = 4r/√3\n");
                    3.0_f64.sqrt() / 4.0
                }
                "sc" => {
                    out.push_str("  r = a/2\n");
                    0.5
                }
                "hcp" => {
                    out.push_str("  r = a/2  (ideal c/a = √(8/3) ≈ 1.633)\n");
                    0.5
                }
                "diamond" => {
                    out.push_str("  r = a·√3/8  →  a = 8r/√3\n");
                    3.0_f64.sqrt() / 8.0
                }
                _ => 0.5,
            };

            if let Some(a_nm) = lattice_a {
                let a = a_nm * 1e-10; // nm → m ... wait, lattice_a in nm
                let r = r_over_a * a_nm; // nm
                out.push_str(&format!("\n  Lattice parameter a = {:.4} nm\n", a_nm));
                out.push_str(&format!("  Atomic radius r     = {:.4} nm\n", r));
                let vol_cell = match *abbr {
                    "hcp" => {
                        let c_a = (8.0_f64 / 3.0).sqrt();
                        let vol = 3.0_f64.sqrt() / 2.0 * a_nm.powi(2) * c_a * a_nm;
                        out.push_str(&format!("  c/a ratio (ideal)   = {:.4}\n", c_a));
                        vol
                    }
                    _ => a_nm.powi(3),
                };
                out.push_str(&format!("  Unit cell volume    = {:.4e} nm³\n", vol_cell));
                let density_theoretical = (*atoms * 1.0) / (vol_cell * 1e-21 * 6.022e23);
                out.push_str(&format!(
                    "  Atoms/nm³           = {:.4e}\n",
                    atoms / vol_cell
                ));
                let _ = (density_theoretical, a); // suppress warnings
            } else {
                out.push_str("\nProvide 'lattice_a' (nm) to compute radius and cell volume.\n");
            }

            // Examples
            let examples = match *abbr {
                "fcc" => "Cu (0.3615 nm), Al (0.4050 nm), Ni (0.3524 nm), Au (0.4078 nm)",
                "bcc" => "Fe-α (0.2866 nm), W (0.3165 nm), Mo (0.3147 nm)",
                "hcp" => "Mg (a=0.321 nm), Zn (a=0.266 nm), Ti-α (a=0.295 nm)",
                "sc" => "Po (0.335 nm) — rare in pure metals",
                "diamond" => "C (0.357 nm), Si (0.543 nm), Ge (0.566 nm)",
                _ => "",
            };
            out.push_str(&format!("\n  Examples: {}\n", examples));
            Ok(out)
        }
    }
}
