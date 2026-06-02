use serde_json::{json, Value};
use std::f64::consts::PI;

const K_E: f64 = 8.9875517923e9; // Coulomb constant (N·m²/C²)
const EPS_0: f64 = 8.8541878128e-12; // Permittivity of free space (F/m)
const MU_0: f64 = 1.25663706212e-6; // Permeability of free space (H/m)
const C: f64 = 2.99792458e8; // Speed of light (m/s)
const H: f64 = 6.62607015e-34; // Planck constant (J·s)
const EV_TO_J: f64 = 1.602176634e-19;

fn n(v: &Value, key: &str) -> Option<f64> {
    v.get(key).and_then(|x| x.as_f64())
}

fn require(v: &Value, key: &str) -> Result<f64, String> {
    n(v, key).ok_or_else(|| format!("missing required argument '{key}'"))
}

pub fn em_tools_schema() -> Value {
    let num = |desc: &str| json!({"type": "number", "description": desc});
    let str_prop = |desc: &str| json!({"type": "string", "description": desc});
    let mut props = serde_json::Map::new();
    props.insert("action".into(), json!({"type":"string","description":"coulomb|electric_field|magnetic_field|capacitance|inductance|em_wave|lorentz|poynting","enum":["coulomb","electric_field","magnetic_field","capacitance","inductance","em_wave","lorentz","poynting"]}));
    props.insert(
        "solve_for".into(),
        str_prop("Variable to solve for (action-dependent)"),
    );
    props.insert("q1".into(), num("Charge 1 (C)"));
    props.insert("q2".into(), num("Charge 2 (C)"));
    props.insert("q".into(), num("Charge (C)"));
    props.insert("r".into(), num("Distance / radius (m)"));
    props.insert("I".into(), num("Current (A)"));
    props.insert("N".into(), num("Number of turns"));
    props.insert("l".into(), num("Length (m)"));
    props.insert("A".into(), num("Area (m²)"));
    props.insert("d".into(), num("Separation distance (m)"));
    props.insert("b".into(), num("Outer radius (m)"));
    props.insert(
        "epsilon_r".into(),
        num("Relative permittivity (dimensionless, default 1)"),
    );
    props.insert("f".into(), num("Frequency (Hz)"));
    props.insert(
        "lambda".into(),
        num("Wavelength (m or nm depending on action)"),
    );
    props.insert("E_field".into(), num("Electric field magnitude (V/m)"));
    props.insert("B_field".into(), num("Magnetic field magnitude (T)"));
    props.insert("v".into(), num("Velocity (m/s)"));
    props.insert(
        "theta".into(),
        num("Angle between v and B (degrees, default 90)"),
    );
    props.insert(
        "geometry".into(),
        str_prop(
            "Geometry: wire|loop|solenoid|toroid|coaxial|parallel_plate|cylindrical|spherical",
        ),
    );
    json!({"name":"em_tools","description":"Electromagnetism calculations without external utilities. Coulomb's law, electric fields/potential, magnetic fields, capacitance, inductance, EM waves, Lorentz force, and Poynting vector. All constants are CODATA 2018 values.","input_schema":{"type":"object","properties":Value::Object(props),"required":["action"]}})
}

fn action_coulomb(args: &Value) -> Result<String, String> {
    let q1 = require(args, "q1")?;
    let q2 = require(args, "q2")?;
    let r = require(args, "r")?;

    let f_mag = K_E * q1.abs() * q2.abs() / (r * r);
    let sign = if q1 * q2 > 0.0 {
        "repulsive"
    } else {
        "attractive"
    };
    let u = K_E * q1 * q2 / r;
    let e1 = K_E * q1 / (r * r);
    let e2 = K_E * q2 / (r * r);
    let v1 = K_E * q1 / r;
    let v2 = K_E * q2 / r;

    Ok(format!(
        "=== COULOMB'S LAW ===\n\
         q₁ = {q1:.4e} C  |  q₂ = {q2:.4e} C  |  r = {r} m\n\n\
         F = k_e·|q₁q₂|/r² = {K_E:.4e}×{:.4e}/{:.4e} = {f_mag:.4e} N  ({sign})\n\n\
         Electric field from q₁ at r: E₁ = k_e·q₁/r² = {e1:.4e} V/m\n\
         Electric field from q₂ at r: E₂ = k_e·q₂/r² = {e2:.4e} V/m\n\n\
         Electric potential from q₁ at r: V₁ = k_e·q₁/r = {v1:.4e} V\n\
         Electric potential from q₂ at r: V₂ = k_e·q₂/r = {v2:.4e} V\n\n\
         Potential energy: U = k_e·q₁q₂/r = {u:.4e} J = {:.4e} eV\n",
        q1.abs() * q2.abs(),
        r * r,
        u / EV_TO_J
    ))
}

