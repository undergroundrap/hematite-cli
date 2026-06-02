use serde_json::{json, Value};
use std::f64::consts::PI;

const G: f64 = 9.80665;

fn n(v: &Value, key: &str) -> Option<f64> {
    v.get(key).and_then(|x| x.as_f64())
}

fn require(v: &Value, key: &str) -> Result<f64, String> {
    n(v, key).ok_or_else(|| format!("missing required argument '{key}'"))
}

pub fn mechanics_tools_schema() -> Value {
    let num = |desc: &str| json!({"type": "number", "description": desc});
    let str_prop = |desc: &str| json!({"type": "string", "description": desc});
    let mut props = serde_json::Map::new();
    props.insert("action".into(), json!({"type":"string","description":"kinematics|forces|energy|momentum|rotation|oscillation|projectile|circular","enum":["kinematics","forces","energy","momentum","rotation","oscillation","projectile","circular"]}));
    props.insert(
        "solve_for".into(),
        str_prop("Variable to solve for (depends on action)"),
    );
    props.insert("u".into(), num("Initial velocity (m/s)"));
    props.insert("v".into(), num("Final velocity (m/s)"));
    props.insert("a".into(), num("Acceleration (m/s²)"));
    props.insert("s".into(), num("Displacement (m)"));
    props.insert("t".into(), num("Time (s)"));
    props.insert("m".into(), num("Mass (kg)"));
    props.insert("m2".into(), num("Second mass (kg)"));
    props.insert("F".into(), num("Force (N)"));
    props.insert("mu".into(), num("Coefficient of friction"));
    props.insert("theta".into(), num("Angle (degrees)"));
    props.insert("h".into(), num("Height (m)"));
    props.insert("k".into(), num("Spring constant (N/m)"));
    props.insert("x".into(), num("Displacement from equilibrium (m)"));
    props.insert("v1".into(), num("Velocity 1 (m/s)"));
    props.insert("v2".into(), num("Velocity 2 (m/s)"));
    props.insert(
        "collision".into(),
        str_prop("elastic|inelastic|perfectly_inelastic"),
    );
    props.insert("I".into(), num("Moment of inertia (kg·m²)"));
    props.insert("omega".into(), num("Angular velocity (rad/s)"));
    props.insert("omega2".into(), num("Final angular velocity (rad/s)"));
    props.insert("alpha".into(), num("Angular acceleration (rad/s²)"));
    props.insert("r".into(), num("Radius (m)"));
    props.insert(
        "shape".into(),
        str_prop("solid_sphere|hollow_sphere|solid_cylinder|ring|rod_center|rod_end|disk"),
    );
    props.insert("L".into(), num("Length (m) for pendulum or rod"));
    props.insert("v0".into(), num("Initial speed (m/s) for projectile"));
    props.insert("P".into(), num("Power (W)"));
    props.insert(
        "g".into(),
        num("Gravitational acceleration (m/s², default 9.80665)"),
    );
    json!({"name":"mechanics_tools","description":"Classical mechanics calculations: kinematics (SUVAT), forces, energy/power, momentum/collisions, rotation, oscillation, projectile motion, and circular motion. Solves for any missing variable.","input_schema":{"type":"object","properties":Value::Object(props),"required":["action"]}})
}

