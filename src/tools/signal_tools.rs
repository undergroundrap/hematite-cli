use serde_json::{json, Value};
use std::f64::consts::PI;

pub fn signal_tools_schema() -> Value {
    json!({
        "name": "signal_tools",
        "description": "Digital signal processing: DFT/IDFT, FIR filter design, convolution, window functions, resampling, and signal statistics without external libraries.",
        "input_schema": {
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["dft", "idft", "convolve", "fir", "window", "stats", "resample", "autocorr"],
                    "description": "Action to perform (default: dft)"
                },
                "samples": {
                    "type": ["array", "string"],
                    "description": "Input signal samples as JSON array or comma-separated string"
                },
                "kernel": {
                    "type": ["array", "string"],
                    "description": "Convolution kernel / FIR coefficients"
                },
                "real": {
                    "type": ["array", "string"],
                    "description": "Real part of complex spectrum for IDFT"
                },
                "imag": {
                    "type": ["array", "string"],
                    "description": "Imaginary part of complex spectrum for IDFT"
                },
                "cutoff": {
                    "type": "number",
                    "description": "Normalized cutoff frequency 0–0.5 for FIR design"
                },
                "taps": {
                    "type": "integer",
                    "description": "Number of FIR filter taps (must be odd, default 31)"
                },
                "filter_type": {
                    "type": "string",
                    "enum": ["lowpass", "highpass", "bandpass", "bandstop"],
                    "description": "FIR filter type (default: lowpass)"
                },
                "cutoff2": {
                    "type": "number",
                    "description": "Second cutoff frequency for bandpass/bandstop (0–0.5)"
                },
                "window_type": {
                    "type": "string",
                    "enum": ["rectangular", "hanning", "hamming", "blackman", "bartlett", "flat_top", "kaiser"],
                    "description": "Window function (default: hamming)"
                },
                "beta": {
                    "type": "number",
                    "description": "Kaiser window beta parameter (default 5.0)"
                },
                "length": {
                    "type": "integer",
                    "description": "Window / output length"
                },
                "sample_rate": {
                    "type": "number",
                    "description": "Sample rate in Hz for frequency labelling"
                },
                "up": {"type": "integer", "description": "Upsampling factor for resample"},
                "down": {"type": "integer", "description": "Downsampling factor for resample"},
                "max_bins": {"type": "integer", "description": "Max DFT bins to display (default 32)"}
            }
        }
    })
}

// ── sample parsing ───────────────────────────────────────────────────────────

fn parse_samples(v: &Value) -> Result<Vec<f64>, String> {
    match v {
        Value::Array(arr) => arr
            .iter()
            .map(|x| {
                x.as_f64()
                    .ok_or_else(|| format!("Non-numeric sample: {}", x))
            })
            .collect(),
        Value::String(s) => s
            .split([',', ' ', '\n', '\t'])
            .filter(|t| !t.is_empty())
            .map(|t| {
                t.trim()
                    .parse::<f64>()
                    .map_err(|_| format!("Cannot parse '{}' as float", t))
            })
            .collect(),
        _ => Err("'samples' must be an array or comma-separated string.".into()),
    }
}

// ── DFT ─────────────────────────────────────────────────────────────────────

fn dft(x: &[f64]) -> Vec<(f64, f64)> {
    let n = x.len();
    (0..n)
        .map(|k| {
            x.iter().enumerate().fold((0.0, 0.0), |(re, im), (t, &xt)| {
                let angle = 2.0 * PI * k as f64 * t as f64 / n as f64;
                (re + xt * angle.cos(), im - xt * angle.sin())
            })
        })
        .collect()
}

fn idft(re: &[f64], im: &[f64]) -> Vec<f64> {
    let n = re.len();
    (0..n)
        .map(|t| {
            let sum: f64 = (0..n)
                .map(|k| {
                    let angle = 2.0 * PI * k as f64 * t as f64 / n as f64;
                    re[k] * angle.cos() - im[k] * angle.sin()
                })
                .sum();
            sum / n as f64
        })
        .collect()
}

