use serde_json::{json, Value};
use std::f64::consts::PI;

fn n(v: &Value, key: &str) -> Option<f64> {
    v.get(key).and_then(|x| x.as_f64())
}

fn require(v: &Value, key: &str) -> Result<f64, String> {
    n(v, key).ok_or_else(|| format!("missing required argument '{key}'"))
}

fn arr_f64(v: &Value, key: &str) -> Option<Vec<f64>> {
    v.get(key)?
        .as_array()
        .map(|arr| arr.iter().filter_map(|x| x.as_f64()).collect())
}

pub fn circuit_tools_schema() -> Value {
    let num = |desc: &str| json!({"type": "number", "description": desc});
    let str_prop = |desc: &str| json!({"type": "string", "description": desc});
    let mut props = serde_json::Map::new();
    props.insert("action".into(), json!({"type":"string","description":"ohm|resistors|power|capacitors|inductors|divider|rlc|ac","enum":["ohm","resistors","power","capacitors","inductors","divider","rlc","ac"]}));
    props.insert(
        "solve_for".into(),
        str_prop("Variable to solve for (depends on action)"),
    );
    props.insert("V".into(), num("Voltage (V)"));
    props.insert("I".into(), num("Current (A)"));
    props.insert("R".into(), num("Resistance (Ω)"));
    props.insert("R1".into(), num("Resistance 1 (Ω)"));
    props.insert("R2".into(), num("Resistance 2 (Ω)"));
    props.insert("Vin".into(), num("Input voltage (V)"));
    props.insert("P".into(), num("Power (W)"));
    props.insert("C".into(), num("Capacitance (F)"));
    props.insert("L".into(), num("Inductance (H)"));
    props.insert("f".into(), num("Frequency (Hz)"));
    props.insert("t".into(), num("Time (s)"));
    props.insert("Q".into(), num("Charge (C) or Q-factor"));
    props.insert("mode".into(), str_prop("series|parallel"));
    props.insert("values".into(), json!({"type":"array","items":{"type":"number"},"description":"Array of resistance/capacitance/inductance values"}));
    json!({"name":"circuit_tools","description":"Electrical circuit calculations: Ohm's law, series/parallel resistors, power, capacitors/inductors (series, parallel, RC/RL time constants), voltage/current dividers, RLC resonance, and AC impedance. No external tools required.","input_schema":{"type":"object","properties":Value::Object(props),"required":["action"]}})
}

fn action_ohm(args: &Value) -> Result<String, String> {
    let solve = args
        .get("solve_for")
        .and_then(|x| x.as_str())
        .unwrap_or("V");
    let mut out = String::from("=== OHM'S LAW (V = IR) ===\n");

    match solve {
        "V" => {
            let i = require(args, "I")?;
            let r = require(args, "R")?;
            let v = i * r;
            out.push_str(&format!("V = IR = {i} × {r} = {v:.4} V\n"));
        }
        "I" => {
            let v = require(args, "V")?;
            let r = require(args, "R")?;
            let i = v / r;
            out.push_str(&format!(
                "I = V/R = {v}/{r} = {i:.4} A  ({:.4} mA)\n",
                i * 1000.0
            ));
        }
        "R" => {
            let v = require(args, "V")?;
            let i = require(args, "I")?;
            let r = v / i;
            out.push_str(&format!("R = V/I = {v}/{i} = {r:.4} Ω\n"));
        }
        _ => return Err(format!("unknown solve_for '{solve}'; use: V|I|R")),
    }

    Ok(out)
}

fn resistance_parallel(values: &[f64]) -> f64 {
    let sum_inv: f64 = values.iter().map(|&r| 1.0 / r).sum();
    1.0 / sum_inv
}

fn action_resistors(args: &Value) -> Result<String, String> {
    let mode = args
        .get("mode")
        .and_then(|x| x.as_str())
        .unwrap_or("series");
    let values = arr_f64(args, "values");
    let (r1, r2) = (n(args, "R1"), n(args, "R2"));

    let vals: Vec<f64> = if let Some(v) = values {
        v
    } else if let (Some(r1), Some(r2)) = (r1, r2) {
        vec![r1, r2]
    } else {
        return Err("provide 'values' array or 'R1'/'R2'".into());
    };

    if vals.is_empty() {
        return Err("at least one resistance value required".into());
    }

    let mut out = String::from("=== RESISTORS ===\n");
    let list: Vec<String> = vals.iter().map(|r| format!("{r} Ω")).collect();
    out.push_str(&format!("Values: {}\n", list.join(", ")));

    match mode {
        "series" => {
            let total: f64 = vals.iter().sum();
            out.push_str(&format!(
                "Mode: Series\nR_total = R1 + R2 + ... = {total:.4} Ω\n"
            ));
        }
        "parallel" => {
            let total = resistance_parallel(&vals);
            out.push_str(&format!(
                "Mode: Parallel\nR_total = 1/(1/R1 + 1/R2 + ...) = {total:.4} Ω\n"
            ));
            if vals.len() == 2 {
                let product_over_sum = vals[0] * vals[1] / (vals[0] + vals[1]);
                out.push_str(&format!(
                    "Two-resistor formula: R1×R2/(R1+R2) = {product_over_sum:.4} Ω\n"
                ));
            }
        }
        _ => return Err(format!("unknown mode '{mode}'; use: series|parallel")),
    }

    Ok(out)
}

