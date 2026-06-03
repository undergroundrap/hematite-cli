use serde_json::Value;
use std::collections::HashMap;

pub fn make_schema() -> Value {
    serde_json::json!({
        "name": "prometheus_tools",
        "description": "Parse and analyze Prometheus/OpenMetrics text exposition format. Works offline — no network or server required.",
        "parameters": {
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["parse", "list", "metrics", "detail", "labels", "filter", "stats"],
                    "description": "parse/list (default — family table), metrics/detail (per-family detail with samples), labels (label distribution), filter (by name query), stats (totals)"
                },
                "text": { "type": "string", "description": "Prometheus metrics exposition text" },
                "metrics": { "type": "string", "description": "Alias for 'text'" },
                "input": { "type": "string", "description": "Alias for 'text'" },
                "file": { "type": "string", "description": "Path to a metrics exposition file" },
                "filter": { "type": "string", "description": "Name substring filter (for metrics/labels actions)" },
                "q": { "type": "string", "description": "Alias for 'filter'" },
                "limit": { "type": "number", "description": "Max families to show in detail view (default 10)" },
                "query": { "type": "string", "description": "Name substring filter for 'filter' action" }
            }
        }
    })
}

pub async fn execute(args: &Value) -> Result<String, String> {
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("parse");

    let text = if let Some(t) = args
        .get("text")
        .or_else(|| args.get("metrics"))
        .or_else(|| args.get("input"))
        .and_then(|v| v.as_str())
    {
        t.to_string()
    } else if let Some(f) = args.get("file").and_then(|v| v.as_str()) {
        std::fs::read_to_string(f).map_err(|e| format!("Failed to read file: {e}"))?
    } else {
        return Err(
            "Provide 'text' with Prometheus metrics exposition or 'file' path.".to_string(),
        );
    };

    let families = parse_exposition(&text);

    match action {
        "parse" | "list" => action_parse(&families),
        "metrics" | "detail" => action_metrics(&families, args),
        "labels" => action_labels(&families, args),
        "filter" => action_filter(&families, args),
        "stats" => action_stats(&families),
        _ => Err(format!(
            "Unknown action '{}'. Use: parse, metrics, labels, filter, stats.",
            action
        )),
    }
}

#[derive(Debug, Clone, Default)]
struct Sample {
    labels: Vec<(String, String)>,
    value: String,
    suffix: String,
    timestamp: Option<String>,
}

#[derive(Debug, Clone, Default)]
struct Family {
    name: String,
    help: Option<String>,
    mtype: String,
    samples: Vec<Sample>,
}

const HISTOGRAM_SUFFIXES: &[&str] = &["_bucket", "_sum", "_count", "_created"];

fn parse_labels(s: &str) -> Vec<(String, String)> {
    let inner = s
        .trim()
        .trim_start_matches('{')
        .trim_end_matches('}')
        .trim();
    if inner.is_empty() {
        return vec![];
    }
    let mut labels = vec![];
    let chars: Vec<char> = inner.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        while i < chars.len() && (chars[i] == ',' || chars[i] == ' ') {
            i += 1;
        }
        if i >= chars.len() {
            break;
        }
        let mut key = String::new();
        while i < chars.len() && chars[i] != '=' {
            key.push(chars[i]);
            i += 1;
        }
        if i < chars.len() {
            i += 1; // skip =
        }
        if i < chars.len() && chars[i] == '"' {
            i += 1; // skip opening "
        }
        let mut val = String::new();
        while i < chars.len() {
            if chars[i] == '\\' && i + 1 < chars.len() {
                i += 1;
                match chars[i] {
                    'n' => val.push('\n'),
                    '"' => val.push('"'),
                    '\\' => val.push('\\'),
                    c => {
                        val.push('\\');
                        val.push(c);
                    }
                }
            } else if chars[i] == '"' {
                i += 1;
                break;
            } else {
                val.push(chars[i]);
            }
            i += 1;
        }
        let k = key.trim().to_string();
        if !k.is_empty() {
            labels.push((k, val));
        }
    }
    labels
}

