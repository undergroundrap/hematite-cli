use serde_json::{json, Value};

pub fn schema() -> Value {
    json!({
        "name": "acoustics_tools",
        "description": "Acoustics and sound physics calculations without external utilities.",
        "parameters": {
            "type": "object",
            "properties": {
                "action": { "type": "string" },
                "freq": { "type": "number" },
                "f1": { "type": "number" },
                "f2": { "type": "number" },
                "wavelength": { "type": "number" },
                "temp_c": { "type": "number" },
                "intensity": { "type": "number" },
                "pressure": { "type": "number" },
                "spl": { "type": "number" },
                "levels": { "type": "array", "items": { "type": "number" } },
                "v_src": { "type": "number" },
                "v_obs": { "type": "number" },
                "type": { "type": "string" },
                "length": { "type": "number" },
                "tension": { "type": "number" },
                "linear_density": { "type": "number" },
                "harmonics": { "type": "integer" },
                "rho1": { "type": "number" },
                "c1": { "type": "number" },
                "rho2": { "type": "number" },
                "c2": { "type": "number" },
                "volume": { "type": "number" },
                "width": { "type": "number" },
                "height": { "type": "number" },
                "absorption": { "type": "number" }
            },
            "required": []
        }
    })
}

pub async fn execute(args: &Value) -> Result<String, String> {
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("wave");
    match action {
        "wave" => action_wave(args),
        "decibels" | "dB" | "db" | "spl" => action_decibels(args),
        "doppler" => action_doppler(args),
        "resonance" | "standing" => action_resonance(args),
        "impedance" => action_impedance(args),
        "rt60" | "room" | "reverberation" => action_rt60(args),
        "hearing" => action_hearing(args),
        "beat" => action_beat(args),
        _ => Err(format!(
            "Unknown action '{action}'. Valid: wave, decibels, doppler, resonance, impedance, rt60, hearing, beat"
        )),
    }
}

fn speed_of_sound(temp_c: f64) -> f64 {
    331.3 * (1.0 + temp_c / 273.15).sqrt()
}

fn action_wave(args: &Value) -> Result<String, String> {
    let temp_c = args.get("temp_c").and_then(|v| v.as_f64()).unwrap_or(20.0);
    let v = speed_of_sound(temp_c);

    if let Some(freq) = args
        .get("freq")
        .or_else(|| args.get("f"))
        .and_then(|v| v.as_f64())
    {
        if freq <= 0.0 {
            return Err("freq must be positive".into());
        }
        let wavelength = v / freq;
        let period = 1.0 / freq;
        let angular = 2.0 * std::f64::consts::PI * freq;
        let wavenumber = 2.0 * std::f64::consts::PI / wavelength;

        let mut out = String::from("SOUND WAVE PROPERTIES\n");
        out.push_str("═══════════════════════════════════════\n");
        out.push_str(&format!(
            "  Frequency (f)       {:.4e} Hz  [{}]\n",
            freq,
            audio_band(freq)
        ));
        out.push_str(&format!(
            "  Speed in air (v)    {:.2} m/s  at {:.1} °C\n",
            v, temp_c
        ));
        out.push_str(&format!(
            "  Wavelength (λ=v/f)  {:.4e} m   = {}\n",
            wavelength,
            fmt_length(wavelength)
        ));
        out.push_str(&format!("  Period (T=1/f)      {:.4e} s\n", period));
        out.push_str(&format!("  Angular freq (ω)    {:.4e} rad/s\n", angular));
        out.push_str(&format!("  Wavenumber (k)      {:.4e} rad/m\n", wavenumber));
        return Ok(out);
    }

    if let Some(wl) = args
        .get("wavelength")
        .or_else(|| args.get("lambda"))
        .and_then(|v| v.as_f64())
    {
        if wl <= 0.0 {
            return Err("wavelength must be positive".into());
        }
        let freq = v / wl;
        let period = 1.0 / freq;
        let mut out = String::from("SOUND WAVE PROPERTIES\n");
        out.push_str("═══════════════════════════════════════\n");
        out.push_str(&format!(
            "  Wavelength (λ)      {:.4e} m   = {}\n",
            wl,
            fmt_length(wl)
        ));
        out.push_str(&format!(
            "  Speed in air (v)    {:.2} m/s  at {:.1} °C\n",
            v, temp_c
        ));
        out.push_str(&format!(
            "  Frequency (f=v/λ)   {:.4e} Hz  [{}]\n",
            freq,
            audio_band(freq)
        ));
        out.push_str(&format!("  Period (T=1/f)      {:.4e} s\n", period));
        return Ok(out);
    }

    // Speed of sound table
    let mut out = String::from("SPEED OF SOUND IN AIR\n");
    out.push_str("═══════════════════════════════════════\n");
    for t in [-20i32, 0, 10, 20, 30, 40] {
        let vs = speed_of_sound(t as f64);
        out.push_str(&format!(
            "  {:>4} °C  →  {:.2} m/s  ({:.1} km/h)\n",
            t,
            vs,
            vs * 3.6
        ));
    }
    out.push_str(
        "\nProvide 'freq' (Hz) or 'wavelength' (m). Optional: 'temp_c' (default 20 °C).\n",
    );
    Ok(out)
}

