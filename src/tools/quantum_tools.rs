use serde_json::{json, Value};
use std::f64::consts::PI;

const H: f64 = 6.62607015e-34;
const HBAR: f64 = 1.054571817e-34;
const M_E: f64 = 9.1093837015e-31;
const E_CHARGE: f64 = 1.602176634e-19;
const C: f64 = 2.99792458e8;
const LAMBDA_C: f64 = 2.42631023867e-12; // Compton wavelength
const R_INF: f64 = 1.0973731568539e7; // Rydberg constant (m⁻¹)
const EV_TO_J: f64 = 1.602176634e-19;

fn n(v: &Value, key: &str) -> Option<f64> {
    v.get(key).and_then(|x| x.as_f64())
}

fn require(v: &Value, key: &str) -> Result<f64, String> {
    n(v, key).ok_or_else(|| format!("missing required argument '{key}'"))
}

pub fn quantum_tools_schema() -> Value {
    let num = |desc: &str| json!({"type": "number", "description": desc});
    let str_prop = |desc: &str| json!({"type": "string", "description": desc});
    let mut props = serde_json::Map::new();
    props.insert("action".into(), json!({"type":"string","description":"particle_box|hydrogen|uncertainty|de_broglie|photoelectric|compton|tunneling|harmonic","enum":["particle_box","hydrogen","uncertainty","de_broglie","photoelectric","compton","tunneling","harmonic"]}));
    props.insert(
        "solve_for".into(),
        str_prop("Variable to solve for (depends on action)"),
    );
    props.insert(
        "n".into(),
        num("Quantum number (principal, energy level, etc.)"),
    );
    props.insert("n1".into(), num("Lower quantum number for transition"));
    props.insert("n2".into(), num("Upper quantum number for transition"));
    props.insert(
        "L".into(),
        num("Length of box / barrier width (m or nm depending on action)"),
    );
    props.insert(
        "m".into(),
        num("Particle mass (kg, default = electron mass)"),
    );
    props.insert("V0".into(), num("Barrier height (eV)"));
    props.insert("E".into(), num("Particle energy (eV)"));
    props.insert("omega".into(), num("Angular frequency (rad/s)"));
    props.insert("k".into(), num("Spring constant (N/m)"));
    props.insert("v".into(), num("Velocity (m/s)"));
    props.insert("p".into(), num("Momentum (kg·m/s)"));
    props.insert("f".into(), num("Frequency (Hz)"));
    props.insert(
        "lambda".into(),
        num("Wavelength (nm for photoelectric; m for compton)"),
    );
    props.insert("phi".into(), num("Work function (eV)"));
    props.insert("theta".into(), num("Scattering angle (degrees)"));
    props.insert("delta_x".into(), num("Position uncertainty (m)"));
    props.insert("delta_p".into(), num("Momentum uncertainty (kg·m/s)"));
    props.insert("delta_E".into(), num("Energy uncertainty (J)"));
    props.insert("delta_t".into(), num("Time uncertainty (s)"));
    json!({"name":"quantum_tools","description":"Quantum mechanics calculations without external utilities. Particle in a box, hydrogen atom energy levels, Heisenberg uncertainty, de Broglie wavelength, photoelectric effect, Compton scattering, quantum tunneling, and quantum harmonic oscillator. All constants are CODATA 2018 values.","input_schema":{"type":"object","properties":Value::Object(props),"required":["action"]}})
}

fn action_particle_box(args: &Value) -> Result<String, String> {
    let qn = n(args, "n").unwrap_or(1.0) as u32;
    let mass = n(args, "m").unwrap_or(M_E);
    let l = require(args, "L")?;

    let mut out = String::from("=== PARTICLE IN A 1D INFINITE SQUARE WELL ===\n");
    out.push_str(&format!(
        "n = {qn}  |  L = {l:.6e} m  |  m = {mass:.6e} kg\n\n"
    ));

    for i in 1..=qn.max(3) {
        let en_j = (i as f64 * PI * HBAR).powi(2) / (2.0 * mass * l * l);
        let en_ev = en_j / EV_TO_J;
        let lambda = 2.0 * l / i as f64;
        let p = i as f64 * PI * HBAR / l;
        out.push_str(&format!(
            "n={i}: E = {en_j:.4e} J = {en_ev:.4e} eV  |  λ = {lambda:.4e} m  |  p = {p:.4e} kg·m/s\n"
        ));
    }

    let e1 = (PI * HBAR).powi(2) / (2.0 * mass * l * l);
    out.push_str(&format!(
        "\nGround state (n=1): E₁ = {:.4e} J = {:.4e} eV\nEn = n²E₁  (energy levels scale as n²)\n",
        e1,
        e1 / EV_TO_J
    ));

    Ok(out)
}

