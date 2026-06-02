use serde_json::{json, Value};

pub fn optics_tools_schema() -> Value {
    json!({
        "name": "optics_tools",
        "description": "Optics and photonics calculations: Snell's law, thin lens, mirror equation, diffraction, interference, polarization, optical fiber, and blackbody radiation. No external dependencies.",
        "input_schema": {
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["refraction","lens","mirror","diffraction","interference","polarization","fiber","blackbody"],
                    "description": "refraction: Snell's law, critical angle, TIR | lens: thin lens eq, lensmaker's eq | mirror: mirror equation | diffraction: single-slit, grating | interference: double-slit, thin film | polarization: Malus's law, Brewster angle | fiber: optical fiber NA, acceptance angle | blackbody: Planck, Wien, Stefan-Boltzmann"
                },
                "n1": {"type": "number", "description": "Refractive index of medium 1"},
                "n2": {"type": "number", "description": "Refractive index of medium 2"},
                "theta1": {"type": "number", "description": "Angle of incidence in degrees"},
                "theta2": {"type": "number", "description": "Angle of refraction in degrees"},
                "f": {"type": "number", "description": "Focal length m (positive: converging, negative: diverging)"},
                "do": {"type": "number", "description": "Object distance m"},
                "di": {"type": "number", "description": "Image distance m"},
                "R1": {"type": "number", "description": "Radius of curvature of first surface m (lensmaker's)"},
                "R2": {"type": "number", "description": "Radius of curvature of second surface m (lensmaker's)"},
                "n_lens": {"type": "number", "description": "Refractive index of lens material"},
                "R": {"type": "number", "description": "Mirror radius of curvature m"},
                "a": {"type": "number", "description": "Slit width m [diffraction] or slit separation m [double-slit]"},
                "lambda": {"type": "number", "description": "Wavelength m (e.g. 500e-9 for 500 nm)"},
                "m": {"type": "number", "description": "Diffraction order (integer)"},
                "L": {"type": "number", "description": "Distance to screen m"},
                "d": {"type": "number", "description": "Grating spacing m (lines/mm → d=1e-3/lines_per_mm)"},
                "n_film": {"type": "number", "description": "Film refractive index [thin film interference]"},
                "t": {"type": "number", "description": "Film thickness m [thin film interference]"},
                "I0": {"type": "number", "description": "Initial intensity W/m² [Malus's law]"},
                "theta": {"type": "number", "description": "Angle between polarizer axes degrees [Malus's] or grazing angle [fiber]"},
                "n_core": {"type": "number", "description": "Core refractive index [fiber]"},
                "n_clad": {"type": "number", "description": "Cladding refractive index [fiber]"},
                "T": {"type": "number", "description": "Temperature K [blackbody]"},
                "wavelength_nm": {"type": "number", "description": "Wavelength nm for spectral radiance [blackbody]"}
            }
        }
    })
}

const C: f64 = 2.99792458e8; // m/s
const H: f64 = 6.62607015e-34; // J·s
const KB: f64 = 1.380649e-23; // J/K
const SIGMA: f64 = 5.670374419e-8; // W/(m²·K⁴)
const PI: f64 = std::f64::consts::PI;

fn deg_to_rad(d: f64) -> f64 {
    d * PI / 180.0
}
fn rad_to_deg(r: f64) -> f64 {
    r * 180.0 / PI
}

