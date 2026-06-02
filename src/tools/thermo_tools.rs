use serde_json::{json, Value};

pub fn thermo_tools_schema() -> Value {
    let num = |desc: &str| json!({"type": "number", "description": desc});
    let str_prop = |desc: &str| json!({"type": "string", "description": desc});
    let mut props = serde_json::Map::new();
    props.insert("action".into(), json!({
        "type": "string",
        "enum": ["ideal_gas","work","entropy","heat","cycles","fluid","properties","psychro"],
        "description": "ideal_gas: PV=nRT solver | work: thermodynamic work for a process | entropy: entropy change | heat: conduction/convection/radiation | cycles: Carnot/Otto/Diesel/Brayton efficiency | fluid: Reynolds/Bernoulli/Poiseuille | properties: gas/fluid properties lookup | psychro: psychrometrics (humidity, dew point)"
    }));
    props.insert(
        "solve_for".into(),
        str_prop("Variable to solve for in ideal_gas: P/V/n/T"),
    );
    props.insert(
        "P".into(),
        num("Pressure Pa (ideal_gas) or W/m² (radiation)"),
    );
    props.insert("V".into(), num("Volume m³"));
    props.insert("n".into(), num("Amount of substance mol"));
    props.insert(
        "T".into(),
        num("Temperature K (ideal_gas/entropy) or °C (psychro)"),
    );
    props.insert("T1".into(), num("Initial temperature K"));
    props.insert("T2".into(), num("Final temperature K"));
    props.insert("P1".into(), num("Initial pressure Pa"));
    props.insert("P2".into(), num("Final pressure Pa"));
    props.insert("V1".into(), num("Initial volume m³"));
    props.insert("V2".into(), num("Final volume m³"));
    props.insert(
        "process".into(),
        str_prop("Thermodynamic process: isothermal/isobaric/isochoric/adiabatic"),
    );
    props.insert(
        "gamma".into(),
        num("Heat capacity ratio Cp/Cv (default 1.4)"),
    );
    props.insert(
        "Cp".into(),
        num("Specific heat at constant pressure J/(mol·K)"),
    );
    props.insert(
        "Cv".into(),
        num("Specific heat at constant volume J/(mol·K)"),
    );
    props.insert("mode".into(), str_prop("heat: conduction/convection/radiation | fluid: reynolds/bernoulli/poiseuille/continuity"));
    props.insert("k".into(), num("Thermal conductivity W/(m·K) [conduction]"));
    props.insert("A".into(), num("Area m² [conduction/convection/radiation]"));
    props.insert(
        "dT".into(),
        num("Temperature difference K [conduction/convection]"),
    );
    props.insert(
        "L".into(),
        num("Length m [conduction] or pipe length m [Poiseuille]"),
    );
    props.insert("h".into(), num("Convection coefficient W/(m²·K)"));
    props.insert("epsilon".into(), num("Emissivity 0-1 [radiation]"));
    props.insert("Ts".into(), num("Surface temperature K [radiation]"));
    props.insert(
        "Tsurr".into(),
        num("Surroundings temperature K [radiation]"),
    );
    props.insert(
        "cycle".into(),
        str_prop("Thermodynamic cycle: carnot/otto/diesel/brayton"),
    );
    props.insert("Th".into(), num("Hot reservoir temperature K [Carnot]"));
    props.insert("Tc".into(), num("Cold reservoir temperature K [Carnot]"));
    props.insert("r".into(), num("Compression ratio [Otto/Diesel]"));
    props.insert("rc".into(), num("Cutoff ratio [Diesel]"));
    props.insert("rp".into(), num("Pressure ratio [Brayton]"));
    props.insert("rho".into(), num("Fluid density kg/m³"));
    props.insert("mu".into(), num("Dynamic viscosity Pa·s"));
    props.insert("v".into(), num("Flow velocity m/s"));
    props.insert("D".into(), num("Pipe diameter m"));
    props.insert("v1".into(), num("Velocity at point 1 m/s [Bernoulli]"));
    props.insert("v2".into(), num("Velocity at point 2 m/s [Bernoulli]"));
    props.insert("p1".into(), num("Pressure at point 1 Pa [Bernoulli]"));
    props.insert("p2".into(), num("Pressure at point 2 Pa [Bernoulli]"));
    props.insert("z1".into(), num("Height at point 1 m [Bernoulli]"));
    props.insert("z2".into(), num("Height at point 2 m [Bernoulli]"));
    props.insert(
        "Q".into(),
        num("Volumetric flow rate m³/s [Poiseuille/continuity]"),
    );
    props.insert("A1".into(), num("Cross-sectional area 1 m² [continuity]"));
    props.insert("A2".into(), num("Cross-sectional area 2 m² [continuity]"));
    props.insert(
        "substance".into(),
        str_prop("Substance name for properties lookup"),
    );
    props.insert("T_dry".into(), num("Dry-bulb temperature °C [psychro]"));
    props.insert("T_wet".into(), num("Wet-bulb temperature °C [psychro]"));
    props.insert("RH".into(), num("Relative humidity 0-100% [psychro]"));
    props.insert(
        "P_atm".into(),
        num("Atmospheric pressure Pa (default 101325) [psychro]"),
    );
    json!({
        "name": "thermo_tools",
        "description": "Thermodynamics and fluid mechanics: ideal gas law, work/entropy, heat transfer, cycle efficiency, fluid flow, substance properties. No external dependencies.",
        "input_schema": {"type": "object", "properties": Value::Object(props)}
    })
}