fn action_kinematics(args: &Value) -> Result<String, String> {
    let solve = args
        .get("solve_for")
        .and_then(|x| x.as_str())
        .unwrap_or("v");
    let u = n(args, "u");
    let v = n(args, "v");
    let a = n(args, "a");
    let _s = n(args, "s");
    let t = n(args, "t");

    let result = match solve {
        "v" => {
            let u = u.ok_or("missing 'u'")?;
            let a = a.ok_or("missing 'a'")?;
            let t = t.ok_or("missing 't'")?;
            let v = u + a * t;
            format!("v = u + at\nv = {u} + {a} × {t} = {v:.4} m/s")
        }
        "s" if u.is_some() && a.is_some() && t.is_some() => {
            let (u, a, t) = (u.unwrap(), a.unwrap(), t.unwrap());
            let s = u * t + 0.5 * a * t * t;
            format!("s = ut + ½at²\ns = {u}×{t} + 0.5×{a}×{t}² = {s:.4} m")
        }
        "s" if u.is_some() && v.is_some() && a.is_some() => {
            let (u, v, a) = (u.unwrap(), v.unwrap(), a.unwrap());
            let s = (v * v - u * u) / (2.0 * a);
            format!("v² = u² + 2as  →  s = (v²-u²)/(2a)\ns = ({v}²-{u}²)/(2×{a}) = {s:.4} m")
        }
        "a" => {
            let u = u.ok_or("missing 'u'")?;
            let v = v.ok_or("missing 'v'")?;
            let t = t.ok_or("missing 't'")?;
            let a = (v - u) / t;
            format!("a = (v-u)/t\na = ({v}-{u})/{t} = {a:.4} m/s²")
        }
        "t" => {
            let u = u.ok_or("missing 'u'")?;
            let v = v.ok_or("missing 'v'")?;
            let a = a.ok_or("missing 'a'")?;
            let t = (v - u) / a;
            format!("t = (v-u)/a\nt = ({v}-{u})/{a} = {t:.4} s")
        }
        "u" => {
            let v = v.ok_or("missing 'v'")?;
            let a = a.ok_or("missing 'a'")?;
            let t = t.ok_or("missing 't'")?;
            let u = v - a * t;
            format!("u = v - at\nu = {v} - {a}×{t} = {u:.4} m/s")
        }
        _ => return Err(format!("unknown solve_for '{solve}'; use: v|s|a|t|u")),
    };

    Ok(format!(
        "=== KINEMATICS (SUVAT) ===\nSolving for: {solve}\n{result}\n\nSUVAT equations:\n  v = u + at\n  s = ut + ½at²\n  v² = u² + 2as\n  s = ½(u+v)t"
    ))
}

fn action_forces(args: &Value) -> Result<String, String> {
    let solve = args
        .get("solve_for")
        .and_then(|x| x.as_str())
        .unwrap_or("F");
    let g_val = n(args, "g").unwrap_or(G);
    let mut out = String::from("=== FORCES ===\n");

    match solve {
        "F" => {
            let m = require(args, "m")?;
            let a = require(args, "a")?;
            let f = m * a;
            out.push_str(&format!(
                "Newton's Second Law: F = ma\nF = {m} × {a} = {f:.4} N\n"
            ));
        }
        "a" => {
            let f = require(args, "F")?;
            let m = require(args, "m")?;
            let a = f / m;
            out.push_str(&format!("a = F/m = {f}/{m} = {a:.4} m/s²\n"));
        }
        "friction" => {
            let mu = require(args, "mu")?;
            let m = require(args, "m")?;
            let theta = n(args, "theta").unwrap_or(0.0);
            let theta_r = theta * PI / 180.0;
            let normal = m * g_val * theta_r.cos();
            let friction = mu * normal;
            out.push_str(&format!(
                "Friction Force (θ={theta}°)\nNormal force N = mg·cos(θ) = {m}×{g_val:.4}×cos({theta}°) = {normal:.4} N\nFriction f = μN = {mu}×{normal:.4} = {friction:.4} N\n"
            ));
        }
        "incline" => {
            let m = require(args, "m")?;
            let theta = require(args, "theta")?;
            let mu = n(args, "mu").unwrap_or(0.0);
            let theta_r = theta * PI / 180.0;
            let normal = m * g_val * theta_r.cos();
            let gravity_parallel = m * g_val * theta_r.sin();
            let friction = mu * normal;
            let net = gravity_parallel - friction;
            let a_net = net / m;
            out.push_str(&format!(
                "Incline Analysis (θ={theta}°, μ={mu})\nWeight component along incline: mg·sin(θ) = {gravity_parallel:.4} N\nNormal force: mg·cos(θ) = {normal:.4} N\nFriction force: μN = {friction:.4} N\nNet force: {net:.4} N\nAcceleration: {a_net:.4} m/s²\n"
            ));
        }
        _ => {
            return Err(format!(
                "unknown solve_for '{solve}'; use: F|a|friction|incline"
            ))
        }
    }

    Ok(out)
}

