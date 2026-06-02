use serde_json::{json, Value};

pub fn latex_tools_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "action": {
                "type": "string",
                "enum": ["escape", "table", "equation", "template", "strip", "symbols", "convert"],
                "description": "escape: escape special chars for LaTeX | table: generate LaTeX table | equation: format a math expression | template: generate a document template | strip: remove LaTeX markup | symbols: look up LaTeX symbols | convert: convert markdown to LaTeX"
            },
            "text": {"type": "string", "description": "Input text for escape/strip/convert actions"},
            "headers": {"type": "array", "items": {"type": "string"}, "description": "Column headers for table action"},
            "rows": {"type": "array", "items": {"type": "array"}, "description": "2D array of table row data"},
            "caption": {"type": "string", "description": "Table caption"},
            "label": {"type": "string", "description": "LaTeX \\label for cross-referencing"},
            "expression": {"type": "string", "description": "Math expression for equation action"},
            "env": {"type": "string", "description": "Math environment: equation/align/gather/multline (default: equation)"},
            "numbered": {"type": "boolean", "description": "Numbered equation (default: true)"},
            "type": {
                "type": "string",
                "enum": ["article", "report", "book", "beamer", "letter"],
                "description": "Document type for template action (default: article)"
            },
            "title": {"type": "string", "description": "Document title for template"},
            "author": {"type": "string", "description": "Document author for template"},
            "packages": {"type": "array", "items": {"type": "string"}, "description": "Extra packages to include in template"},
            "query": {"type": "string", "description": "Symbol name or description to search"},
            "border": {"type": "string", "description": "Table border style: full/outer/none (default: full)"},
            "position": {"type": "string", "description": "Table float position (default: h)"}
        },
        "required": []
    })
}

fn escape_latex(text: &str) -> String {
    let mut out = String::with_capacity(text.len() * 2);
    for ch in text.chars() {
        match ch {
            '&' => out.push_str(r"\&"),
            '%' => out.push_str(r"\%"),
            '$' => out.push_str(r"\$"),
            '#' => out.push_str(r"\#"),
            '_' => out.push_str(r"\_"),
            '{' => out.push_str(r"\{"),
            '}' => out.push_str(r"\}"),
            '~' => out.push_str(r"\textasciitilde{}"),
            '^' => out.push_str(r"\textasciicircum{}"),
            '\\' => out.push_str(r"\textbackslash{}"),
            '<' => out.push_str(r"\textless{}"),
            '>' => out.push_str(r"\textgreater{}"),
            c => out.push(c),
        }
    }
    out
}

fn action_escape(args: &Value) -> String {
    let text = args.get("text").and_then(|v| v.as_str()).unwrap_or("");
    if text.is_empty() {
        return "Provide 'text' to escape for LaTeX.".to_string();
    }
    let escaped = escape_latex(text);
    let mut out = String::from("LaTeX ESCAPE\n============\n\n");
    out.push_str(&format!("Input  : {}\nEscaped: {}\n", text, escaped));
    out.push_str("\nCharacters escaped: & % $ # _ { } ~ ^ \\ < >");
    out
}