const R: f64 = 8.314462618; // J/(mol·K)
const SIGMA: f64 = 5.670374419e-8; // Stefan-Boltzmann W/(m²·K⁴)
const G: f64 = 9.80665; // m/s²

fn action_ideal_gas(args: &Value) -> Result<String, String> {
    let solve = args["solve_for"].as_str().unwrap_or("").to_uppercase();
    let get = |key: &str| args[key].as_f64();

    match solve.as_str() {
        "P" => {
            let (n, v, t) = (
                get("n").ok_or("n required")?,
                get("V").ok_or("V required")?,
                get("T").ok_or("T required")?,
            );
            if t <= 0.0 {
                return Err("T must be > 0 K".into());
            }
            if v <= 0.0 {
                return Err("V must be > 0".into());
            }
            let p = n * R * t / v;
            Ok(format!(
                "Ideal Gas Law: PV = nRT\n\nGiven: n = {n} mol, V = {v} m³, T = {t} K\n\nP = nRT/V = {} × {R:.4} × {} / {}\n  = {p:.4} Pa  ({:.4} kPa  /  {:.4} atm)\n\nNote: T_celsius = {:.2} °C",
                n, t, v, p / 1e3, p / 101325.0, t - 273.15
            ))
        }
        "V" => {
            let (n, p, t) = (
                get("n").ok_or("n required")?,
                get("P").ok_or("P required")?,
                get("T").ok_or("T required")?,
            );
            if t <= 0.0 {
                return Err("T must be > 0 K".into());
            }
            if p <= 0.0 {
                return Err("P must be > 0".into());
            }
            let v = n * R * t / p;
            Ok(format!(
                "Ideal Gas Law: PV = nRT\n\nGiven: n = {n} mol, P = {p} Pa, T = {t} K\n\nV = nRT/P = {} × {R:.4} × {} / {}\n  = {v:.6} m³  ({:.4} L)\n",
                n, t, p, v * 1000.0
            ))
        }
        "N" => {
            let (p, v, t) = (
                get("P").ok_or("P required")?,
                get("V").ok_or("V required")?,
                get("T").ok_or("T required")?,
            );
            if t <= 0.0 {
                return Err("T must be > 0 K".into());
            }
            let n = p * v / (R * t);
            Ok(format!(
                "Ideal Gas Law: PV = nRT\n\nGiven: P = {p} Pa, V = {v} m³, T = {t} K\n\nn = PV/RT = {} × {} / ({R:.4} × {})\n  = {n:.6} mol  ({:.4} g if air ≈28 g/mol)\n",
                p, v, t, n * 28.97
            ))
        }
        "T" => {
            let (p, v, n) = (
                get("P").ok_or("P required")?,
                get("V").ok_or("V required")?,
                get("n").ok_or("n required")?,
            );
            if n <= 0.0 {
                return Err("n must be > 0".into());
            }
            let t = p * v / (n * R);
            Ok(format!(
                "Ideal Gas Law: PV = nRT\n\nGiven: P = {p} Pa, V = {v} m³, n = {n} mol\n\nT = PV/nR = {} × {} / ({} × {R:.4})\n  = {t:.4} K  ({:.2} °C)\n",
                p, v, n, t - 273.15
            ))
        }
        _ => Err("solve_for must be P, V, n, or T".into()),
    }
}