fn audio_band(freq: f64) -> &'static str {
    if freq < 20.0 {
        "infrasound"
    } else if freq <= 20_000.0 {
        "audible"
    } else if freq <= 200_000.0 {
        "ultrasound"
    } else {
        "hypersound"
    }
}

fn fmt_length(m: f64) -> String {
    if m >= 1.0 {
        format!("{:.3} m", m)
    } else if m >= 1e-2 {
        format!("{:.1} cm", m * 100.0)
    } else if m >= 1e-3 {
        format!("{:.2} mm", m * 1000.0)
    } else {
        format!("{:.3} µm", m * 1e6)
    }
}

fn action_decibels(args: &Value) -> Result<String, String> {
    let i_ref = 1e-12_f64;
    let p_ref = 20e-6_f64;

    // Combine multiple levels
    if let Some(levels) = args.get("levels").and_then(|v| v.as_array()) {
        let dbs: Vec<f64> = levels.iter().filter_map(|v| v.as_f64()).collect();
        if dbs.is_empty() {
            return Err("'levels' array must contain dB values".into());
        }
        let combined_intensity: f64 = dbs.iter().map(|&db| 10.0_f64.powf(db / 10.0)).sum();
        let combined_db = 10.0 * combined_intensity.log10();
        let mut out = String::from("COMBINING SOUND LEVELS\n");
        out.push_str("═══════════════════════════════════════\n");
        for db in &dbs {
            out.push_str(&format!("  {:>8.2} dB SPL\n", db));
        }
        out.push_str("  ──────────────────────\n");
        out.push_str(&format!("  Combined  {:.2} dB SPL\n", combined_db));
        return Ok(out);
    }

    if let Some(intensity) = args.get("intensity").and_then(|v| v.as_f64()) {
        if intensity <= 0.0 {
            return Err("intensity must be positive".into());
        }
        let spl = 10.0 * (intensity / i_ref).log10();
        let mut out = String::from("SOUND PRESSURE LEVEL\n");
        out.push_str("═══════════════════════════════════════\n");
        out.push_str(&format!("  Intensity           {:.4e} W/m²\n", intensity));
        out.push_str(&format!("  SPL = 10·log₁₀(I/I₀) = {:.2} dB\n", spl));
        out.push_str(&format!("  Context             {}\n", spl_context(spl)));
        return Ok(out);
    }

    if let Some(pressure) = args.get("pressure").and_then(|v| v.as_f64()) {
        if pressure <= 0.0 {
            return Err("pressure must be positive".into());
        }
        let spl = 20.0 * (pressure / p_ref).log10();
        let mut out = String::from("SOUND PRESSURE LEVEL\n");
        out.push_str("═══════════════════════════════════════\n");
        out.push_str(&format!("  Sound pressure (p)  {:.4e} Pa\n", pressure));
        out.push_str(&format!("  Reference (p₀)      20 µPa\n"));
        out.push_str(&format!("  SPL = 20·log₁₀(p/p₀) = {:.2} dB\n", spl));
        out.push_str(&format!("  Context             {}\n", spl_context(spl)));
        return Ok(out);
    }

    if let Some(spl) = args.get("spl").and_then(|v| v.as_f64()) {
        let intensity = i_ref * 10.0_f64.powf(spl / 10.0);
        let pressure = p_ref * 10.0_f64.powf(spl / 20.0);
        let mut out = String::from("DECIBEL CONVERSION\n");
        out.push_str("═══════════════════════════════════════\n");
        out.push_str(&format!("  SPL                 {:.2} dB\n", spl));
        out.push_str(&format!("  Intensity           {:.4e} W/m²\n", intensity));
        out.push_str(&format!("  Sound pressure      {:.4e} Pa\n", pressure));
        out.push_str(&format!("  Context             {}\n", spl_context(spl)));
        return Ok(out);
    }

    // Reference table
    let mut out = String::from("SOUND LEVEL REFERENCE TABLE\n");
    out.push_str("═══════════════════════════════════════\n");
    let refs: &[(&str, i32)] = &[
        ("Threshold of hearing", 0),
        ("Rustling leaves", 10),
        ("Quiet room", 30),
        ("Library", 40),
        ("Normal conversation", 60),
        ("Busy restaurant", 70),
        ("Hearing damage threshold", 85),
        ("Lawnmower (1 m)", 90),
        ("Rock concert", 110),
        ("Jet engine (100 m)", 130),
        ("Pain threshold", 140),
    ];
    for (label, spl) in refs {
        out.push_str(&format!("  {:>4} dB  {}\n", spl, label));
    }
    out.push_str(
        "\nProvide: 'intensity' (W/m²), 'pressure' (Pa), 'spl' (dB), or 'levels' (array of dB).\n",
    );
    Ok(out)
}