fn action_energy(args: &Value) -> Result<String, String> {
    let solve = args
        .get("solve_for")
        .and_then(|x| x.as_str())
        .unwrap_or("KE");
    let g_val = n(args, "g").unwrap_or(G);
    let mut out = String::from("=== ENERGY & POWER ===\n");

    match solve {
        "KE" => {
            let m = require(args, "m")?;
            let v = require(args, "v")?;
            let ke = 0.5 * m * v * v;
            out.push_str(&format!("KE = ½mv²\nKE = 0.5 × {m} × {v}² = {ke:.4} J\n"));
        }
        "GPE" => {
            let m = require(args, "m")?;
            let h = require(args, "h")?;
            let pe = m * g_val * h;
            out.push_str(&format!(
                "GPE = mgh\nGPE = {m} × {g_val:.4} × {h} = {pe:.4} J\n"
            ));
        }
        "spring" => {
            let k = require(args, "k")?;
            let x = require(args, "x")?;
            let pe = 0.5 * k * x * x;
            out.push_str(&format!(
                "Elastic PE = ½kx²\nPE = 0.5 × {k} × {x}² = {pe:.4} J\n"
            ));
        }
        "conservation" => {
            let m = require(args, "m")?;
            let h = require(args, "h")?;
            let v0 = n(args, "v0").unwrap_or(0.0);
            let ke0 = 0.5 * m * v0 * v0;
            let pe0 = m * g_val * h;
            let total = ke0 + pe0;
            let v_bottom = (2.0 * total / m).sqrt();
            out.push_str(&format!(
                "Conservation of Mechanical Energy\nInitial KE = {ke0:.4} J  |  Initial GPE = {pe0:.4} J\nTotal E = {total:.4} J\nMax speed at ground: v = √(2E/m) = {v_bottom:.4} m/s\n"
            ));
        }
        "power" => {
            let f = require(args, "F")?;
            let v = require(args, "v")?;
            let p = f * v;
            out.push_str(&format!(
                "Power P = Fv\nP = {f} × {v} = {p:.4} W  ({:.4} kW)\n",
                p / 1000.0
            ));
        }
        "work" => {
            let f = require(args, "F")?;
            let s = require(args, "s")?;
            let theta = n(args, "theta").unwrap_or(0.0);
            let theta_r = theta * PI / 180.0;
            let w = f * s * theta_r.cos();
            out.push_str(&format!(
                "Work W = Fs·cos(θ)\nW = {f} × {s} × cos({theta}°) = {w:.4} J\n"
            ));
        }
        _ => {
            return Err(format!(
                "unknown solve_for '{solve}'; use: KE|GPE|spring|conservation|power|work"
            ))
        }
    }

    Ok(out)
}