fn action_hydrogen(args: &Value) -> Result<String, String> {
    let mut out = String::from("=== HYDROGEN ATOM ENERGY LEVELS ===\n");

    if let (Some(n1), Some(n2)) = (n(args, "n1"), n(args, "n2")) {
        if n2 <= n1 {
            return Err("n2 must be greater than n1 for emission transition".into());
        }
        let inv_lambda = R_INF * (1.0 / (n1 * n1) - 1.0 / (n2 * n2));
        let lambda_m = 1.0 / inv_lambda;
        let lambda_nm = lambda_m * 1e9;
        let e_j = H * C / lambda_m;
        let e_ev = e_j / EV_TO_J;
        let series = match n1 as u32 {
            1 => "Lyman (UV)",
            2 => "Balmer (visible/UV)",
            3 => "Paschen (near-IR)",
            4 => "Brackett (IR)",
            5 => "Pfund (IR)",
            _ => "Higher series",
        };
        out.push_str(&format!(
            "Transition: n={n1} → n={n2} emission  ({series})\n\n\
             1/λ = R_∞(1/n1² - 1/n2²) = {inv_lambda:.4e} m⁻¹\n\
             λ = {lambda_m:.4e} m = {lambda_nm:.2} nm\n\
             Photon energy: ΔE = {e_ev:.4} eV = {e_j:.4e} J\n"
        ));
    } else {
        let qn = n(args, "n").unwrap_or(1.0) as u32;
        for i in 1..=qn.max(5) {
            let e = -13.60569 / (i * i) as f64;
            let e_j = e * EV_TO_J;
            out.push_str(&format!("n={i}: E = {e:.4} eV = {e_j:.4e} J\n"));
        }
        let e_ion = 13.60569;
        out.push_str(&format!(
            "\nGround state (n=1): E = -13.6057 eV\nIonization energy: {e_ion:.4} eV\n\
             En = -13.6057 / n² eV\n"
        ));
    }

    Ok(out)
}

fn action_uncertainty(args: &Value) -> Result<String, String> {
    let solve = args
        .get("solve_for")
        .and_then(|x| x.as_str())
        .unwrap_or("delta_p");
    let min_xp = HBAR / 2.0;
    let mut out = String::from("=== HEISENBERG UNCERTAINTY PRINCIPLE ===\n");
    out.push_str(&format!(
        "ΔxΔp ≥ ħ/2 = {:.4e} J·s\nΔEΔt ≥ ħ/2 = {:.4e} J·s\n\n",
        min_xp, min_xp
    ));

    match solve {
        "delta_p" => {
            let dx = require(args, "delta_x")?;
            let dp_min = min_xp / dx;
            out.push_str(&format!(
                "Δp_min = ħ/(2Δx) = {min_xp:.4e} / {dx:.4e} = {dp_min:.4e} kg·m/s\n\
                 Min KE = Δp²/(2m_e) = {:.4e} J = {:.4e} eV\n",
                dp_min * dp_min / (2.0 * M_E),
                dp_min * dp_min / (2.0 * M_E * EV_TO_J)
            ));
        }
        "delta_x" => {
            let dp = require(args, "delta_p")?;
            let dx_min = min_xp / dp;
            out.push_str(&format!(
                "Δx_min = ħ/(2Δp) = {min_xp:.4e} / {dp:.4e} = {dx_min:.4e} m\n"
            ));
        }
        "delta_E" => {
            let dt = require(args, "delta_t")?;
            let de_min = min_xp / dt;
            out.push_str(&format!(
                "ΔE_min = ħ/(2Δt) = {min_xp:.4e} / {dt:.4e} = {de_min:.4e} J = {:.4e} eV\n",
                de_min / EV_TO_J
            ));
        }
        "delta_t" => {
            let de = require(args, "delta_E")?;
            let dt_min = min_xp / de;
            out.push_str(&format!(
                "Δt_min = ħ/(2ΔE) = {min_xp:.4e} / {de:.4e} = {dt_min:.4e} s\n"
            ));
        }
        _ => {
            return Err(format!(
                "unknown solve_for '{solve}'; use: delta_p|delta_x|delta_E|delta_t"
            ))
        }
    }

    Ok(out)
}

