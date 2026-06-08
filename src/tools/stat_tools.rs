use serde_json::Value;

pub async fn execute(args: &Value) -> Result<String, String> {
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("describe");
    match action {
        "describe" => describe_action(args),
        "histogram" => histogram_action(args),
        "percentile" => percentile_action(args),
        "mode" => mode_action(args),
        "outliers" => outliers_action(args),
        "zscore" => zscore_action(args),
        "correlate" => correlate_action(args),
        other => Err(format!(
            "stat_tools: unknown action '{other}'. Valid: describe, histogram, percentile, mode, outliers, zscore, correlate"
        )),
    }
}

fn parse_numbers(args: &Value) -> Result<Vec<f64>, String> {
    if let Some(arr) = args
        .get("numbers")
        .or_else(|| args.get("data_a"))
        .or_else(|| args.get("data"))
    {
        if let Some(a) = arr.as_array() {
            let mut nums = Vec::with_capacity(a.len());
            for (i, v) in a.iter().enumerate() {
                let n = v
                    .as_f64()
                    .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
                    .ok_or_else(|| format!("stat_tools: element {i} is not a number"))?;
                nums.push(n);
            }
            return Ok(nums);
        }
        if let Some(s) = arr.as_str() {
            return parse_delimited(s);
        }
    }
    Err("stat_tools: provide 'numbers' (JSON array) or 'data' (comma/space/newline delimited string)".to_string())
}

fn parse_delimited(s: &str) -> Result<Vec<f64>, String> {
    s.split([',', '\n', ' ', '\t', ';'])
        .filter(|s| !s.trim().is_empty())
        .enumerate()
        .map(|(i, tok)| {
            tok.trim()
                .parse::<f64>()
                .map_err(|_| format!("stat_tools: cannot parse '{tok}' as number (element {i})"))
        })
        .collect()
}

fn mean(data: &[f64]) -> f64 {
    data.iter().sum::<f64>() / data.len() as f64
}

fn variance(data: &[f64], m: f64) -> f64 {
    data.iter().map(|x| (x - m).powi(2)).sum::<f64>() / data.len() as f64
}

fn percentile_sorted(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    if sorted.len() == 1 {
        return sorted[0];
    }
    let idx = p / 100.0 * (sorted.len() - 1) as f64;
    let lo = idx.floor() as usize;
    let hi = idx.ceil() as usize;
    if lo == hi {
        sorted[lo]
    } else {
        sorted[lo] + (idx - lo as f64) * (sorted[hi] - sorted[lo])
    }
}

fn format_f(v: f64) -> String {
    if v.fract() == 0.0 && v.abs() < 1e12 {
        format!("{}", v as i64)
    } else {
        format!("{v:.4}")
    }
}

fn describe_action(args: &Value) -> Result<String, String> {
    let data = parse_numbers(args)?;
    if data.is_empty() {
        return Err("stat_tools describe: no data provided".to_string());
    }

    let n = data.len();
    let m = mean(&data);
    let var = variance(&data, m);
    let std = var.sqrt();

    let mut sorted = data.clone();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let min = sorted[0];
    let max = sorted[n - 1];
    let range = max - min;
    let median = percentile_sorted(&sorted, 50.0);
    let q1 = percentile_sorted(&sorted, 25.0);
    let q3 = percentile_sorted(&sorted, 75.0);
    let iqr = q3 - q1;
    let sum: f64 = data.iter().sum();

    let mut out = format!("Descriptive Statistics  (n={n})\n");
    out.push_str(&format!("  Count:    {n}\n"));
    out.push_str(&format!("  Sum:      {}\n", format_f(sum)));
    out.push_str(&format!("  Min:      {}\n", format_f(min)));
    out.push_str(&format!("  Max:      {}\n", format_f(max)));
    out.push_str(&format!("  Range:    {}\n", format_f(range)));
    out.push_str(&format!("  Mean:     {}\n", format_f(m)));
    out.push_str(&format!("  Median:   {}\n", format_f(median)));
    out.push_str(&format!("  Std Dev:  {}\n", format_f(std)));
    out.push_str(&format!("  Variance: {}\n", format_f(var)));
    out.push_str(&format!("  Q1 (25%): {}\n", format_f(q1)));
    out.push_str(&format!("  Q3 (75%): {}\n", format_f(q3)));
    out.push_str(&format!("  IQR:      {}\n", format_f(iqr)));
    Ok(out)
}