fn spl_context(spl: f64) -> &'static str {
    if spl < 10.0 {
        "threshold of hearing"
    } else if spl < 30.0 {
        "very quiet"
    } else if spl < 50.0 {
        "quiet room"
    } else if spl < 70.0 {
        "normal conversation"
    } else if spl < 85.0 {
        "busy environment"
    } else if spl < 100.0 {
        "hearing damage risk with prolonged exposure"
    } else if spl < 120.0 {
        "very loud — hearing damage with short exposure"
    } else {
        "pain threshold / immediate hearing damage"
    }
}

fn action_doppler(args: &Value) -> Result<String, String> {
    let f_src = match args
        .get("freq")
        .or_else(|| args.get("f"))
        .and_then(|v| v.as_f64())
    {
        Some(f) if f > 0.0 => f,
        Some(_) => return Err("freq must be positive".into()),
        None => return Err("provide 'freq' (source frequency in Hz)".into()),
    };

    let temp_c = args.get("temp_c").and_then(|v| v.as_f64()).unwrap_or(20.0);
    let v = speed_of_sound(temp_c);
    // positive v_src = source moving toward observer
    // positive v_obs = observer moving toward source
    let v_src = args.get("v_src").and_then(|v| v.as_f64()).unwrap_or(0.0);
    let v_obs = args.get("v_obs").and_then(|v| v.as_f64()).unwrap_or(0.0);

    let denom = v - v_src;
    if denom.abs() < 1e-6 {
        return Err(
            "Source speed equals speed of sound — sonic boom singularity, formula undefined".into(),
        );
    }

    let f_obs = f_src * (v + v_obs) / denom;
    let shift = f_obs - f_src;
    let shift_pct = shift / f_src * 100.0;
    let mach = v_src.abs() / v;

    let mut out = String::from("DOPPLER EFFECT (SOUND)\n");
    out.push_str("═══════════════════════════════════════\n");
    out.push_str(&format!("  Source frequency     {:.4e} Hz\n", f_src));
    out.push_str(&format!(
        "  Speed of sound       {:.2} m/s  ({:.1} °C)\n",
        v, temp_c
    ));
    out.push_str(&format!(
        "  Source velocity      {:+.2} m/s  ({} observer)\n",
        v_src,
        if v_src > 0.0 {
            "approaching"
        } else if v_src < 0.0 {
            "receding from"
        } else {
            "stationary rel."
        }
    ));
    out.push_str(&format!(
        "  Observer velocity    {:+.2} m/s  ({} source)\n",
        v_obs,
        if v_obs > 0.0 {
            "approaching"
        } else if v_obs < 0.0 {
            "receding from"
        } else {
            "stationary rel."
        }
    ));
    out.push_str("  f_obs = f_src · (v + v_obs) / (v − v_src)\n");
    out.push_str("  ─────────────────────────────────────────\n");
    out.push_str(&format!("  Observed frequency   {:.4e} Hz\n", f_obs));
    out.push_str(&format!(
        "  Frequency shift      {:+.4e} Hz  ({:+.2}%)\n",
        shift, shift_pct
    ));
    out.push_str(&format!(
        "  Pitch change         {}\n",
        if shift > 1e-6 {
            "higher (approaching)"
        } else if shift < -1e-6 {
            "lower (receding)"
        } else {
            "unchanged"
        }
    ));
    if v_src.abs() > 0.01 {
        out.push_str(&format!("  Mach number          {:.4}\n", mach));
        if mach >= 1.0 {
            out.push_str("  ⚠  Supersonic: shock wave / sonic boom\n");
        }
    }
    Ok(out)
}

