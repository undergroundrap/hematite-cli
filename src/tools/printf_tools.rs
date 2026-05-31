use serde_json::Value;

pub async fn execute(args: &Value) -> Result<String, String> {
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("explain");
    match action {
        "explain" | "parse" => action_explain(args),
        "simulate" => action_simulate(args),
        "validate" => action_validate(args),
        "convert" => action_convert(args),
        other => Err(format!(
            "printf_tools: unknown action '{other}'. Valid: explain, simulate, validate, convert"
        )),
    }
}

// ── Format spec ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
struct FormatSpec {
    full_match: String,
    flags: String,
    width: Option<String>,
    precision: Option<String>,
    type_char: char,
    arg_index: usize,
    is_star_width: bool,
    is_star_precision: bool,
}

fn parse_format_specs(fmt: &str) -> Vec<FormatSpec> {
    let mut specs = Vec::new();
    let chars: Vec<char> = fmt.chars().collect();
    let mut i = 0;
    let mut arg_idx = 1usize;

    while i < chars.len() {
        if chars[i] != '%' {
            i += 1;
            continue;
        }
        let start = i;
        i += 1;
        if i >= chars.len() {
            break;
        }
        // %% literal
        if chars[i] == '%' {
            i += 1;
            continue;
        }
        // flags
        let mut flags = String::new();
        while i < chars.len() && "-+ #0".contains(chars[i]) {
            flags.push(chars[i]);
            i += 1;
        }
        // width
        let mut width: Option<String> = None;
        let mut is_star_width = false;
        if i < chars.len() && chars[i] == '*' {
            is_star_width = true;
            width = Some("*".into());
            i += 1;
            arg_idx += 1; // * consumes an arg
        } else if i < chars.len() && chars[i].is_ascii_digit() {
            let mut w = String::new();
            while i < chars.len() && chars[i].is_ascii_digit() {
                w.push(chars[i]);
                i += 1;
            }
            width = Some(w);
        }
        // precision
        let mut precision: Option<String> = None;
        let mut is_star_precision = false;
        if i < chars.len() && chars[i] == '.' {
            i += 1;
            if i < chars.len() && chars[i] == '*' {
                is_star_precision = true;
                precision = Some("*".into());
                i += 1;
                arg_idx += 1;
            } else {
                let mut p = String::new();
                while i < chars.len() && chars[i].is_ascii_digit() {
                    p.push(chars[i]);
                    i += 1;
                }
                precision = Some(p);
            }
        }
        // length modifier (skip)
        if i < chars.len() && "hlLqjzt".contains(chars[i]) {
            i += 1;
            if i < chars.len() && chars[i] == 'h' {
                i += 1; // hh
            }
            if i < chars.len() && chars[i] == 'l' {
                i += 1; // ll
            }
        }
        // type
        if i >= chars.len() {
            break;
        }
        let type_char = chars[i];
        i += 1;
        let full_match: String = chars[start..i].iter().collect();
        let current_arg = arg_idx;
        arg_idx += 1;
        specs.push(FormatSpec {
            full_match,
            flags,
            width,
            precision,
            type_char,
            arg_index: current_arg,
            is_star_width,
            is_star_precision,
        });
    }
    specs
}

fn type_description(c: char) -> &'static str {
    match c {
        'd' | 'i' => "signed decimal integer",
        'u' => "unsigned decimal integer",
        'f' => "decimal floating-point",
        'e' => "scientific notation (lowercase)",
        'E' => "scientific notation (uppercase)",
        'g' => "shorter of %f/%e (lowercase)",
        'G' => "shorter of %F/%E (uppercase)",
        'x' => "hexadecimal integer (lowercase)",
        'X' => "hexadecimal integer (uppercase)",
        'o' => "octal integer",
        's' => "string",
        'c' => "single character",
        'p' => "pointer address",
        'n' => "⚠ DANGEROUS: writes char count to pointer",
        _ => "unknown specifier",
    }
}