fn action_power(args: &Value) -> Result<String, String> {
    let solve = args
        .get("solve_for")
        .and_then(|x| x.as_str())
        .unwrap_or("P");
    let mut out = String::from("=== ELECTRICAL POWER ===\n");
    let t = n(args, "t");

    match solve {
        "P_VI" => {
            let v = require(args, "V")?;
            let i = require(args, "I")?;
            let p = v * i;
            out.push_str(&format!("P = VI = {v} × {i} = {p:.4} W\n"));
            if let Some(t) = t {
                out.push_str(&format!(
                    "Energy E = Pt = {p:.4} × {t} = {:.4} J  ({:.4} Wh)\n",
                    p * t,
                    p * t / 3600.0
                ));
            }
        }
        "P_IR" | "P" => {
            let i = require(args, "I")?;
            let r = require(args, "R")?;
            let p = i * i * r;
            let v = i * r;
            out.push_str(&format!(
                "P = I²R = {i}² × {r} = {p:.4} W  (V = {v:.4} V)\n"
            ));
            if let Some(t) = t {
                out.push_str(&format!("Energy E = Pt = {:.4} J\n", p * t));
            }
        }
        "P_VR" => {
            let v = require(args, "V")?;
            let r = require(args, "R")?;
            let p = v * v / r;
            let i = v / r;
            out.push_str(&format!("P = V²/R = {v}²/{r} = {p:.4} W  (I = {i:.4} A)\n"));
            if let Some(t) = t {
                out.push_str(&format!("Energy E = Pt = {:.4} J\n", p * t));
            }
        }
        "efficiency" => {
            let p_out = require(args, "P")?;
            let v = require(args, "V")?;
            let i = require(args, "I")?;
            let p_in = v * i;
            let eta = p_out / p_in * 100.0;
            out.push_str(&format!(
                "Efficiency η = P_out/P_in\nP_in = VI = {v}×{i} = {p_in:.4} W\nP_out = {p_out} W\nη = {eta:.2}%\nLosses = {:.4} W\n",
                p_in - p_out
            ));
        }
        _ => {
            return Err(format!(
                "unknown solve_for '{solve}'; use: P_VI|P_IR|P_VR|efficiency"
            ))
        }
    }

    Ok(out)
}

fn action_capacitors(args: &Value) -> Result<String, String> {
    let solve = args
        .get("solve_for")
        .and_then(|x| x.as_str())
        .unwrap_or("energy");
    let mut out = String::from("=== CAPACITORS ===\n");

    match solve {
        "series" | "parallel" => {
            let vals = arr_f64(args, "values");
            let (c1, c2) = (n(args, "R1").or_else(|| n(args, "C")), n(args, "R2"));
            let vals: Vec<f64> = if let Some(v) = vals {
                v
            } else if let (Some(c1), Some(c2)) = (c1, c2) {
                vec![c1, c2]
            } else {
                return Err("provide 'values' array".into());
            };

            if solve == "series" {
                let ct = 1.0 / vals.iter().map(|c| 1.0 / c).sum::<f64>();
                out.push_str(&format!(
                    "Series: C_total = 1/(1/C1+1/C2+...) = {:.6e} F  ({:.6e} μF)\n",
                    ct,
                    ct * 1e6
                ));
            } else {
                let ct: f64 = vals.iter().sum();
                out.push_str(&format!(
                    "Parallel: C_total = C1+C2+... = {:.6e} F  ({:.6e} μF)\n",
                    ct,
                    ct * 1e6
                ));
            }
        }
        "energy" => {
            let c = require(args, "C")?;
            let v = require(args, "V")?;
            let e = 0.5 * c * v * v;
            let q = c * v;
            out.push_str(&format!(
                "C = {c:.6e} F  |  V = {v} V\nCharge Q = CV = {q:.6e} C\nEnergy E = ½CV² = {e:.6e} J\n"
            ));
        }
        "rc" => {
            let r = require(args, "R")?;
            let c = require(args, "C")?;
            let tau = r * c;
            let v = n(args, "V");
            let t = n(args, "t");
            out.push_str(&format!(
                "RC Time Constant τ = RC = {r} × {c:.6e} = {tau:.6e} s\n5τ ≈ full charge: {:.6e} s\n",
                5.0 * tau
            ));
            if let (Some(v0), Some(t)) = (v, t) {
                let v_charge = v0 * (1.0 - (-t / tau).exp());
                let v_discharge = v0 * (-t / tau).exp();
                out.push_str(&format!(
                    "\nAt t={t} s (V₀={v0} V):\nCharging: V = V₀(1-e^(-t/τ)) = {v_charge:.4} V\nDischarging: V = V₀·e^(-t/τ) = {v_discharge:.4} V\n"
                ));
            }
        }
        _ => {
            return Err(format!(
                "unknown solve_for '{solve}'; use: series|parallel|energy|rc"
            ))
        }
    }

    Ok(out)
}