fn action_resonance(args: &Value) -> Result<String, String> {
    let kind = args
        .get("type")
        .or_else(|| args.get("kind"))
        .and_then(|v| v.as_str())
        .unwrap_or("string");
    let n_harmonics = args
        .get("harmonics")
        .and_then(|v| v.as_u64())
        .unwrap_or(5)
        .min(10) as usize;
    let temp_c = args.get("temp_c").and_then(|v| v.as_f64()).unwrap_or(20.0);
    let v_sound = speed_of_sound(temp_c);

    let length = match args
        .get("length")
        .or_else(|| args.get("l"))
        .and_then(|v| v.as_f64())
    {
        Some(l) if l > 0.0 => l,
        _ => {
            return Err(
                "provide 'length' (metres) and 'type' (string/open_pipe/closed_pipe)".into(),
            )
        }
    };

    let wave_speed = if kind == "string" {
        match (
            args.get("tension").and_then(|v| v.as_f64()),
            args.get("linear_density")
                .or_else(|| args.get("mu"))
                .and_then(|v| v.as_f64()),
        ) {
            (Some(t), Some(mu)) if t > 0.0 && mu > 0.0 => (t / mu).sqrt(),
            _ => v_sound,
        }
    } else {
        v_sound
    };

    let mut out = String::from("STANDING WAVES — RESONANCE\n");
    out.push_str("═══════════════════════════════════════\n");

    let (formula, step) = match kind {
        "open_pipe" => {
            out.push_str("  Type: Open Pipe (both ends open)\n");
            out.push_str(&format!("  Length L = {:.4} m\n", length));
            out.push_str(&format!("  Wave speed v = {:.2} m/s\n", wave_speed));
            out.push_str("  f_n = n·v / (2L)  —  all harmonics present\n");
            ("n·v/(2L)", 1usize)
        }
        "closed_pipe" => {
            out.push_str("  Type: Closed Pipe (one end closed)\n");
            out.push_str(&format!("  Length L = {:.4} m\n", length));
            out.push_str(&format!("  Wave speed v = {:.2} m/s\n", wave_speed));
            out.push_str("  f_n = n·v / (4L)  —  odd harmonics only (n=1,3,5,...)\n");
            ("n·v/(4L)", 2usize)
        }
        "string" => {
            out.push_str("  Type: Vibrating String (both ends fixed)\n");
            out.push_str(&format!("  Length L = {:.4} m\n", length));
            out.push_str(&format!("  Wave speed v = {:.2} m/s\n", wave_speed));
            out.push_str("  f_n = n·v / (2L)  —  all harmonics present\n");
            ("n·v/(2L)", 1usize)
        }
        _ => {
            return Err(format!(
                "Unknown type '{kind}'. Valid: string, open_pipe, closed_pipe"
            ))
        }
    };
    let _ = formula;

    out.push_str("\n   n   Frequency (Hz)     Wavelength (m)   Label\n");
    out.push_str("  ──────────────────────────────────────────────────\n");

    let divisor = if kind == "closed_pipe" { 4.0 } else { 2.0 };
    let mut count = 0;
    let mut n = 1usize;
    while count < n_harmonics {
        let freq = n as f64 * wave_speed / (divisor * length);
        let wl = wave_speed / freq;
        let label = match n {
            1 => "fundamental",
            2 => "2nd harmonic / 1st overtone",
            3 => "3rd harmonic / 2nd overtone",
            4 => "4th harmonic / 3rd overtone",
            5 => "5th harmonic / 4th overtone",
            _ => "nth harmonic",
        };
        out.push_str(&format!(
            "  {:3}   {:>14.4}     {:>14.4}   {}\n",
            n, freq, wl, label
        ));
        n += step;
        count += 1;
    }

    Ok(out)
}