fn arg_type_name(c: char) -> &'static str {
    match c {
        'd' | 'i' | 'u' | 'x' | 'X' | 'o' => "integer",
        'f' | 'e' | 'E' | 'g' | 'G' => "float",
        's' => "string",
        'c' => "char/integer",
        'p' => "pointer",
        'n' => "int*",
        _ => "unknown",
    }
}

// ── actions ───────────────────────────────────────────────────────────────────

fn action_explain(args: &Value) -> Result<String, String> {
    let fmt = args
        .get("format")
        .or_else(|| args.get("fmt"))
        .and_then(|v| v.as_str())
        .ok_or("printf_tools explain: pass 'format' with the format string")?;

    let specs = parse_format_specs(fmt);
    let sep = "─".repeat(54);

    let mut out = format!("Format String: \"{}\"\n{}\n", fmt, sep);

    if specs.is_empty() {
        out.push_str("No format specifiers found.\n");
        return Ok(out);
    }

    out.push_str(&format!("Specifiers: {}\n", specs.len()));

    for spec in &specs {
        out.push('\n');
        out.push_str(&format!(
            "#{:<3} {}  (arg {})\n",
            spec.arg_index, spec.full_match, spec.arg_index
        ));
        out.push_str(&format!(
            "    Type:       {}\n",
            type_description(spec.type_char)
        ));

        if let Some(w) = &spec.width {
            if spec.is_star_width {
                out.push_str("    Width:      * (taken from next argument)\n");
            } else {
                out.push_str(&format!("    Width:      {} (minimum field width)\n", w));
            }
        }
        if let Some(p) = &spec.precision {
            if spec.is_star_precision {
                out.push_str("    Precision:  * (taken from next argument)\n");
            } else if spec.type_char == 's' {
                out.push_str(&format!("    Precision:  {} (max chars from string)\n", p));
            } else {
                out.push_str(&format!(
                    "    Precision:  {} decimal place{}\n",
                    p,
                    if p == "1" { "" } else { "s" }
                ));
            }
        }
        if !spec.flags.is_empty() {
            let mut flag_desc = Vec::new();
            for f in spec.flags.chars() {
                let d = match f {
                    '-' => "- (left-align in field)",
                    '+' => "+ (always show sign)",
                    ' ' => "  (space for positive sign)",
                    '#' => "# (alternate form: 0x prefix for hex, 0 for octal)",
                    '0' => "0 (zero-pad instead of space-pad)",
                    _ => "unknown flag",
                };
                flag_desc.push(d);
            }
            out.push_str(&format!("    Flags:      {}\n", flag_desc.join(", ")));
        }

        // build meaning sentence
        let mut meaning_parts = Vec::new();
        meaning_parts.push(type_description(spec.type_char).to_string());
        if let Some(w) = &spec.width {
            if spec.flags.contains('-') {
                meaning_parts.push(format!("left-aligned in {} char field", w));
            } else if spec.flags.contains('0') {
                meaning_parts.push(format!("zero-padded to {} chars wide", w));
            } else {
                meaning_parts.push(format!("minimum {} chars wide", w));
            }
        }
        if let Some(p) = &spec.precision {
            if !spec.is_star_precision {
                meaning_parts.push(format!("{} decimal places", p));
            }
        }
        if spec.flags.contains('#') {
            if spec.type_char == 'x' || spec.type_char == 'X' {
                meaning_parts.push("with 0x prefix".into());
            } else if spec.type_char == 'o' {
                meaning_parts.push("with 0 prefix".into());
            }
        }
        if spec.type_char == 'n' {
            meaning_parts.clear();
            meaning_parts.push("⚠ SECURITY RISK — writes character count to a pointer".into());
        }
        out.push_str(&format!("    Meaning:    {}\n", meaning_parts.join(", ")));
    }

    out.push('\n');
    let arg_types: Vec<String> = specs
        .iter()
        .map(|s| arg_type_name(s.type_char).to_string())
        .collect();
    out.push_str(&format!(
        "Arguments needed: {} ({})\n",
        specs.len(),
        arg_types.join(", ")
    ));

    let has_n = specs.iter().any(|s| s.type_char == 'n');
    if has_n {
        out.push_str(
            "\n⚠ WARNING: %n specifier is dangerous in C/C++ — never use with untrusted input.\n",
        );
    }

    Ok(out)
}