fn action_refraction(args: &Value) -> Result<String, String> {
    let get = |k: &str| args[k].as_f64();
    let n1 = get("n1").ok_or("n1 required: refractive index of medium 1")?;
    let n2 = get("n2").ok_or("n2 required: refractive index of medium 2")?;

    let mut out = format!("Snell's Law: n1·sin(θ1) = n2·sin(θ2)\nn1 = {n1},  n2 = {n2}\n\n");

    if let Some(t1_deg) = get("theta1") {
        let t1 = deg_to_rad(t1_deg);
        let sin_t2 = n1 * t1.sin() / n2;
        if sin_t2.abs() > 1.0 {
            out += &format!(
                "θ1 = {t1_deg}°\nsin(θ2) = n1·sin(θ1)/n2 = {sin_t2:.6} > 1\n→ TOTAL INTERNAL REFLECTION (θ1 > critical angle)\n"
            );
        } else {
            let t2_deg = rad_to_deg(sin_t2.asin());
            out += &format!(
                "θ1 = {t1_deg}°\nsin(θ2) = n1·sin(θ1)/n2 = {sin_t2:.6}\nθ2 = {t2_deg:.4}°\n"
            );
        }
    } else if let Some(t2_deg) = get("theta2") {
        let t2 = deg_to_rad(t2_deg);
        let sin_t1 = n2 * t2.sin() / n1;
        let t1_deg = rad_to_deg(sin_t1.asin());
        out += &format!("θ2 = {t2_deg}°\nθ1 = {t1_deg:.4}°\n");
    }

    // Critical angle (if n1 > n2)
    if n1 > n2 {
        let theta_c = rad_to_deg((n2 / n1).asin());
        out += &format!(
            "\nCritical angle (for TIR, light going from n1 → n2):\nθ_c = arcsin(n2/n1) = arcsin({:.6}) = {theta_c:.4}°\n→ TIR occurs when θ1 ≥ {theta_c:.4}°",
            n2 / n1
        );
    }

    // Common refractive indices reference
    out += "\n\nCommon n values:\n  Vacuum/Air: 1.000\n  Water:      1.333\n  Glass:      1.5–1.9\n  Diamond:    2.417\n  Silicon:    3.48 (IR)";
    Ok(out)
}

fn action_lens(args: &Value) -> Result<String, String> {
    let get = |k: &str| args[k].as_f64();

    // Lensmaker's equation branch
    if let (Some(r1), Some(r2), Some(n_lens)) = (get("R1"), get("R2"), get("n_lens")) {
        let inv_f = (n_lens - 1.0) * (1.0 / r1 - 1.0 / r2);
        let f = 1.0 / inv_f;
        return Ok(format!(
            "Lensmaker's Equation: 1/f = (n-1)·(1/R1 - 1/R2)\n\nn_lens = {n_lens},  R1 = {r1} m,  R2 = {r2} m\n\n1/f = ({n_lens}-1)·(1/{r1} - 1/{r2})\n    = {:.6} × {:.6}\n    = {inv_f:.6} m⁻¹\n\nf = {f:.4} m  ({:.2} mm)\n\nLens type: {}\n\nSign convention:\n  R > 0: surface curves toward incoming light\n  R < 0: surface curves away from incoming light",
            n_lens - 1.0, 1.0 / r1 - 1.0 / r2,
            f * 1000.0,
            if f > 0.0 { "Converging (convex)" } else { "Diverging (concave)" }
        ));
    }

    // Thin lens equation: 1/f = 1/do + 1/di
    let known_count = [get("f"), get("do"), get("di")]
        .iter()
        .filter(|x| x.is_some())
        .count();
    if known_count < 2 {
        return Err("Provide at least 2 of: f (focal length), do (object distance), di (image distance). Or provide R1, R2, n_lens for lensmaker's equation.".into());
    }

    let (f_val, do_val, di_val) = match (get("f"), get("do"), get("di")) {
        (Some(f), Some(do_), None) => {
            let di = 1.0 / (1.0 / f - 1.0 / do_);
            (f, do_, di)
        }
        (Some(f), None, Some(di)) => {
            let do_ = 1.0 / (1.0 / f - 1.0 / di);
            (f, do_, di)
        }
        (None, Some(do_), Some(di)) => {
            let f = 1.0 / (1.0 / do_ + 1.0 / di);
            (f, do_, di)
        }
        (Some(f), Some(do_), Some(di)) => (f, do_, di),
        _ => return Err("Need 2 of f, do, di".into()),
    };

    let m = -di_val / do_val;
    let img_type = if di_val > 0.0 { "Real" } else { "Virtual" };
    let orientation = if m > 0.0 { "Upright" } else { "Inverted" };
    let size = if m.abs() > 1.0 {
        format!("Magnified ({:.4}×)", m.abs())
    } else {
        format!("Diminished ({:.4}×)", m.abs())
    };

    Ok(format!(
        "Thin Lens Equation: 1/f = 1/do + 1/di\n\nf = {f_val} m,  do = {do_val} m\n\n1/f = 1/do + 1/di\n1/{f_val} = 1/{do_val} + 1/di\n\ndi = {di_val:.4} m\n\nLateral magnification m = -di/do = {m:.4}\n\nImage properties:\n  Type:        {img_type} (di {} 0)\n  Orientation: {orientation} (m {} 0)\n  Size:        {size}\n\nLens type: {} (f {} 0)",
        if di_val > 0.0 { ">" } else { "<" },
        if m > 0.0 { ">" } else { "<" },
        if f_val > 0.0 { "Converging" } else { "Diverging" },
        if f_val > 0.0 { ">" } else { "<" }
    ))
}