fn action_inductors(args: &Value) -> Result<String, String> {
    let solve = args
        .get("solve_for")
        .and_then(|x| x.as_str())
        .unwrap_or("energy");
    let mut out = String::from("=== INDUCTORS ===\n");

    match solve {
        "series" | "parallel" => {
            let vals = arr_f64(args, "values");
            let (l1, l2) = (n(args, "R1").or_else(|| n(args, "L")), n(args, "R2"));
            let vals: Vec<f64> = if let Some(v) = vals {
                v
            } else if let (Some(l1), Some(l2)) = (l1, l2) {
                vec![l1, l2]
            } else {
                return Err("provide 'values' array".into());
            };

            if solve == "series" {
                let lt: f64 = vals.iter().sum();
                out.push_str(&format!(
                    "Series: L_total = L1+L2+... = {lt:.6e} H  (no mutual inductance)\n"
                ));
            } else {
                let lt = 1.0 / vals.iter().map(|l| 1.0 / l).sum::<f64>();
                out.push_str(&format!(
                    "Parallel: L_total = 1/(1/L1+1/L2+...) = {lt:.6e} H\n"
                ));
            }
        }
        "energy" => {
            let l = require(args, "L")?;
            let i = require(args, "I")?;
            let e = 0.5 * l * i * i;
            out.push_str(&format!(
                "L = {l:.6e} H  |  I = {i} A\nEnergy E = ½LI² = {e:.6e} J\n"
            ));
        }
        "rl" => {
            let r = require(args, "R")?;
            let l = require(args, "L")?;
            let tau = l / r;
            let v = n(args, "V");
            let t = n(args, "t");
            out.push_str(&format!(
                "RL Time Constant τ = L/R = {l:.6e}/{r} = {tau:.6e} s\n5τ ≈ full current: {:.6e} s\n",
                5.0 * tau
            ));
            if let (Some(v0), Some(t)) = (v, t) {
                let i_max = v0 / r;
                let i_t = i_max * (1.0 - (-t / tau).exp());
                out.push_str(&format!(
                    "\nAt t={t} s (V={v0} V):\nI_max = V/R = {i_max:.4} A\nI(t) = I_max(1-e^(-t/τ)) = {i_t:.4} A\nVoltage across L: VL = V·e^(-t/τ) = {:.4} V\n",
                    v0 * (-t / tau).exp()
                ));
            }
        }
        "voltage" => {
            let l = require(args, "L")?;
            let di_dt = require(args, "I")?;
            let vl = l * di_dt;
            out.push_str(&format!(
                "Inductor voltage: VL = L·(dI/dt)\nVL = {l:.6e} × {di_dt} = {vl:.4} V\n(I here is dI/dt in A/s)\n"
            ));
        }
        _ => {
            return Err(format!(
                "unknown solve_for '{solve}'; use: series|parallel|energy|rl|voltage"
            ))
        }
    }

    Ok(out)
}

fn action_divider(args: &Value) -> Result<String, String> {
    let solve = args
        .get("solve_for")
        .and_then(|x| x.as_str())
        .unwrap_or("voltage");
    let mut out = String::from("=== VOLTAGE / CURRENT DIVIDER ===\n");

    match solve {
        "voltage" => {
            let vin = require(args, "Vin")?;
            let r1 = require(args, "R1")?;
            let r2 = require(args, "R2")?;
            let vout = vin * r2 / (r1 + r2);
            let i = vin / (r1 + r2);
            out.push_str(&format!(
                "Voltage Divider\nVin={vin} V  R1={r1} Ω  R2={r2} Ω\nVout = Vin × R2/(R1+R2) = {vout:.4} V\nDivision ratio: {:.4}\nCurrent I = Vin/(R1+R2) = {i:.4} A\n",
                vout / vin
            ));
        }
        "current" => {
            let i_total = require(args, "I")?;
            let r1 = require(args, "R1")?;
            let r2 = require(args, "R2")?;
            let i1 = i_total * r2 / (r1 + r2);
            let i2 = i_total * r1 / (r1 + r2);
            out.push_str(&format!(
                "Current Divider\nI_total={i_total} A  R1={r1} Ω  R2={r2} Ω\nI1 = I × R2/(R1+R2) = {i1:.4} A\nI2 = I × R1/(R1+R2) = {i2:.4} A\nVerification: I1+I2 = {:.4} A\n",
                i1 + i2
            ));
        }
        _ => return Err(format!("unknown solve_for '{solve}'; use: voltage|current")),
    }

    Ok(out)
}