fn action_work(args: &Value) -> Result<String, String> {
    let process = args["process"]
        .as_str()
        .unwrap_or("isothermal")
        .to_lowercase();
    let gamma = args["gamma"].as_f64().unwrap_or(1.4);
    let get = |k: &str| args[k].as_f64();

    match process.as_str() {
        "isothermal" => {
            let (n, t, v1, v2) = (
                get("n").ok_or("n required (mol)")?,
                get("T").ok_or("T required (K)")?,
                get("V1").ok_or("V1 required (m³)")?,
                get("V2").ok_or("V2 required (m³)")?,
            );
            let w = n * R * t * (v2 / v1).ln();
            Ok(format!(
                "Isothermal Process (T = {t} K constant)\n\nW = nRT·ln(V2/V1)\n  = {n} × {R:.4} × {t} × ln({v2}/{v1})\n  = {w:.4} J  ({:.4} kJ)\n\nHeat Q = W = {w:.4} J (ΔU = 0 for ideal gas)\nΔS = Q/T = {:.6} J/K",
                w / 1000.0, w / t
            ))
        }
        "isobaric" => {
            let (p, v1, v2, n_opt, cp_opt) = (
                get("P").ok_or("P required (Pa)")?,
                get("V1").ok_or("V1 required (m³)")?,
                get("V2").ok_or("V2 required (m³)")?,
                get("n"), get("Cp"),
            );
            let w = p * (v2 - v1);
            let q = if let (Some(n), Some(cp)) = (n_opt, cp_opt) {
                let t1 = p * v1 / (n * R);
                let t2 = p * v2 / (n * R);
                let q = n * cp * (t2 - t1);
                format!("\nHeat Q = n·Cp·ΔT = {n} × {cp} × {:.4} = {q:.4} J\nΔU = Q - W = {:.4} J", t2 - t1, q - w)
            } else {
                String::new()
            };
            Ok(format!(
                "Isobaric Process (P = {p} Pa constant)\n\nW = P·ΔV = {p} × ({v2} - {v1})\n  = {w:.4} J  ({:.4} kJ){q}",
                w / 1000.0
            ))
        }
        "isochoric" => {
            Ok("Isochoric Process (V constant)\n\nW = 0  (no volume change)\n\nAll energy transferred as heat: Q = ΔU = n·Cv·ΔT".into())
        }
        "adiabatic" => {
            let (p1, v1, p2, v2) = (
                get("P1").ok_or("P1 required (Pa)")?,
                get("V1").ok_or("V1 required (m³)")?,
                get("P2").ok_or("P2 required")?,
                get("V2").ok_or("V2 required")?,
            );
            let w = (p1 * v1 - p2 * v2) / (gamma - 1.0);
            Ok(format!(
                "Adiabatic Process (Q = 0, γ = {gamma})\n\nRelation: P·Vᵞ = const → P1·V1ᵞ = P2·V2ᵞ\nVerify: {p1}·{v1}^{gamma} vs {p2}·{v2}^{gamma}\n       = {:.4} vs {:.4}\n\nW = (P1·V1 - P2·V2) / (γ-1)\n  = ({p1}×{v1} - {p2}×{v2}) / {:.4}\n  = {w:.4} J  ({:.4} kJ)\n\nΔU = -W = {:.4} J  (Q=0)",
                p1 * v1.powf(gamma), p2 * v2.powf(gamma),
                gamma - 1.0, w / 1000.0, -w
            ))
        }
        _ => Err("process must be: isothermal, isobaric, isochoric, adiabatic".into()),
    }
}