fn action_electric_field(args: &Value) -> Result<String, String> {
    let q = require(args, "q")?;
    let r = require(args, "r")?;

    let e = K_E * q / (r * r);
    let e_sign = if q > 0.0 {
        "outward (away from charge)"
    } else {
        "inward (toward charge)"
    };
    let v = K_E * q / r;
    let u_density = 0.5 * EPS_0 * e * e;

    let mut out = format!(
        "=== ELECTRIC FIELD & POTENTIAL ===\nPoint charge q = {q:.4e} C at distance r = {r} m\n\n\
         E = k_e·q/r² = {K_E:.4e}×{q:.4e}/{:.4e} = {e:.4e} V/m  ({e_sign})\n\
         V = k_e·q/r = {v:.4e} V\n\
         Energy density: u = ε₀E²/2 = {u_density:.4e} J/m³\n\n\
         Gauss's Law: ∮ E·dA = Q_enc/ε₀\n",
        r * r
    );

    out.push_str(&format!(
        "Flux through sphere of radius r: Φ = q/ε₀ = {:.4e} N·m²/C\n",
        q / EPS_0
    ));

    Ok(out)
}

fn action_magnetic_field(args: &Value) -> Result<String, String> {
    let geometry = args
        .get("geometry")
        .and_then(|x| x.as_str())
        .unwrap_or("wire");
    let current = require(args, "I")?;
    let mut out = String::from("=== MAGNETIC FIELD ===\n");

    match geometry {
        "wire" => {
            let r = require(args, "r")?;
            let b = MU_0 * current / (2.0 * PI * r);
            out.push_str(&format!(
                "Infinite straight wire: I = {current} A, r = {r} m\n\
                 B = μ₀I/(2πr) = {MU_0:.4e}×{current}/(2π×{r}) = {b:.4e} T\n\
                 Direction: use right-hand rule — curl fingers along B, thumb in direction of I\n"
            ));
        }
        "loop" => {
            let r = require(args, "r")?;
            let n_turns = n(args, "N").unwrap_or(1.0);
            let b = n_turns * MU_0 * current / (2.0 * r);
            out.push_str(&format!(
                "Circular loop at center: I = {current} A, R = {r} m, N = {n_turns} turns\n\
                 B = N·μ₀I/(2R) = {n_turns}×{MU_0:.4e}×{current}/(2×{r}) = {b:.4e} T\n"
            ));
        }
        "solenoid" => {
            let l = require(args, "l")?;
            let n_turns = require(args, "N")?;
            let n_per_m = n_turns / l;
            let b = MU_0 * n_per_m * current;
            out.push_str(&format!(
                "Solenoid: I = {current} A, N = {n_turns} turns, L = {l} m\n\
                 n = N/L = {n_per_m:.2} turns/m\n\
                 B = μ₀nI = {MU_0:.4e}×{n_per_m:.2}×{current} = {b:.4e} T (inside, uniform)\n\
                 B ≈ 0 outside ideal solenoid\n"
            ));
        }
        "toroid" => {
            let r = require(args, "r")?;
            let n_turns = require(args, "N")?;
            let b = MU_0 * n_turns * current / (2.0 * PI * r);
            out.push_str(&format!(
                "Toroid: I = {current} A, N = {n_turns} turns, r = {r} m (mean radius)\n\
                 B = μ₀NI/(2πr) = {b:.4e} T (inside toroid)\n\
                 B = 0 outside toroid\n"
            ));
        }
        _ => {
            return Err(format!(
                "unknown geometry '{geometry}'; use: wire|loop|solenoid|toroid"
            ))
        }
    }

    Ok(out)
}