fn histogram_action(args: &Value) -> Result<String, String> {
    let data = parse_numbers(args)?;
    if data.is_empty() {
        return Err("stat_tools histogram: no data provided".to_string());
    }

    let bins = args
        .get("bins")
        .and_then(|v| v.as_u64())
        .unwrap_or(10)
        .clamp(2, 50) as usize;
    let width = args.get("width").and_then(|v| v.as_u64()).unwrap_or(40) as usize;

    let min = data.iter().cloned().fold(f64::INFINITY, f64::min);
    let max = data.iter().cloned().fold(f64::NEG_INFINITY, f64::max);

    if (max - min).abs() < 1e-12 {
        return Ok(format!("Histogram: all values equal {}\n", format_f(min)));
    }

    let bin_size = (max - min) / bins as f64;
    let mut counts = vec![0usize; bins];
    for &v in &data {
        let idx = ((v - min) / bin_size).floor() as usize;
        let idx = idx.min(bins - 1);
        counts[idx] += 1;
    }

    let max_count = *counts.iter().max().unwrap_or(&1);
    let mut out = format!("Histogram  (n={}, bins={bins})\n\n", data.len());

    for (i, &count) in counts.iter().enumerate() {
        let lo = min + i as f64 * bin_size;
        let hi = lo + bin_size;
        let bar_len = if max_count > 0 {
            (count * width) / max_count
        } else {
            0
        };
        let bar = "█".repeat(bar_len);
        out.push_str(&format!(
            "  [{:>8} – {:>8}]  {:>4}  {}\n",
            format_f(lo),
            format_f(hi),
            count,
            bar
        ));
    }
    Ok(out)
}

fn percentile_action(args: &Value) -> Result<String, String> {
    let data = parse_numbers(args)?;
    if data.is_empty() {
        return Err("stat_tools percentile: no data provided".to_string());
    }

    let mut sorted = data.clone();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    // Accept a custom list or default to common percentiles
    let percs: Vec<f64> = if let Some(p) = args.get("p").or_else(|| args.get("percentiles")) {
        if let Some(arr) = p.as_array() {
            arr.iter().filter_map(|v| v.as_f64()).collect()
        } else if let Some(n) = p.as_f64() {
            vec![n]
        } else {
            vec![1.0, 5.0, 10.0, 25.0, 50.0, 75.0, 90.0, 95.0, 99.0]
        }
    } else {
        vec![1.0, 5.0, 10.0, 25.0, 50.0, 75.0, 90.0, 95.0, 99.0]
    };

    let mut out = format!("Percentiles  (n={})\n\n", data.len());
    for p in percs {
        let val = percentile_sorted(&sorted, p);
        out.push_str(&format!("  p{:<5}  {}\n", p, format_f(val)));
    }
    Ok(out)
}

fn mode_action(args: &Value) -> Result<String, String> {
    let data = parse_numbers(args)?;
    if data.is_empty() {
        return Err("stat_tools mode: no data provided".to_string());
    }

    // Convert to ordered counts (round to 6 decimal places for grouping)
    let mut counts: std::collections::HashMap<i64, usize> = std::collections::HashMap::new();
    for &v in &data {
        let key = (v * 1_000_000.0).round() as i64;
        *counts.entry(key).or_insert(0) += 1;
    }

    let max_count = *counts.values().max().unwrap_or(&0);
    let mut modes: Vec<(f64, usize)> = counts
        .iter()
        .filter(|(_, &c)| c == max_count)
        .map(|(&k, &c)| (k as f64 / 1_000_000.0, c))
        .collect();
    modes.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

    let top_n = args.get("top").and_then(|v| v.as_u64()).unwrap_or(10) as usize;

    // Also get top-N by frequency
    let mut by_freq: Vec<(f64, usize)> = counts
        .iter()
        .map(|(&k, &c)| (k as f64 / 1_000_000.0, c))
        .collect();
    by_freq.sort_by(|a, b| {
        b.1.cmp(&a.1)
            .then(a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal))
    });

    let mut out = format!("Mode Analysis  (n={})\n\n", data.len());
    out.push_str(&format!("  Unique values: {}\n", counts.len()));

    if max_count == 1 {
        out.push_str("  Mode: none (all values appear exactly once)\n");
    } else {
        out.push_str(&format!(
            "  Mode(s): {} (appears {} times)\n",
            modes
                .iter()
                .map(|(v, _)| format_f(*v))
                .collect::<Vec<_>>()
                .join(", "),
            max_count
        ));
    }

    out.push_str(&format!("\n  Top {} by frequency:\n", top_n));
    for (val, count) in by_freq.iter().take(top_n) {
        let pct = *count as f64 / data.len() as f64 * 100.0;
        out.push_str(&format!(
            "    {}  ×{}  ({:.1}%)\n",
            format_f(*val),
            count,
            pct
        ));
    }
    Ok(out)
}