fn apply_spec(spec: &FormatSpec, val: &Value) -> String {
    let width: usize = spec
        .width
        .as_deref()
        .and_then(|w| w.parse().ok())
        .unwrap_or(0);
    let precision: Option<usize> = spec.precision.as_deref().and_then(|p| p.parse().ok());
    let left_align = spec.flags.contains('-');
    let zero_pad = spec.flags.contains('0') && !left_align;
    let show_sign = spec.flags.contains('+');
    let alt_form = spec.flags.contains('#');

    match spec.type_char {
        's' => {
            let s = val
                .as_str()
                .map(|s| s.to_string())
                .unwrap_or_else(|| val.to_string().trim_matches('"').to_string());
            let s = if let Some(p) = precision {
                s.chars().take(p).collect()
            } else {
                s
            };
            pad_str(&s, width, left_align, ' ')
        }
        'd' | 'i' => {
            let n = val.as_i64().unwrap_or(0);
            let s = if show_sign && n >= 0 {
                format!("+{}", n)
            } else {
                format!("{}", n)
            };
            let pad_char = if zero_pad { '0' } else { ' ' };
            pad_str(&s, width, left_align, pad_char)
        }
        'u' => {
            let n = val.as_u64().unwrap_or(0);
            pad_str(
                &n.to_string(),
                width,
                left_align,
                if zero_pad { '0' } else { ' ' },
            )
        }
        'f' => {
            let f = val.as_f64().unwrap_or(0.0);
            let prec = precision.unwrap_or(6);
            let s = if show_sign && f >= 0.0 {
                format!("+{:.prec$}", f, prec = prec)
            } else {
                format!("{:.prec$}", f, prec = prec)
            };
            pad_str(&s, width, left_align, if zero_pad { '0' } else { ' ' })
        }
        'e' => {
            let f = val.as_f64().unwrap_or(0.0);
            let prec = precision.unwrap_or(6);
            let s = format!("{:.prec$e}", f, prec = prec);
            // Rust uses 'e' notation; reformat to match C style e+02
            pad_str(&s, width, left_align, if zero_pad { '0' } else { ' ' })
        }
        'E' => {
            let f = val.as_f64().unwrap_or(0.0);
            let prec = precision.unwrap_or(6);
            let s = format!("{:.prec$E}", f, prec = prec);
            pad_str(&s, width, left_align, if zero_pad { '0' } else { ' ' })
        }
        'g' | 'G' => {
            let f = val.as_f64().unwrap_or(0.0);
            let prec = precision.unwrap_or(6).max(1);
            // approximate: use {:.*} which uses significant digits
            let s = format!("{:.prec$}", f, prec = prec);
            let s = if spec.type_char == 'G' {
                s.to_uppercase()
            } else {
                s
            };
            pad_str(&s, width, left_align, if zero_pad { '0' } else { ' ' })
        }
        'x' => {
            let n = val.as_i64().unwrap_or(0) as u64;
            let s = if alt_form {
                format!("{:#x}", n)
            } else {
                format!("{:x}", n)
            };
            pad_str(&s, width, left_align, if zero_pad { '0' } else { ' ' })
        }
        'X' => {
            let n = val.as_i64().unwrap_or(0) as u64;
            let s = if alt_form {
                format!("{:#X}", n)
            } else {
                format!("{:X}", n)
            };
            pad_str(&s, width, left_align, if zero_pad { '0' } else { ' ' })
        }
        'o' => {
            let n = val.as_i64().unwrap_or(0) as u64;
            let s = if alt_form {
                format!("{:#o}", n)
            } else {
                format!("{:o}", n)
            };
            pad_str(&s, width, left_align, if zero_pad { '0' } else { ' ' })
        }
        'c' => {
            let ch = if let Some(s) = val.as_str() {
                s.chars().next().unwrap_or(' ')
            } else {
                char::from_u32(val.as_u64().unwrap_or(32) as u32).unwrap_or(' ')
            };
            pad_str(&ch.to_string(), width, left_align, ' ')
        }
        'p' => format!("0x{:016x}", val.as_u64().unwrap_or(0)),
        'n' => "[%n skipped — dangerous]".into(),
        _ => spec.full_match.clone(),
    }
}