fn action_capacitance(args: &Value) -> Result<String, String> {
    let geometry = args
        .get("geometry")
        .and_then(|x| x.as_str())
        .unwrap_or("parallel_plate");
    let eps_r = n(args, "epsilon_r").unwrap_or(1.0);
    let eps = EPS_0 * eps_r;
    let mut out = format!("=== CAPACITANCE ===\nε_r = {eps_r}  |  ε = ε₀·ε_r = {eps:.4e} F/m\n\n");

    let c = match geometry {
        "parallel_plate" => {
            let area = require(args, "A")?;
            let d = require(args, "d")?;
            let c = eps * area / d;
            out.push_str(&format!(
                "Parallel plate: A = {area} m², d = {d} m\nC = ε·A/d = {c:.4e} F\n"
            ));
            c
        }
        "cylindrical" => {
            let r = require(args, "r")?;
            let b = require(args, "b")?;
            let l = require(args, "l")?;
            let c = 2.0 * PI * eps * l / (b / r).ln();
            out.push_str(&format!(
                "Cylindrical: inner r = {r} m, outer b = {b} m, L = {l} m\n\
                 C = 2πεL/ln(b/r) = {c:.4e} F\n"
            ));
            c
        }
        "spherical" => {
            let r = require(args, "r")?;
            let b = require(args, "b")?;
            let c = 4.0 * PI * eps * r * b / (b - r);
            out.push_str(&format!(
                "Spherical: inner r = {r} m, outer b = {b} m\nC = 4πε·rb/(b-r) = {c:.4e} F\n"
            ));
            c
        }
        _ => {
            return Err(format!(
                "unknown geometry '{geometry}'; use: parallel_plate|cylindrical|spherical"
            ))
        }
    };

    if let Some(v_val) = n(args, "v") {
        let q_stored = c * v_val;
        let energy = 0.5 * c * v_val * v_val;
        out.push_str(&format!(
            "\nWith V = {v_val} V:\nQ = CV = {q_stored:.4e} C\nEnergy U = ½CV² = {energy:.4e} J\n"
        ));
    }

    Ok(out)
}

fn action_inductance(args: &Value) -> Result<String, String> {
    let geometry = args
        .get("geometry")
        .and_then(|x| x.as_str())
        .unwrap_or("solenoid");
    let mut out = String::from("=== INDUCTANCE ===\n\n");

    let l_val = match geometry {
        "solenoid" => {
            let n_turns = require(args, "N")?;
            let length = require(args, "l")?;
            let area = require(args, "A")?;
            let l = MU_0 * n_turns * n_turns * area / length;
            out.push_str(&format!(
                "Solenoid: N = {n_turns}, L = {length} m, A = {area} m²\n\
                 L = μ₀N²A/l = {MU_0:.4e}×{:.0}×{area}/{length} = {l:.4e} H\n",
                n_turns * n_turns
            ));
            l
        }
        "toroid" => {
            let n_turns = require(args, "N")?;
            let r = require(args, "r")?;
            let area = require(args, "A")?;
            let l = MU_0 * n_turns * n_turns * area / (2.0 * PI * r);
            out.push_str(&format!(
                "Toroid: N = {n_turns}, r = {r} m (mean), A = {area} m²\n\
                 L = μ₀N²A/(2πr) = {l:.4e} H\n"
            ));
            l
        }
        "coaxial" => {
            let r = require(args, "r")?;
            let b = require(args, "b")?;
            let length = require(args, "l")?;
            let l = MU_0 * length / (2.0 * PI) * (b / r).ln();
            out.push_str(&format!(
                "Coaxial cable: inner r = {r} m, outer b = {b} m, L = {length} m\n\
                 L = (μ₀·l)/(2π) × ln(b/r) = {l:.4e} H\n"
            ));
            l
        }
        _ => {
            return Err(format!(
                "unknown geometry '{geometry}'; use: solenoid|toroid|coaxial"
            ))
        }
    };

    if let Some(i_val) = n(args, "I") {
        let energy = 0.5 * l_val * i_val * i_val;
        let flux = l_val * i_val;
        out.push_str(&format!(
            "\nWith I = {i_val} A:\nMagnetic flux linkage: NΦ = LI = {flux:.4e} Wb\nEnergy U = ½LI² = {energy:.4e} J\n"
        ));
    }

    Ok(out)
}