fn parse_exposition(text: &str) -> Vec<Family> {
    // First pass: collect all declared TYPE family names
    let mut type_names: std::collections::HashSet<String> = Default::default();
    for line in text.lines() {
        if let Some(rest) = line.trim().strip_prefix("# TYPE ") {
            if let Some(name) = rest.split_whitespace().next() {
                type_names.insert(name.to_string());
            }
        }
    }

    let mut families: HashMap<String, Family> = HashMap::new();
    let mut order: Vec<String> = vec![];

    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        if let Some(rest) = line.strip_prefix("# HELP ") {
            let mut parts = rest.splitn(2, ' ');
            if let Some(name) = parts.next() {
                let help = parts.next().map(|s| s.to_string());
                let f = families.entry(name.to_string()).or_insert_with(|| {
                    order.push(name.to_string());
                    Family {
                        name: name.to_string(),
                        mtype: "untyped".to_string(),
                        ..Default::default()
                    }
                });
                if f.help.is_none() {
                    f.help = help;
                }
            }
        } else if let Some(rest) = line.strip_prefix("# TYPE ") {
            let mut parts = rest.splitn(2, ' ');
            if let (Some(name), Some(mtype)) = (parts.next(), parts.next()) {
                let f = families.entry(name.to_string()).or_insert_with(|| {
                    order.push(name.to_string());
                    Family {
                        name: name.to_string(),
                        ..Default::default()
                    }
                });
                f.mtype = mtype.trim().to_string();
            }
        } else if !line.starts_with('#') {
            // Metric sample: name{labels} value [timestamp]
            let label_start = line.find('{');
            let first_space = line.find(' ');
            let name_end = match (label_start, first_space) {
                (Some(b), Some(s)) => b.min(s),
                (Some(b), None) => b,
                (None, Some(s)) => s,
                _ => continue,
            };
            if name_end == 0 {
                continue;
            }
            let full_name = &line[..name_end];

            // Determine base family name by checking known suffixes
            let (base_name, suffix) = HISTOGRAM_SUFFIXES
                .iter()
                .find_map(|&sfx| {
                    full_name
                        .strip_suffix(sfx)
                        .filter(|b| type_names.contains(*b))
                        .map(|b| (b, sfx))
                })
                .unwrap_or((full_name, ""));

            let labels = if let Some(b) = label_start {
                if let Some(c) = line.find('}') {
                    parse_labels(&line[b..=c])
                } else {
                    vec![]
                }
            } else {
                vec![]
            };

            let after = if let Some(b) = label_start {
                let c = line.find('}').unwrap_or(name_end);
                line[c + 1..].trim()
            } else {
                line[name_end..].trim()
            };

            let mut vp = after.splitn(2, ' ');
            let value = vp.next().unwrap_or("").to_string();
            let timestamp = vp.next().map(|s| s.trim().to_string());
            if value.is_empty() {
                continue;
            }

            let f = families.entry(base_name.to_string()).or_insert_with(|| {
                order.push(base_name.to_string());
                Family {
                    name: base_name.to_string(),
                    mtype: "untyped".to_string(),
                    ..Default::default()
                }
            });
            f.samples.push(Sample {
                labels,
                value,
                suffix: suffix.to_string(),
                timestamp,
            });
        }
    }

    order
        .into_iter()
        .filter_map(|n| families.remove(&n))
        .collect()
}

fn fmt_labels(labels: &[(String, String)]) -> String {
    if labels.is_empty() {
        return String::new();
    }
    let parts: Vec<String> = labels
        .iter()
        .map(|(k, v)| format!("{}=\"{}\"", k, v))
        .collect();
    format!("{{{}}}", parts.join(", "))
}

fn action_parse(families: &[Family]) -> Result<String, String> {
    if families.is_empty() {
        return Ok("No metrics found.".to_string());
    }
    let mut out = format!("{} metric families\n\n", families.len());
    out.push_str(&format!(
        "{:<50} {:<12} {:>7}  {}\n",
        "Metric Family", "Type", "Samples", "Help"
    ));
    out.push_str(&"-".repeat(95));
    out.push('\n');
    for f in families {
        let help = f.help.as_deref().unwrap_or("—");
        let help_short = if help.len() > 30 { &help[..30] } else { help };
        out.push_str(&format!(
            "{:<50} {:<12} {:>7}  {}\n",
            f.name,
            f.mtype,
            f.samples.len(),
            help_short
        ));
    }
    Ok(out)
}

