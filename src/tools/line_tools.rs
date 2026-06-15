use regex::Regex;

pub async fn execute(args: &serde_json::Value) -> Result<String, String> {
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("grep");

    match action {
        "grep" | "filter" => grep(args),
        "head" => head(args),
        "tail" => tail(args),
        "sort" => sort(args),
        "unique" | "dedup" => unique(args),
        "count" => count(args),
        "slice" => slice(args),
        "number" | "number-lines" => number_lines(args),
        "join" => join(args),
        "replace" => replace(args),
        "cut" => cut(args),
        other => Err(format!(
            "line_tools: unknown action '{other}'. Valid: grep, head, tail, sort, unique, count, slice, number, join, replace, cut"
        )),
    }
}

// ── Helpers ────────────────────────────────────────────────────────────────────

fn load_text(args: &serde_json::Value) -> Result<String, String> {
    if let Some(text) = args
        .get("text")
        .or_else(|| args.get("input"))
        .and_then(|v| v.as_str())
    {
        return Ok(text.to_string());
    }
    if let Some(file) = args.get("file").and_then(|v| v.as_str()) {
        let path = if std::path::Path::new(file).is_absolute() {
            std::path::PathBuf::from(file)
        } else {
            let root = if let Some(r) = args.get("_root").and_then(|v| v.as_str()) {
                std::path::PathBuf::from(r)
            } else {
                crate::tools::file_ops::workspace_root()
            };
            root.join(file)
        };
        return std::fs::read_to_string(&path)
            .map_err(|e| format!("line_tools: cannot read '{}': {e}", path.display()));
    }
    Err("line_tools: 'text' (inline) or 'file' (path) is required".to_string())
}

fn lines_of(text: &str) -> Vec<&str> {
    text.lines().collect()
}

fn fmt_header(_action: &str, line_count: usize, total: usize) -> String {
    if line_count == total {
        format!("{line_count} line(s)\n")
    } else {
        format!("{line_count} of {total} line(s)\n")
    }
}

// ── Actions ────────────────────────────────────────────────────────────────────