fn action_table(args: &Value) -> String {
    let headers: Vec<String> = args
        .get("headers")
        .and_then(|v| v.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();
    let rows: Vec<Vec<String>> = args
        .get("rows")
        .and_then(|v| v.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|row| {
                    row.as_array().map(|cells| {
                        cells
                            .iter()
                            .map(|c| match c {
                                Value::String(s) => s.clone(),
                                Value::Number(n) => n.to_string(),
                                Value::Bool(b) => b.to_string(),
                                _ => String::new(),
                            })
                            .collect()
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    if headers.is_empty() && rows.is_empty() {
        return "Provide 'headers' and 'rows' arrays to generate a LaTeX table.".to_string();
    }
    let ncols = headers
        .len()
        .max(rows.iter().map(|r| r.len()).max().unwrap_or(0));
    let caption = args.get("caption").and_then(|v| v.as_str()).unwrap_or("");
    let label = args.get("label").and_then(|v| v.as_str()).unwrap_or("");
    let border = args
        .get("border")
        .and_then(|v| v.as_str())
        .unwrap_or("full");
    let pos = args.get("position").and_then(|v| v.as_str()).unwrap_or("h");
    let col_spec = match border {
        "none" => (0..ncols).map(|_| "c").collect::<Vec<_>>().join(" "),
        "outer" => format!(
            "|{}|",
            (0..ncols).map(|_| "c").collect::<Vec<_>>().join(" ")
        ),
        _ => format!(
            "|{}|",
            (0..ncols).map(|_| "c").collect::<Vec<_>>().join("|")
        ),
    };
    let hrule = match border {
        "none" => "",
        _ => "\\hline\n",
    };
    let mut out = String::new();
    out.push_str(&format!("\\begin{{table}}[{}]\n  \\centering\n", pos));
    if !caption.is_empty() {
        out.push_str(&format!("  \\caption{{{}}}\n", escape_latex(caption)));
    }
    if !label.is_empty() {
        out.push_str(&format!("  \\label{{{}}}\n", label));
    }
    out.push_str(&format!("  \\begin{{tabular}}{{{}}}\n", col_spec));
    out.push_str(&format!("    {}", hrule));
    if !headers.is_empty() {
        let header_row = headers
            .iter()
            .map(|h| format!("\\textbf{{{}}}", escape_latex(h)))
            .collect::<Vec<_>>()
            .join(" & ");
        out.push_str(&format!("    {} \\\\\n", header_row));
        out.push_str(&format!("    {}", hrule));
    }
    for row in &rows {
        let mut cells: Vec<String> = row.iter().map(|c| escape_latex(c)).collect();
        while cells.len() < ncols {
            cells.push(String::new());
        }
        out.push_str(&format!("    {} \\\\\n", cells.join(" & ")));
        out.push_str(&format!("    {}", hrule));
    }
    out.push_str("  \\end{tabular}\n\\end{table}");
    out
}

fn action_equation(args: &Value) -> String {
    let expr = args
        .get("expression")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    if expr.is_empty() {
        return "Provide 'expression' for the equation (e.g. 'E = mc^2').".to_string();
    }
    let env = args
        .get("env")
        .and_then(|v| v.as_str())
        .unwrap_or("equation");
    let numbered = args
        .get("numbered")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);
    let env_name = if numbered {
        env.to_string()
    } else {
        format!("{}*", env)
    };
    let label = args.get("label").and_then(|v| v.as_str()).unwrap_or("");
    let mut out = format!("\\begin{{{}}}\n", env_name);
    out.push_str(&format!("  {}\n", expr));
    if !label.is_empty() {
        out.push_str(&format!("  \\label{{{}}}\n", label));
    }
    out.push_str(&format!("\\end{{{}}}", env_name));
    out
}

fn action_template(args: &Value) -> String {
    let doc_type = args
        .get("type")
        .and_then(|v| v.as_str())
        .unwrap_or("article");
    let title = args
        .get("title")
        .and_then(|v| v.as_str())
        .unwrap_or("My Document");
    let author = args
        .get("author")
        .and_then(|v| v.as_str())
        .unwrap_or("Author Name");
    let extra_pkgs: Vec<String> = args
        .get("packages")
        .and_then(|v| v.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();
    let mut out = String::new();
    out.push_str(&format!("\\documentclass{{{}}}\n\n", doc_type));
    // Default packages per type
    let defaults: &[&str] = match doc_type {
        "beamer" => &["inputenc", "fontenc", "babel", "graphicx"],
        "letter" => &["inputenc", "fontenc"],
        _ => &[
            "inputenc", "fontenc", "babel", "amsmath", "amssymb", "graphicx", "geometry",
            "hyperref",
        ],
    };
    for pkg in defaults {
        out.push_str(&format!("\\usepackage{{{}}}\n", pkg));
    }
    for pkg in &extra_pkgs {
        out.push_str(&format!("\\usepackage{{{}}}\n", pkg));
    }
    out.push('\n');
    match doc_type {
        "beamer" => {
            out.push_str(&format!(
                "\\title{{{}}}\n\\author{{{}}}\n\\date{{\\today}}\n\n",
                escape_latex(title),
                escape_latex(author)
            ));
            out.push_str("\\begin{document}\n\\maketitle\n\n\\begin{frame}{Introduction}\n  % Frame content\n\\end{frame}\n\n\\end{document}\n");
        }
        "letter" => {
            out.push_str("\\begin{document}\n\n\\begin{letter}{Recipient Name\\\\Address}\n\n\\opening{Dear Sir/Madam,}\n\n% Letter body\n\n\\closing{Yours faithfully,}\n\n\\end{letter}\n\\end{document}\n");
        }
        "book" => {
            out.push_str(&format!(
                "\\title{{{}}}\n\\author{{{}}}\n\\date{{\\today}}\n\n",
                escape_latex(title),
                escape_latex(author)
            ));
            out.push_str("\\begin{document}\n\\maketitle\n\\tableofcontents\n\n\\chapter{Introduction}\n\n% Chapter content\n\n\\end{document}\n");
        }
        "report" => {
            out.push_str(&format!(
                "\\title{{{}}}\n\\author{{{}}}\n\\date{{\\today}}\n\n",
                escape_latex(title),
                escape_latex(author)
            ));
            out.push_str("\\begin{document}\n\\maketitle\n\\tableofcontents\n\n\\section{Introduction}\n\n% Section content\n\n\\end{document}\n");
        }
        _ => {
            out.push_str(&format!(
                "\\title{{{}}}\n\\author{{{}}}\n\\date{{\\today}}\n\n",
                escape_latex(title),
                escape_latex(author)
            ));
            out.push_str("\\begin{document}\n\\maketitle\n\n\\section{Introduction}\n\n% Section content\n\n\\end{document}\n");
        }
    }
    out
}

fn action_strip(args: &Value) -> String {
    let text = args.get("text").and_then(|v| v.as_str()).unwrap_or("");
    if text.is_empty() {
        return "Provide 'text' containing LaTeX markup to strip.".to_string();
    }
    let mut out = String::new();
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\\' {
            // Skip command name
            while chars
                .peek()
                .map(|c| c.is_ascii_alphabetic())
                .unwrap_or(false)
            {
                chars.next();
            }
            // Skip optional [] arg
            if chars.peek() == Some(&'[') {
                while chars.peek().map(|c| *c != ']').unwrap_or(false) {
                    chars.next();
                }
                chars.next(); // consume ]
            }
            // Pass through {} content
            if chars.peek() == Some(&'{') {
                chars.next(); // consume {
                let mut depth = 1;
                while let Some(c) = chars.next() {
                    if c == '{' {
                        depth += 1;
                    } else if c == '}' {
                        depth -= 1;
                        if depth == 0 {
                            break;
                        }
                    } else {
                        out.push(c);
                    }
                }
            }
        } else if c == '$' {
            // Inline math -- keep content
            while let Some(c) = chars.next() {
                if c == '$' {
                    break;
                }
                out.push(c);
            }
        } else if c == '{' || c == '}' {
            // Skip bare braces
        } else {
            out.push(c);
        }
    }
    let stripped = out.split_whitespace().collect::<Vec<_>>().join(" ");
    format!("LaTeX STRIP\n===========\n\nStripped: {}", stripped)
}

fn action_symbols(args: &Value) -> String {
    let query = args
        .get("query")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_lowercase();
    let symbols: &[(&str, &str, &str)] = &[
        ("alpha", r"\alpha", "Greek: alpha"),
        ("beta", r"\beta", "Greek: beta"),
        ("gamma", r"\gamma", "Greek: gamma"),
        ("delta", r"\delta", "Greek: delta"),
        ("Delta", r"\Delta", "Greek: Delta"),
        ("epsilon", r"\epsilon", "Greek: epsilon"),
        ("zeta", r"\zeta", "Greek: zeta"),
        ("eta", r"\eta", "Greek: eta"),
        ("theta", r"\theta", "Greek: theta"),
        ("Theta", r"\Theta", "Greek: Theta"),
        ("lambda", r"\lambda", "Greek: lambda"),
        ("Lambda", r"\Lambda", "Greek: Lambda"),
        ("mu", r"\mu", "Greek: mu"),
        ("nu", r"\nu", "Greek: nu"),
        ("xi", r"\xi", "Greek: xi"),
        ("pi", r"\pi", "Greek: pi"),
        ("Pi", r"\Pi", "Greek: Pi"),
        ("rho", r"\rho", "Greek: rho"),
        ("sigma", r"\sigma", "Greek: sigma"),
        ("Sigma", r"\Sigma", "Greek: Sigma"),
        ("tau", r"\tau", "Greek: tau"),
        ("phi", r"\phi", "Greek: phi"),
        ("Phi", r"\Phi", "Greek: Phi"),
        ("chi", r"\chi", "Greek: chi"),
        ("psi", r"\psi", "Greek: psi"),
        ("omega", r"\omega", "Greek: omega"),
        ("Omega", r"\Omega", "Greek: Omega"),
        ("partial", r"\partial", "Math: partial derivative"),
        ("nabla", r"\nabla", "Math: del/gradient"),
        ("infty", r"\infty", "Math: infinity"),
        ("sum", r"\sum", "Math: summation"),
        ("prod", r"\prod", "Math: product"),
        ("int", r"\int", "Math: integral"),
        ("oint", r"\oint", "Math: contour integral"),
        ("sqrt", r"\sqrt{x}", "Math: square root"),
        ("frac", r"\frac{a}{b}", "Math: fraction a/b"),
        ("cdot", r"\cdot", "Math: center dot"),
        ("times", r"\times", "Math: multiplication"),
        ("div", r"\div", "Math: division"),
        ("pm", r"\pm", "Math: plus-minus"),
        ("leq", r"\leq", "Math: less or equal"),
        ("geq", r"\geq", "Math: greater or equal"),
        ("neq", r"\neq", "Math: not equal"),
        ("approx", r"\approx", "Math: approximately equal"),
        ("equiv", r"\equiv", "Math: equivalent"),
        ("forall", r"\forall", "Logic: for all"),
        ("exists", r"\exists", "Logic: there exists"),
        ("in", r"\in", "Set: element of"),
        ("notin", r"\notin", "Set: not element"),
        ("subset", r"\subset", "Set: subset"),
        ("cup", r"\cup", "Set: union"),
        ("cap", r"\cap", "Set: intersection"),
        ("emptyset", r"\emptyset", "Set: empty set"),
        ("rightarrow", r"\rightarrow", "Arrow: right"),
        ("leftarrow", r"\leftarrow", "Arrow: left"),
        ("Rightarrow", r"\Rightarrow", "Arrow: double right"),
        ("leftrightarrow", r"\leftrightarrow", "Arrow: left-right"),
        ("hbar", r"\hbar", "Physics: reduced Planck constant"),
        ("ell", r"\ell", "Physics: script l"),
        ("Re", r"\Re", "Math: real part"),
        ("Im", r"\Im", "Math: imaginary part"),
        ("dag", r"\dagger", "Math: dagger"),
        ("hat", r"\hat{x}", "Accent: hat"),
        ("bar", r"\bar{x}", "Accent: bar"),
        ("vec", r"\vec{x}", "Accent: vector arrow"),
        ("dot", r"\dot{x}", "Accent: dot"),
        ("ddot", r"\ddot{x}", "Accent: double dot"),
        ("tilde", r"\tilde{x}", "Accent: tilde"),
        ("boldsymbol", r"\boldsymbol{x}", "Bold math symbol"),
        ("mathbb", r"\mathbb{R}", "Blackboard bold"),
        ("mathcal", r"\mathcal{L}", "Calligraphic"),
        ("text", r"\text{text}", "Text in math mode"),
        ("label", r"\label{key}", "Cross-reference label"),
        ("ref", r"\ref{key}", "Reference to label"),
        ("cite", r"\cite{key}", "Citation"),
        ("begin", r"\begin{env}", "Environment"),
    ];
    let matches: Vec<&(&str, &str, &str)> = if query.is_empty() {
        symbols.iter().collect()
    } else {
        symbols
            .iter()
            .filter(|(name, cmd, desc)| {
                name.to_lowercase().contains(&query)
                    || cmd.to_lowercase().contains(&query)
                    || desc.to_lowercase().contains(&query)
            })
            .collect()
    };
    if matches.is_empty() {
        return format!(
            "No symbols matching '{}'. Try 'alpha', 'integral', 'arrow', 'set', 'physics'.",
            query
        );
    }
    let header = if query.is_empty() {
        String::from("LaTeX SYMBOLS")
    } else {
        format!("LaTeX SYMBOLS ({})", query)
    };
    let mut out = format!("{}\n{}\n\n", header, "=".repeat(20));
    out.push_str(&format!(
        "{:<18} {:<30} {}\n",
        "Name", "Command", "Description"
    ));
    out.push_str(&format!("{}\n", "-".repeat(70)));
    for (name, cmd, desc) in &matches {
        out.push_str(&format!("{:<18} {:<30} {}\n", name, cmd, desc));
    }
    out.trim_end().to_string()
}

fn action_convert(args: &Value) -> String {
    let text = args.get("text").and_then(|v| v.as_str()).unwrap_or("");
    if text.is_empty() {
        return "Provide 'text' with Markdown content to convert to LaTeX.".to_string();
    }
    let mut out = String::new();
    for line in text.lines() {
        let l = line.trim();
        if l.starts_with("### ") {
            out.push_str(&format!("\\subsubsection{{{}}}\n", escape_latex(&l[4..])));
        } else if l.starts_with("## ") {
            out.push_str(&format!("\\subsection{{{}}}\n", escape_latex(&l[3..])));
        } else if l.starts_with("# ") {
            out.push_str(&format!("\\section{{{}}}\n", escape_latex(&l[2..])));
        } else if l.starts_with("- ") || l.starts_with("* ") {
            out.push_str(&format!("\\item {}\n", escape_latex(&l[2..])));
        } else if l.is_empty() {
            out.push('\n');
        } else {
            // Inline bold/italic/code via simple find-based replacement
            let mut processed = l.to_string();
            // **bold** -> \textbf{bold}
            while let Some(s) = processed.find("**") {
                if let Some(e) = processed[s + 2..].find("**").map(|i| i + s + 2) {
                    let inner = processed[s + 2..e].to_string();
                    processed = format!(
                        "{}\\textbf{{{}}}{}",
                        &processed[..s],
                        escape_latex(&inner),
                        &processed[e + 2..]
                    );
                } else {
                    break;
                }
            }
            // *italic* -> \textit{italic}
            while let Some(s) = processed.find('*') {
                if let Some(e) = processed[s + 1..].find('*').map(|i| i + s + 1) {
                    let inner = processed[s + 1..e].to_string();
                    processed = format!(
                        "{}\\textit{{{}}}{}",
                        &processed[..s],
                        escape_latex(&inner),
                        &processed[e + 1..]
                    );
                } else {
                    break;
                }
            }
            // `code` -> \texttt{code}
            while let Some(s) = processed.find('`') {
                if let Some(e) = processed[s + 1..].find('`').map(|i| i + s + 1) {
                    let inner = processed[s + 1..e].to_string();
                    processed = format!(
                        "{}\\texttt{{{}}}{}",
                        &processed[..s],
                        escape_latex(&inner),
                        &processed[e + 1..]
                    );
                } else {
                    break;
                }
            }
            out.push_str(&processed);
            out.push('\n');
        }
    }
    format!("LaTeX CONVERSION\n================\n\n{}", out.trim())
}

pub async fn execute(args: &Value) -> Result<String, String> {
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("escape");
    Ok(match action {
        "escape" => action_escape(args),
        "table" => action_table(args),
        "equation" => action_equation(args),
        "template" => action_template(args),
        "strip" => action_strip(args),
        "symbols" => action_symbols(args),
        "convert" => action_convert(args),
        other => format!(
            "Unknown action '{}'. Use: escape, table, equation, template, strip, symbols, convert",
            other
        ),
    })
}
