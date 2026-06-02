use serde_json::{Map, Value};

const C: f64 = 2.99792458e8;
const EV_TO_J: f64 = 1.602176634e-19;
const M_E: f64 = 9.1093837015e-31;
const M_P: f64 = 1.67262192369e-27;
const M_N: f64 = 1.67492749804e-27;
const M_U_KG: f64 = 1.66053906660e-27;
const M_MUON: f64 = 1.883531627e-28;

fn get_f64(args: &Value, key: &str) -> Option<f64> {
    args.get(key).and_then(|v| v.as_f64())
}

fn get_str<'a>(args: &'a Value, key: &str) -> Option<&'a str> {
    args.get(key).and_then(|v| v.as_str())
}

pub fn schema() -> Value {
    let mut root = Map::new();
    root.insert("name".into(), Value::String("relativity_tools".into()));
    root.insert(
        "description".into(),
        Value::String(
            "Special relativity calculations without external utilities. 8 actions: \
             gamma (default — Lorentz factor from v or beta), lorentz (time dilation + \
             length contraction + velocity addition), energy (relativistic energy and rest \
             mass equivalence), momentum (relativistic momentum and energy-momentum relation), \
             transform (Lorentz coordinate transformation), doppler (relativistic Doppler \
             shift), interval (spacetime interval and proper time), kinematics (full \
             kinematic summary for a particle). Uses CODATA 2018 constants."
                .into(),
        ),
    );

    let mut props = Map::new();

    let mut action = Map::new();
    action.insert("type".into(), Value::String("string".into()));
    action.insert(
        "description".into(),
        Value::String(
            "Action: gamma (Lorentz factor), lorentz (time dilation/length contraction), \
         energy (E=mc² and relativistic energy), momentum (relativistic p and E-p relation), \
         transform (Lorentz transformation of coordinates), doppler (relativistic Doppler), \
         interval (spacetime interval), kinematics (full particle summary)"
                .into(),
        ),
    );
    props.insert("action".into(), Value::Object(action));

    let mut v = Map::new();
    v.insert("type".into(), Value::String("number".into()));
    v.insert(
        "description".into(),
        Value::String("Velocity in m/s".into()),
    );
    props.insert("v".into(), Value::Object(v));

    let mut beta = Map::new();
    beta.insert("type".into(), Value::String("number".into()));
    beta.insert(
        "description".into(),
        Value::String("Velocity as fraction of c (v/c), 0 < beta < 1".into()),
    );
    props.insert("beta".into(), Value::Object(beta));

    let mut m = Map::new();
    m.insert("type".into(), Value::String("number".into()));
    m.insert(
        "description".into(),
        Value::String("Rest mass in kg".into()),
    );
    props.insert("m".into(), Value::Object(m));

    let mut m_mev = Map::new();
    m_mev.insert("type".into(), Value::String("number".into()));
    m_mev.insert(
        "description".into(),
        Value::String("Rest mass energy in MeV (mc²)".into()),
    );
    props.insert("m_mev".into(), Value::Object(m_mev));

    let mut particle = Map::new();
    particle.insert("type".into(), Value::String("string".into()));
    particle.insert(
        "description".into(),
        Value::String("Named particle: electron, proton, neutron, muon. Or '4u' for 4 AMU.".into()),
    );
    props.insert("particle".into(), Value::Object(particle));

    let mut t = Map::new();
    t.insert("type".into(), Value::String("number".into()));
    t.insert(
        "description".into(),
        Value::String("Proper time interval in seconds (for lorentz)".into()),
    );
    props.insert("t".into(), Value::Object(t));

    let mut l = Map::new();
    l.insert("type".into(), Value::String("number".into()));
    l.insert(
        "description".into(),
        Value::String("Proper length in meters (for lorentz)".into()),
    );
    props.insert("l".into(), Value::Object(l));

    let mut u = Map::new();
    u.insert("type".into(), Value::String("number".into()));
    u.insert(
        "description".into(),
        Value::String(
            "Second velocity in m/s for relativistic velocity addition (for lorentz)".into(),
        ),
    );
    props.insert("u".into(), Value::Object(u));

    let mut x = Map::new();
    x.insert("type".into(), Value::String("number".into()));
    x.insert(
        "description".into(),
        Value::String("Spatial coordinate in meters (for transform/interval)".into()),
    );
    props.insert("x".into(), Value::Object(x));

    let mut t_coord = Map::new();
    t_coord.insert("type".into(), Value::String("number".into()));
    t_coord.insert(
        "description".into(),
        Value::String("Time coordinate in seconds (for transform/interval)".into()),
    );
    props.insert("t_coord".into(), Value::Object(t_coord));

    let mut freq = Map::new();
    freq.insert("type".into(), Value::String("number".into()));
    freq.insert(
        "description".into(),
        Value::String("Source frequency in Hz (for doppler)".into()),
    );
    props.insert("freq".into(), Value::Object(freq));

    let mut direction = Map::new();
    direction.insert("type".into(), Value::String("string".into()));
    direction.insert(
        "description".into(),
        Value::String("'approaching' or 'receding' for doppler (default: approaching)".into()),
    );
    props.insert("direction".into(), Value::Object(direction));

    let mut params = Map::new();
    params.insert("type".into(), Value::String("object".into()));
    params.insert("properties".into(), Value::Object(props));
    root.insert("parameters".into(), Value::Object(params));
    Value::Object(root)
}