fn action_rlc(args: &Value) -> Result<String, String> {
    let r = require(args, "R")?;
    let l = require(args, "L")?;
    let c = require(args, "C")?;

    let f0 = 1.0 / (2.0 * PI * (l * c).sqrt());
    let omega0 = 2.0 * PI * f0;
    let q_factor = (1.0 / r) * (l / c).sqrt();
    let bandwidth = f0 / q_factor;
    let zeta = r / (2.0 * (l / c).sqrt());

    let response_type = if zeta < 1.0 {
        "Underdamped (oscillatory)"
    } else if (zeta - 1.0).abs() < 1e-9 {
        "Critically damped"
    } else {
        "Overdamped"
    };

    let mut out = format!(
        "=== RLC CIRCUIT ===\nR={r} Ω  L={l:.6e} H  C={c:.6e} F\n\nResonant frequency: f₀ = 1/(2π√(LC)) = {f0:.4} Hz\nAngular frequency: ω₀ = 1/√(LC) = {omega0:.4} rad/s\nQ-factor: Q = (1/R)√(L/C) = {q_factor:.4}\nBandwidth: BW = f₀/Q = {bandwidth:.4} Hz\nDamping ratio: ζ = R/(2√(L/C)) = {zeta:.4}\nResponse type: {response_type}\n"
    );

    if let Some(f) = n(args, "f") {
        let omega = 2.0 * PI * f;
        let xl = omega * l;
        let xc = 1.0 / (omega * c);
        let z = (r * r + (xl - xc) * (xl - xc)).sqrt();
        let phase = (xl - xc).atan2(r).to_degrees();
        out.push_str(&format!(
            "\nAt f={f} Hz:\n  XL = ωL = {xl:.4} Ω\n  XC = 1/(ωC) = {xc:.4} Ω\n  Z = √(R²+(XL-XC)²) = {z:.4} Ω\n  Phase angle = {phase:.2}°\n"
        ));
    }

    Ok(out)
}

fn action_ac(args: &Value) -> Result<String, String> {
    let r = n(args, "R").unwrap_or(0.0);
    let l = n(args, "L").unwrap_or(0.0);
    let c = n(args, "C");
    let f = require(args, "f")?;
    let omega = 2.0 * PI * f;

    let xl = omega * l;
    let xc = c.map(|cv| 1.0 / (omega * cv)).unwrap_or(0.0);
    let z = (r * r + (xl - xc) * (xl - xc)).sqrt();
    let phase = (xl - xc).atan2(r).to_degrees();
    let pf = phase.to_radians().cos();

    let mut out = format!(
        "=== AC CIRCUIT IMPEDANCE ===\nR={r} Ω  |  XL={xl:.4} Ω  |  XC={xc:.4} Ω\nf={f} Hz  |  ω={omega:.4} rad/s\n\nImpedance Z = √(R²+(XL-XC)²) = {z:.4} Ω\nPhase angle φ = {phase:.2}°  ({} — current {} voltage)\nPower factor = cos(φ) = {pf:.4}\n",
        if phase > 0.0 { "inductive" } else if phase < 0.0 { "capacitive" } else { "resistive" },
        if phase > 0.0 { "lags" } else if phase < 0.0 { "leads" } else { "in phase with" }
    );

    if let Some(v) = n(args, "V") {
        let i = v / z;
        let p_real = v * i * pf;
        let p_apparent = v * i;
        let p_reactive = v * i * phase.to_radians().sin().abs();
        out.push_str(&format!(
            "\nWith V={v} V:\n  I = V/Z = {i:.4} A\n  Real power P = {p_real:.4} W\n  Apparent power S = {p_apparent:.4} VA\n  Reactive power Q = {p_reactive:.4} VAR\n"
        ));
    }

    Ok(out)
}

pub async fn execute(args: &Value) -> Result<String, String> {
    let action = args.get("action").and_then(|x| x.as_str()).unwrap_or("ohm");
    match action {
        "ohm" => action_ohm(args),
        "resistors" => action_resistors(args),
        "power" => action_power(args),
        "capacitors" => action_capacitors(args),
        "inductors" => action_inductors(args),
        "divider" => action_divider(args),
        "rlc" => action_rlc(args),
        "ac" => action_ac(args),
        _ => Err(format!("unknown action '{action}'; use: ohm|resistors|power|capacitors|inductors|divider|rlc|ac")),
    }
}