fn action_entropy(args: &Value) -> Result<String, String> {
    let process = args["process"]
        .as_str()
        .unwrap_or("isothermal")
        .to_lowercase();
    let get = |k: &str| args[k].as_f64();

    match process.as_str() {
        "isothermal" => {
            let (n, t, v1, v2) = (
                get("n").ok_or("n required")?,
                get("T").ok_or("T required (K)")?,
                get("V1").ok_or("V1 required")?,
                get("V2").ok_or("V2 required")?,
            );
            let ds = n * R * (v2 / v1).ln();
            Ok(format!(
                "Entropy Change — Isothermal Expansion\n\nΔS = nR·ln(V2/V1)\n   = {n} × {R:.4} × ln({v2}/{v1})\n   = {ds:.6} J/K\n\nQ = T·ΔS = {t} × {ds:.6} = {:.4} J",
                t * ds
            ))
        }
        "isobaric" => {
            let (n, cp, t1, t2) = (
                get("n").ok_or("n required")?,
                get("Cp").ok_or("Cp required J/(mol·K)")?,
                get("T1").ok_or("T1 required (K)")?,
                get("T2").ok_or("T2 required (K)")?,
            );
            let ds = n * cp * (t2 / t1).ln();
            Ok(format!(
                "Entropy Change — Isobaric Process\n\nΔS = n·Cp·ln(T2/T1)\n   = {n} × {cp} × ln({t2}/{t1})\n   = {ds:.6} J/K",
            ))
        }
        "isochoric" => {
            let (n, cv, t1, t2) = (
                get("n").ok_or("n required")?,
                get("Cv").ok_or("Cv required J/(mol·K)")?,
                get("T1").ok_or("T1 required (K)")?,
                get("T2").ok_or("T2 required (K)")?,
            );
            let ds = n * cv * (t2 / t1).ln();
            Ok(format!(
                "Entropy Change — Isochoric Process\n\nΔS = n·Cv·ln(T2/T1)\n   = {n} × {cv} × ln({t2}/{t1})\n   = {ds:.6} J/K",
            ))
        }
        "mixing" => {
            let (n1, n2) = (
                get("n").ok_or("n required (mol of gas 1)")?,
                get("V").ok_or("V required (mol of gas 2)")?,
            );
            let n_total = n1 + n2;
            let x1 = n1 / n_total;
            let x2 = n2 / n_total;
            let ds = -n_total * R * (x1 * x1.ln() + x2 * x2.ln());
            Ok(format!(
                "Entropy of Mixing (2 ideal gases)\n\nn1 = {n1} mol, n2 = {n2} mol, n_total = {n_total}\nx1 = {x1:.4}, x2 = {x2:.4}\n\nΔS_mix = -n_total·R·Σ(xi·ln(xi))\n       = {ds:.6} J/K",
            ))
        }
        _ => Err("process must be: isothermal, isobaric, isochoric, mixing".into()),
    }
}

fn action_heat(args: &Value) -> Result<String, String> {
    let mode = args["mode"].as_str().unwrap_or("conduction").to_lowercase();
    let get = |k: &str| args[k].as_f64();

    match mode.as_str() {
        "conduction" => {
            let (k, a, dt, l) = (
                get("k").ok_or("k required: thermal conductivity W/(m·K)")?,
                get("A").ok_or("A required: area m²")?,
                get("dT").ok_or("dT required: temperature difference K")?,
                get("L").ok_or("L required: thickness m")?,
            );
            let q = k * a * dt / l;
            let r_val = l / (k * a);
            Ok(format!(
                "Fourier's Law of Heat Conduction\n\nq = k·A·ΔT / L\n  = {k} × {a} × {dt} / {l}\n  = {q:.4} W\n\nThermal resistance R = L/(k·A) = {r_val:.6} K/W\n\nMaterial context:\n  Copper:    k ≈ 401 W/(m·K)\n  Aluminum:  k ≈ 237 W/(m·K)\n  Steel:     k ≈ 50  W/(m·K)\n  Concrete:  k ≈ 1.7 W/(m·K)\n  Glass:     k ≈ 1.0 W/(m·K)\n  Air:       k ≈ 0.026 W/(m·K)"
            ))
        }
        "convection" => {
            let (h, a, dt) = (
                get("h").ok_or("h required: convection coefficient W/(m²·K)")?,
                get("A").ok_or("A required: area m²")?,
                get("dT").ok_or("dT required: temperature difference K")?,
            );
            let q = h * a * dt;
            Ok(format!(
                "Newton's Law of Cooling (Convection)\n\nq = h·A·ΔT\n  = {h} × {a} × {dt}\n  = {q:.4} W\n\nTypical h values:\n  Natural air conv.:  5–25 W/(m²·K)\n  Forced air:         25–250 W/(m²·K)\n  Forced water:       200–10000 W/(m²·K)\n  Boiling water:      2500–35000 W/(m²·K)"
            ))
        }
        "radiation" => {
            let (epsilon, a, ts, tsurr) = (
                get("epsilon").ok_or("epsilon required: emissivity 0-1")?,
                get("A").ok_or("A required: area m²")?,
                get("Ts").ok_or("Ts required: surface temperature K")?,
                get("Tsurr").ok_or("Tsurr required: surroundings temperature K")?,
            );
            let q = epsilon * SIGMA * a * (ts.powi(4) - tsurr.powi(4));
            Ok(format!(
                "Stefan-Boltzmann Radiation\n\nq = ε·σ·A·(Ts⁴ - T_surr⁴)\n  = {epsilon} × {SIGMA:.4e} × {a} × ({ts}⁴ - {tsurr}⁴)\n  = {q:.4} W\n\nσ = {SIGMA:.6e} W/(m²·K⁴)\n\nBlackbody (ε=1) at same T: {:.4} W",
                SIGMA * a * (ts.powi(4) - tsurr.powi(4))
            ))
        }
        _ => Err("mode must be: conduction, convection, radiation".into()),
    }
}