fn action_mirror(args: &Value) -> Result<String, String> {
    let get = |k: &str| args[k].as_f64();

    let f_val = if let Some(r) = get("R") {
        r / 2.0
    } else if let Some(f) = get("f") {
        f
    } else {
        return Err("Provide f (focal length) or R (radius of curvature)".into());
    };

    let (do_val, di_val) = match (get("do"), get("di")) {
        (Some(do_), None) => {
            let di = 1.0 / (1.0 / f_val - 1.0 / do_);
            (do_, di)
        }
        (None, Some(di)) => {
            let do_ = 1.0 / (1.0 / f_val - 1.0 / di);
            (do_, di)
        }
        (Some(do_), Some(di)) => (do_, di),
        _ => {
            return Err(
                "Provide at least one of do (object distance) or di (image distance)".into(),
            )
        }
    };

    let m = -di_val / do_val;
    let r = 2.0 * f_val;
    let img_type = if di_val > 0.0 { "Real" } else { "Virtual" };
    let orientation = if m > 0.0 { "Upright" } else { "Inverted" };

    Ok(format!(
        "Mirror Equation: 1/f = 1/do + 1/di\n\nf = {f_val:.4} m  (R = {r:.4} m)\nMirror type: {} (f {} 0)\n\ndo = {do_val:.4} m\ndi = {di_val:.4} m\n\nMagnification m = -di/do = {m:.4}\n\nImage: {img_type}, {orientation}, |m| = {:.4}\n\nSign convention (mirrors):\n  do > 0: object in front (real object)\n  di > 0: image in front (real image)\n  di < 0: image behind mirror (virtual image)\n  f > 0: concave mirror\n  f < 0: convex mirror",
        if f_val > 0.0 { "Concave" } else { "Convex" },
        if f_val > 0.0 { ">" } else { "<" },
        m.abs()
    ))
}