fn action_em_wave(args: &Value) -> Result<String, String> {
    let (freq, lam_m, source) = if let Some(f_hz) = n(args, "f") {
        let lam = C / f_hz;
        (f_hz, lam, format!("f = {f_hz:.4e} Hz"))
    } else if let Some(lam_arg) = n(args, "lambda") {
        let lam = if lam_arg > 1e-3 {
            lam_arg * 1e-9
        } else {
            lam_arg
        };
        let f = C / lam;
        (f, lam, format!("λ = {lam_arg}"))
    } else {
        return Err("provide f (frequency Hz) or lambda (wavelength m or nm)".into());
    };

    let lam_nm = lam_m * 1e9;
    let e_photon = H * freq;
    let e_ev = e_photon / EV_TO_J;
    let e0 = n(args, "E_field");
    let b0 = n(args, "B_field");

    let region = if lam_nm < 0.001 {
        "Gamma ray"
    } else if lam_nm < 10.0 {
        "X-ray"
    } else if lam_nm < 400.0 {
        "Ultraviolet"
    } else if lam_nm < 700.0 {
        "Visible light"
    } else if lam_nm < 1_000_000.0 {
        "Infrared"
    } else if lam_nm < 1e12 {
        "Microwave"
    } else {
        "Radio wave"
    };

    let mut out = format!(
        "=== ELECTROMAGNETIC WAVE ===\nSource: {source}\n\n\
         Frequency: f = {freq:.4e} Hz\n\
         Wavelength: λ = {lam_m:.4e} m = {lam_nm:.4e} nm\n\
         Period: T = 1/f = {:.4e} s\n\
         Wave speed: c = λf = {C:.4e} m/s\n\
         EM spectrum region: {region}\n\n\
         Photon energy: E = hf = {e_photon:.4e} J = {e_ev:.4e} eV\n",
        1.0 / freq
    );

    if let Some(e_amp) = e0 {
        let b_amp = e_amp / C;
        let intensity = e_amp * b_amp / (2.0 * MU_0);
        out.push_str(&format!(
            "\nElectric amplitude: E₀ = {e_amp:.4e} V/m\n\
             Magnetic amplitude: B₀ = E₀/c = {b_amp:.4e} T\n\
             Intensity: I = E₀B₀/(2μ₀) = {intensity:.4e} W/m²\n"
        ));
    } else if let Some(b_amp) = b0 {
        let e_amp = b_amp * C;
        let intensity = e_amp * b_amp / (2.0 * MU_0);
        out.push_str(&format!(
            "\nMagnetic amplitude: B₀ = {b_amp:.4e} T\n\
             Electric amplitude: E₀ = B₀·c = {e_amp:.4e} V/m\n\
             Intensity: I = E₀B₀/(2μ₀) = {intensity:.4e} W/m²\n"
        ));
    } else {
        out.push_str("Add E_field or B_field to compute intensity.\n");
    }

    Ok(out)
}