fn action_cycles(args: &Value) -> Result<String, String> {
    let cycle = args["cycle"].as_str().unwrap_or("carnot").to_lowercase();
    let gamma = args["gamma"].as_f64().unwrap_or(1.4);
    let get = |k: &str| args[k].as_f64();

    match cycle.as_str() {
        "carnot" => {
            let (th, tc) = (
                get("Th").ok_or("Th required: hot reservoir K")?,
                get("Tc").ok_or("Tc required: cold reservoir K")?,
            );
            if tc >= th {
                return Err("Tc must be < Th".into());
            }
            let eta = 1.0 - tc / th;
            let cop_heat = th / (th - tc);
            let cop_cool = tc / (th - tc);
            Ok(format!(
                "Carnot Cycle (maximum possible efficiency)\n\nTh = {th} K  ({:.2} °C)\nTc = {tc} K  ({:.2} °C)\n\nη_Carnot = 1 - Tc/Th = 1 - {tc}/{th}\n         = {eta:.6}  ({:.4}%)\n\nFor heat pump: COP_HP = Th/(Th-Tc) = {cop_heat:.4}\nFor refrigerator: COP_cool = Tc/(Th-Tc) = {cop_cool:.4}\n\nNo real engine can exceed Carnot efficiency.",
                th - 273.15, tc - 273.15, eta * 100.0
            ))
        }
        "otto" => {
            let r = get("r").ok_or("r required: compression ratio")?;
            let eta = 1.0 - 1.0 / r.powf(gamma - 1.0);
            Ok(format!(
                "Otto Cycle (gasoline engine)\n\nCompression ratio r = {r},  γ = {gamma}\n\nη_Otto = 1 - 1/r^(γ-1)\n       = 1 - 1/{r}^{:.4}\n       = 1 - {:.6}\n       = {eta:.6}  ({:.4}%)\n\nTypical r: 8–12 for gasoline engines\nTypical η: 25–35% (thermal; real engines lower due to friction)",
                gamma - 1.0, 1.0 / r.powf(gamma - 1.0), eta * 100.0
            ))
        }
        "diesel" => {
            let (r, rc) = (
                get("r").ok_or("r required: compression ratio")?,
                get("rc").ok_or("rc required: cutoff ratio")?,
            );
            let term = (rc.powf(gamma) - 1.0) / (gamma * (rc - 1.0));
            let eta = 1.0 - term / r.powf(gamma - 1.0);
            Ok(format!(
                "Diesel Cycle\n\nCompression ratio r = {r},  Cutoff ratio rc = {rc},  γ = {gamma}\n\nη_Diesel = 1 - [rc^γ - 1] / [γ·(rc-1)·r^(γ-1)]\n         = 1 - {term:.6} / {:.6}\n         = {eta:.6}  ({:.4}%)\n\nTypical r: 14–25 for diesel engines",
                r.powf(gamma - 1.0), eta * 100.0
            ))
        }
        "brayton" => {
            let rp = get("rp").ok_or("rp required: pressure ratio")?;
            let eta = 1.0 - 1.0 / rp.powf((gamma - 1.0) / gamma);
            Ok(format!(
                "Brayton Cycle (gas turbine / jet engine)\n\nPressure ratio rp = {rp},  γ = {gamma}\n\nη_Brayton = 1 - 1/rp^((γ-1)/γ)\n          = 1 - 1/{rp}^{:.4}\n          = {eta:.6}  ({:.4}%)\n\nTypical rp: 10–40 for gas turbines",
                (gamma - 1.0) / gamma, eta * 100.0
            ))
        }
        _ => Err("cycle must be: carnot, otto, diesel, brayton".into()),
    }
}