fn action_impedance(args: &Value) -> Result<String, String> {
    let rho1 = args.get("rho1").and_then(|v| v.as_f64()).unwrap_or(1.21);
    let c1 = args.get("c1").and_then(|v| v.as_f64()).unwrap_or(343.0);
    let z1 = rho1 * c1;

    let mut out = String::from("ACOUSTIC IMPEDANCE\n");
    out.push_str("═══════════════════════════════════════\n");
    out.push_str(&format!(
        "  Medium 1: ρ₁ = {:.4} kg/m³,  c₁ = {:.2} m/s\n",
        rho1, c1
    ));
    out.push_str(&format!("  Z₁ = ρ₁·c₁ = {:.4e} Pa·s/m  (rayl)\n", z1));

    match (
        args.get("rho2").and_then(|v| v.as_f64()),
        args.get("c2").and_then(|v| v.as_f64()),
    ) {
        (Some(rho2), Some(c2)) => {
            let z2 = rho2 * c2;
            let r = (z2 - z1) / (z2 + z1);
            let t_power = 4.0 * z1 * z2 / (z1 + z2).powi(2);
            let r_power = r * r;
            let tl_db = -10.0 * t_power.log10();

            out.push_str(&format!(
                "\n  Medium 2: ρ₂ = {:.4} kg/m³,  c₂ = {:.2} m/s\n",
                rho2, c2
            ));
            out.push_str(&format!("  Z₂ = ρ₂·c₂ = {:.4e} Pa·s/m\n", z2));
            out.push_str("\n  At normal incidence interface:\n");
            out.push_str(&format!("  Pressure reflection coeff  r  = {:.4}\n", r));
            out.push_str(&format!(
                "  Power reflection coeff     R  = r² = {:.4}  ({:.2}%)\n",
                r_power,
                r_power * 100.0
            ));
            out.push_str(&format!(
                "  Power transmission coeff   T  = {:.4}  ({:.2}%)\n",
                t_power,
                t_power * 100.0
            ));
            out.push_str(&format!(
                "  Transmission loss          TL = {:.2} dB\n",
                tl_db
            ));
            if t_power < 0.01 {
                out.push_str("  → Near-total reflection (large impedance mismatch)\n");
            } else if t_power > 0.99 {
                out.push_str("  → Near-perfect transmission (matched impedances)\n");
            }
        }
        _ => {
            out.push_str("\nProvide 'rho2' (kg/m³) and 'c2' (m/s) for transmission analysis.\n\n");
            out.push_str("  Common acoustic impedances (rayl = Pa·s/m):\n");
            let mats: &[(&str, f64)] = &[
                ("Air at 20 °C", 413.0),
                ("Water at 20 °C", 1.48e6),
                ("Human tissue", 1.63e6),
                ("Rubber", 1.5e6),
                ("Concrete", 8.0e6),
                ("Steel", 45.0e6),
                ("Aluminum", 17.0e6),
                ("Wood (pine)", 1.5e6),
            ];
            for (name, z) in mats {
                out.push_str(&format!("    {:22}  Z = {:.3e}\n", name, z));
            }
        }
    }

    Ok(out)
}