fn action_dft(args: &Value) -> Result<String, String> {
    let x = parse_samples(&args["samples"])?;
    let n = x.len();
    if n < 2 {
        return Err("Need at least 2 samples.".into());
    }
    if n > 8192 {
        return Err("Max 8192 samples for DFT.".into());
    }

    let spec = dft(&x);
    let max_bins = args["max_bins"].as_u64().unwrap_or(32) as usize;
    let display_n = (n / 2 + 1).min(max_bins);
    let sr = args["sample_rate"].as_f64();

    let mut out = format!("DFT — {} samples", n);
    if let Some(fs) = sr {
        out.push_str(&format!(", fs={} Hz", fs));
    }
    out.push_str(&format!(
        "\n\n{:<6} {:<14} {:<14} {:<14} {:<12}\n",
        "Bin", "Frequency", "Magnitude", "Phase (deg)", "Real / Imag"
    ));
    out.push_str(&"─".repeat(68));
    out.push('\n');

    let mag_max = spec[..n / 2 + 1]
        .iter()
        .map(|(r, i)| (r * r + i * i).sqrt())
        .fold(0.0_f64, f64::max);

    for (k, &(re, im)) in spec[..display_n].iter().enumerate() {
        let mag = (re * re + im * im).sqrt();
        let phase = im.atan2(re).to_degrees();
        let freq_str = if let Some(fs) = sr {
            format!("{:.4} Hz", k as f64 * fs / n as f64)
        } else {
            format!("{:.4} (norm)", k as f64 / n as f64)
        };
        let bar_len = if mag_max > 0.0 {
            (mag / mag_max * 20.0).round() as usize
        } else {
            0
        };
        let bar = "█".repeat(bar_len);
        out.push_str(&format!(
            "{:<6} {:<14} {:>10.4}  {:>12.2}°  {:>8.4} {:>+8.4}j  {}\n",
            k, freq_str, mag, phase, re, im, bar
        ));
    }
    if display_n < n / 2 + 1 {
        out.push_str(&format!(
            "  ... ({} more bins, use max_bins to show all)\n",
            n / 2 + 1 - display_n
        ));
    }

    // Signal power and DC
    let power: f64 = x.iter().map(|v| v * v).sum::<f64>() / n as f64;
    let dc = spec[0].0 / n as f64;
    out.push_str(&format!(
        "\nDC component: {:.6}\nSignal power: {:.6}\n",
        dc, power
    ));
    Ok(out)
}

fn action_idft(args: &Value) -> Result<String, String> {
    let re = parse_samples(&args["real"])?;
    let im = parse_samples(&args["imag"])?;
    if re.len() != im.len() {
        return Err("'real' and 'imag' must have the same length.".into());
    }
    if re.is_empty() {
        return Err("'real' and 'imag' arrays required.".into());
    }
    let x = idft(&re, &im);
    let mut out = format!("IDFT — {} samples recovered\n\n", x.len());
    out.push_str("Reconstructed signal:\n");
    for (i, v) in x.iter().enumerate() {
        out.push_str(&format!("  x[{}] = {:.8}\n", i, v));
    }
    Ok(out)
}

// ── convolution ──────────────────────────────────────────────────────────────

fn convolve(x: &[f64], h: &[f64]) -> Vec<f64> {
    let ny = x.len() + h.len() - 1;
    let mut y = vec![0.0; ny];
    for (i, &xi) in x.iter().enumerate() {
        for (j, &hj) in h.iter().enumerate() {
            y[i + j] += xi * hj;
        }
    }
    y
}

fn action_convolve(args: &Value) -> Result<String, String> {
    let x = parse_samples(&args["samples"])?;
    let h = if !args["kernel"].is_null() {
        parse_samples(&args["kernel"])?
    } else {
        return Err("'kernel' array is required for convolution.".into());
    };
    if x.is_empty() || h.is_empty() {
        return Err("Both 'samples' and 'kernel' must be non-empty.".into());
    }
    let y = convolve(&x, &h);
    let mut out = "Convolution\n\n".to_string();
    out.push_str(&format!("  Input  x: {} samples\n", x.len()));
    out.push_str(&format!("  Kernel h: {} taps\n", h.len()));
    out.push_str(&format!("  Output y: {} samples\n\n", y.len()));
    out.push_str("Output y[n]:\n");
    for (i, v) in y.iter().enumerate() {
        out.push_str(&format!("  y[{:>4}] = {:>12.8}\n", i, v));
    }
    Ok(out)
}

// ── window functions ─────────────────────────────────────────────────────────