fn action_fluid(args: &Value) -> Result<String, String> {
    let mode = args["mode"].as_str().unwrap_or("reynolds").to_lowercase();
    let get = |k: &str| args[k].as_f64();

    match mode.as_str() {
        "reynolds" => {
            let (rho, v, d, mu) = (
                get("rho").ok_or("rho required: density kg/m³")?,
                get("v").ok_or("v required: velocity m/s")?,
                get("D").ok_or("D required: diameter m")?,
                get("mu").ok_or("mu required: dynamic viscosity Pa·s")?,
            );
            let re = rho * v * d / mu;
            let regime = if re < 2300.0 {
                "LAMINAR"
            } else if re < 4000.0 {
                "TRANSITIONAL"
            } else {
                "TURBULENT"
            };
            Ok(format!(
                "Reynolds Number\n\nRe = ρ·v·D / μ\n   = {rho} × {v} × {d} / {mu}\n   = {re:.2}\n\nFlow regime: {regime}\n  Re < 2300:  Laminar\n  2300–4000:  Transitional\n  Re > 4000:  Turbulent\n\nCommon μ values:\n  Water (20°C): 1.002×10⁻³ Pa·s\n  Air (20°C):   1.81×10⁻⁵ Pa·s\n  Engine oil:   0.1–0.3 Pa·s"
            ))
        }
        "bernoulli" => {
            let (rho, v1, v2, p1, z1, z2) = (
                get("rho").ok_or("rho required: density kg/m³")?,
                get("v1").ok_or("v1 required: velocity at point 1 m/s")?,
                get("v2").ok_or("v2 required: velocity at point 2 m/s")?,
                get("p1").ok_or("p1 required: pressure at point 1 Pa")?,
                get("z1").unwrap_or(0.0),
                get("z2").unwrap_or(0.0),
            );
            let p2 = p1 + 0.5 * rho * (v1 * v1 - v2 * v2) + rho * G * (z1 - z2);
            let bernoulli_const = p1 + 0.5 * rho * v1 * v1 + rho * G * z1;
            Ok(format!(
                "Bernoulli's Equation (incompressible, inviscid flow)\n\nP + ½ρv² + ρgz = const = {bernoulli_const:.4} Pa\n\nPoint 1: P1={p1} Pa, v1={v1} m/s, z1={z1} m\nPoint 2: v2={v2} m/s, z2={z2} m\n\nP2 = P1 + ½ρ(v1²-v2²) + ρg(z1-z2)\n   = {p2:.4} Pa  ({:.4} kPa)\n\nDynamic pressure ½ρv²: {:.4} Pa at point 1",
                p2 / 1e3, 0.5 * rho * v1 * v1
            ))
        }
        "poiseuille" => {
            let (r_pipe, l, dp, mu) = (
                get("D").ok_or("D required: pipe inner diameter m")? / 2.0,
                get("L").ok_or("L required: pipe length m")?,
                get("P1")
                    .and_then(|p1| args["P2"].as_f64().map(|p2| p1 - p2))
                    .or_else(|| get("dT"))
                    .ok_or("P1+P2 or dT (pressure drop) required")?,
                get("mu").ok_or("mu required: dynamic viscosity Pa·s")?,
            );
            let q = std::f64::consts::PI * r_pipe.powi(4) * dp / (8.0 * mu * l);
            let v_avg = q / (std::f64::consts::PI * r_pipe * r_pipe);
            let v_max = 2.0 * v_avg;
            Ok(format!(
                "Hagen-Poiseuille Flow (laminar, fully developed)\n\nPipe diameter D = {:.4} m (r = {r_pipe:.6} m)\nLength L = {l} m, ΔP = {dp} Pa, μ = {mu} Pa·s\n\nQ = π·r⁴·ΔP / (8μL)\n  = π × {r_pipe:.6}⁴ × {dp} / (8 × {mu} × {l})\n  = {q:.6} m³/s  ({:.4} L/s)\n\nAverage velocity: {v_avg:.6} m/s\nMax velocity (centerline): {v_max:.6} m/s\n\nValid for: Re < 2300 (laminar flow)",
                r_pipe * 2.0, q * 1000.0
            ))
        }
        "continuity" => {
            let (a1, v1, a2) = (
                get("A1").ok_or("A1 required: area at point 1 m²")?,
                get("v1").ok_or("v1 required: velocity at point 1 m/s")?,
                get("A2").ok_or("A2 required: area at point 2 m²")?,
            );
            let q = a1 * v1;
            let v2 = q / a2;
            Ok(format!(
                "Continuity Equation (incompressible flow)\n\nA1·v1 = A2·v2 = Q (constant)\n\nA1 = {a1} m², v1 = {v1} m/s\nQ = A1·v1 = {q:.6} m³/s\n\nA2 = {a2} m²\nv2 = Q/A2 = {v2:.6} m/s\n\nVelocity ratio: v2/v1 = {:.4}",
                v2 / v1
            ))
        }
        _ => Err("mode must be: reynolds, bernoulli, poiseuille, continuity".into()),
    }
}