fn get_beta(args: &Value) -> Result<f64, String> {
    if let Some(b) = get_f64(args, "beta") {
        if b <= 0.0 || b >= 1.0 {
            return Err(format!("beta must be in (0, 1), got {b}"));
        }
        return Ok(b);
    }
    if let Some(v) = get_f64(args, "v") {
        if v <= 0.0 || v >= C {
            return Err(format!(
                "v must be in (0, c), got {v:.3e} m/s (c = {C:.4e} m/s)"
            ));
        }
        return Ok(v / C);
    }
    Err("Provide velocity via 'v' (m/s) or 'beta' (v/c, dimensionless 0–1)".into())
}

fn get_mass(args: &Value) -> Result<f64, String> {
    if let Some(m) = get_f64(args, "m") {
        return Ok(m);
    }
    if let Some(mev) = get_f64(args, "m_mev") {
        return Ok(mev * 1e6 * EV_TO_J / (C * C));
    }
    if let Some(p) = get_str(args, "particle") {
        return match p.to_lowercase().as_str() {
            "electron" | "e" | "e-" => Ok(M_E),
            "proton" | "p" => Ok(M_P),
            "neutron" | "n" => Ok(M_N),
            "muon" | "mu" => Ok(M_MUON),
            other => {
                if let Some(s) = other.strip_suffix('u') {
                    let amu: f64 = s
                        .parse()
                        .map_err(|_| format!("Cannot parse AMU value from '{other}'"))?;
                    return Ok(amu * M_U_KG);
                }
                Err(format!(
                    "Unknown particle '{other}'. Use: electron, proton, neutron, muon, or e.g. '4u'"
                ))
            }
        };
    }
    Err("Provide rest mass via 'm' (kg), 'm_mev' (MeV), or 'particle' (electron/proton/neutron/muon/'Nu')".into())
}

fn gamma(beta: f64) -> f64 {
    1.0 / (1.0 - beta * beta).sqrt()
}