fn action_de_broglie(args: &Value) -> Result<String, String> {
    let mass = n(args, "m").unwrap_or(M_E);
    let mut out = String::from("=== DE BROGLIE WAVELENGTH ===\n");

    let lambda = if let Some(vel) = n(args, "v") {
        let p = mass * vel;
        let lam = H / p;
        let ke = 0.5 * mass * vel * vel;
        out.push_str(&format!(
            "m = {mass:.4e} kg  |  v = {vel:.4e} m/s\np = mv = {p:.4e} kg·m/s\n\
             λ = h/p = {H:.4e} / {p:.4e} = {lam:.4e} m  ({:.4e} nm)\nKE = ½mv² = {ke:.4e} J = {:.4e} eV\n",
            lam * 1e9,
            ke / EV_TO_J
        ));
        lam
    } else if let Some(p) = n(args, "p") {
        let lam = H / p;
        out.push_str(&format!(
            "p = {p:.4e} kg·m/s\nλ = h/p = {H:.4e} / {p:.4e} = {lam:.4e} m  ({:.4e} nm)\n",
            lam * 1e9
        ));
        lam
    } else if let Some(e_ev) = n(args, "E") {
        let e_j = e_ev * EV_TO_J;
        let p = (2.0 * mass * e_j).sqrt();
        let lam = H / p;
        out.push_str(&format!(
            "KE = {e_ev} eV = {e_j:.4e} J  |  m = {mass:.4e} kg\np = √(2mKE) = {p:.4e} kg·m/s\n\
             λ = h/p = {lam:.4e} m  ({:.4e} nm)\n",
            lam * 1e9
        ));
        lam
    } else {
        return Err(
            "provide v (velocity m/s), p (momentum kg·m/s), or E (kinetic energy eV)".into(),
        );
    };

    let _ = lambda;
    Ok(out)
}

fn action_photoelectric(args: &Value) -> Result<String, String> {
    let phi_ev = require(args, "phi")?;
    let phi_j = phi_ev * EV_TO_J;

    let (e_photon_j, e_photon_ev, source_str) = if let Some(f_hz) = n(args, "f") {
        let e = H * f_hz;
        (e, e / EV_TO_J, format!("f = {f_hz:.4e} Hz"))
    } else if let Some(lam_nm) = n(args, "lambda") {
        let lam_m = lam_nm * 1e-9;
        let e = H * C / lam_m;
        (e, e / EV_TO_J, format!("λ = {lam_nm} nm"))
    } else {
        return Err("provide f (frequency Hz) or lambda (wavelength nm)".into());
    };

    let mut out = format!(
        "=== PHOTOELECTRIC EFFECT ===\nPhoton: {source_str}\nWork function: φ = {phi_ev} eV\n\n"
    );

    out.push_str(&format!(
        "Photon energy: E = hf = {e_photon_j:.4e} J = {e_photon_ev:.4} eV\n"
    ));

    let threshold_f = phi_j / H;
    let threshold_lam_nm = H * C / phi_j * 1e9;
    out.push_str(&format!(
        "Threshold frequency: f₀ = φ/h = {threshold_f:.4e} Hz\nThreshold wavelength: λ₀ = {threshold_lam_nm:.2} nm\n\n"
    ));

    if e_photon_j > phi_j {
        let ke_j = e_photon_j - phi_j;
        let ke_ev = ke_j / EV_TO_J;
        let v_max = (2.0 * ke_j / M_E).sqrt();
        let v_stop = ke_ev;
        out.push_str(&format!(
            "KE_max = E - φ = {e_photon_ev:.4} - {phi_ev} = {ke_ev:.4} eV = {ke_j:.4e} J\n\
             Max electron speed: v = √(2·KE/m_e) = {v_max:.4e} m/s\n\
             Stopping potential: V_s = KE/e = {v_stop:.4} V\n"
        ));
    } else {
        out.push_str("No photoelectric emission: photon energy < work function\n");
    }

    Ok(out)
}

fn action_compton(args: &Value) -> Result<String, String> {
    let theta_deg = require(args, "theta")?;
    let theta_rad = theta_deg * PI / 180.0;

    let (lambda0_m, lambda_str) = if let Some(lam) = n(args, "lambda") {
        if lam > 1e-6 {
            (lam * 1e-9, format!("{lam} nm"))
        } else {
            (lam, format!("{lam:.4e} m"))
        }
    } else {
        return Err("provide lambda (incident wavelength in nm or m)".into());
    };

    let delta_lambda = LAMBDA_C * (1.0 - theta_rad.cos());
    let lambda1_m = lambda0_m + delta_lambda;

    let e0 = H * C / lambda0_m;
    let e1 = H * C / lambda1_m;
    let ke_recoil = e0 - e1;

    let out = format!(
        "=== COMPTON SCATTERING ===\nIncident wavelength: λ₀ = {lambda_str}\nScattering angle: θ = {theta_deg}°\n\n\
         Compton wavelength: λ_C = h/(m_e·c) = {LAMBDA_C:.4e} m\n\
         Wavelength shift: Δλ = λ_C(1-cos θ) = {LAMBDA_C:.4e}×(1-cos{theta_deg}°) = {delta_lambda:.4e} m\n\
         Scattered wavelength: λ' = λ₀ + Δλ = {lambda1_m:.4e} m  ({:.4e} nm)\n\n\
         Incident photon energy: E₀ = {e0:.4e} J = {:.4} eV\n\
         Scattered photon energy: E' = {e1:.4e} J = {:.4} eV\n\
         Electron recoil KE = E₀ - E' = {ke_recoil:.4e} J = {:.4} eV\n",
        lambda1_m * 1e9,
        e0 / EV_TO_J,
        e1 / EV_TO_J,
        ke_recoil / EV_TO_J
    );

    Ok(out)
}