fn grep(args: &serde_json::Value) -> Result<String, String> {
    let text = load_text(args)?;
    let pattern = args
        .get("pattern")
        .or_else(|| args.get("query"))
        .or_else(|| args.get("search"))
        .and_then(|v| v.as_str())
        .ok_or("line_tools grep: 'pattern' is required")?;

    let invert = args
        .get("invert")
        .or_else(|| args.get("v"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let case_insensitive = args
        .get("ignore_case")
        .or_else(|| args.get("i"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let use_regex = args.get("regex").and_then(|v| v.as_bool()).unwrap_or(false);
    let show_numbers = args
        .get("line_numbers")
        .or_else(|| args.get("n"))
        .and_then(|v| v.as_bool())
        .unwrap_or(true);
    let max_results = args.get("max").and_then(|v| v.as_u64()).unwrap_or(500) as usize;

    let all_lines: Vec<&str> = lines_of(&text);
    let total = all_lines.len();

    // Build matcher
    let matches: Vec<(usize, &str)> = if use_regex {
        let re_pattern = if case_insensitive {
            format!("(?i){pattern}")
        } else {
            pattern.to_string()
        };
        let re = Regex::new(&re_pattern)
            .map_err(|e| format!("line_tools grep: invalid regex '{pattern}': {e}"))?;
        all_lines
            .iter()
            .enumerate()
            .filter(|(_, line)| re.is_match(line) != invert)
            .map(|(i, l)| (i + 1, *l))
            .collect()
    } else {
        let needle = if case_insensitive {
            pattern.to_lowercase()
        } else {
            pattern.to_string()
        };
        all_lines
            .iter()
            .enumerate()
            .filter(|(_, line)| {
                let hay = if case_insensitive {
                    line.to_lowercase()
                } else {
                    line.to_string()
                };
                hay.contains(&needle) != invert
            })
            .map(|(i, l)| (i + 1, *l))
            .collect()
    };

    let truncated = matches.len() > max_results;
    let shown: Vec<_> = matches.iter().take(max_results).collect();

    let verb = if invert { "not matching" } else { "matching" };
    let mut out = format!("LINE GREP — {verb} '{pattern}'\n{}\n", "─".repeat(50));

    for (lineno, line) in &shown {
        if show_numbers {
            out.push_str(&format!("{lineno:>6}: {line}\n"));
        } else {
            out.push_str(&format!("{line}\n"));
        }
    }

    out.push('\n');
    out.push_str(&fmt_header("grep", shown.len(), total));
    if truncated {
        out.push_str(&format!(
            "(showing first {max_results} of {} matches)\n",
            matches.len()
        ));
    }
    Ok(out)
}

fn head(args: &serde_json::Value) -> Result<String, String> {
    let text = load_text(args)?;
    let n = args
        .get("n")
        .or_else(|| args.get("count"))
        .and_then(|v| v.as_u64())
        .unwrap_or(10) as usize;

    let all_lines = lines_of(&text);
    let total = all_lines.len();
    let shown: Vec<_> = all_lines.iter().take(n).collect();

    let mut out = format!("LINE HEAD — first {n}\n{}\n", "─".repeat(50));
    for line in &shown {
        out.push_str(line);
        out.push('\n');
    }
    out.push('\n');
    out.push_str(&fmt_header("head", shown.len(), total));
    Ok(out)
}

fn tail(args: &serde_json::Value) -> Result<String, String> {
    let text = load_text(args)?;
    let n = args
        .get("n")
        .or_else(|| args.get("count"))
        .and_then(|v| v.as_u64())
        .unwrap_or(10) as usize;

    let all_lines = lines_of(&text);
    let total = all_lines.len();
    let start = total.saturating_sub(n);
    let shown = &all_lines[start..];

    let mut out = format!("LINE TAIL — last {n}\n{}\n", "─".repeat(50));
    for line in shown {
        out.push_str(line);
        out.push('\n');
    }
    out.push('\n');
    out.push_str(&fmt_header("tail", shown.len(), total));
    Ok(out)
}

fn sort(args: &serde_json::Value) -> Result<String, String> {
    let text = load_text(args)?;
    let reverse = args
        .get("reverse")
        .or_else(|| args.get("desc"))
        .or_else(|| args.get("r"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let numeric = args
        .get("numeric")
        .or_else(|| args.get("n"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let ignore_case = args
        .get("ignore_case")
        .or_else(|| args.get("i"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let dedup = args
        .get("unique")
        .or_else(|| args.get("u"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let mut lines: Vec<String> = lines_of(&text).iter().map(|l| l.to_string()).collect();

    if numeric {
        lines.sort_by(|a, b| {
            let na = a.trim().parse::<f64>().unwrap_or(f64::MAX);
            let nb = b.trim().parse::<f64>().unwrap_or(f64::MAX);
            na.partial_cmp(&nb).unwrap_or(std::cmp::Ordering::Equal)
        });
    } else if ignore_case {
        lines.sort_by_key(|a| a.to_lowercase());
    } else {
        lines.sort();
    }

    if dedup {
        lines.dedup_by(|a, b| {
            if ignore_case {
                a.to_lowercase() == b.to_lowercase()
            } else {
                a == b
            }
        });
    }

    if reverse {
        lines.reverse();
    }

    let total = text.lines().count();
    let mut out = format!("LINE SORT\n{}\n", "─".repeat(50));
    for line in &lines {
        out.push_str(line);
        out.push('\n');
    }
    out.push('\n');
    out.push_str(&fmt_header("sort", lines.len(), total));
    Ok(out)
}

fn unique(args: &serde_json::Value) -> Result<String, String> {
    let text = load_text(args)?;
    let count = args
        .get("count")
        .or_else(|| args.get("c"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let ignore_case = args
        .get("ignore_case")
        .or_else(|| args.get("i"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let sorted = args
        .get("sorted")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let all: Vec<String> = lines_of(&text).iter().map(|l| l.to_string()).collect();
    let total = all.len();

    // Track unique lines while preserving order, optionally counting
    let mut order: Vec<String> = Vec::new();
    let mut freq: std::collections::HashMap<String, usize> = Default::default();
    for line in &all {
        let key = if ignore_case {
            line.to_lowercase()
        } else {
            line.clone()
        };
        let e = freq.entry(key.clone()).or_insert(0);
        if *e == 0 {
            order.push(key);
        }
        *e += 1;
    }

    let mut entries: Vec<(String, usize)> = order
        .into_iter()
        .map(|k| {
            let c = freq[&k];
            (k, c)
        })
        .collect();
    if sorted {
        entries.sort_by_key(|b| std::cmp::Reverse(b.1)); // sort by frequency desc
    }

    let mut out = format!("LINE UNIQUE\n{}\n", "─".repeat(50));
    for (line, freq) in &entries {
        if count {
            out.push_str(&format!("{freq:>6}  {line}\n"));
        } else {
            out.push_str(line);
            out.push('\n');
        }
    }
    out.push('\n');
    out.push_str(&fmt_header("unique", entries.len(), total));
    Ok(out)
}

fn count(args: &serde_json::Value) -> Result<String, String> {
    let text = load_text(args)?;

    let lines = text.lines().count();
    let words = text.split_whitespace().count();
    let chars = text.chars().count();
    let bytes = text.len();
    let blank_lines = text.lines().filter(|l| l.trim().is_empty()).count();
    let non_blank = lines - blank_lines;

    let mut out = format!("LINE COUNT\n{}\n", "─".repeat(50));
    out.push_str(&format!("Lines      : {lines}\n"));
    out.push_str(&format!("  Non-blank: {non_blank}\n"));
    out.push_str(&format!("  Blank    : {blank_lines}\n"));
    out.push_str(&format!("Words      : {words}\n"));
    out.push_str(&format!("Characters : {chars}\n"));
    out.push_str(&format!("Bytes      : {bytes}\n"));
    Ok(out)
}

fn slice(args: &serde_json::Value) -> Result<String, String> {
    let text = load_text(args)?;
    let all_lines = lines_of(&text);
    let total = all_lines.len();

    let from = args
        .get("from")
        .or_else(|| args.get("start"))
        .and_then(|v| v.as_u64())
        .unwrap_or(1)
        .saturating_sub(1) as usize;

    let to = args
        .get("to")
        .or_else(|| args.get("end"))
        .and_then(|v| v.as_u64())
        .map(|n| n as usize)
        .unwrap_or(total)
        .min(total);

    if from >= total {
        return Ok(format!(
            "LINE SLICE\n{}\n(from={} exceeds total {total} lines)\n",
            "─".repeat(50),
            from + 1
        ));
    }

    let shown = &all_lines[from..to];

    let mut out = format!(
        "LINE SLICE — lines {}–{}\n{}\n",
        from + 1,
        to,
        "─".repeat(50)
    );
    for line in shown {
        out.push_str(line);
        out.push('\n');
    }
    out.push('\n');
    out.push_str(&fmt_header("slice", shown.len(), total));
    Ok(out)
}

fn number_lines(args: &serde_json::Value) -> Result<String, String> {
    let text = load_text(args)?;
    let start = args.get("start").and_then(|v| v.as_u64()).unwrap_or(1) as usize;
    let step = args.get("step").and_then(|v| v.as_u64()).unwrap_or(1) as usize;
    let skip_blank = args
        .get("skip_blank")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let all_lines = lines_of(&text);
    let width = {
        let last_num = start + (all_lines.len() - 1) * step;
        last_num.to_string().len()
    };

    let mut out = format!("LINE NUMBER\n{}\n", "─".repeat(50));
    let mut n = start;
    for line in &all_lines {
        if skip_blank && line.trim().is_empty() {
            out.push('\n');
        } else {
            out.push_str(&format!("{n:>width$}: {line}\n"));
            n += step;
        }
    }
    Ok(out)
}

fn join(args: &serde_json::Value) -> Result<String, String> {
    let text = load_text(args)?;
    let sep = args
        .get("sep")
        .or_else(|| args.get("separator"))
        .and_then(|v| v.as_str())
        .unwrap_or(", ");
    let trim = args.get("trim").and_then(|v| v.as_bool()).unwrap_or(true);
    let skip_blank = args
        .get("skip_blank")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);

    let mut lines: Vec<String> = text
        .lines()
        .map(|l| {
            if trim {
                l.trim().to_string()
            } else {
                l.to_string()
            }
        })
        .collect();

    if skip_blank {
        lines.retain(|l| !l.is_empty());
    }

    let joined = lines.join(sep);
    let mut out = format!("LINE JOIN\n{}\n", "─".repeat(50));
    out.push_str(&joined);
    out.push('\n');
    Ok(out)
}

fn replace(args: &serde_json::Value) -> Result<String, String> {
    let text = load_text(args)?;
    let from = args
        .get("from")
        .or_else(|| args.get("pattern"))
        .and_then(|v| v.as_str())
        .ok_or("line_tools replace: 'from' is required")?;
    let to = args
        .get("to")
        .or_else(|| args.get("replacement"))
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let use_regex = args.get("regex").and_then(|v| v.as_bool()).unwrap_or(false);
    let case_insensitive = args
        .get("ignore_case")
        .or_else(|| args.get("i"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let limit = args
        .get("limit")
        .and_then(|v| v.as_u64())
        .map(|n| n as usize);

    let result = if use_regex {
        let re_pattern = if case_insensitive {
            format!("(?i){from}")
        } else {
            from.to_string()
        };
        let re = Regex::new(&re_pattern)
            .map_err(|e| format!("line_tools replace: invalid regex '{from}': {e}"))?;
        match limit {
            Some(n) => re.replacen(&text, n, to).to_string(),
            None => re.replace_all(&text, to).to_string(),
        }
    } else {
        match limit {
            Some(n) => {
                let mut result = text.to_string();
                let mut count = 0;
                while count < n {
                    if let Some(pos) = result.find(from) {
                        result.replace_range(pos..pos + from.len(), to);
                        count += 1;
                    } else {
                        break;
                    }
                }
                result
            }
            None => text.replace(from, to),
        }
    };

    let changes = if use_regex {
        let re_pattern = if case_insensitive {
            format!("(?i){from}")
        } else {
            from.to_string()
        };
        let re = Regex::new(&re_pattern).ok();
        re.as_ref().map(|r| r.find_iter(&text).count()).unwrap_or(0)
    } else {
        text.matches(from).count()
    };

    let mut out = format!("LINE REPLACE\n{}\n", "─".repeat(50));
    out.push_str(&result);
    if !result.ends_with('\n') {
        out.push('\n');
    }
    out.push('\n');
    out.push_str(&format!("({changes} replacement(s) made)\n"));
    Ok(out)
}

fn cut(args: &serde_json::Value) -> Result<String, String> {
    let text = load_text(args)?;
    let delimiter = args
        .get("delimiter")
        .or_else(|| args.get("sep"))
        .or_else(|| args.get("d"))
        .and_then(|v| v.as_str())
        .unwrap_or("\t");
    let field = args
        .get("field")
        .or_else(|| args.get("f"))
        .and_then(|v| v.as_u64())
        .unwrap_or(1)
        .saturating_sub(1) as usize;

    let all_lines = lines_of(&text);
    let total = all_lines.len();
    let mut out = format!(
        "LINE CUT — field {} (delimiter: {:?})\n{}\n",
        field + 1,
        delimiter,
        "─".repeat(50)
    );
    let mut found = 0;

    for line in &all_lines {
        let parts: Vec<&str> = line.split(delimiter).collect();
        if let Some(val) = parts.get(field) {
            out.push_str(val);
            out.push('\n');
            found += 1;
        } else {
            out.push('\n');
        }
    }

    out.push('\n');
    out.push_str(&fmt_header("cut", found, total));
    Ok(out)
}