fn pad_str(s: &str, width: usize, left_align: bool, pad_char: char) -> String {
    if s.len() >= width {
        return s.to_string();
    }
    let pad = pad_char.to_string().repeat(width - s.len());
    if left_align {
        format!("{}{}", s, pad)
    } else {
        format!("{}{}", pad, s)
    }
}

fn action_simulate(args: &Value) -> Result<String, String> {
    let fmt = args
        .get("format")
        .or_else(|| args.get("fmt"))
        .and_then(|v| v.as_str())
        .ok_or("printf_tools simulate: pass 'format' and 'args' array")?;

    let arg_vals: Vec<Value> = args
        .get("args")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    let specs = parse_format_specs(fmt);
    let sep = "─".repeat(54);

    // Rebuild simulated output from scratch by walking format string
    let mut result = String::new();
    let chars: Vec<char> = fmt.chars().collect();
    let mut out_str = String::new();
    let mut i = 0;
    let mut spec_iter = specs.iter();
    let mut cur_spec = spec_iter.next();

    while i < chars.len() {
        if chars[i] == '%' {
            if i + 1 < chars.len() && chars[i + 1] == '%' {
                out_str.push('%');
                i += 2;
                continue;
            }
            if let Some(spec) = cur_spec {
                let arg_idx = spec.arg_index;
                let val = arg_vals.get(arg_idx - 1).cloned().unwrap_or(Value::Null);
                out_str.push_str(&apply_spec(spec, &val));
                // advance past the spec
                i += spec.full_match.chars().count();
                cur_spec = spec_iter.next();
                continue;
            }
        }
        out_str.push(chars[i]);
        i += 1;
    }
    result = out_str;

    let mut out = format!("Simulate\n{}\n", sep);
    out.push_str(&format!("Format:  \"{}\"\n", fmt));
    out.push_str(&format!(
        "Args:    [{}]\n",
        arg_vals
            .iter()
            .map(|v| v.to_string())
            .collect::<Vec<_>>()
            .join(", ")
    ));
    out.push_str(&format!("\nResult:  \"{}\"\n", result));
    out.push('\n');
    out.push_str(&format!("{}\n", sep));

    // mapping table
    if !specs.is_empty() {
        out.push_str("Spec      Arg         Value         Rendered\n");
        out.push_str(&format!("{}\n", "─".repeat(60)));
        for spec in &specs {
            let val = arg_vals
                .get(spec.arg_index - 1)
                .cloned()
                .unwrap_or(Value::Null);
            let rendered = apply_spec(spec, &val);
            out.push_str(&format!(
                "{:<10}{:<12}{:<14}{}\n",
                spec.full_match,
                format!("arg {}", spec.arg_index),
                val.to_string(),
                rendered
            ));
        }
    }

    Ok(out)
}