fn action_metrics(families: &[Family], args: &Value) -> Result<String, String> {
    let filter = args
        .get("filter")
        .or_else(|| args.get("q"))
        .and_then(|v| v.as_str());
    let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(10) as usize;
    let mut out = String::new();
    let mut shown = 0usize;
    let total = families.len();
    for f in families {
        if let Some(q) = filter {
            if !f.name.contains(q) {
                continue;
            }
        }
        if shown >= limit {
            out.push_str(&format!(
                "... {} more families (increase 'limit' to see more)\n",
                total.saturating_sub(shown)
            ));
            break;
        }
        shown += 1;
        out.push_str(&format!("━━ {} [{}]\n", f.name, f.mtype.to_uppercase()));
        if let Some(h) = &f.help {
            out.push_str(&format!("   Help: {}\n", h));
        }
        out.push_str(&format!("   Samples: {}\n", f.samples.len()));
        for (i, s) in f.samples.iter().enumerate().take(8) {
            let lbl = fmt_labels(&s.labels);
            let ts = s
                .timestamp
                .as_deref()
                .map(|t| format!("  @{}", t))
                .unwrap_or_default();
            out.push_str(&format!(
                "   [{}] {}{}{} = {}{}\n",
                i + 1,
                f.name,
                s.suffix,
                lbl,
                s.value,
                ts
            ));
        }
        if f.samples.len() > 8 {
            out.push_str(&format!("   ... {} more samples\n", f.samples.len() - 8));
        }
        out.push('\n');
    }
    if out.is_empty() {
        return Ok("No metrics matched.".to_string());
    }
    Ok(out)
}

fn action_labels(families: &[Family], args: &Value) -> Result<String, String> {
    let filter = args
        .get("filter")
        .or_else(|| args.get("metric"))
        .and_then(|v| v.as_str());
    let mut lv: HashMap<String, HashMap<String, usize>> = HashMap::new();
    for f in families {
        if let Some(q) = filter {
            if !f.name.contains(q) {
                continue;
            }
        }
        for s in &f.samples {
            for (k, v) in &s.labels {
                lv.entry(k.clone())
                    .or_default()
                    .entry(v.clone())
                    .and_modify(|c| *c += 1)
                    .or_insert(1);
            }
        }
    }
    if lv.is_empty() {
        return Ok("No labels found.".to_string());
    }
    let mut out = format!("{} unique label names\n\n", lv.len());
    let mut keys: Vec<&String> = lv.keys().collect();
    keys.sort();
    for k in keys {
        let vals = &lv[k];
        let total: usize = vals.values().sum();
        out.push_str(&format!(
            "{} ({} occurrences, {} unique values)\n",
            k,
            total,
            vals.len()
        ));
        let mut sv: Vec<(&String, &usize)> = vals.iter().collect();
        sv.sort_by(|a, b| b.1.cmp(a.1));
        for (v, c) in sv.iter().take(6) {
            out.push_str(&format!("  {:>5}x  {}\n", c, v));
        }
        if sv.len() > 6 {
            out.push_str(&format!("  ... {} more values\n", sv.len() - 6));
        }
        out.push('\n');
    }
    Ok(out)
}

fn action_filter(families: &[Family], args: &Value) -> Result<String, String> {
    let q = args
        .get("query")
        .or_else(|| args.get("q"))
        .or_else(|| args.get("name"))
        .and_then(|v| v.as_str())
        .ok_or("Provide 'query' to filter metrics by name.")?;
    let matched: Vec<Family> = families
        .iter()
        .filter(|f| f.name.contains(q))
        .cloned()
        .collect();
    if matched.is_empty() {
        return Ok(format!("No metrics matching '{}'.", q));
    }
    action_metrics(&matched, args)
}

fn action_stats(families: &[Family]) -> Result<String, String> {
    let mut tc: HashMap<String, usize> = HashMap::new();
    let mut total_samples = 0usize;
    let mut lnames: std::collections::HashSet<String> = Default::default();
    for f in families {
        *tc.entry(f.mtype.clone()).or_insert(0) += 1;
        total_samples += f.samples.len();
        for s in &f.samples {
            for (k, _) in &s.labels {
                lnames.insert(k.clone());
            }
        }
    }
    let mut out = String::from("Prometheus Metrics Stats\n");
    out.push_str(&format!("{}\n\n", "═".repeat(40)));
    out.push_str(&format!("  Metric families : {}\n", families.len()));
    out.push_str(&format!("  Total samples   : {}\n", total_samples));
    out.push_str(&format!("  Unique labels   : {}\n\n", lnames.len()));
    out.push_str("  By type:\n");
    let mut types: Vec<(&String, &usize)> = tc.iter().collect();
    types.sort_by(|a, b| b.1.cmp(a.1));
    for (t, c) in types {
        out.push_str(&format!("    {:<14} {}\n", t, c));
    }
    if !lnames.is_empty() {
        let mut ln: Vec<&String> = lnames.iter().collect();
        ln.sort();
        out.push_str(&format!(
            "\n  Label names: {}\n",
            ln.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", ")
        ));
    }
    Ok(out)
}