fn make_window(wtype: &str, n: usize, beta: f64) -> Vec<f64> {
    (0..n)
        .map(|i| {
            let m = (n - 1) as f64;
            let x = i as f64 / m;
            match wtype {
                "hanning" => 0.5 - 0.5 * (2.0 * PI * x).cos(),
                "hamming" => 0.54 - 0.46 * (2.0 * PI * x).cos(),
                "blackman" => 0.42 - 0.5 * (2.0 * PI * x).cos() + 0.08 * (4.0 * PI * x).cos(),
                "bartlett" => 1.0 - (2.0 * i as f64 / m - 1.0).abs(),
                "flat_top" => {
                    1.0 - 1.93 * (2.0 * PI * x).cos() + 1.29 * (4.0 * PI * x).cos()
                        - 0.388 * (6.0 * PI * x).cos()
                        + 0.0322 * (8.0 * PI * x).cos()
                }
                "kaiser" => {
                    let a = 2.0 * i as f64 / m - 1.0;
                    bessel_i0(beta * (1.0 - a * a).max(0.0).sqrt()) / bessel_i0(beta)
                }
                _ => 1.0, // rectangular
            }
        })
        .collect()
}

fn bessel_i0(x: f64) -> f64 {
    // Modified Bessel function I0 approximation (good to 1e-9 for x < 100)
    let mut sum = 1.0;
    let mut term = 1.0;
    let x2 = x * x / 4.0;
    for k in 1..=50u32 {
        term *= x2 / (k * k) as f64;
        sum += term;
        if term < 1e-12 {
            break;
        }
    }
    sum
}

fn action_window(args: &Value) -> Result<String, String> {
    let wtype = args["window_type"].as_str().unwrap_or("hamming");
    let n = args["length"].as_u64().unwrap_or(32) as usize;
    if !(2..=4096).contains(&n) {
        return Err("'length' must be between 2 and 4096.".into());
    }
    let beta = args["beta"].as_f64().unwrap_or(5.0);

    let w = make_window(wtype, n, beta);
    let coherent_gain = w.iter().sum::<f64>() / n as f64;
    let power_gain = (w.iter().map(|v| v * v).sum::<f64>() / n as f64).sqrt();
    let peak = w.iter().cloned().fold(0.0_f64, f64::max);

    // Approximate sidelobe level (rough estimate via spectral analysis)
    let padded = {
        let mut p = w.clone();
        p.extend(vec![0.0; n * 3]);
        p
    };
    let spec = dft(&padded);
    let magnitudes: Vec<f64> = spec.iter().map(|(r, i)| (r * r + i * i).sqrt()).collect();
    let main_mag = magnitudes[0..4].iter().cloned().fold(0.0_f64, f64::max);
    let side_mag = magnitudes[4..magnitudes.len() / 2]
        .iter()
        .cloned()
        .fold(0.0_f64, f64::max);
    let sidelobe_db = if main_mag > 0.0 && side_mag > 0.0 {
        20.0 * (side_mag / main_mag).log10()
    } else {
        f64::NEG_INFINITY
    };

    let mut out = format!("Window Function: {} (N={})\n\n", wtype, n);
    if wtype == "kaiser" {
        out.push_str(&format!("  Kaiser β = {:.2}\n\n", beta));
    }
    out.push_str(&format!("  Coherent gain:  {:.6}\n", coherent_gain));
    out.push_str(&format!("  Power gain:     {:.6}\n", power_gain));
    out.push_str(&format!("  Peak value:     {:.6}\n", peak));
    out.push_str(&format!("  Sidelobe est.:  {:.1} dB\n\n", sidelobe_db));

    out.push_str("Coefficients (visual):\n");
    for (i, &v) in w.iter().enumerate() {
        let bar_len = (v.max(0.0) * 30.0).round() as usize;
        out.push_str(&format!(
            "  w[{:>4}] = {:>8.6}  {}\n",
            i,
            v,
            "█".repeat(bar_len)
        ));
    }
    Ok(out)
}

// ── FIR filter design (windowed sinc) ────────────────────────────────────────

fn sinc(x: f64) -> f64 {
    if x.abs() < 1e-9 {
        1.0
    } else {
        (PI * x).sin() / (PI * x)
    }
}