fn action_validate(args: &Value) -> Result<String, String> {
    let fmt = args
        .get("format")
        .or_else(|| args.get("fmt"))
        .and_then(|v| v.as_str())
        .ok_or("printf_tools validate: pass 'format' with the format string")?;

    let arg_vals_count = args.get("args").and_then(|v| v.as_array()).map(|a| a.len());

    let specs = parse_format_specs(fmt);
    let mut warnings: Vec<String> = Vec::new();

    for spec in &specs {
        if spec.type_char == 'n' {
            warnings.push(format!(
                "%n specifier at arg {} is dangerous — writes to pointer (format injection attack vector)",
                spec.arg_index
            ));
        }
        let valid_types = "diouxXeEfgGscp%n";
        if !valid_types.contains(spec.type_char) {
            warnings.push(format!(
                "%{} at arg {} is not a standard format specifier",
                spec.type_char, spec.arg_index
            ));
        }
        if spec.is_star_width && arg_vals_count.is_some() {
            // star width was already counted in arg_index
        }
    }

    if let Some(provided) = arg_vals_count {
        let needed = specs.len();
        if provided < needed {
            warnings.push(format!(
                "Too few arguments: {} provided, {} specifiers found (undefined behavior in C)",
                provided, needed
            ));
        } else if provided > needed {
            warnings.push(format!(
                "Too many arguments: {} provided, only {} specifiers (extra args ignored)",
                provided, needed
            ));
        }
    }

    if fmt.contains('\0') {
        warnings.push("Format string contains a null byte — dangerous in C".into());
    }

    let sep = "─".repeat(54);
    let mut out = format!("Validate\n{}\n", sep);
    out.push_str(&format!("Format: \"{}\"\n\n", fmt));

    if warnings.is_empty() {
        out.push_str("✓ VALID — no issues found\n");
    } else {
        out.push_str(&format!(
            "⚠ {} WARNING{}\n\n",
            warnings.len(),
            if warnings.len() == 1 { "" } else { "S" }
        ));
        for w in &warnings {
            out.push_str(&format!("  • {}\n", w));
        }
    }

    Ok(out)
}

fn action_convert(args: &Value) -> Result<String, String> {
    let fmt = args
        .get("format")
        .or_else(|| args.get("fmt"))
        .and_then(|v| v.as_str())
        .ok_or("printf_tools convert: pass 'format' with the format string")?;

    let specs = parse_format_specs(fmt);

    // Build equivalent strings for each language
    let py_percent = fmt.to_string();
    let mut py_fstring = fmt.to_string();
    let mut rust_fmt = fmt.to_string();
    let mut go_fmt = fmt.to_string();
    let mut js_template = fmt.to_string();

    // Replace specifiers in reverse order to avoid offset shifts
    let mut replacements: Vec<(usize, usize, String, String, String, String, String)> = Vec::new();

    let chars: Vec<char> = fmt.chars().collect();
    let mut i = 0;
    let mut spec_idx = 0;

    while i < chars.len() && spec_idx < specs.len() {
        if chars[i] == '%' && i + 1 < chars.len() && chars[i + 1] != '%' {
            let spec = &specs[spec_idx];
            let end = i + spec.full_match.chars().count();
            let n = spec.arg_index;
            let py_p = spec.full_match.clone();
            let py_f = convert_to_py_fstring(spec, n);
            let rs = convert_to_rust(spec, n);
            let go = spec.full_match.clone(); // Go is mostly C-compatible
            let js = format!("${{arg{}}}", n);
            replacements.push((i, end, py_p, py_f, rs, go, js));
            i = end;
            spec_idx += 1;
        } else {
            i += 1;
        }
    }

    // Apply replacements in reverse
    for (start, end, _py_p, py_f, rs, go, js) in replacements.iter().rev() {
        let start_byte = fmt
            .char_indices()
            .nth(*start)
            .map(|(b, _)| b)
            .unwrap_or(fmt.len());
        let end_byte = fmt
            .char_indices()
            .nth(*end)
            .map(|(b, _)| b)
            .unwrap_or(fmt.len());
        py_fstring.replace_range(start_byte..end_byte, py_f);
        rust_fmt.replace_range(start_byte..end_byte, rs);
        go_fmt.replace_range(start_byte..end_byte, go);
        js_template.replace_range(start_byte..end_byte, js);
    }

    let sep = "─".repeat(54);
    let mut out = format!("Convert Format String\n{}\n", sep);
    out.push_str(&format!("Original (C):  \"{}\"\n\n", fmt));
    out.push_str(&format!("Language       Format String\n"));
    out.push_str(&format!("{}\n", "─".repeat(54)));
    out.push_str(&format!("C/C++          \"{}\"\n", fmt));
    out.push_str(&format!("Python (%%)     \"{}\"\n", py_percent));
    out.push_str(&format!("Python f-str   f\"{}\"\n", py_fstring));
    out.push_str(&format!("Rust           \"{}\"\n", rust_fmt));
    out.push_str(&format!("Go             \"{}\"\n", go_fmt));
    out.push_str(&format!("JS template    `{}`\n", js_template));

    if !specs.is_empty() {
        out.push_str(&format!("\n{}\n", sep));
        out.push_str("Notes:\n");
        out.push_str("  • Python %s/%d/%f are compatible with C; %g/%e differ slightly\n");
        out.push_str("  • Rust uses {} placeholders; width/precision follow :width.prec syntax\n");
        out.push_str("  • Go fmt is C-compatible for basic specifiers\n");
        out.push_str("  • JS template literals use ${} interpolation; no width/precision\n");
    }

    Ok(out)
}