fn action_rt60(args: &Value) -> Result<String, String> {
    // Sabine formula: RT60 = 0.161 V / A
    let volume = if let Some(v) = args
        .get("volume")
        .or_else(|| args.get("v"))
        .and_then(|v| v.as_f64())
    {
        if v <= 0.0 {
            return Err("volume must be positive".into());
        }
        v
    } else {
        let l = args.get("length").and_then(|v| v.as_f64());
        let w = args.get("width").and_then(|v| v.as_f64());
        let h = args.get("height").and_then(|v| v.as_f64());
        match (l, w, h) {
            (Some(l), Some(w), Some(h)) if l > 0.0 && w > 0.0 && h > 0.0 => l * w * h,
            _ => {
                return Err(
                    "provide 'volume' (m³) or 'length'+'width'+'height' (all in metres)".into(),
                )
            }
        }
    };

    let mut out = String::from("ROOM ACOUSTICS — RT60\n");
    out.push_str("═══════════════════════════════════════\n");
    out.push_str(&format!("  Room volume (V)     {:.2} m³\n", volume));

    if let Some(absorption) = args
        .get("absorption")
        .or_else(|| args.get("a"))
        .and_then(|v| v.as_f64())
    {
        if absorption <= 0.0 {
            return Err("absorption must be positive".into());
        }
        let rt60 = 0.161 * volume / absorption;
        let schroeder = 2000.0 * (rt60 / volume).sqrt();

        out.push_str(&format!(
            "  Total absorption (A) {:.2} m² sabine\n",
            absorption
        ));
        out.push_str("\n  Sabine formula: RT60 = 0.161·V / A\n");
        out.push_str(&format!("  RT60               {:.3} s\n", rt60));
        out.push_str(&format!("  Assessment         {}\n", rt60_label(rt60)));
        out.push_str(&format!("\n  Schroeder frequency  {:.1} Hz\n", schroeder));
        out.push_str(
            "  (modal ↔ statistical crossover; treat as statistical above this frequency)\n",
        );
    } else {
        out.push_str("\n  Required absorption for target RT60:\n");
        out.push_str("  A = 0.161·V / RT60\n\n");
        out.push_str("  Space              RT60 target    Absorption needed\n");
        out.push_str("  ───────────────────────────────────────────────────\n");
        let targets: &[(&str, f64, f64)] = &[
            ("Recording studio", 0.2, 0.4),
            ("Home cinema", 0.3, 0.5),
            ("Conference room", 0.4, 0.7),
            ("Classroom", 0.4, 0.8),
            ("Concert hall", 1.5, 2.2),
            ("Cathedral", 3.0, 8.0),
        ];
        for (space, lo, hi) in targets {
            let a_hi = 0.161 * volume / lo;
            let a_lo = 0.161 * volume / hi;
            out.push_str(&format!(
                "  {:20} {:.1}–{:.1} s   {:.1}–{:.1} m²\n",
                space, lo, hi, a_lo, a_hi
            ));
        }
        out.push_str("\nProvide 'absorption' (m² sabine) to compute RT60.\n");
    }

    Ok(out)
}

fn rt60_label(rt60: f64) -> &'static str {
    if rt60 < 0.2 {
        "very dry / near-anechoic"
    } else if rt60 < 0.5 {
        "recording studio / home cinema range"
    } else if rt60 < 1.0 {
        "conference room / classroom range"
    } else if rt60 < 2.0 {
        "concert hall / live music range"
    } else if rt60 < 4.0 {
        "large auditorium / church"
    } else {
        "cathedral / very reverberant"
    }
}

fn action_hearing(args: &Value) -> Result<String, String> {
    let _ = args;
    let mut out = String::from("HUMAN HEARING — REFERENCE DATA\n");
    out.push_str("═══════════════════════════════════════\n");
    out.push_str("  Audible range       20 Hz – 20 kHz  (young adults)\n");
    out.push_str("  Dynamic range       0 – 140 dB SPL\n");
    out.push_str("  Reference pressure  p₀ = 20 µPa  (0 dB SPL)\n");
    out.push_str("  Reference intensity I₀ = 10⁻¹² W/m²\n");
    out.push_str("  Most sensitive      ~3–4 kHz  (ear canal resonance)\n\n");

    out.push_str("  Audiometric test frequencies (ISO 8253-1):\n");
    out.push_str("  125, 250, 500, 1000, 2000, 4000, 6000, 8000 Hz\n\n");

    out.push_str("  Noise exposure limits (OSHA 1910.95):\n");
    out.push_str("  ────────────────────────────────────────\n");
    let limits: &[(&str, &str)] = &[
        ("85 dB", "8 hrs (NIOSH recommended limit)"),
        ("90 dB", "8 hrs"),
        ("92 dB", "6 hrs"),
        ("95 dB", "4 hrs"),
        ("100 dB", "2 hrs"),
        ("105 dB", "1 hr"),
        ("110 dB", "30 min"),
        ("115 dB", "15 min  (OSHA ceiling)"),
    ];
    for (lvl, limit) in limits {
        out.push_str(&format!("  {:7}  max {}\n", lvl, limit));
    }

    out.push_str("\n  Common SPL references:\n");
    let refs: &[(&str, i32)] = &[
        ("Threshold of hearing", 0),
        ("Rustling leaves", 10),
        ("Quiet library", 40),
        ("Normal conversation", 60),
        ("Vacuum cleaner (1 m)", 75),
        ("City traffic", 85),
        ("Rock concert", 110),
        ("Jet engine (100 m)", 130),
        ("Pain threshold", 140),
    ];
    for (label, spl) in refs {
        out.push_str(&format!("  {:>4} dB  {}\n", spl, label));
    }
    Ok(out)
}