fn action_diffraction(args: &Value) -> Result<String, String> {
    let get = |k: &str| args[k].as_f64();
    let lambda = get("lambda").ok_or("lambda required: wavelength m (e.g. 500e-9)")?;

    let lambda_nm = lambda * 1e9;

    // Grating diffraction
    if let Some(d) = get("d") {
        let m = get("m").unwrap_or(1.0) as i32;
        let sin_theta = m as f64 * lambda / d;
        if sin_theta.abs() > 1.0 {
            return Ok(format!(
                "Diffraction Grating\n\nd·sin(θ) = m·λ\nd = {d:.4e} m ({:.2} lines/mm)\nλ = {lambda_nm:.2} nm,  m = {m}\n\nsin(θ) = m·λ/d = {sin_theta:.4} > 1\n→ Order m={m} does not exist for this grating/wavelength",
                1e-3 / d
            ));
        }
        let theta_deg = rad_to_deg(sin_theta.asin());
        let resolving_power_per_order_per_slit = m as f64;
        return Ok(format!(
            "Diffraction Grating\n\nd·sin(θ) = m·λ\nd = {d:.4e} m ({:.2} lines/mm)\nλ = {lambda_nm:.2} nm\n\nm = {m}: sin(θ) = {:.6},  θ = {theta_deg:.4}°\n\nFor N slits: resolving power R = m·N\nChromatic resolving power λ/Δλ = m·N\n\nMax order: m_max = floor(d/λ) = {}",
            1e-3 / d, m as f64 * lambda / d, (d / lambda) as i32
        ));
    }

    // Single-slit diffraction
    let a = get("a").ok_or("a required: slit width m")?;
    let l = get("L");

    let mut out = format!(
        "Single-Slit Diffraction\n\nFirst minimum: a·sin(θ) = m·λ  (m = ±1, ±2, ...)\n\na = {a:.4e} m,  λ = {lambda_nm:.2} nm\nRatio a/λ = {:.2}\n\n",
        a / lambda
    );

    for m in 1i32..=3 {
        let sin_t = m as f64 * lambda / a;
        if sin_t > 1.0 {
            break;
        }
        let theta_deg = rad_to_deg(sin_t.asin());
        out += &format!("Minimum m={m}: θ = {theta_deg:.4}°\n");
        if let Some(l_val) = l {
            let y = l_val * sin_t / (1.0 - sin_t * sin_t).sqrt();
            out += &format!(
                "  at L={l_val} m screen: y = {y:.6} m ({:.4} mm)\n",
                y * 1000.0
            );
        }
    }

    // Central maximum width
    let theta_1 = (lambda / a).asin();
    out += &format!(
        "\nCentral maximum half-width: θ = {:.4}°\nAngular width of central max: {:.4}°",
        rad_to_deg(theta_1),
        2.0 * rad_to_deg(theta_1)
    );
    Ok(out)
}

fn action_interference(args: &Value) -> Result<String, String> {
    let get = |k: &str| args[k].as_f64();
    let lambda = get("lambda").ok_or("lambda required: wavelength m")?;
    let lambda_nm = lambda * 1e9;

    // Thin film interference
    if let Some(n_film) = get("n_film") {
        let t = get("t").ok_or("t required: film thickness m")?;
        let n1 = get("n1").unwrap_or(1.0);
        let n2 = get("n2").unwrap_or(n_film + 0.5);
        let phase_shifts = if n1 < n_film && n_film < n2 {
            0
        } else if n1 > n_film || n_film > n2 {
            1
        } else {
            2
        };

        let path_diff = 2.0 * n_film * t;
        let lambda_film = lambda / n_film;

        let condition = if phase_shifts % 2 == 0 {
            format!("constructive: 2·n·t = m·λ  →  bright for m = 1,2,3...\ndestructive: 2·n·t = (m+½)·λ  →  dark for m = 0,1,2...")
        } else {
            format!("constructive: 2·n·t = (m+½)·λ  →  bright for m = 0,1,2...\ndestructive: 2·n·t = m·λ  →  dark for m = 1,2,3...")
        };

        let mut orders = String::new();
        for m in 0i32..=5 {
            let t_constr = if phase_shifts % 2 == 0 {
                m as f64 * lambda / (2.0 * n_film)
            } else {
                (m as f64 + 0.5) * lambda / (2.0 * n_film)
            };
            if t_constr > 0.0 {
                orders += &format!(
                    "  m={m}: constructive at t = {:.4e} m ({:.2} nm)\n",
                    t_constr,
                    t_constr * 1e9
                );
            }
        }

        return Ok(format!(
            "Thin Film Interference\n\nFilm: n_film = {n_film},  thickness t = {t:.4e} m ({:.2} nm)\nMedia: n1 = {n1} (above),  n2 = {n2} (below)\nPhase shifts at boundaries: {phase_shifts}\n\nPath difference = 2·n·t = {path_diff:.6e} m\nλ in film = λ/n = {lambda_nm:.2}/{n_film} = {:.2} nm\n\n{condition}\n\nConstructive-interference thicknesses:\n{orders}",
            t * 1e9, lambda_film * 1e9
        ));
    }

    // Young's double-slit
    let a = get("a").ok_or("a required: slit separation m")?;
    let l = get("L").ok_or("L required: screen distance m")?;

    let fringe_spacing = lambda * l / a;
    let mut out = format!(
        "Young's Double-Slit Interference\n\nSlit separation: a = {a:.4e} m\nScreen distance: L = {l} m\nWavelength: λ = {lambda_nm:.2} nm\n\nFringe spacing: Δy = λL/a = {:.6} m ({:.4} mm)\n\nBright fringes (maxima): y_m = m·λL/a\n",
        fringe_spacing, fringe_spacing * 1000.0
    );

    for m in 0i32..=5 {
        let y = m as f64 * fringe_spacing;
        out += &format!("  m={m}: y = {:.6} m ({:.4} mm)\n", y, y * 1000.0);
    }

    out += &format!(
        "\nDark fringes (minima): y_m = (m+½)·λL/a\nFirst dark fringe: y = {:.6} m ({:.4} mm)",
        0.5 * fringe_spacing,
        0.5 * fringe_spacing * 1000.0
    );
    Ok(out)
}