fn action_properties(args: &Value) -> Result<String, String> {
    let substance = args["substance"]
        .as_str()
        .unwrap_or(args["name"].as_str().unwrap_or(""))
        .to_lowercase();

    struct GasProp {
        name: &'static str,
        aliases: &'static [&'static str],
        molar_mass: f64,
        cp: f64,
        cv: f64,
        gamma: f64,
        mu_20c: f64,
        k_20c: f64,
        desc: &'static str,
    }

    let props = [
        GasProp {
            name: "Air",
            aliases: &["air"],
            molar_mass: 28.97,
            cp: 29.10,
            cv: 20.79,
            gamma: 1.400,
            mu_20c: 1.81e-5,
            k_20c: 0.02563,
            desc: "Dry air at 1 atm",
        },
        GasProp {
            name: "Nitrogen (N₂)",
            aliases: &["nitrogen", "n2"],
            molar_mass: 28.014,
            cp: 29.12,
            cv: 20.80,
            gamma: 1.400,
            mu_20c: 1.76e-5,
            k_20c: 0.02583,
            desc: "Diatomic",
        },
        GasProp {
            name: "Oxygen (O₂)",
            aliases: &["oxygen", "o2"],
            molar_mass: 31.998,
            cp: 29.38,
            cv: 21.07,
            gamma: 1.395,
            mu_20c: 2.04e-5,
            k_20c: 0.02658,
            desc: "Diatomic",
        },
        GasProp {
            name: "Carbon Dioxide (CO₂)",
            aliases: &["co2", "carbon dioxide"],
            molar_mass: 44.010,
            cp: 37.11,
            cv: 28.82,
            gamma: 1.289,
            mu_20c: 1.47e-5,
            k_20c: 0.01662,
            desc: "Triatomic linear",
        },
        GasProp {
            name: "Hydrogen (H₂)",
            aliases: &["hydrogen", "h2"],
            molar_mass: 2.016,
            cp: 28.82,
            cv: 20.44,
            gamma: 1.405,
            mu_20c: 8.89e-6,
            k_20c: 0.18720,
            desc: "Lightest gas",
        },
        GasProp {
            name: "Helium (He)",
            aliases: &["helium", "he"],
            molar_mass: 4.003,
            cp: 20.79,
            cv: 12.47,
            gamma: 1.667,
            mu_20c: 1.96e-5,
            k_20c: 0.15230,
            desc: "Monatomic noble gas",
        },
        GasProp {
            name: "Argon (Ar)",
            aliases: &["argon", "ar"],
            molar_mass: 39.948,
            cp: 20.79,
            cv: 12.47,
            gamma: 1.667,
            mu_20c: 2.23e-5,
            k_20c: 0.01772,
            desc: "Monatomic noble gas",
        },
        GasProp {
            name: "Methane (CH₄)",
            aliases: &["methane", "ch4"],
            molar_mass: 16.043,
            cp: 35.69,
            cv: 27.35,
            gamma: 1.305,
            mu_20c: 1.11e-5,
            k_20c: 0.03408,
            desc: "Natural gas main component",
        },
        GasProp {
            name: "Steam (H₂O)",
            aliases: &["steam", "water vapor", "water_vapor"],
            molar_mass: 18.015,
            cp: 33.59,
            cv: 25.28,
            gamma: 1.329,
            mu_20c: 9.73e-6,
            k_20c: 0.01810,
            desc: "At 100°C, 1 atm",
        },
    ];

    let mat = props
        .iter()
        .find(|p| p.aliases.iter().any(|a| substance.contains(a)));

    if substance.is_empty() || substance == "list" {
        let list = props
            .iter()
            .map(|p| format!("  {:30} (alias: {})", p.name, p.aliases[0]))
            .collect::<Vec<_>>()
            .join("\n");
        return Ok(format!("Available substance properties:\n\n{list}\n\nPass substance='air', 'co2', 'helium', etc."));
    }

    match mat {
        None => Err(format!("Unknown substance '{}'. Try: air, nitrogen, oxygen, co2, hydrogen, helium, argon, methane, steam", substance)),
        Some(p) => Ok(format!(
            "Thermodynamic Properties: {}\n{}\n\nMolar mass M:     {:.4} g/mol\nCp (molar):       {:.4} J/(mol·K)  ({:.4} J/(kg·K))\nCv (molar):       {:.4} J/(mol·K)  ({:.4} J/(kg·K))\nγ = Cp/Cv:        {:.4}\nμ at 20°C:        {:.4e} Pa·s\nk at 20°C:        {:.6} W/(m·K)\nR (specific):     {:.4} J/(kg·K)\n\nNote: Cp and Cv are per mol; divide by M×10⁻³ for per kg.",
            p.name, p.desc, p.molar_mass,
            p.cp, p.cp / (p.molar_mass * 1e-3),
            p.cv, p.cv / (p.molar_mass * 1e-3),
            p.gamma, p.mu_20c, p.k_20c,
            R / (p.molar_mass * 1e-3)
        )),
    }
}