fn action_gamma(args: &Value) -> Result<String, String> {
    let beta = get_beta(args)?;
    let g = gamma(beta);
    let v = beta * C;

    let mut out = String::new();
    out.push_str("SPECIAL RELATIVITY — LORENTZ FACTOR\n");
    out.push_str(&format!("  v = {v:.6e} m/s\n"));
    out.push_str(&format!("  β = v/c = {beta:.10}\n"));
    out.push_str(&format!("  γ = 1/√(1−β²) = {g:.8}\n"));
    out.push_str(&format!("  1/γ = {:.8}\n", 1.0 / g));
    out.push_str(&format!("  βγ  = {:.6}\n", beta * g));
    out.push_str(&format!(
        "  γ−1 = {:.6e}  (proportional to kinetic energy)\n",
        g - 1.0
    ));

    if beta > 0.99 {
        let g_approx = 1.0 / (2.0 * (1.0 - beta)).sqrt();
        out.push_str(&format!(
            "\n  Ultra-relativistic approx: γ ≈ 1/√(2(1−β)) = {g_approx:.4}\n"
        ));
    } else if beta < 0.1 {
        let g_approx = 1.0 + 0.5 * beta * beta;
        out.push_str(&format!(
            "\n  Non-relativistic approx:  γ ≈ 1 + β²/2 = {g_approx:.8}\n"
        ));
    }
    out.push_str("\n  Moving clocks run slower by factor γ.\n");
    out.push_str("  Moving objects are shorter along motion by factor γ.\n");
    Ok(out)
}

fn action_lorentz(args: &Value) -> Result<String, String> {
    let beta = get_beta(args)?;
    let g = gamma(beta);
    let v = beta * C;

    let mut out = String::new();
    out.push_str("SPECIAL RELATIVITY — TIME DILATION & LENGTH CONTRACTION\n");
    out.push_str(&format!("  v = {v:.6e} m/s  β = {beta:.6}  γ = {g:.6}\n\n"));

    out.push_str("TIME DILATION\n");
    if let Some(t_proper) = get_f64(args, "t") {
        let t_dilated = g * t_proper;
        out.push_str(&format!("  Proper time τ  = {t_proper:.4e} s\n"));
        out.push_str(&format!(
            "  Δt = γτ        = {t_dilated:.4e} s  ({g:.4}× slower)\n\n"
        ));
    } else {
        out.push_str("  Δt_dilated = γ × τ_proper  [pass 't' for proper time in s]\n\n");
    }

    out.push_str("LENGTH CONTRACTION\n");
    if let Some(l_proper) = get_f64(args, "l") {
        let l_contracted = l_proper / g;
        out.push_str(&format!("  Proper length L₀ = {l_proper:.4e} m\n"));
        out.push_str(&format!(
            "  L = L₀/γ         = {l_contracted:.4e} m  ({:.2}% of rest length)\n\n",
            100.0 / g
        ));
    } else {
        out.push_str("  L = L₀/γ  [pass 'l' for proper length in m]\n\n");
    }

    if let Some(u) = get_f64(args, "u") {
        let w = (u + v) / (1.0 + u * v / (C * C));
        let beta_w = w / C;
        out.push_str("RELATIVISTIC VELOCITY ADDITION\n");
        out.push_str(&format!("  u = {u:.4e} m/s  (β_u = {:.6})\n", u / C));
        out.push_str("  w = (u + v)/(1 + uv/c²)\n");
        out.push_str(&format!("    = {w:.6e} m/s\n"));
        out.push_str(&format!("    β_w = {beta_w:.8}  (always < 1)\n"));
    }
    Ok(out)
}