fn action_momentum(args: &Value) -> Result<String, String> {
    let solve = args
        .get("solve_for")
        .and_then(|x| x.as_str())
        .unwrap_or("p");
    let mut out = String::from("=== MOMENTUM & COLLISIONS ===\n");

    match solve {
        "p" => {
            let m = require(args, "m")?;
            let v = require(args, "v")?;
            let p = m * v;
            out.push_str(&format!("Momentum p = mv\np = {m} × {v} = {p:.4} kg·m/s\n"));
        }
        "impulse" => {
            let f = require(args, "F")?;
            let t = require(args, "t")?;
            let j = f * t;
            out.push_str(&format!(
                "Impulse J = FΔt\nJ = {f} × {t} = {j:.4} N·s = {j:.4} kg·m/s\n"
            ));
        }
        "elastic" => {
            let m1 = require(args, "m")?;
            let m2 = require(args, "m2")?;
            let v1 = require(args, "v1")?;
            let v2 = n(args, "v2").unwrap_or(0.0);
            let v1f = ((m1 - m2) * v1 + 2.0 * m2 * v2) / (m1 + m2);
            let v2f = ((m2 - m1) * v2 + 2.0 * m1 * v1) / (m1 + m2);
            let ke_i = 0.5 * m1 * v1 * v1 + 0.5 * m2 * v2 * v2;
            let ke_f = 0.5 * m1 * v1f * v1f + 0.5 * m2 * v2f * v2f;
            out.push_str(&format!(
                "Elastic Collision (KE conserved)\nm1={m1} kg at {v1} m/s  |  m2={m2} kg at {v2} m/s\nv1' = {v1f:.4} m/s  |  v2' = {v2f:.4} m/s\nKE before: {ke_i:.4} J  |  KE after: {ke_f:.4} J\n"
            ));
        }
        "inelastic" => {
            let m1 = require(args, "m")?;
            let m2 = require(args, "m2")?;
            let v1 = require(args, "v1")?;
            let v2 = n(args, "v2").unwrap_or(0.0);
            let vf = (m1 * v1 + m2 * v2) / (m1 + m2);
            let ke_i = 0.5 * m1 * v1 * v1 + 0.5 * m2 * v2 * v2;
            let ke_f = 0.5 * (m1 + m2) * vf * vf;
            let ke_lost = ke_i - ke_f;
            out.push_str(&format!(
                "Perfectly Inelastic Collision (objects stick)\nm1={m1} kg at {v1} m/s  |  m2={m2} kg at {v2} m/s\nFinal velocity vf = {vf:.4} m/s\nKE before: {ke_i:.4} J  |  KE after: {ke_f:.4} J  |  KE lost: {ke_lost:.4} J\n"
            ));
        }
        _ => {
            return Err(format!(
                "unknown solve_for '{solve}'; use: p|impulse|elastic|inelastic"
            ))
        }
    }

    Ok(out)
}

fn action_rotation(args: &Value) -> Result<String, String> {
    let solve = args
        .get("solve_for")
        .and_then(|x| x.as_str())
        .unwrap_or("torque");
    let mut out = String::from("=== ROTATIONAL MECHANICS ===\n");

    match solve {
        "torque" => {
            let f = require(args, "F")?;
            let r = require(args, "r")?;
            let theta = n(args, "theta").unwrap_or(90.0);
            let theta_r = theta * PI / 180.0;
            let tau = f * r * theta_r.sin();
            out.push_str(&format!(
                "Torque τ = r×F·sin(θ)\nτ = {r} × {f} × sin({theta}°) = {tau:.4} N·m\n"
            ));
        }
        "inertia" => {
            let m = require(args, "m")?;
            let r = require(args, "r")?;
            let shape = args
                .get("shape")
                .and_then(|x| x.as_str())
                .unwrap_or("solid_cylinder");
            let (i, formula) = match shape {
                "solid_sphere" => (2.0 / 5.0 * m * r * r, "2/5 mr²"),
                "hollow_sphere" => (2.0 / 3.0 * m * r * r, "2/3 mr²"),
                "solid_cylinder" | "disk" => (0.5 * m * r * r, "½mr²"),
                "ring" => (m * r * r, "mr²"),
                "rod_center" => {
                    let l = require(args, "L")?;
                    (m * l * l / 12.0, "mL²/12")
                }
                "rod_end" => {
                    let l = require(args, "L")?;
                    (m * l * l / 3.0, "mL²/3")
                }
                _ => return Err(format!("unknown shape '{shape}'")),
            };
            out.push_str(&format!(
                "Moment of Inertia — {shape}\nI = {formula}\nI = {i:.4} kg·m²\n"
            ));
        }
        "alpha" => {
            let f = require(args, "F")?;
            let r = require(args, "r")?;
            let i = require(args, "I")?;
            let tau = f * r;
            let alpha = tau / i;
            out.push_str(&format!(
                "Angular acceleration: α = τ/I = Fr/I\nτ = {f}×{r} = {tau:.4} N·m\nα = {tau:.4}/{i} = {alpha:.4} rad/s²\n"
            ));
        }
        "ke_rot" => {
            let i = require(args, "I")?;
            let omega = require(args, "omega")?;
            let ke = 0.5 * i * omega * omega;
            out.push_str(&format!(
                "Rotational KE = ½Iω²\nKE = 0.5 × {i} × {omega}² = {ke:.4} J\n"
            ));
        }
        "L" => {
            let i = require(args, "I")?;
            let omega = require(args, "omega")?;
            let l = i * omega;
            out.push_str(&format!(
                "Angular Momentum L = Iω\nL = {i} × {omega} = {l:.4} kg·m²/s\n"
            ));
        }
        _ => {
            return Err(format!(
                "unknown solve_for '{solve}'; use: torque|inertia|alpha|ke_rot|L"
            ))
        }
    }

    Ok(out)
}