fn design_fir_lp(cutoff: f64, taps: usize, win: &[f64]) -> Vec<f64> {
    let m = (taps - 1) as f64 / 2.0;
    (0..taps)
        .map(|i| {
            let n = i as f64 - m;
            2.0 * cutoff * sinc(2.0 * cutoff * n) * win[i]
        })
        .collect()
}

fn action_fir(args: &Value) -> Result<String, String> {
    let cutoff = match args["cutoff"].as_f64() {
        Some(c) => c,
        None => return Err("'cutoff' (normalized frequency 0–0.5) is required.".into()),
    };
    if !(0.001..0.499).contains(&cutoff) {
        return Err("'cutoff' must be between 0.001 and 0.499.".into());
    }

    let mut taps = args["taps"].as_u64().unwrap_or(31) as usize;
    if taps < 3 {
        taps = 3;
    }
    if taps.is_multiple_of(2) {
        taps += 1;
    } // force odd

    let ftype = args["filter_type"].as_str().unwrap_or("lowpass");
    let wtype = args["window_type"].as_str().unwrap_or("hamming");
    let beta = args["beta"].as_f64().unwrap_or(5.0);
    let sr = args["sample_rate"].as_f64();

    let win = make_window(wtype, taps, beta);

    let h: Vec<f64> = match ftype {
        "highpass" => {
            let lp = design_fir_lp(cutoff, taps, &win);
            let m = (taps - 1) / 2;
            lp.iter()
                .enumerate()
                .map(|(i, &v)| {
                    let impulse = if i == m { 1.0 } else { 0.0 };
                    impulse - v
                })
                .collect()
        }
        "bandpass" => {
            let c2 = args["cutoff2"].as_f64().unwrap_or(cutoff + 0.1).min(0.499);
            let lp1 = design_fir_lp(c2, taps, &win);
            let lp2 = design_fir_lp(cutoff, taps, &win);
            lp1.iter().zip(lp2.iter()).map(|(a, b)| a - b).collect()
        }
        "bandstop" => {
            let c2 = args["cutoff2"].as_f64().unwrap_or(cutoff + 0.1).min(0.499);
            let lp1 = design_fir_lp(cutoff, taps, &win);
            let lp2 = design_fir_lp(c2, taps, &win);
            let m = (taps - 1) / 2;
            lp1.iter()
                .zip(lp2.iter())
                .enumerate()
                .map(|(i, (a, b))| {
                    let impulse = if i == m { 1.0 } else { 0.0 };
                    impulse - b + a
                })
                .collect()
        }
        _ => design_fir_lp(cutoff, taps, &win), // lowpass
    };

    let mut out = "FIR Filter Design\n\n".to_string();
    out.push_str(&format!("  Type:    {} ({})\n", ftype, wtype));
    out.push_str(&format!("  Taps:    {}\n", taps));
    out.push_str(&format!("  Cutoff:  {:.4}", cutoff));
    if let Some(fs) = sr {
        out.push_str(&format!(" ({:.2} Hz at fs={} Hz)", cutoff * fs, fs));
    }
    out.push('\n');
    if ftype == "bandpass" || ftype == "bandstop" {
        let c2 = args["cutoff2"].as_f64().unwrap_or(cutoff + 0.1);
        out.push_str(&format!("  Cutoff2: {:.4}", c2));
        if let Some(fs) = sr {
            out.push_str(&format!(" ({:.2} Hz)", c2 * fs));
        }
        out.push('\n');
    }

    // Frequency response at key points
    let freqs = [0.0, cutoff / 2.0, cutoff, cutoff * 1.5, 0.25, 0.5];
    out.push_str("\nFrequency Response (sampled):\n");
    out.push_str(&format!(
        "{:<14} {:<14} {:<10}\n",
        "Norm Freq", "Magnitude", "dB"
    ));
    for &f in &freqs {
        if f > 0.5 {
            continue;
        }
        let (re, im) = h.iter().enumerate().fold((0.0, 0.0), |(re, im), (n, &hn)| {
            let angle = 2.0 * PI * f * n as f64;
            (re + hn * angle.cos(), im - hn * angle.sin())
        });
        let mag = (re * re + im * im).sqrt();
        let db = if mag > 1e-12 {
            20.0 * mag.log10()
        } else {
            -120.0
        };
        out.push_str(&format!("  {:<12.4} {:>10.6}    {:>8.2} dB\n", f, mag, db));
    }

    out.push_str("\nCoefficients h[n]:\n");
    for (i, v) in h.iter().enumerate() {
        out.push_str(&format!("  h[{:>4}] = {:>14.10}\n", i, v));
    }
    Ok(out)
}