fn action_polarization(args: &Value) -> Result<String, String> {
    let get = |k: &str| args[k].as_f64();

    // Malus's Law
    if let Some(i0) = get("I0") {
        let theta_deg =
            get("theta").ok_or("theta required: angle between polarizer axes (degrees)")?;
        let theta = deg_to_rad(theta_deg);
        let i = i0 * theta.cos().powi(2);
        return Ok(format!(
            "Malus's Law: I = I₀·cos²(θ)\n\nI₀ = {i0} W/m²\nθ  = {theta_deg}°\n\nI = {i0} × cos²({theta_deg}°)\n  = {i0} × {:.6}\n  = {i:.4} W/m²\n\nTransmission: {:.4}%",
            theta.cos().powi(2), i / i0 * 100.0
        ));
    }

    // Brewster's angle
    if let (Some(n1), Some(n2)) = (get("n1"), get("n2")) {
        let theta_b = rad_to_deg((n2 / n1).atan());
        let theta_r = 90.0 - theta_b;
        return Ok(format!(
            "Brewster's Angle (polarization by reflection)\n\ntan(θ_B) = n2/n1\n\nn1 = {n1},  n2 = {n2}\n\nθ_B = arctan(n2/n1) = arctan({:.6})\n    = {theta_b:.4}°\n\nAt Brewster's angle:\n  - Reflected ray is fully s-polarized (⊥ to plane)\n  - Refracted ray has partial p-polarization (∥ to plane)\n  - Refracted angle = {theta_r:.4}° (complementary)\n\nVerify: θ_B + θ_r = {:.4}° (should be 90°)",
            n2 / n1, theta_b + theta_r
        ));
    }

    Err("Provide I0 + theta (Malus's law) or n1 + n2 (Brewster's angle)".into())
}

fn action_fiber(args: &Value) -> Result<String, String> {
    let get = |k: &str| args[k].as_f64();
    let n_core = get("n_core").ok_or("n_core required: core refractive index")?;
    let n_clad = get("n_clad").ok_or("n_clad required: cladding refractive index")?;

    if n_core <= n_clad {
        return Err("n_core must be > n_clad for total internal reflection".into());
    }

    let na = (n_core * n_core - n_clad * n_clad).sqrt();
    let theta_accept = rad_to_deg(na.asin().min(PI / 2.0));
    let theta_c = rad_to_deg((n_clad / n_core).asin());
    let delta = (n_core - n_clad) / n_core;

    let v_number = if let Some(lambda) = get("lambda") {
        let d = get("D").unwrap_or(get("a").unwrap_or(0.0));
        if d > 0.0 {
            let v = PI * d * na / lambda;
            let lambda_nm = lambda * 1e9;
            let mode_type = if v < 2.405 {
                "Single-mode"
            } else {
                format!("Multi-mode (≈{:.0} modes)", v * v / 2.0).leak()
            };
            format!("\nV-number (core diameter d = {d:.4e} m, λ = {lambda_nm:.2} nm):\nV = π·d·NA/λ = {v:.4}  →  {mode_type}\nSingle-mode condition: V < 2.405")
        } else {
            String::new()
        }
    } else {
        String::new()
    };

    Ok(format!(
        "Optical Fiber Parameters\n\nn_core = {n_core},  n_clad = {n_clad}\n\nNumerical Aperture:\nNA = √(n_core² - n_clad²) = √({:.6} - {:.6})\n   = {na:.6}\n\nAcceptance angle (in air):\nθ_accept = arcsin(NA) = {theta_accept:.4}°  (half-angle)\nFull acceptance cone: {:.4}°\n\nCritical angle (inside core):\nθ_c = arcsin(n_clad/n_core) = {theta_c:.4}°\n→ TIR occurs for θ > {theta_c:.4}° at core-cladding interface\n\nRelative refractive index difference:\nΔ = (n_core - n_clad)/n_core = {delta:.6}  ({:.4}%){v_number}",
        n_core * n_core, n_clad * n_clad, theta_accept * 2.0, delta * 100.0
    ))
}

