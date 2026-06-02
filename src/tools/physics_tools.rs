use serde_json::{json, Value};

pub fn physics_tools_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "action": {
                "type": "string",
                "enum": ["constant", "formula", "list", "domains"],
                "description": "constant: look up a physical constant | formula: evaluate a named physics formula | list: browse constants or formulas | domains: show all domains"
            },
            "query": {"type": "string", "description": "Constant name, symbol, or keyword (e.g. 'speed of light', 'c', 'planck')"},
            "name": {"type": "string", "description": "Formula name (e.g. 'kinetic_energy', 'ohms_law', 'ideal_gas')"},
            "vars": {"type": "object", "description": "Known variables as {symbol: value} — omit the one to solve for"},
            "domain": {"type": "string", "description": "Domain filter for list action (e.g. 'mechanics', 'electromagnetism')"},
            "what": {"type": "string", "enum": ["constants", "formulas"], "description": "What to list (default: constants)"}
        },
        "required": []
    })
}

struct Constant {
    name: &'static str,
    symbol: &'static str,
    value: f64,
    unit: &'static str,
    domain: &'static str,
    aliases: &'static [&'static str],
}

const CONSTANTS: &[Constant] = &[
    Constant {
        name: "Speed of light in vacuum",
        symbol: "c",
        value: 299_792_458.0,
        unit: "m/s",
        domain: "electromagnetism",
        aliases: &["speed of light", "light speed", "c"],
    },
    Constant {
        name: "Planck constant",
        symbol: "h",
        value: 6.626_070_15e-34,
        unit: "J·s",
        domain: "quantum",
        aliases: &["planck", "planck constant", "h"],
    },
    Constant {
        name: "Reduced Planck constant",
        symbol: "ħ",
        value: 1.054_571_817e-34,
        unit: "J·s",
        domain: "quantum",
        aliases: &["reduced planck", "h-bar", "hbar", "ħ"],
    },
    Constant {
        name: "Gravitational constant",
        symbol: "G",
        value: 6.674_30e-11,
        unit: "m³/(kg·s²)",
        domain: "gravity",
        aliases: &["gravitational", "gravity", "newton", "G"],
    },
    Constant {
        name: "Boltzmann constant",
        symbol: "k_B",
        value: 1.380_649e-23,
        unit: "J/K",
        domain: "thermodynamics",
        aliases: &["boltzmann", "k_b", "kb", "k"],
    },
    Constant {
        name: "Avogadro constant",
        symbol: "N_A",
        value: 6.022_140_76e23,
        unit: "mol⁻¹",
        domain: "chemistry",
        aliases: &["avogadro", "n_a", "na", "avogadros number"],
    },
    Constant {
        name: "Molar gas constant",
        symbol: "R",
        value: 8.314_462_618,
        unit: "J/(mol·K)",
        domain: "thermodynamics",
        aliases: &["gas constant", "molar gas", "universal gas", "R"],
    },
    Constant {
        name: "Stefan-Boltzmann constant",
        symbol: "σ",
        value: 5.670_374_419e-8,
        unit: "W/(m²·K⁴)",
        domain: "thermodynamics",
        aliases: &["stefan-boltzmann", "stefan boltzmann", "sigma", "σ"],
    },
    Constant {
        name: "Elementary charge",
        symbol: "e",
        value: 1.602_176_634e-19,
        unit: "C",
        domain: "electromagnetism",
        aliases: &["elementary charge", "electron charge", "proton charge", "e"],
    },
    Constant {
        name: "Vacuum permittivity",
        symbol: "ε₀",
        value: 8.854_187_812_8e-12,
        unit: "F/m",
        domain: "electromagnetism",
        aliases: &["vacuum permittivity", "permittivity", "epsilon0", "ε₀"],
    },
    Constant {
        name: "Vacuum permeability",
        symbol: "μ₀",
        value: 1.256_637_062_12e-6,
        unit: "H/m",
        domain: "electromagnetism",
        aliases: &["vacuum permeability", "permeability", "mu0", "μ₀"],
    },
    Constant {
        name: "Coulomb constant",
        symbol: "k_e",
        value: 8.987_551_792_3e9,
        unit: "N·m²/C²",
        domain: "electromagnetism",
        aliases: &["coulomb", "coulombs constant", "k_e", "ke"],
    },
    Constant {
        name: "Faraday constant",
        symbol: "F",
        value: 96_485.332_12,
        unit: "C/mol",
        domain: "chemistry",
        aliases: &["faraday", "faraday constant", "F"],
    },
    Constant {
        name: "Electron mass",
        symbol: "m_e",
        value: 9.109_383_701_5e-31,
        unit: "kg",
        domain: "atomic",
        aliases: &["electron mass", "m_e", "me"],
    },
    Constant {
        name: "Proton mass",
        symbol: "m_p",
        value: 1.672_621_923_69e-27,
        unit: "kg",
        domain: "atomic",
        aliases: &["proton mass", "m_p", "mp"],
    },
    Constant {
        name: "Neutron mass",
        symbol: "m_n",
        value: 1.674_927_498_04e-27,
        unit: "kg",
        domain: "atomic",
        aliases: &["neutron mass", "m_n", "mn"],
    },
    Constant {
        name: "Atomic mass unit",
        symbol: "m_u",
        value: 1.660_539_066_60e-27,
        unit: "kg",
        domain: "atomic",
        aliases: &["atomic mass unit", "dalton", "amu", "m_u", "mu"],
    },
    Constant {
        name: "Bohr radius",
        symbol: "a₀",
        value: 5.291_772_109_03e-11,
        unit: "m",
        domain: "atomic",
        aliases: &["bohr radius", "a0", "a₀"],
    },
    Constant {
        name: "Fine structure constant",
        symbol: "α",
        value: 7.297_352_569_3e-3,
        unit: "dimensionless",
        domain: "atomic",
        aliases: &["fine structure", "alpha", "α"],
    },
    Constant {
        name: "Rydberg constant",
        symbol: "R_∞",
        value: 10_973_731.568_160,
        unit: "m⁻¹",
        domain: "atomic",
        aliases: &["rydberg", "R_∞", "r_inf"],
    },
    Constant {
        name: "Bohr magneton",
        symbol: "μ_B",
        value: 9.274_010_078_3e-24,
        unit: "J/T",
        domain: "atomic",
        aliases: &["bohr magneton", "mu_b", "μ_B"],
    },
    Constant {
        name: "Nuclear magneton",
        symbol: "μ_N",
        value: 5.050_783_746_1e-27,
        unit: "J/T",
        domain: "atomic",
        aliases: &["nuclear magneton", "mu_n", "μ_N"],
    },
    Constant {
        name: "Planck length",
        symbol: "ℓ_P",
        value: 1.616_255e-35,
        unit: "m",
        domain: "planck",
        aliases: &["planck length", "lp", "ℓ_P"],
    },
    Constant {
        name: "Planck mass",
        symbol: "m_P",
        value: 2.176_434e-8,
        unit: "kg",
        domain: "planck",
        aliases: &["planck mass", "m_P"],
    },
    Constant {
        name: "Planck time",
        symbol: "t_P",
        value: 5.391_247e-44,
        unit: "s",
        domain: "planck",
        aliases: &["planck time", "t_P"],
    },
    Constant {
        name: "Standard gravity",
        symbol: "g",
        value: 9.806_65,
        unit: "m/s²",
        domain: "mechanics",
        aliases: &[
            "gravity",
            "standard gravity",
            "g",
            "gravitational acceleration",
        ],
    },
    Constant {
        name: "Standard atmosphere",
        symbol: "atm",
        value: 101_325.0,
        unit: "Pa",
        domain: "thermodynamics",
        aliases: &["atmosphere", "atm", "standard atmosphere"],
    },
    Constant {
        name: "Speed of sound in air",
        symbol: "v_s",
        value: 343.0,
        unit: "m/s",
        domain: "waves",
        aliases: &["speed of sound", "sound speed", "v_s"],
    },
    Constant {
        name: "Electron volt",
        symbol: "eV",
        value: 1.602_176_634e-19,
        unit: "J",
        domain: "atomic",
        aliases: &["electron volt", "ev"],
    },
];