fn action_energy(args: &Value) -> Result<String, String> {
    let beta = get_beta(args)?;
    let g = gamma(beta);
    let m = get_mass(args)?;

    let mc2_j = m * C * C;
    let mc2_mev = mc2_j / (1e6 * EV_TO_J);
    let e_total_j = g * mc2_j;
    let e_total_mev = g * mc2_mev;
    let ke_j = (g - 1.0) * mc2_j;
    let ke_mev = (g - 1.0) * mc2_mev;
    let ke_classical_j = 0.5 * m * (beta * C).powi(2);
    let ke_classical_mev = ke_classical_j / (1e6 * EV_TO_J);

    let mut out = String::new();
    out.push_str("SPECIAL RELATIVITY — RELATIVISTIC ENERGY\n");
    out.push_str(&format!("  β = {beta:.6}  γ = {g:.6}  m = {m:.4e} kg\n\n"));
    out.push_str(&format!(
        "  Rest energy:     E₀ = mc²      = {mc2_j:.4e} J = {mc2_mev:.4} MeV\n"
    ));
    out.push_str(&format!(
        "  Total energy:    E  = γmc²     = {e_total_j:.4e} J = {e_total_mev:.4} MeV\n"
    ));
    out.push_str(&format!(
        "  Kinetic energy:  KE = (γ−1)mc² = {ke_j:.4e} J = {ke_mev:.4} MeV\n"
    ));
    out.push_str(&format!(
        "  Classical KE:    ½mv²          = {ke_classical_j:.4e} J = {ke_classical_mev:.4} MeV\n"
    ));
    out.push_str(&format!(
        "  Relativistic/classical ratio:    {:.4}×\n",
        ke_j / ke_classical_j
    ));
    Ok(out)
}

fn action_momentum(args: &Value) -> Result<String, String> {
    let beta = get_beta(args)?;
    let g = gamma(beta);
    let m = get_mass(args)?;

    let v = beta * C;
    let p = g * m * v;
    let p_mev = p * C / (1e6 * EV_TO_J);
    let mc2_mev = m * C * C / (1e6 * EV_TO_J);
    let e_mev = g * mc2_mev;
    let p_classical = m * v;
    let p_classical_mev = p_classical * C / (1e6 * EV_TO_J);
    let verify = ((p_mev * p_mev) + (mc2_mev * mc2_mev)).sqrt();

    let mut out = String::new();
    out.push_str("SPECIAL RELATIVITY — RELATIVISTIC MOMENTUM\n");
    out.push_str(&format!("  β = {beta:.6}  γ = {g:.6}  m = {m:.4e} kg\n\n"));
    out.push_str(&format!("  p = γmv = {p:.4e} kg·m/s = {p_mev:.4} MeV/c\n"));
    out.push_str(&format!(
        "  Classical p = mv = {p_classical:.4e} kg·m/s = {p_classical_mev:.4} MeV/c\n"
    ));
    out.push_str(&format!(
        "  Relativistic/classical: {:.4}×\n\n",
        p / p_classical
    ));
    out.push_str("  ENERGY-MOMENTUM RELATION:  E² = (pc)² + (mc²)²\n");
    out.push_str(&format!("  E   = {e_mev:.4} MeV\n"));
    out.push_str(&format!("  pc  = {p_mev:.4} MeV\n"));
    out.push_str(&format!("  mc² = {mc2_mev:.4} MeV\n"));
    out.push_str(&format!("  √((pc)²+(mc²)²) = {verify:.4} MeV  ✓\n"));
    Ok(out)
}