// ── signal statistics ─────────────────────────────────────────────────────────

fn action_stats(args: &Value) -> Result<String, String> {
    let x = parse_samples(&args["samples"])?;
    if x.is_empty() {
        return Err("'samples' array is required and must be non-empty.".into());
    }
    let n = x.len();

    let mean = x.iter().sum::<f64>() / n as f64;
    let var = x.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / n as f64;
    let std = var.sqrt();
    let rms = (x.iter().map(|v| v * v).sum::<f64>() / n as f64).sqrt();
    let min = x.iter().cloned().fold(f64::INFINITY, f64::min);
    let max = x.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let energy: f64 = x.iter().map(|v| v * v).sum();
    let power = energy / n as f64;

    let mut xs = x.clone();
    xs.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let median = if n % 2 == 0 {
        (xs[n / 2 - 1] + xs[n / 2]) / 2.0
    } else {
        xs[n / 2]
    };

    // Crest factor (peak-to-RMS)
    let crest = max.abs().max(min.abs()) / rms;

    // Zero crossings
    let zero_cross = x
        .windows(2)
        .filter(|w| w[0].signum() != w[1].signum())
        .count();

    // Shannon entropy (histogram with 16 bins)
    let range = max - min;
    let entropy = if range > 0.0 {
        let bins = 16_usize;
        let mut hist = vec![0u32; bins];
        for &v in &x {
            let b = ((v - min) / range * (bins - 1) as f64).round() as usize;
            hist[b.min(bins - 1)] += 1;
        }
        -hist
            .iter()
            .filter(|&&c| c > 0)
            .map(|&c| {
                let p = c as f64 / n as f64;
                p * p.log2()
            })
            .sum::<f64>()
    } else {
        0.0
    };

    let sr = args["sample_rate"].as_f64();
    let mut out = format!("Signal Statistics — {} samples\n\n", n);
    if let Some(fs) = sr {
        out.push_str(&format!("  Duration:     {:.6} s\n", n as f64 / fs));
        out.push_str(&format!("  Sample rate:  {} Hz\n", fs));
    }
    out.push_str(&format!("  Mean:         {:>14.8}\n", mean));
    out.push_str(&format!("  Median:       {:>14.8}\n", median));
    out.push_str(&format!("  Variance:     {:>14.8}\n", var));
    out.push_str(&format!("  Std dev:      {:>14.8}\n", std));
    out.push_str(&format!("  RMS:          {:>14.8}\n", rms));
    out.push_str(&format!("  Min:          {:>14.8}\n", min));
    out.push_str(&format!("  Max:          {:>14.8}\n", max));
    out.push_str(&format!("  Range:        {:>14.8}\n", max - min));
    out.push_str(&format!("  Energy:       {:>14.8}\n", energy));
    out.push_str(&format!("  Power:        {:>14.8}\n", power));
    out.push_str(&format!("  Crest factor: {:>14.8}\n", crest));
    out.push_str(&format!("  Zero crossings: {:>10}\n", zero_cross));
    out.push_str(&format!(
        "  Shannon entropy: {:>10.4} bits (16 bins)\n",
        entropy
    ));
    Ok(out)
}

// ── resample (polyphase) ──────────────────────────────────────────────────────