fn search_constant(query: &str) -> Vec<&'static Constant> {
    let q = query.to_lowercase();
    CONSTANTS
        .iter()
        .filter(|c| {
            c.name.to_lowercase().contains(&q)
                || c.symbol.to_lowercase().contains(&q)
                || c.aliases.iter().any(|a| a.to_lowercase().contains(&q))
        })
        .collect()
}

fn fmt_sci(v: f64) -> String {
    if v == 0.0 {
        return "0".to_string();
    }
    let exp = v.abs().log10().floor() as i32;
    if exp >= -3 && exp <= 6 {
        if exp >= 0 {
            format!("{}", v)
        } else {
            format!("{:.6}", v)
                .trim_end_matches('0')
                .trim_end_matches('.')
                .to_string()
        }
    } else {
        let mantissa = v / 10f64.powi(exp);
        let m_str = format!("{:.6}", mantissa)
            .trim_end_matches('0')
            .trim_end_matches('.')
            .to_string();
        if exp == 0 {
            m_str
        } else {
            format!("{} × 10^{}", m_str, exp)
        }
    }
}

fn action_constant(args: &Value) -> String {
    let query = args.get("query").and_then(|v| v.as_str()).unwrap_or("");
    if query.is_empty() {
        return "Provide 'query' with a constant name or symbol (e.g. 'c', 'planck', 'boltzmann')."
            .to_string();
    }
    let results = search_constant(query);
    if results.is_empty() {
        return format!(
            "No constant found matching '{}'. Try 'list' to browse all constants.",
            query
        );
    }
    let mut out = String::from("PHYSICAL CONSTANTS\n==================\n\n");
    for c in results {
        out.push_str(&format!("{} ({})\n", c.name, c.symbol));
        out.push_str(&format!("  Value  : {} {}\n", fmt_sci(c.value), c.unit));
        out.push_str(&format!("  Domain : {}\n", c.domain));
        out.push_str(&format!("  Exact  : {:.15e}\n\n", c.value));
    }
    out.trim_end().to_string()
}