fn action_oscillation(args: &Value) -> Result<String, String> {
    let solve = args
        .get("solve_for")
        .and_then(|x| x.as_str())
        .unwrap_or("T_spring");
    let g_val = n(args, "g").unwrap_or(G);
    let mut out = String::from("=== OSCILLATION & SHM ===\n");

    match solve {
        "T_spring" => {
            let m = require(args, "m")?;
            let k = require(args, "k")?;
            let t = 2.0 * PI * (m / k).sqrt();
            let f = 1.0 / t;
            let omega = 2.0 * PI * f;
            out.push_str(&format!(
                "Spring-Mass Period T = 2π√(m/k)\nT = 2π√({m}/{k}) = {t:.4} s\nf = 1/T = {f:.4} Hz\nω = 2πf = {omega:.4} rad/s\n"
            ));
        }
        "T_pendulum" => {
            let l = require(args, "L")?;
            let t = 2.0 * PI * (l / g_val).sqrt();
            let f = 1.0 / t;
            out.push_str(&format!(
                "Simple Pendulum Period T = 2π√(L/g)\nT = 2π√({l}/{g_val:.4}) = {t:.4} s\nf = 1/T = {f:.4} Hz\n"
            ));
        }
        "k" => {
            let m = require(args, "m")?;
            let t = require(args, "t")?;
            let k = m * (2.0 * PI / t).powi(2);
            out.push_str(&format!(
                "Spring constant from T: k = m(2π/T)²\nk = {m} × (2π/{t})² = {k:.4} N/m\n"
            ));
        }
        "shm" => {
            let a_amp = require(args, "a")?;
            let omega = require(args, "omega")?;
            let t_val = n(args, "t").unwrap_or(0.0);
            let x = a_amp * (omega * t_val).cos();
            let v = -a_amp * omega * (omega * t_val).sin();
            let acc = -a_amp * omega * omega * (omega * t_val).cos();
            out.push_str(&format!(
                "SHM at t={t_val} s (amplitude={a_amp} m, ω={omega} rad/s)\nx(t) = A·cos(ωt) = {x:.4} m\nv(t) = -Aω·sin(ωt) = {v:.4} m/s\na(t) = -Aω²·cos(ωt) = {acc:.4} m/s²\nMax speed = Aω = {:.4} m/s\n", a_amp * omega));
        }
        _ => {
            return Err(format!(
                "unknown solve_for '{solve}'; use: T_spring|T_pendulum|k|shm"
            ))
        }
    }

    Ok(out)
}