fn action_tunneling(args: &Value) -> Result<String, String> {
    let e_ev = require(args, "E")?;
    let v0_ev = require(args, "V0")?;
    let l_nm = require(args, "L")?;
    let mass = n(args, "m").unwrap_or(M_E);

    let mut out = String::from("=== QUANTUM TUNNELING ===\n");
    out.push_str(&format!(
        "Particle KE: E = {e_ev} eV  |  Barrier: V₀ = {v0_ev} eV, L = {l_nm} nm\nm = {mass:.4e} kg\n\n"
    ));

    if e_ev >= v0_ev {
        out.push_str("E ≥ V₀: classically allowed — no tunneling, wave passes over barrier.\n");
        return Ok(out);
    }

    let delta_e = (v0_ev - e_ev) * EV_TO_J;
    let l_m = l_nm * 1e-9;
    let kappa = (2.0 * mass * delta_e).sqrt() / HBAR;
    let transmission = (-2.0 * kappa * l_m).exp();
    let penetration_depth = 1.0 / kappa;

    out.push_str(&format!(
        "Evanescent decay constant: κ = √(2m(V₀-E))/ħ = {kappa:.4e} m⁻¹\n\
         Penetration depth: 1/κ = {penetration_depth:.4e} m  ({:.4} nm)\n\
         Transmission T ≈ e^(-2κL) = e^(-{:.4}) = {transmission:.4e}\n\
         Transmission probability: T ≈ {:.2e}  ({:.4e}%)\n",
        penetration_depth * 1e9,
        2.0 * kappa * l_m,
        transmission,
        transmission * 100.0
    ));

    Ok(out)
}

fn action_harmonic(args: &Value) -> Result<String, String> {
    let qn = n(args, "n").unwrap_or(0.0) as u32;
    let mass = n(args, "m").unwrap_or(M_E);

    let omega = if let Some(w) = n(args, "omega") {
        w
    } else if let Some(k) = n(args, "k") {
        (k / mass).sqrt()
    } else {
        return Err("provide omega (angular frequency rad/s) or k (spring constant N/m)".into());
    };

    let mut out = format!(
        "=== QUANTUM HARMONIC OSCILLATOR ===\nm = {mass:.4e} kg  |  ω = {omega:.4e} rad/s\nħω = {:.4e} J = {:.4e} eV\n\n",
        HBAR * omega,
        HBAR * omega / EV_TO_J
    );

    for i in 0..=(qn.max(3)) {
        let e_j = (i as f64 + 0.5) * HBAR * omega;
        let e_ev = e_j / EV_TO_J;
        let marker = if i == 0 { " ← zero-point energy" } else { "" };
        out.push_str(&format!("n={i}: E = {e_j:.4e} J = {e_ev:.4e} eV{marker}\n"));
    }

    let e0 = 0.5 * HBAR * omega;
    out.push_str(&format!(
        "\nEn = (n + ½)ħω\nZero-point energy E₀ = ½ħω = {e0:.4e} J = {:.4e} eV\nLevel spacing ΔE = ħω = {:.4e} eV\n",
        e0 / EV_TO_J,
        HBAR * omega / EV_TO_J
    ));

    Ok(out)
}

pub async fn execute(args: &Value) -> Result<String, String> {
    let action = args
        .get("action")
        .and_then(|x| x.as_str())
        .unwrap_or("particle_box");
    match action {
        "particle_box" => action_particle_box(args),
        "hydrogen" => action_hydrogen(args),
        "uncertainty" => action_uncertainty(args),
        "de_broglie" => action_de_broglie(args),
        "photoelectric" => action_photoelectric(args),
        "compton" => action_compton(args),
        "tunneling" => action_tunneling(args),
        "harmonic" => action_harmonic(args),
        _ => Err(format!("unknown action '{action}'; use: particle_box|hydrogen|uncertainty|de_broglie|photoelectric|compton|tunneling|harmonic")),
    }
}