fn action_psychro(args: &Value) -> Result<String, String> {
    let get = |k: &str| args[k].as_f64();
    let p_atm = get("P_atm").unwrap_or(101325.0);

    // Antoine equation for saturation pressure (water, 0-100°C)
    // log10(P_sat/Pa) = A - B/(C+T) where T in °C, P in Pa
    // Using NIST constants: A=8.07131, B=1730.63, C=233.426 (valid 1–100°C)
    let p_sat = |t_c: f64| -> f64 {
        10_f64.powf(8.07131 - 1730.63 / (233.426 + t_c)) * 133.322 // mmHg → Pa
    };

    if let (Some(t_dry), Some(t_wet)) = (get("T_dry"), get("T_wet")) {
        let p_sat_wet = p_sat(t_wet);
        // Sprung's formula approximation
        let p_v = p_sat_wet - 0.000799 * p_atm * (t_dry - t_wet);
        let p_sat_dry = p_sat(t_dry);
        let rh = (p_v / p_sat_dry * 100.0).max(0.0).min(100.0);
        let w = 0.622 * p_v / (p_atm - p_v);
        // Dew point via Magnus formula
        let ln_term = (rh / 100.0).ln();
        let t_dew = 243.04 * (17.625 * t_dry / (243.04 + t_dry) + ln_term)
            / (17.625 - (17.625 * t_dry / (243.04 + t_dry) + ln_term));
        return Ok(format!(
            "Psychrometric Analysis\n\nDry-bulb temperature:  {t_dry:.2} °C\nWet-bulb temperature:  {t_wet:.2} °C\nAtmospheric pressure:  {:.2} kPa\n\nResults:\n  Relative humidity (RH):   {rh:.1}%\n  Vapor pressure (Pv):      {:.2} Pa\n  Saturation pressure:      {:.2} Pa\n  Humidity ratio (w):       {:.6} kg water/kg dry air\n  Dew point:                {t_dew:.2} °C\n\nHumidity ratio w = 0.622·Pv / (P - Pv)",
            p_atm / 1e3, p_v, p_sat_dry, w
        ));
    }

    if let (Some(t_dry), Some(rh)) = (get("T_dry").or(get("T")), get("RH")) {
        let rh = rh / 100.0;
        let p_sat_dry = p_sat(t_dry);
        let p_v = rh * p_sat_dry;
        let w = 0.622 * p_v / (p_atm - p_v);
        let ln_term = rh.ln();
        let t_dew = 243.04 * (17.625 * t_dry / (243.04 + t_dry) + ln_term)
            / (17.625 - (17.625 * t_dry / (243.04 + t_dry) + ln_term));
        return Ok(format!(
            "Psychrometric Analysis\n\nDry-bulb temperature:  {t_dry:.2} °C\nRelative humidity:     {:.1}%\nAtmospheric pressure:  {:.2} kPa\n\nResults:\n  Saturation pressure:      {:.2} Pa  (Antoine eq.)\n  Vapor pressure (Pv):      {:.2} Pa\n  Humidity ratio (w):       {:.6} kg water/kg dry air\n  Dew point:                {t_dew:.2} °C\n  Wet-bulb ≈:               {:.2} °C  (approx.)",
            rh * 100.0, p_atm / 1e3, p_sat_dry, p_v, w,
            t_dry - (1.0 - rh) * (t_dry + 112.0) / 30.0
        ));
    }

    Err("Provide T_dry + T_wet, or T_dry (or T) + RH".into())
}

pub async fn execute(args: &Value) -> Result<String, String> {
    let action = args["action"].as_str().unwrap_or("ideal_gas");
    match action {
        "ideal_gas"  => action_ideal_gas(args),
        "work"       => action_work(args),
        "entropy"    => action_entropy(args),
        "heat"       => action_heat(args),
        "cycles"     => action_cycles(args),
        "fluid"      => action_fluid(args),
        "properties" => action_properties(args),
        "psychro"    => action_psychro(args),
        _ => Err(format!("Unknown action '{action}'. Use: ideal_gas, work, entropy, heat, cycles, fluid, properties, psychro")),
    }
}