fn action_beat(args: &Value) -> Result<String, String> {
    let f1 = match args
        .get("f1")
        .or_else(|| args.get("freq1"))
        .and_then(|v| v.as_f64())
    {
        Some(f) if f > 0.0 => f,
        _ => return Err("provide 'f1' and 'f2' (frequencies in Hz)".into()),
    };
    let f2 = match args
        .get("f2")
        .or_else(|| args.get("freq2"))
        .and_then(|v| v.as_f64())
    {
        Some(f) if f > 0.0 => f,
        _ => return Err("provide 'f2' (second frequency in Hz)".into()),
    };

    let beat_freq = (f1 - f2).abs();
    let avg_freq = (f1 + f2) / 2.0;
    let ratio = if f1 >= f2 { f1 / f2 } else { f2 / f1 };

    let mut out = String::from("BEAT FREQUENCY\n");
    out.push_str("═══════════════════════════════════════\n");
    out.push_str(&format!(
        "  f₁                  {:.4} Hz  ({})\n",
        f1,
        freq_to_note(f1)
    ));
    out.push_str(&format!(
        "  f₂                  {:.4} Hz  ({})\n",
        f2,
        freq_to_note(f2)
    ));
    out.push_str(&format!(
        "  Beat frequency      {:.4} Hz  = |f₁ − f₂|\n",
        beat_freq
    ));
    if beat_freq > 0.0 {
        out.push_str(&format!(
            "  Beat period         {:.4} s  ({:.2} beats/s)\n",
            1.0 / beat_freq,
            beat_freq
        ));
    }
    out.push_str(&format!(
        "  Carrier frequency   {:.4} Hz  (average)\n",
        avg_freq
    ));
    out.push_str(&format!("  Frequency ratio     {:.6}\n", ratio));
    out.push_str(&format!(
        "  Musical interval    {}\n",
        musical_interval(ratio)
    ));

    // Overtone series
    let base = f1.min(f2);
    out.push_str(&format!("\n  Harmonic series of {:.2} Hz:\n", base));
    for n in 1..=8usize {
        let h = n as f64 * base;
        out.push_str(&format!(
            "    n={}: {:>10.3} Hz  {}\n",
            n,
            h,
            freq_to_note(h)
        ));
    }
    Ok(out)
}

fn freq_to_note(freq: f64) -> String {
    if freq <= 0.0 {
        return "—".to_string();
    }
    let notes = [
        "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
    ];
    let a4 = 440.0_f64;
    let semitones = 12.0 * (freq / a4).log2();
    let n = (semitones.round() as i32) + 57;
    if n < 0 || n > 127 {
        return "—".to_string();
    }
    let octave = n / 12;
    let note_idx = ((n % 12) as usize).min(11);
    let cents = ((semitones - semitones.round()) * 100.0).round() as i32;
    if cents == 0 {
        format!("{}{}", notes[note_idx], octave)
    } else {
        format!("{}{} {:+}¢", notes[note_idx], octave, cents)
    }
}

fn musical_interval(ratio: f64) -> &'static str {
    let intervals: &[(f64, f64, &str)] = &[
        (1.0, 0.005, "Unison"),
        (16.0 / 15.0, 0.005, "Minor Second"),
        (9.0 / 8.0, 0.005, "Major Second"),
        (6.0 / 5.0, 0.005, "Minor Third"),
        (5.0 / 4.0, 0.005, "Major Third"),
        (4.0 / 3.0, 0.005, "Perfect Fourth"),
        (7.0 / 5.0, 0.005, "Tritone"),
        (3.0 / 2.0, 0.005, "Perfect Fifth"),
        (8.0 / 5.0, 0.005, "Minor Sixth"),
        (5.0 / 3.0, 0.005, "Major Sixth"),
        (9.0 / 5.0, 0.005, "Minor Seventh"),
        (15.0 / 8.0, 0.005, "Major Seventh"),
        (2.0, 0.005, "Octave"),
    ];
    for &(target, tol, name) in intervals {
        if (ratio - target).abs() < tol {
            return name;
        }
    }
    "Unknown interval"
}