use std::collections::HashMap;

fn solve_kinematics_v(vars: &HashMap<String, f64>) -> Result<(String, f64, &'static str), String> {
    let u = vars.get("u").copied();
    let a = vars.get("a").copied();
    let t = vars.get("t").copied();
    let v = vars.get("v").copied();
    let s = vars.get("s").copied();
    match (v, u, a, t, s) {
        (None, Some(u), Some(a), Some(t), _) => Ok(("v".into(), u + a * t, "m/s")),
        (Some(v), None, Some(a), Some(t), _) => Ok(("u".into(), v - a * t, "m/s")),
        (Some(v), Some(u), None, Some(t), _) => Ok(("a".into(), (v - u) / t, "m/s²")),
        (Some(v), Some(u), Some(a), None, _) => Ok(("t".into(), (v - u) / a, "s")),
        (None, Some(u), Some(a), _, Some(s)) => {
            Ok(("v".into(), (u * u + 2.0 * a * s).sqrt(), "m/s"))
        }
        _ => Err("Provide u,a,t to solve for v — or u,a,s to solve for v via v²=u²+2as".into()),
    }
}
fn solve_newtons_second(
    vars: &HashMap<String, f64>,
) -> Result<(String, f64, &'static str), String> {
    let f = vars.get("F").copied();
    let m = vars.get("m").copied();
    let a = vars.get("a").copied();
    match (f, m, a) {
        (None, Some(m), Some(a)) => Ok(("F".into(), m * a, "N")),
        (Some(f), None, Some(a)) => Ok(("m".into(), f / a, "kg")),
        (Some(f), Some(m), None) => Ok(("a".into(), f / m, "m/s²")),
        _ => Err("Provide exactly two of: F (N), m (kg), a (m/s²)".into()),
    }
}
fn solve_kinetic_energy(
    vars: &HashMap<String, f64>,
) -> Result<(String, f64, &'static str), String> {
    let ke = vars.get("KE").copied();
    let m = vars.get("m").copied();
    let v = vars.get("v").copied();
    match (ke, m, v) {
        (None, Some(m), Some(v)) => Ok(("KE".into(), 0.5 * m * v * v, "J")),
        (Some(ke), None, Some(v)) => Ok(("m".into(), 2.0 * ke / (v * v), "kg")),
        (Some(ke), Some(m), None) => Ok(("v".into(), (2.0 * ke / m).sqrt(), "m/s")),
        _ => Err("Provide exactly two of: KE (J), m (kg), v (m/s)".into()),
    }
}
fn solve_gpe(vars: &HashMap<String, f64>) -> Result<(String, f64, &'static str), String> {
    let pe = vars.get("PE").copied();
    let m = vars.get("m").copied();
    let h = vars.get("h").copied();
    const G: f64 = 9.806_65;
    match (pe, m, h) {
        (None, Some(m), Some(h)) => Ok(("PE".into(), m * G * h, "J")),
        (Some(pe), None, Some(h)) => Ok(("m".into(), pe / (G * h), "kg")),
        (Some(pe), Some(m), None) => Ok(("h".into(), pe / (m * G), "m")),
        _ => Err("Provide exactly two of: PE (J), m (kg), h (m)".into()),
    }
}
fn solve_mass_energy(vars: &HashMap<String, f64>) -> Result<(String, f64, &'static str), String> {
    const C: f64 = 299_792_458.0;
    let e = vars.get("E").copied();
    let m = vars.get("m").copied();
    match (e, m) {
        (None, Some(m)) => Ok(("E".into(), m * C * C, "J")),
        (Some(e), None) => Ok(("m".into(), e / (C * C), "kg")),
        _ => Err("Provide one of: E (J), m (kg)".into()),
    }
}
fn solve_momentum(vars: &HashMap<String, f64>) -> Result<(String, f64, &'static str), String> {
    let p = vars.get("p").copied();
    let m = vars.get("m").copied();
    let v = vars.get("v").copied();
    match (p, m, v) {
        (None, Some(m), Some(v)) => Ok(("p".into(), m * v, "kg·m/s")),
        (Some(p), None, Some(v)) => Ok(("m".into(), p / v, "kg")),
        (Some(p), Some(m), None) => Ok(("v".into(), p / m, "m/s")),
        _ => Err("Provide exactly two of: p (kg·m/s), m (kg), v (m/s)".into()),
    }
}
fn solve_ohms_law(vars: &HashMap<String, f64>) -> Result<(String, f64, &'static str), String> {
    let v = vars.get("V").copied();
    let i = vars.get("I").copied();
    let r = vars.get("R").copied();
    match (v, i, r) {
        (None, Some(i), Some(r)) => Ok(("V".into(), i * r, "V")),
        (Some(v), None, Some(r)) => Ok(("I".into(), v / r, "A")),
        (Some(v), Some(i), None) => Ok(("R".into(), v / i, "Ω")),
        _ => Err("Provide exactly two of: V (V), I (A), R (Ω)".into()),
    }
}
fn solve_electric_power(
    vars: &HashMap<String, f64>,
) -> Result<(String, f64, &'static str), String> {
    let p = vars.get("P").copied();
    let v = vars.get("V").copied();
    let i = vars.get("I").copied();
    match (p, v, i) {
        (None, Some(v), Some(i)) => Ok(("P".into(), v * i, "W")),
        (Some(p), None, Some(i)) => Ok(("V".into(), p / i, "V")),
        (Some(p), Some(v), None) => Ok(("I".into(), p / v, "A")),
        _ => Err("Provide exactly two of: P (W), V (V), I (A)".into()),
    }
}
fn solve_coulombs_law(vars: &HashMap<String, f64>) -> Result<(String, f64, &'static str), String> {
    const KE: f64 = 8.987_551_792_3e9;
    let f = vars.get("F").copied();
    let q1 = vars.get("q1").copied();
    let q2 = vars.get("q2").copied();
    let r = vars.get("r").copied();
    match (f, q1, q2, r) {
        (None, Some(q1), Some(q2), Some(r)) => Ok(("F".into(), KE * q1 * q2 / (r * r), "N")),
        (Some(f), None, Some(q2), Some(r)) => Ok(("q1".into(), f * r * r / (KE * q2), "C")),
        (Some(f), Some(q1), None, Some(r)) => Ok(("q2".into(), f * r * r / (KE * q1), "C")),
        (Some(f), Some(q1), Some(q2), None) => {
            Ok(("r".into(), (KE * q1 * q2 / f).abs().sqrt(), "m"))
        }
        _ => Err("Provide three of: F (N), q1 (C), q2 (C), r (m)".into()),
    }
}
fn solve_wave_speed(vars: &HashMap<String, f64>) -> Result<(String, f64, &'static str), String> {
    let v = vars.get("v").copied();
    let f = vars.get("f").copied();
    let lam = vars.get("λ").or_else(|| vars.get("lambda")).copied();
    match (v, f, lam) {
        (None, Some(f), Some(l)) => Ok(("v".into(), f * l, "m/s")),
        (Some(v), None, Some(l)) => Ok(("f".into(), v / l, "Hz")),
        (Some(v), Some(f), None) => Ok(("λ".into(), v / f, "m")),
        _ => Err("Provide exactly two of: v (m/s), f (Hz), λ/lambda (m)".into()),
    }
}
fn solve_photon_energy(vars: &HashMap<String, f64>) -> Result<(String, f64, &'static str), String> {
    const H: f64 = 6.626_070_15e-34;
    let e = vars.get("E").copied();
    let f = vars.get("f").copied();
    match (e, f) {
        (None, Some(f)) => Ok(("E".into(), H * f, "J")),
        (Some(e), None) => Ok(("f".into(), e / H, "Hz")),
        _ => Err("Provide one of: E (J), f (Hz)".into()),
    }
}
fn solve_ideal_gas(vars: &HashMap<String, f64>) -> Result<(String, f64, &'static str), String> {
    const R: f64 = 8.314_462_618;
    let p = vars.get("P").copied();
    let v = vars.get("V").copied();
    let n = vars.get("n").copied();
    let t = vars.get("T").copied();
    match (p, v, n, t) {
        (None, Some(v), Some(n), Some(t)) => Ok(("P".into(), n * R * t / v, "Pa")),
        (Some(p), None, Some(n), Some(t)) => Ok(("V".into(), n * R * t / p, "m³")),
        (Some(p), Some(v), None, Some(t)) => Ok(("n".into(), p * v / (R * t), "mol")),
        (Some(p), Some(v), Some(n), None) => Ok(("T".into(), p * v / (n * R), "K")),
        _ => Err("Provide three of: P (Pa), V (m³), n (mol), T (K)".into()),
    }
}
fn solve_heat(vars: &HashMap<String, f64>) -> Result<(String, f64, &'static str), String> {
    let q = vars.get("Q").copied();
    let m = vars.get("m").copied();
    let c = vars.get("c").copied();
    let dt = vars.get("ΔT").or_else(|| vars.get("dT")).copied();
    match (q, m, c, dt) {
        (None, Some(m), Some(c), Some(dt)) => Ok(("Q".into(), m * c * dt, "J")),
        (Some(q), None, Some(c), Some(dt)) => Ok(("m".into(), q / (c * dt), "kg")),
        (Some(q), Some(m), None, Some(dt)) => Ok(("c".into(), q / (m * dt), "J/(kg·K)")),
        (Some(q), Some(m), Some(c), None) => Ok(("ΔT".into(), q / (m * c), "K")),
        _ => Err("Provide three of: Q (J), m (kg), c (J/(kg·K)), ΔT/dT (K)".into()),
    }
}
fn solve_carnot(vars: &HashMap<String, f64>) -> Result<(String, f64, &'static str), String> {
    let eta = vars.get("η").or_else(|| vars.get("eta")).copied();
    let th = vars.get("T_h").or_else(|| vars.get("Th")).copied();
    let tc = vars.get("T_c").or_else(|| vars.get("Tc")).copied();
    match (eta, th, tc) {
        (None, Some(th), Some(tc)) => Ok(("η".into(), 1.0 - tc / th, "dimensionless")),
        (Some(eta), None, Some(tc)) => Ok(("T_h".into(), tc / (1.0 - eta), "K")),
        (Some(eta), Some(th), None) => Ok(("T_c".into(), th * (1.0 - eta), "K")),
        _ => Err("Provide two of: η/eta, T_h/Th (K), T_c/Tc (K)".into()),
    }
}
fn solve_thin_lens(vars: &HashMap<String, f64>) -> Result<(String, f64, &'static str), String> {
    let f = vars.get("f").copied();
    let do_ = vars.get("d_o").or_else(|| vars.get("do")).copied();
    let di = vars.get("d_i").or_else(|| vars.get("di")).copied();
    match (f, do_, di) {
        (None, Some(do_), Some(di)) => Ok(("f".into(), 1.0 / (1.0 / do_ + 1.0 / di), "m")),
        (Some(f), None, Some(di)) => Ok(("d_o".into(), 1.0 / (1.0 / f - 1.0 / di), "m")),
        (Some(f), Some(do_), None) => Ok(("d_i".into(), 1.0 / (1.0 / f - 1.0 / do_), "m")),
        _ => Err("Provide two of: f (m), d_o/do (m), d_i/di (m)".into()),
    }
}
fn solve_snell(vars: &HashMap<String, f64>) -> Result<(String, f64, &'static str), String> {
    let n1 = vars.get("n1").copied();
    let theta1 = vars.get("θ1").or_else(|| vars.get("theta1")).copied();
    let n2 = vars.get("n2").copied();
    let theta2 = vars.get("θ2").or_else(|| vars.get("theta2")).copied();
    match (n1, theta1, n2, theta2) {
        (Some(n1), Some(t1), Some(n2), None) => {
            let sin_t2 = n1 * t1.to_radians().sin() / n2;
            if sin_t2.abs() > 1.0 {
                return Err("Total internal reflection — sin(θ₂) > 1".into());
            }
            Ok(("θ2".into(), sin_t2.asin().to_degrees(), "degrees"))
        }
        (Some(n1), None, Some(n2), Some(t2)) => {
            let sin_t1 = n2 * t2.to_radians().sin() / n1;
            if sin_t1.abs() > 1.0 {
                return Err("Impossible configuration — sin(θ₁) > 1".into());
            }
            Ok(("θ1".into(), sin_t1.asin().to_degrees(), "degrees"))
        }
        (None, Some(t1), Some(n2), Some(t2)) => Ok((
            "n1".into(),
            n2 * t2.to_radians().sin() / t1.to_radians().sin(),
            "dimensionless",
        )),
        (Some(n1), Some(t1), None, Some(t2)) => Ok((
            "n2".into(),
            n1 * t1.to_radians().sin() / t2.to_radians().sin(),
            "dimensionless",
        )),
        _ => Err("Provide three of: n1, θ1/theta1 (degrees), n2, θ2/theta2 (degrees)".into()),
    }
}
fn solve_de_broglie(vars: &HashMap<String, f64>) -> Result<(String, f64, &'static str), String> {
    const H: f64 = 6.626_070_15e-34;
    let lam = vars.get("λ").or_else(|| vars.get("lambda")).copied();
    let p = vars.get("p").copied();
    let m = vars.get("m").copied();
    let v = vars.get("v").copied();
    match (lam, p, m, v) {
        (None, Some(p), _, _) => Ok(("λ".into(), H / p, "m")),
        (None, None, Some(m), Some(v)) => Ok(("λ".into(), H / (m * v), "m")),
        (Some(lam), None, _, _) => Ok(("p".into(), H / lam, "kg·m/s")),
        _ => Err("Provide p (kg·m/s) or m,v (kg, m/s) to find λ; or λ to find p".into()),
    }
}
fn solve_centripetal(vars: &HashMap<String, f64>) -> Result<(String, f64, &'static str), String> {
    let f = vars.get("F").copied();
    let m = vars.get("m").copied();
    let v = vars.get("v").copied();
    let r = vars.get("r").copied();
    match (f, m, v, r) {
        (None, Some(m), Some(v), Some(r)) => Ok(("F".into(), m * v * v / r, "N")),
        (Some(f), None, Some(v), Some(r)) => Ok(("m".into(), f * r / (v * v), "kg")),
        (Some(f), Some(m), None, Some(r)) => Ok(("v".into(), (f * r / m).sqrt(), "m/s")),
        (Some(f), Some(m), Some(v), None) => Ok(("r".into(), m * v * v / f, "m")),
        _ => Err("Provide three of: F (N), m (kg), v (m/s), r (m)".into()),
    }
}
fn solve_gravitational_force(
    vars: &HashMap<String, f64>,
) -> Result<(String, f64, &'static str), String> {
    const G: f64 = 6.674_30e-11;
    let f = vars.get("F").copied();
    let m1 = vars.get("m1").copied();
    let m2 = vars.get("m2").copied();
    let r = vars.get("r").copied();
    match (f, m1, m2, r) {
        (None, Some(m1), Some(m2), Some(r)) => Ok(("F".into(), G * m1 * m2 / (r * r), "N")),
        (Some(f), None, Some(m2), Some(r)) => Ok(("m1".into(), f * r * r / (G * m2), "kg")),
        (Some(f), Some(m1), None, Some(r)) => Ok(("m2".into(), f * r * r / (G * m1), "kg")),
        (Some(f), Some(m1), Some(m2), None) => Ok(("r".into(), (G * m1 * m2 / f).sqrt(), "m")),
        _ => Err("Provide three of: F (N), m1 (kg), m2 (kg), r (m)".into()),
    }
}
fn solve_work(vars: &HashMap<String, f64>) -> Result<(String, f64, &'static str), String> {
    let w = vars.get("W").copied();
    let f = vars.get("F").copied();
    let d = vars.get("d").copied();
    let theta = vars
        .get("θ")
        .or_else(|| vars.get("theta"))
        .copied()
        .unwrap_or(0.0);
    match (w, f, d) {
        (None, Some(f), Some(d)) => Ok(("W".into(), f * d * theta.to_radians().cos(), "J")),
        (Some(w), None, Some(d)) => Ok(("F".into(), w / (d * theta.to_radians().cos()), "N")),
        (Some(w), Some(f), None) => Ok(("d".into(), w / (f * theta.to_radians().cos()), "m")),
        _ => Err(
            "Provide two of: W (J), F (N), d (m); optional θ/theta (degrees, default 0°)".into(),
        ),
    }
}