fn action_projectile(args: &Value) -> Result<String, String> {
    let v0 = require(args, "v0")?;
    let theta = require(args, "theta")?;
    let g_val = n(args, "g").unwrap_or(G);
    let theta_r = theta * PI / 180.0;

    let vx = v0 * theta_r.cos();
    let vy0 = v0 * theta_r.sin();
    let t_flight = 2.0 * vy0 / g_val;
    let range = vx * t_flight;
    let h_max = vy0 * vy0 / (2.0 * g_val);
    let t_at_query = n(args, "t");

    let mut out = format!(
        "=== PROJECTILE MOTION ===\nv₀ = {v0} m/s  |  θ = {theta}°  |  g = {g_val:.4} m/s²\n\nvx = v₀·cos(θ) = {vx:.4} m/s\nvy₀ = v₀·sin(θ) = {vy0:.4} m/s\n\nTime of flight: T = 2vy₀/g = {t_flight:.4} s\nHorizontal range: R = vx·T = {range:.4} m\nMax height: H = vy₀²/(2g) = {h_max:.4} m\n"
    );

    if let Some(t) = t_at_query {
        let x = vx * t;
        let y = vy0 * t - 0.5 * g_val * t * t;
        let vx_t = vx;
        let vy_t = vy0 - g_val * t;
        let speed = (vx_t * vx_t + vy_t * vy_t).sqrt();
        out.push_str(&format!(
            "\nAt t = {t} s:\n  x = {x:.4} m  |  y = {y:.4} m\n  speed = {speed:.4} m/s\n"
        ));
    }

    Ok(out)
}

fn action_circular(args: &Value) -> Result<String, String> {
    let solve = args
        .get("solve_for")
        .and_then(|x| x.as_str())
        .unwrap_or("Fc");
    let mut out = String::from("=== CIRCULAR MOTION ===\n");

    match solve {
        "Fc" => {
            let m = require(args, "m")?;
            let v = require(args, "v")?;
            let r = require(args, "r")?;
            let fc = m * v * v / r;
            let ac = v * v / r;
            let omega = v / r;
            out.push_str(&format!(
                "Centripetal motion: v={v} m/s, r={r} m, m={m} kg\na_c = v²/r = {ac:.4} m/s²\nF_c = mv²/r = {fc:.4} N\nω = v/r = {omega:.4} rad/s\nT = 2πr/v = {:.4} s\n",
                2.0 * PI * r / v
            ));
        }
        "orbital" => {
            let m_central = require(args, "m")?;
            let r = require(args, "r")?;
            const BIG_G: f64 = 6.674e-11;
            let v_orb = (BIG_G * m_central / r).sqrt();
            let t_orb = 2.0 * PI * r / v_orb;
            out.push_str(&format!(
                "Circular Orbit (Gravitational)\nM = {m_central:.4e} kg, r = {r:.4e} m\nOrbital speed: v = √(GM/r) = {v_orb:.4} m/s\nOrbital period: T = 2πr/v = {t_orb:.4} s  ({:.4} h)\n",
                t_orb / 3600.0
            ));
        }
        _ => return Err(format!("unknown solve_for '{solve}'; use: Fc|orbital")),
    }

    Ok(out)
}

pub async fn execute(args: &Value) -> Result<String, String> {
    let action = args
        .get("action")
        .and_then(|x| x.as_str())
        .unwrap_or("kinematics");
    match action {
        "kinematics" => action_kinematics(args),
        "forces" => action_forces(args),
        "energy" => action_energy(args),
        "momentum" => action_momentum(args),
        "rotation" => action_rotation(args),
        "oscillation" => action_oscillation(args),
        "projectile" => action_projectile(args),
        "circular" => action_circular(args),
        _ => Err(format!("unknown action '{action}'; use: kinematics|forces|energy|momentum|rotation|oscillation|projectile|circular")),
    }
}