fn action_transform(args: &Value) -> Result<String, String> {
    let beta = get_beta(args)?;
    let g = gamma(beta);
    let v = beta * C;
    let x = get_f64(args, "x").ok_or("Provide spatial coordinate 'x' in meters")?;
    let t = get_f64(args, "t_coord").ok_or("Provide time coordinate 't_coord' in seconds")?;

    let x_prime = g * (x - v * t);
    let t_prime = g * (t - v * x / (C * C));
    let x_back = g * (x_prime + v * t_prime);
    let t_back = g * (t_prime + v * x_prime / (C * C));
    let interval_sq = (C * t).powi(2) - x * x;
    let interval_sq_prime = (C * t_prime).powi(2) - x_prime * x_prime;

    let mut out = String::new();
    out.push_str("SPECIAL RELATIVITY — LORENTZ TRANSFORMATION\n");
    out.push_str(&format!(
        "  Frame S' moves at v = {v:.4e} m/s  β = {beta:.6}  γ = {g:.6}\n\n"
    ));
    out.push_str("  S frame (given)          →   S' frame:\n");
    out.push_str(&format!(
        "  x  = {x:+.6e} m    x' = γ(x−vt)    = {x_prime:+.6e} m\n"
    ));
    out.push_str(&format!(
        "  t  = {t:+.6e} s    t' = γ(t−vx/c²)  = {t_prime:+.6e} s\n\n"
    ));
    out.push_str("  INVERSE CHECK (S'→S should recover originals):\n");
    out.push_str(&format!("  x_back = {x_back:+.6e} m\n"));
    out.push_str(&format!("  t_back = {t_back:+.6e} s\n\n"));
    out.push_str("  SPACETIME INTERVAL (Lorentz invariant):\n");
    out.push_str(&format!("  s² = c²t²−x²  = {interval_sq:+.4e} m²\n"));
    out.push_str(&format!(
        "  s'²= c²t'²−x'² = {interval_sq_prime:+.4e} m²  (should match)\n"
    ));
    Ok(out)
}

fn action_doppler(args: &Value) -> Result<String, String> {
    let beta = get_beta(args)?;
    let approaching = get_str(args, "direction")
        .map(|d| !d.starts_with('r'))
        .unwrap_or(true);

    let ratio_approach = ((1.0 + beta) / (1.0 - beta)).sqrt();
    let ratio_recede = ((1.0 - beta) / (1.0 + beta)).sqrt();
    let ratio = if approaching {
        ratio_approach
    } else {
        ratio_recede
    };
    let redshift_z = 1.0 / ratio_recede - 1.0;

    let mut out = String::new();
    out.push_str("SPECIAL RELATIVITY — RELATIVISTIC DOPPLER EFFECT\n");
    out.push_str(&format!(
        "  β = {beta:.6}  ({} source)\n\n",
        if approaching {
            "approaching"
        } else {
            "receding"
        }
    ));
    out.push_str(&format!(
        "  f_obs/f_src = {ratio:.6}  ({})\n",
        if ratio > 1.0 { "blueshift" } else { "redshift" }
    ));

    if let Some(f_src) = get_f64(args, "freq") {
        let f_obs = ratio * f_src;
        out.push_str(&format!("  f_source   = {f_src:.4e} Hz\n"));
        out.push_str(&format!("  f_observed = {f_obs:.4e} Hz\n"));
        out.push_str(&format!("  Δf/f = {:+.4}%\n", (ratio - 1.0) * 100.0));
        if f_src > 4e14 && f_src < 7.5e14 {
            out.push_str(&format!(
                "  λ_source = {:.1} nm → λ_obs = {:.1} nm\n",
                C / f_src * 1e9,
                C / f_obs * 1e9
            ));
        }
    } else {
        out.push_str("  [pass 'freq' in Hz for absolute values]\n");
    }

    out.push_str(&format!(
        "\n  Approaching: f_obs/f_src = {ratio_approach:.6}\n"
    ));
    out.push_str(&format!("  Receding:    f_obs/f_src = {ratio_recede:.6}\n"));
    out.push_str(&format!("  Cosmological redshift z  = {redshift_z:.6}\n"));
    Ok(out)
}