struct FormulaEntry {
    name: &'static str,
    description: &'static str,
    domain: &'static str,
    vars: &'static [(&'static str, &'static str)],
    solve: fn(&HashMap<String, f64>) -> Result<(String, f64, &'static str), String>,
}

const FORMULAS: &[FormulaEntry] = &[
    FormulaEntry {
        name: "kinematics",
        description: "v = u + at / v² = u² + 2as",
        domain: "mechanics",
        vars: &[
            ("u", "initial velocity m/s"),
            ("v", "final velocity m/s"),
            ("a", "acceleration m/s²"),
            ("t", "time s"),
            ("s", "displacement m"),
        ],
        solve: solve_kinematics_v,
    },
    FormulaEntry {
        name: "newtons_second",
        description: "F = ma",
        domain: "mechanics",
        vars: &[
            ("F", "force N"),
            ("m", "mass kg"),
            ("a", "acceleration m/s²"),
        ],
        solve: solve_newtons_second,
    },
    FormulaEntry {
        name: "kinetic_energy",
        description: "KE = ½mv²",
        domain: "mechanics",
        vars: &[
            ("KE", "kinetic energy J"),
            ("m", "mass kg"),
            ("v", "velocity m/s"),
        ],
        solve: solve_kinetic_energy,
    },
    FormulaEntry {
        name: "gravitational_pe",
        description: "PE = mgh",
        domain: "mechanics",
        vars: &[
            ("PE", "potential energy J"),
            ("m", "mass kg"),
            ("h", "height m"),
        ],
        solve: solve_gpe,
    },
    FormulaEntry {
        name: "mass_energy",
        description: "E = mc²",
        domain: "relativity",
        vars: &[("E", "energy J"), ("m", "mass kg")],
        solve: solve_mass_energy,
    },
    FormulaEntry {
        name: "momentum",
        description: "p = mv",
        domain: "mechanics",
        vars: &[
            ("p", "momentum kg·m/s"),
            ("m", "mass kg"),
            ("v", "velocity m/s"),
        ],
        solve: solve_momentum,
    },
    FormulaEntry {
        name: "work",
        description: "W = Fd·cos(θ)",
        domain: "mechanics",
        vars: &[
            ("W", "work J"),
            ("F", "force N"),
            ("d", "displacement m"),
            ("θ", "angle degrees (default 0)"),
        ],
        solve: solve_work,
    },
    FormulaEntry {
        name: "centripetal_force",
        description: "F = mv²/r",
        domain: "mechanics",
        vars: &[
            ("F", "force N"),
            ("m", "mass kg"),
            ("v", "velocity m/s"),
            ("r", "radius m"),
        ],
        solve: solve_centripetal,
    },
    FormulaEntry {
        name: "gravitational_force",
        description: "F = Gm₁m₂/r²",
        domain: "gravity",
        vars: &[
            ("F", "force N"),
            ("m1", "mass 1 kg"),
            ("m2", "mass 2 kg"),
            ("r", "separation m"),
        ],
        solve: solve_gravitational_force,
    },
    FormulaEntry {
        name: "ohms_law",
        description: "V = IR",
        domain: "electromagnetism",
        vars: &[
            ("V", "voltage V"),
            ("I", "current A"),
            ("R", "resistance Ω"),
        ],
        solve: solve_ohms_law,
    },
    FormulaEntry {
        name: "electric_power",
        description: "P = VI",
        domain: "electromagnetism",
        vars: &[("P", "power W"), ("V", "voltage V"), ("I", "current A")],
        solve: solve_electric_power,
    },
    FormulaEntry {
        name: "coulombs_law",
        description: "F = k_e·q₁q₂/r²",
        domain: "electromagnetism",
        vars: &[
            ("F", "force N"),
            ("q1", "charge 1 C"),
            ("q2", "charge 2 C"),
            ("r", "separation m"),
        ],
        solve: solve_coulombs_law,
    },
    FormulaEntry {
        name: "wave_speed",
        description: "v = fλ",
        domain: "waves",
        vars: &[
            ("v", "speed m/s"),
            ("f", "frequency Hz"),
            ("λ", "wavelength m (also: lambda)"),
        ],
        solve: solve_wave_speed,
    },
    FormulaEntry {
        name: "photon_energy",
        description: "E = hf",
        domain: "quantum",
        vars: &[("E", "energy J"), ("f", "frequency Hz")],
        solve: solve_photon_energy,
    },
    FormulaEntry {
        name: "ideal_gas",
        description: "PV = nRT",
        domain: "thermodynamics",
        vars: &[
            ("P", "pressure Pa"),
            ("V", "volume m³"),
            ("n", "moles mol"),
            ("T", "temperature K"),
        ],
        solve: solve_ideal_gas,
    },
    FormulaEntry {
        name: "heat_capacity",
        description: "Q = mcΔT",
        domain: "thermodynamics",
        vars: &[
            ("Q", "heat J"),
            ("m", "mass kg"),
            ("c", "specific heat J/(kg·K)"),
            ("ΔT", "temp change K (also: dT)"),
        ],
        solve: solve_heat,
    },
    FormulaEntry {
        name: "carnot_efficiency",
        description: "η = 1 - T_c/T_h",
        domain: "thermodynamics",
        vars: &[
            ("η", "efficiency (also: eta)"),
            ("T_h", "hot temp K (also: Th)"),
            ("T_c", "cold temp K (also: Tc)"),
        ],
        solve: solve_carnot,
    },
    FormulaEntry {
        name: "thin_lens",
        description: "1/f = 1/d_o + 1/d_i",
        domain: "optics",
        vars: &[
            ("f", "focal length m"),
            ("d_o", "object distance m (also: do)"),
            ("d_i", "image distance m (also: di)"),
        ],
        solve: solve_thin_lens,
    },
    FormulaEntry {
        name: "snells_law",
        description: "n₁·sin(θ₁) = n₂·sin(θ₂)",
        domain: "optics",
        vars: &[
            ("n1", "index of refraction 1"),
            ("θ1", "angle of incidence degrees (also: theta1)"),
            ("n2", "index of refraction 2"),
            ("θ2", "angle of refraction degrees (also: theta2)"),
        ],
        solve: solve_snell,
    },
    FormulaEntry {
        name: "de_broglie",
        description: "λ = h/p",
        domain: "quantum",
        vars: &[
            ("λ", "wavelength m (also: lambda)"),
            ("p", "momentum kg·m/s"),
            ("m", "mass kg"),
            ("v", "velocity m/s"),
        ],
        solve: solve_de_broglie,
    },
];

fn action_formula(args: &Value) -> String {
    let name = match args.get("name").and_then(|v| v.as_str()) {
        Some(n) => n.to_lowercase(),
        None => return "Provide 'name' of the formula (e.g. 'kinetic_energy'). Use 'list' with what='formulas' to browse.".to_string(),
    };
    let formula = match FORMULAS.iter().find(|f| f.name == name) {
        Some(f) => f,
        None => {
            let partial: Vec<&str> = FORMULAS
                .iter()
                .filter(|f| f.name.contains(&name) || f.domain.contains(&name))
                .map(|f| f.name)
                .collect();
            if partial.is_empty() {
                return format!("Unknown formula '{}'. Use 'list' with what='formulas' to browse all {} formulas.", name, FORMULAS.len());
            }
            return format!(
                "No exact match for '{}'. Did you mean one of: {}",
                name,
                partial.join(", ")
            );
        }
    };
    let vars_obj = args
        .get("vars")
        .and_then(|v| v.as_object())
        .cloned()
        .unwrap_or_default();
    let mut var_map: HashMap<String, f64> = HashMap::new();
    for (k, v) in &vars_obj {
        if let Some(n) = v.as_f64() {
            var_map.insert(k.clone(), n);
        }
    }
    let mut out = format!(
        "PHYSICS FORMULA: {}\n{}\n",
        formula.name.to_uppercase(),
        "=".repeat(40)
    );
    out.push_str(&format!(
        "\nFormula  : {}\nDomain   : {}\n\nVariables:\n",
        formula.description, formula.domain
    ));
    for (sym, desc) in formula.vars {
        let val = var_map
            .get(*sym)
            .map(|v| format!(" = {}", v))
            .unwrap_or_default();
        let mark = if var_map.contains_key(*sym) {
            "✓"
        } else {
            "?"
        };
        out.push_str(&format!("  {} {:8} — {}{}\n", mark, sym, desc, val));
    }
    match (formula.solve)(&var_map) {
        Ok((sym, val, unit)) => {
            out.push_str(&format!("\nResult: {} = {} {}\n", sym, fmt_sci(val), unit));
        }
        Err(e) => {
            out.push_str(&format!("\nError: {}\n", e));
        }
    }
    out.trim_end().to_string()
}

fn action_list(args: &Value) -> String {
    let what = args
        .get("what")
        .and_then(|v| v.as_str())
        .unwrap_or("constants");
    let domain_filter = args
        .get("domain")
        .and_then(|v| v.as_str())
        .map(|s| s.to_lowercase());
    if what == "formulas" {
        let mut out = String::from("PHYSICS FORMULAS\n================\n\n");
        let mut domains: Vec<&str> = FORMULAS.iter().map(|f| f.domain).collect();
        domains.sort_unstable();
        domains.dedup();
        for d in &domains {
            if let Some(ref df) = domain_filter {
                if !d.contains(df.as_str()) {
                    continue;
                }
            }
            out.push_str(&format!("  [{}]\n", d.to_uppercase()));
            for f in FORMULAS.iter().filter(|f| f.domain == *d) {
                out.push_str(&format!("    {:25} {}\n", f.name, f.description));
            }
            out.push('\n');
        }
        out.trim_end().to_string()
    } else {
        let mut out = String::from("PHYSICAL CONSTANTS\n==================\n\n");
        out.push_str(&format!(
            "{:<30} {:<8} {:<22} {}\n",
            "Name", "Symbol", "Value", "Unit"
        ));
        out.push_str(&format!("{}\n", "-".repeat(80)));
        for c in CONSTANTS {
            if let Some(ref df) = domain_filter {
                if !c.domain.contains(df.as_str()) {
                    continue;
                }
            }
            out.push_str(&format!(
                "{:<30} {:<8} {:<22} {}\n",
                c.name,
                c.symbol,
                fmt_sci(c.value),
                c.unit
            ));
        }
        out.trim_end().to_string()
    }
}

fn action_domains(_args: &Value) -> String {
    let mut out = String::from("PHYSICS DOMAINS\n===============\n\n");
    let mut domains: std::collections::BTreeMap<&str, (usize, usize)> = Default::default();
    for c in CONSTANTS {
        *domains.entry(c.domain).or_default() = (
            domains.get(c.domain).map(|v| v.0).unwrap_or(0) + 1,
            domains.get(c.domain).map(|v| v.1).unwrap_or(0),
        );
    }
    for f in FORMULAS {
        let e = domains.entry(f.domain).or_default();
        e.1 += 1;
    }
    out.push_str(&format!(
        "  {:<20} {:>10} {:>10}\n",
        "Domain", "Constants", "Formulas"
    ));
    out.push_str(&format!("  {}\n", "-".repeat(42)));
    for (d, (nc, nf)) in &domains {
        out.push_str(&format!("  {:<20} {:>10} {:>10}\n", d, nc, nf));
    }
    out.trim_end().to_string()
}

pub async fn execute(args: &Value) -> Result<String, String> {
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("constant");
    Ok(match action {
        "constant" => action_constant(args),
        "formula" => action_formula(args),
        "list" => action_list(args),
        "domains" => action_domains(args),
        other => format!(
            "Unknown action '{}'. Use: constant, formula, list, domains",
            other
        ),
    })
}