fn action_blackbody(args: &Value) -> Result<String, String> {
    let get = |k: &str| args[k].as_f64();
    let t = get("T").ok_or("T required: temperature K")?;
    if t <= 0.0 {
        return Err("T must be > 0 K".into());
    }

    // Wien's displacement law
    let b_wien = 2.897771955e-3; // m·K
    let lambda_max = b_wien / t;
    let lambda_max_nm = lambda_max * 1e9;

    // Stefan-Boltzmann total power
    let m_total = SIGMA * t.powi(4);

    // Color temperature description
    let color_desc = match t as u32 {
        0..=1799 => "Deep red / infrared",
        1800..=2799 => "Warm red-orange (candle/incandescent)",
        2800..=3499 => "Warm white (tungsten lamp)",
        3500..=4499 => "Neutral white",
        4500..=5999 => "Cool white / daylight",
        6000..=7999 => "Blue-white (overcast sky)",
        8000..=19999 => "Blue / ultraviolet fringe",
        _ => "Far UV / X-ray regime",
    };

    let mut out = format!("Blackbody Radiation: T = {t} K  ({:.2} °C)\n\n", t - 273.15);

    out += &format!(
        "Wien's Displacement Law:\nλ_max = b/T = {b_wien:.6e} / {t}\n      = {lambda_max:.6e} m  ({lambda_max_nm:.2} nm)\nColor: {color_desc}\n\n"
    );

    out += &format!(
        "Stefan-Boltzmann Law (total emitted power per unit area):\nM = σ·T⁴ = {SIGMA:.6e} × {t}⁴\n  = {m_total:.6e} W/m²\n\n"
    );

    // Spectral radiance at specific wavelength
    if let Some(lam_nm) = get("wavelength_nm").or_else(|| get("lambda").map(|l| l * 1e9)) {
        let lam = lam_nm * 1e-9;
        let exponent = H * C / (lam * KB * t);
        let b_lambda = if exponent > 700.0 {
            0.0
        } else {
            2.0 * H * C * C / (lam.powi(5) * (exponent.exp() - 1.0))
        };
        out += &format!(
            "Planck's Law at λ = {lam_nm:.2} nm:\nB_λ = 2hc²/λ⁵ · 1/(e^(hc/λkT) - 1)\nhc/λkT exponent = {exponent:.4}\nB_λ = {b_lambda:.6e} W·sr⁻¹·m⁻³\n\n"
        );
    }

    out += &format!(
        "Constants used:\n  h = {H:.6e} J·s (Planck)\n  c = {C:.6e} m/s\n  k = {KB:.6e} J/K (Boltzmann)\n  σ = {SIGMA:.6e} W/(m²·K⁴)"
    );
    Ok(out)
}

pub async fn execute(args: &Value) -> Result<String, String> {
    let action = args["action"].as_str().unwrap_or("refraction");
    match action {
        "refraction"    => action_refraction(args),
        "lens"          => action_lens(args),
        "mirror"        => action_mirror(args),
        "diffraction"   => action_diffraction(args),
        "interference"  => action_interference(args),
        "polarization"  => action_polarization(args),
        "fiber"         => action_fiber(args),
        "blackbody"     => action_blackbody(args),
        _ => Err(format!("Unknown action '{action}'. Use: refraction, lens, mirror, diffraction, interference, polarization, fiber, blackbody")),
    }
}