fn convert_to_py_fstring(spec: &FormatSpec, n: usize) -> String {
    // Python f-string: {argN:format_spec}
    let mut fs = String::new();
    if spec.flags.contains('-') {
        fs.push('<');
    } else if spec.flags.contains('+') {
        fs.push('+');
    }
    if spec.flags.contains('0') {
        fs.push('0');
    }
    if let Some(w) = &spec.width {
        if w != "*" {
            fs.push_str(w);
        }
    }
    if let Some(p) = &spec.precision {
        if p != "*" {
            fs.push('.');
            fs.push_str(p);
        }
    }
    match spec.type_char {
        'd' | 'i' | 'u' => fs.push('d'),
        'f' => fs.push('f'),
        'e' => fs.push('e'),
        'E' => fs.push('E'),
        'g' => fs.push('g'),
        'G' => fs.push('G'),
        'x' => fs.push('x'),
        'X' => fs.push('X'),
        'o' => fs.push('o'),
        's' => {} // no type needed for string
        'c' => fs.push('c'),
        _ => fs.push(spec.type_char),
    }
    if fs.is_empty() {
        format!("{{arg{}}}", n)
    } else {
        format!("{{arg{}:{}}}", n, fs)
    }
}

fn convert_to_rust(spec: &FormatSpec, n: usize) -> String {
    let mut fs = String::new();
    if spec.flags.contains('<') || spec.flags.contains('-') {
        fs.push('<');
    } else if spec.flags.contains('+') {
        fs.push('+');
    }
    if spec.flags.contains('0') {
        fs.push('0');
    }
    if spec.flags.contains('#') {
        fs.push('#');
    }
    if let Some(w) = &spec.width {
        if w != "*" {
            fs.push_str(w);
        }
    }
    if let Some(p) = &spec.precision {
        if p != "*" {
            fs.push('.');
            fs.push_str(p);
        }
    }
    match spec.type_char {
        'd' | 'i' | 'u' => {} // default {} for integers
        'f' | 'g' | 'G' => {} // default for floats
        'e' => fs.push('e'),
        'E' => fs.push('E'),
        'x' => fs.push('x'),
        'X' => fs.push('X'),
        'o' => fs.push('o'),
        's' => {} // {}
        'c' => {} // {}
        _ => {}
    }
    if fs.is_empty() {
        format!("{{{}}}", n)
    } else {
        format!("{{{}:{}}}", n, fs)
    }
}