fn action_resample(args: &Value) -> Result<String, String> {
    let x = parse_samples(&args["samples"])?;
    if x.is_empty() {
        return Err("'samples' array is required.".into());
    }
    let up = args["up"].as_u64().unwrap_or(1) as usize;
    let down = args["down"].as_u64().unwrap_or(1) as usize;
    if up == 0 || down == 0 {
        return Err("'up' and 'down' must be >= 1.".into());
    }
    if up > 64 || down > 64 {
        return Err("'up' and 'down' must be <= 64.".into());
    }

    // Simple polyphase: upsample, apply anti-alias FIR, downsample
    let cutoff = 0.5 / up.max(down) as f64;
    let taps = 31;
    let win = make_window("hamming", taps, 5.0);
    let h = design_fir_lp(cutoff, taps, &win);

    // Upsample
    let mut upsampled = vec![0.0; x.len() * up];
    for (i, &v) in x.iter().enumerate() {
        upsampled[i * up] = v * up as f64;
    }

    // Convolve
    let filtered = convolve(&upsampled, &h);

    // Downsample (skip filter transient)
    let delay = taps / 2;
    let start = delay.min(filtered.len());
    let y: Vec<f64> = filtered[start..].iter().step_by(down).cloned().collect();

    let expected = (x.len() * up).div_ceil(down);
    let mut out = format!("Resample — {}↑ {}↓\n\n", up, down);
    out.push_str(&format!("  Input:    {} samples\n", x.len()));
    out.push_str(&format!(
        "  Output:   {} samples (expected ~{})\n",
        y.len(),
        expected
    ));
    out.push_str(&format!(
        "  Ratio:    {}/{} = {:.6}\n",
        up,
        down,
        up as f64 / down as f64
    ));
    out.push_str(&format!(
        "  Anti-alias cutoff: {:.4} (normalized)\n\n",
        cutoff
    ));
    out.push_str("Output y[n]:\n");
    for (i, v) in y.iter().enumerate() {
        out.push_str(&format!("  y[{:>4}] = {:>12.8}\n", i, v));
    }
    Ok(out)
}

// ── autocorrelation ──────────────────────────────────────────────────────────

fn action_autocorr(args: &Value) -> Result<String, String> {
    let x = parse_samples(&args["samples"])?;
    let n = x.len();
    if n < 2 {
        return Err("Need at least 2 samples.".into());
    }

    let mean = x.iter().sum::<f64>() / n as f64;
    let xm: Vec<f64> = x.iter().map(|v| v - mean).collect();

    let max_lag = (n / 2).min(64);
    let mut out = format!(
        "Autocorrelation — {} samples, up to {} lags\n\n",
        n, max_lag
    );
    out.push_str(&format!(
        "{:<6} {:<14} {:<10} {}\n",
        "Lag", "R[k]", "Norm", "Bar"
    ));
    out.push_str(&"─".repeat(60));
    out.push('\n');

    let mut rk_vals = Vec::with_capacity(max_lag + 1);
    for lag in 0..=max_lag {
        let rk: f64 = (0..n - lag).map(|i| xm[i] * xm[i + lag]).sum();
        rk_vals.push(rk);
    }

    let r0v = rk_vals[0];
    for (lag, &rk) in rk_vals.iter().enumerate() {
        let norm = if r0v.abs() > 1e-12 { rk / r0v } else { 0.0 };
        let bar_len = (norm.abs() * 20.0).round() as usize;
        let bar = if norm >= 0.0 {
            "█".repeat(bar_len)
        } else {
            format!("-{}", "▒".repeat(bar_len))
        };
        out.push_str(&format!(
            "{:<6} {:>12.6}  {:>8.4}  {}\n",
            lag, rk, norm, bar
        ));
    }

    // Find dominant period
    let peak_lag = rk_vals[1..]
        .iter()
        .enumerate()
        .filter(|(_, &v)| v > 0.0)
        .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
        .map(|(i, _)| i + 1);
    if let Some(lag) = peak_lag {
        out.push_str(&format!(
            "\nDominant period estimate: lag = {} samples",
            lag
        ));
        if let Some(fs) = args["sample_rate"].as_f64() {
            out.push_str(&format!(" ({:.4} Hz at fs={} Hz)", fs / lag as f64, fs));
        }
        out.push('\n');
    }
    Ok(out)
}

// ── dispatch ─────────────────────────────────────────────────────────────────

pub async fn execute(args: &Value) -> Result<String, String> {
    let action = args["action"].as_str().unwrap_or("dft");
    match action {
        "dft"      => action_dft(args),
        "idft"     => action_idft(args),
        "convolve" => action_convolve(args),
        "fir"      => action_fir(args),
        "window"   => action_window(args),
        "stats"    => action_stats(args),
        "resample" => action_resample(args),
        "autocorr" => action_autocorr(args),
        other      => Err(format!("Unknown action '{}'. Valid: dft, idft, convolve, fir, window, stats, resample, autocorr.", other)),
    }
}