fn action_interval(args: &Value) -> Result<String, String> {
    let x = get_f64(args, "x").ok_or("Provide spatial separation 'x' in meters")?;
    let t = get_f64(args, "t_coord").ok_or("Provide time separation 't_coord' in seconds")?;

    let ct = C * t;
    let s_sq = ct * ct - x * x;
    let kind = if s_sq > 1e-30 {
        "TIMELIKE — causally connected (|cΔt| > |Δx|)"
    } else if s_sq < -1e-30 {
        "SPACELIKE — cannot be causally connected (|Δx| > |cΔt|)"
    } else {
        "LIGHTLIKE / NULL — connected by light signal"
    };

    let mut out = String::new();
    out.push_str("SPECIAL RELATIVITY — SPACETIME INTERVAL\n");
    out.push_str(&format!("  Δx = {x:.4e} m\n"));
    out.push_str(&format!("  Δt = {t:.4e} s  (cΔt = {ct:.4e} m)\n\n"));
    out.push_str(&format!("  s² = (cΔt)²−(Δx)² = {s_sq:+.4e} m²\n"));
    out.push_str(&format!("  |s| = {:.4e} m\n", s_sq.abs().sqrt()));
    out.push_str(&format!("  Type: {kind}\n\n"));

    if s_sq > 1e-30 {
        let tau = s_sq.sqrt() / C;
        out.push_str(&format!("  Proper time τ = |s|/c = {tau:.4e} s\n"));
        out.push_str("  (time in rest frame of traveler between events)\n");
    } else if s_sq < -1e-30 {
        let proper_dist = (-s_sq).sqrt();
        out.push_str(&format!("  Proper distance = |s| = {proper_dist:.4e} m\n"));
        out.push_str("  (spatial separation in the simultaneous frame)\n");
    }

    out.push_str(&format!(
        "\n  Light travel time for Δx = {:.4e} s\n",
        x.abs() / C
    ));
    Ok(out)
}

fn action_kinematics(args: &Value) -> Result<String, String> {
    let beta = get_beta(args)?;
    let g = gamma(beta);
    let v = beta * C;

    let (label, m) = if let Ok(mass) = get_mass(args) {
        let name = get_str(args, "particle")
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("{:.4e} kg", mass));
        (name, mass)
    } else {
        ("electron".into(), M_E)
    };

    let mc2_j = m * C * C;
    let mc2_mev = mc2_j / (1e6 * EV_TO_J);
    let e_mev = g * mc2_mev;
    let ke_mev = (g - 1.0) * mc2_mev;
    let p = g * m * v;
    let p_mev = p * C / (1e6 * EV_TO_J);
    let ke_classical_mev = 0.5 * m * v * v / (1e6 * EV_TO_J);

    let mut out = String::new();
    out.push_str(&format!("SPECIAL RELATIVITY — KINEMATICS ({label})\n"));
    out.push_str(&format!("  v = {v:.6e} m/s\n"));
    out.push_str(&format!("  β = {beta:.8}\n"));
    out.push_str(&format!("  γ = {g:.8}\n"));
    out.push_str(&format!("  m = {m:.4e} kg  (mc² = {mc2_mev:.4} MeV)\n\n"));
    out.push_str("  ENERGY\n");
    out.push_str(&format!("  Total:     E   = γmc²     = {e_mev:.4} MeV\n"));
    out.push_str(&format!("  Kinetic:   KE  = (γ−1)mc² = {ke_mev:.4} MeV\n"));
    out.push_str(&format!(
        "  Classical: ½mv²           = {ke_classical_mev:.4} MeV\n"
    ));
    out.push_str(&format!(
        "  Relativistic correction:    {:.4}×\n\n",
        ke_mev / ke_classical_mev
    ));
    out.push_str("  MOMENTUM\n");
    out.push_str(&format!("  p = γmv = {p:.4e} kg·m/s = {p_mev:.4} MeV/c\n"));
    Ok(out)
}

pub async fn execute(args: &Value) -> Result<String, String> {
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("gamma");
    match action {
        "gamma" => action_gamma(args),
        "lorentz" => action_lorentz(args),
        "energy" => action_energy(args),
        "momentum" => action_momentum(args),
        "transform" => action_transform(args),
        "doppler" => action_doppler(args),
        "interval" => action_interval(args),
        "kinematics" => action_kinematics(args),
        _ => Err(format!(
            "Unknown action '{action}'. Valid: gamma, lorentz, energy, momentum, transform, doppler, interval, kinematics"
        )),
    }
}