fn outliers_action(args: &Value) -> Result<String, String> {
    let data = parse_numbers(args)?;
    if data.is_empty() {
        return Err("stat_tools outliers: no data provided".to_string());
    }

    let threshold = args
        .get("threshold")
        .or_else(|| args.get("sigma"))
        .and_then(|v| v.as_f64())
        .unwrap_or(2.0);

    let method = args
        .get("method")
        .and_then(|v| v.as_str())
        .unwrap_or("zscore");

    let m = mean(&data);
    let var = variance(&data, m);
    let std = var.sqrt();

    let mut out = format!(
        "Outliers  (n={}, threshold={threshold}σ, method={method})\n\n",
        data.len()
    );

    if std < 1e-12 {
        out.push_str("  Std dev is effectively zero — no outliers detectable.\n");
        return Ok(out);
    }

    let mut sorted = data.clone();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let (low_fence, high_fence) = if method == "iqr" {
        let q1 = percentile_sorted(&sorted, 25.0);
        let q3 = percentile_sorted(&sorted, 75.0);
        let iqr = q3 - q1;
        (q1 - threshold * iqr, q3 + threshold * iqr)
    } else {
        (m - threshold * std, m + threshold * std)
    };

    let outliers: Vec<(usize, f64, f64)> = data
        .iter()
        .enumerate()
        .filter(|(_, &v)| v < low_fence || v > high_fence)
        .map(|(i, &v)| (i, v, (v - m) / std))
        .collect();

    out.push_str(&format!(
        "  Mean: {}  Std: {}  Fence: [{}, {}]\n\n",
        format_f(m),
        format_f(std),
        format_f(low_fence),
        format_f(high_fence)
    ));

    if outliers.is_empty() {
        out.push_str("  No outliers found.\n");
    } else {
        out.push_str(&format!("  {} outlier(s):\n", outliers.len()));
        for (idx, val, z) in &outliers {
            out.push_str(&format!(
                "    [index {idx}]  value={}  z={:.2}σ  {}\n",
                format_f(*val),
                z,
                if *val > high_fence { "HIGH" } else { "LOW" }
            ));
        }
    }
    Ok(out)
}

fn zscore_action(args: &Value) -> Result<String, String> {
    let data = parse_numbers(args)?;
    if data.is_empty() {
        return Err("stat_tools zscore: no data provided".to_string());
    }

    let m = mean(&data);
    let std = variance(&data, m).sqrt();

    let mut out = format!(
        "Z-Scores  (mean={}, std={})\n\n",
        format_f(m),
        format_f(std)
    );

    if std < 1e-12 {
        out.push_str("  Std dev is effectively zero — all z-scores are 0.\n");
        return Ok(out);
    }

    let limit = data.len().min(100);
    for (i, &v) in data[..limit].iter().enumerate() {
        let z = (v - m) / std;
        out.push_str(&format!("  [{}]  {}  →  z = {:.4}\n", i, format_f(v), z));
    }
    if data.len() > 100 {
        out.push_str(&format!("  ... ({} more values)\n", data.len() - 100));
    }
    Ok(out)
}

fn correlate_action(args: &Value) -> Result<String, String> {
    // Parse a and b arrays
    let parse_arr = |key: &str| -> Result<Vec<f64>, String> {
        args.get(key)
            .ok_or_else(|| format!("stat_tools correlate: '{key}' array required"))
            .and_then(|v| {
                if let Some(arr) = v.as_array() {
                    arr.iter()
                        .enumerate()
                        .map(|(i, x)| {
                            x.as_f64()
                                .or_else(|| x.as_str().and_then(|s| s.parse().ok()))
                                .ok_or_else(|| {
                                    format!("stat_tools: element {i} of '{key}' is not a number")
                                })
                        })
                        .collect()
                } else if let Some(s) = v.as_str() {
                    parse_delimited(s)
                } else {
                    Err(format!(
                        "stat_tools: '{key}' must be an array or delimited string"
                    ))
                }
            })
    };

    let a = parse_arr("a")?;
    let b = parse_arr("b")?;

    if a.len() != b.len() {
        return Err(format!(
            "stat_tools correlate: arrays must be same length (a={}, b={})",
            a.len(),
            b.len()
        ));
    }
    if a.is_empty() {
        return Err("stat_tools correlate: arrays are empty".to_string());
    }

    let n = a.len();
    let ma = mean(&a);
    let mb = mean(&b);
    let std_a = variance(&a, ma).sqrt();
    let std_b = variance(&b, mb).sqrt();

    let pearson = if std_a < 1e-12 || std_b < 1e-12 {
        f64::NAN
    } else {
        a.iter()
            .zip(b.iter())
            .map(|(&x, &y)| (x - ma) * (y - mb))
            .sum::<f64>()
            / (n as f64 * std_a * std_b)
    };

    let interpretation = if pearson.is_nan() {
        "N/A (zero variance in one series)".to_string()
    } else if pearson > 0.9 {
        "very strong positive".to_string()
    } else if pearson > 0.7 {
        "strong positive".to_string()
    } else if pearson > 0.4 {
        "moderate positive".to_string()
    } else if pearson > 0.1 {
        "weak positive".to_string()
    } else if pearson < -0.9 {
        "very strong negative".to_string()
    } else if pearson < -0.7 {
        "strong negative".to_string()
    } else if pearson < -0.4 {
        "moderate negative".to_string()
    } else if pearson < -0.1 {
        "weak negative".to_string()
    } else {
        "negligible".to_string()
    };

    let mut out = format!("Pearson Correlation  (n={n})\n\n");
    out.push_str(&format!(
        "  Series A: mean={} std={}\n",
        format_f(ma),
        format_f(std_a)
    ));
    out.push_str(&format!(
        "  Series B: mean={} std={}\n",
        format_f(mb),
        format_f(std_b)
    ));
    out.push_str(&format!(
        "\n  r = {:.6}\n  Interpretation: {interpretation}\n",
        if pearson.is_nan() { 0.0 } else { pearson }
    ));
    Ok(out)
}