fn action_lorentz(args: &Value) -> Result<String, String> {
    let q = require(args, "q")?;
    let mut out = format!("=== LORENTZ FORCE ===\nF = q(E + v×B)\nq = {q:.4e} C\n\n");

    let e_field = n(args, "E_field");
    let b_field = n(args, "B_field");
    let vel = n(args, "v");
    let theta_deg = n(args, "theta").unwrap_or(90.0);
    let theta_rad = theta_deg * PI / 180.0;

    match (e_field, b_field, vel) {
        (Some(e), Some(b), Some(v)) => {
            let f_e = q * e;
            let f_b = q * v * b * theta_rad.sin();
            let f_total = f_e + f_b;
            out.push_str(&format!(
                "Electric force: F_E = qE = {q:.4e}×{e:.4e} = {f_e:.4e} N\n\
                 Magnetic force: F_B = qvB·sin(θ) = {q:.4e}×{v:.4e}×{b:.4e}×sin({theta_deg}°) = {f_b:.4e} N\n\
                 Total force: F = F_E + F_B = {f_total:.4e} N\n"
            ));
        }
        (Some(e), None, _) => {
            let f_e = q * e;
            out.push_str(&format!(
                "Electric force: F_E = qE = {q:.4e}×{e:.4e} = {f_e:.4e} N\n\
                 Acceleration (electron): a = F/m_e = {:.4e} m/s²\n",
                f_e.abs() / 9.1093837015e-31
            ));
        }
        (None, Some(b), Some(v)) => {
            let f_b = q * v * b * theta_rad.sin();
            let r_circular = (9.1093837015e-31 * v) / (q.abs() * b);
            out.push_str(&format!(
                "Magnetic force: F_B = qvB·sin(θ) = {q:.4e}×{v:.4e}×{b:.4e}×sin({theta_deg}°) = {f_b:.4e} N\n\
                 (Magnetic force does no work — always perpendicular to velocity)\n\
                 Circular orbit radius (electron): r = m_e·v/(|q|B) = {r_circular:.4e} m\n"
            ));
        }
        _ => {
            out.push_str(
                "Provide E_field, B_field and/or v to compute force.\n\
                 F_E = qE  (electric)\nF_B = qv×B  (magnetic, magnitude = qvB·sin θ)\n",
            );
        }
    }

    Ok(out)
}

fn action_poynting(args: &Value) -> Result<String, String> {
    let e_field = require(args, "E_field")?;
    let b_field = require(args, "B_field")?;

    let s_mag = e_field * b_field / MU_0;
    let u_e = 0.5 * EPS_0 * e_field * e_field;
    let u_b = b_field * b_field / (2.0 * MU_0);
    let u_total = u_e + u_b;
    let intensity = s_mag / 2.0; // time-averaged for sinusoidal wave
    let pressure = s_mag / C;

    Ok(format!(
        "=== POYNTING VECTOR & ENERGY DENSITY ===\nE = {e_field:.4e} V/m  |  B = {b_field:.4e} T\n\n\
         Poynting vector magnitude: S = E×B/μ₀ = {e_field:.4e}×{b_field:.4e}/{MU_0:.4e} = {s_mag:.4e} W/m²\n\
         Time-averaged intensity: <S> = S/2 = {intensity:.4e} W/m²  (for sinusoidal wave)\n\n\
         Electric energy density: u_E = ε₀E²/2 = {u_e:.4e} J/m³\n\
         Magnetic energy density: u_B = B²/(2μ₀) = {u_b:.4e} J/m³\n\
         Total EM energy density: u = u_E + u_B = {u_total:.4e} J/m³\n\
         (For plane wave: u_E = u_B = u/2)\n\n\
         Radiation pressure: P = S/c = {pressure:.4e} Pa\n"
    ))
}

pub async fn execute(args: &Value) -> Result<String, String> {
    let action = args
        .get("action")
        .and_then(|x| x.as_str())
        .unwrap_or("coulomb");
    match action {
        "coulomb" => action_coulomb(args),
        "electric_field" => action_electric_field(args),
        "magnetic_field" => action_magnetic_field(args),
        "capacitance" => action_capacitance(args),
        "inductance" => action_inductance(args),
        "em_wave" => action_em_wave(args),
        "lorentz" => action_lorentz(args),
        "poynting" => action_poynting(args),
        _ => Err(format!("unknown action '{action}'; use: coulomb|electric_field|magnetic_field|capacitance|inductance|em_wave|lorentz|poynting")),
    }
}
