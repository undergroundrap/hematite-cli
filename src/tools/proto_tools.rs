use serde_json::Value;

pub async fn execute(args: &Value) -> Result<String, String> {
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("info");
    match action {
        "info" => info_action(args),
        "messages" => messages_action(args),
        "services" => services_action(args),
        "validate" => validate_action(args),
        _ => Err(format!(
            "Unknown action '{}'. Valid: info, messages, services, validate",
            action
        )),
    }
}

fn get_text(args: &Value) -> Result<String, String> {
    args.get("text")
        .or_else(|| args.get("proto"))
        .or_else(|| args.get("content"))
        .or_else(|| args.get("input"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| "Missing 'text' — pass the .proto file content as a string".to_string())
}

// ── Parser ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
struct ProtoFile {
    syntax: String,
    package: Option<String>,
    imports: Vec<String>,
    options: Vec<(String, String)>,
    messages: Vec<Message>,
    enums: Vec<ProtoEnum>,
    services: Vec<Service>,
}

#[derive(Debug, Clone)]
struct Field {
    label: String, // optional, repeated, required, (empty for proto3 scalar)
    field_type: String,
    name: String,
    number: u32,
    options: Vec<String>, // e.g. deprecated=true
}

#[derive(Debug, Clone)]
struct Message {
    name: String,
    fields: Vec<Field>,
    nested_messages: Vec<String>,
    nested_enums: Vec<String>,
    oneofs: Vec<(String, Vec<Field>)>,
    reserved: Vec<String>,
}

#[derive(Debug, Clone)]
struct EnumValue {
    name: String,
    number: i32,
}

#[derive(Debug, Clone)]
struct ProtoEnum {
    name: String,
    values: Vec<EnumValue>,
}

#[derive(Debug, Clone)]
struct RpcMethod {
    name: String,
    input: String,
    output: String,
    client_streaming: bool,
    server_streaming: bool,
}

#[derive(Debug, Clone)]
struct Service {
    name: String,
    methods: Vec<RpcMethod>,
}

// Tokeniser for proto files
#[derive(Debug, Clone, PartialEq)]
enum PTok {
    Word(String),
    Str(String),
    Punct(char),
    Comment(String),
    Number(String),
}

fn proto_tokenise(text: &str) -> Vec<PTok> {
    let mut tokens = Vec::new();
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i].is_whitespace() {
            i += 1;
            continue;
        }
        // Line comment
        if i + 1 < chars.len() && chars[i] == '/' && chars[i + 1] == '/' {
            let start = i;
            while i < chars.len() && chars[i] != '\n' {
                i += 1;
            }
            tokens.push(PTok::Comment(chars[start..i].iter().collect()));
            continue;
        }
        // Block comment
        if i + 1 < chars.len() && chars[i] == '/' && chars[i + 1] == '*' {
            i += 2;
            let mut buf = String::from("/*");
            while i + 1 < chars.len() && !(chars[i] == '*' && chars[i + 1] == '/') {
                buf.push(chars[i]);
                i += 1;
            }
            buf.push_str("*/");
            i += 2;
            tokens.push(PTok::Comment(buf));
            continue;
        }
        // String
        if chars[i] == '"' {
            i += 1;
            let mut buf = String::new();
            while i < chars.len() && chars[i] != '"' {
                if chars[i] == '\\' && i + 1 < chars.len() {
                    buf.push(chars[i + 1]);
                    i += 2;
                } else {
                    buf.push(chars[i]);
                    i += 1;
                }
            }
            if i < chars.len() {
                i += 1;
            }
            tokens.push(PTok::Str(buf));
            continue;
        }
        // Number (including negative)
        if chars[i].is_ascii_digit()
            || (chars[i] == '-' && i + 1 < chars.len() && chars[i + 1].is_ascii_digit())
        {
            let mut buf = String::new();
            if chars[i] == '-' {
                buf.push('-');
                i += 1;
            }
            while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.') {
                buf.push(chars[i]);
                i += 1;
            }
            tokens.push(PTok::Number(buf));
            continue;
        }
        // Identifier / keyword
        if chars[i].is_alphabetic() || chars[i] == '_' {
            let mut buf = String::new();
            while i < chars.len()
                && (chars[i].is_alphanumeric() || chars[i] == '_' || chars[i] == '.')
            {
                buf.push(chars[i]);
                i += 1;
            }
            tokens.push(PTok::Word(buf));
            continue;
        }
        tokens.push(PTok::Punct(chars[i]));
        i += 1;
    }
    tokens
}

fn words_from_tokens(tokens: &[PTok]) -> Vec<String> {
    tokens
        .iter()
        .filter_map(|t| match t {
            PTok::Word(w) => Some(w.clone()),
            PTok::Number(n) => Some(n.clone()),
            _ => None,
        })
        .collect()
}

fn parse_proto(text: &str) -> ProtoFile {
    let mut pf = ProtoFile {
        syntax: "proto2".to_string(),
        package: None,
        imports: Vec::new(),
        options: Vec::new(),
        messages: Vec::new(),
        enums: Vec::new(),
        services: Vec::new(),
    };

    // Strip comments from text for structural parsing
    let stripped = strip_comments(text);
    let mut pos = 0;
    let bytes = stripped.as_bytes();

    // Simple line-by-line top-level extraction
    for line in stripped.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("syntax") {
            if let Some(v) = extract_string_value(trimmed) {
                pf.syntax = v;
            }
        } else if trimmed.starts_with("package") {
            let rest = trimmed
                .trim_start_matches("package")
                .trim()
                .trim_end_matches(';')
                .trim();
            pf.package = Some(rest.to_string());
        } else if trimmed.starts_with("import") {
            if let Some(v) = extract_string_value(trimmed) {
                pf.imports.push(v);
            }
        } else if trimmed.starts_with("option ") {
            if let Some((k, v)) = extract_option(trimmed) {
                pf.options.push((k, v));
            }
        }
    }

    // Block-level extraction for messages, enums, services
    let _ = (pos, bytes);
    let blocks = extract_top_level_blocks(&stripped);
    for (kind, name, body) in &blocks {
        match kind.as_str() {
            "message" => {
                pf.messages.push(parse_message(name, body));
            }
            "enum" => {
                pf.enums.push(parse_enum(name, body));
            }
            "service" => {
                pf.services.push(parse_service(name, body));
            }
            _ => {}
        }
    }

    pf
}

fn strip_comments(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    let mut in_str = false;
    while i < chars.len() {
        if in_str {
            if chars[i] == '"' {
                in_str = false;
            }
            out.push(chars[i]);
            i += 1;
        } else if chars[i] == '"' {
            in_str = true;
            out.push(chars[i]);
            i += 1;
        } else if i + 1 < chars.len() && chars[i] == '/' && chars[i + 1] == '/' {
            while i < chars.len() && chars[i] != '\n' {
                i += 1;
            }
        } else if i + 1 < chars.len() && chars[i] == '/' && chars[i + 1] == '*' {
            i += 2;
            while i + 1 < chars.len() && !(chars[i] == '*' && chars[i + 1] == '/') {
                i += 1;
            }
            i += 2;
        } else {
            out.push(chars[i]);
            i += 1;
        }
    }
    out
}

fn extract_string_value(line: &str) -> Option<String> {
    let start = line.find('"')? + 1;
    let rest = &line[start..];
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}

fn extract_option(line: &str) -> Option<(String, String)> {
    // option java_package = "com.example";
    let rest = line.trim_start_matches("option").trim();
    let eq = rest.find('=')?;
    let key = rest[..eq].trim().to_string();
    let val = rest[eq + 1..]
        .trim()
        .trim_end_matches(';')
        .trim()
        .trim_matches('"')
        .to_string();
    Some((key, val))
}

fn extract_top_level_blocks(text: &str) -> Vec<(String, String, String)> {
    let mut results = Vec::new();
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        // Skip whitespace
        if chars[i].is_whitespace() {
            i += 1;
            continue;
        }
        // Try to read a word
        if chars[i].is_alphabetic() || chars[i] == '_' {
            let word_start = i;
            while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') {
                i += 1;
            }
            let word: String = chars[word_start..i].iter().collect();
            if matches!(word.as_str(), "message" | "enum" | "service") {
                // Skip whitespace
                while i < chars.len() && chars[i].is_whitespace() {
                    i += 1;
                }
                // Read name
                let name_start = i;
                while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') {
                    i += 1;
                }
                let name: String = chars[name_start..i].iter().collect();
                // Find opening brace
                while i < chars.len() && chars[i] != '{' {
                    i += 1;
                }
                if i < chars.len() {
                    i += 1; // skip '{'
                }
                // Collect body
                let body_start = i;
                let mut depth = 1i32;
                while i < chars.len() && depth > 0 {
                    if chars[i] == '{' {
                        depth += 1;
                    } else if chars[i] == '}' {
                        depth -= 1;
                    }
                    if depth > 0 {
                        i += 1;
                    } else {
                        break;
                    }
                }
                let body: String = chars[body_start..i].iter().collect();
                if i < chars.len() {
                    i += 1; // skip closing '}'
                }
                if !name.is_empty() {
                    results.push((word, name, body));
                }
            }
        } else {
            i += 1;
        }
    }
    results
}

fn parse_message(name: &str, body: &str) -> Message {
    let mut msg = Message {
        name: name.to_string(),
        fields: Vec::new(),
        nested_messages: Vec::new(),
        nested_enums: Vec::new(),
        oneofs: Vec::new(),
        reserved: Vec::new(),
    };

    // Collect nested block names first
    let nested = extract_top_level_blocks(body);
    for (kind, nested_name, nested_body) in &nested {
        match kind.as_str() {
            "message" => msg.nested_messages.push(nested_name.clone()),
            "enum" => msg.nested_enums.push(nested_name.clone()),
            _ => {}
        }
        let _ = nested_body;
    }

    // Parse fields line by line (skip nested blocks by brace detection)
    for line in body.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty()
            || trimmed.starts_with("//")
            || trimmed.starts_with("message ")
            || trimmed.starts_with("enum ")
            || trimmed.starts_with("oneof ")
            || trimmed.starts_with("reserved")
            || trimmed == "{"
            || trimmed == "}"
            || trimmed.starts_with("option ")
        {
            if trimmed.starts_with("reserved") {
                msg.reserved.push(trimmed.to_string());
            }
            continue;
        }
        if let Some(f) = parse_field_line(trimmed) {
            msg.fields.push(f);
        }
    }

    msg
}

fn parse_field_line(line: &str) -> Option<Field> {
    // Remove trailing ';' and trim
    let line = line.trim_end_matches(';').trim();
    // Remove inline [...] options block
    let (line, options_str) = if let Some(start) = line.find('[') {
        let opts = line[start..].trim_matches(|c: char| c == '[' || c == ']');
        (&line[..start], opts.to_string())
    } else {
        (line, String::new())
    };
    let line = line.trim();
    let options: Vec<String> = if options_str.is_empty() {
        Vec::new()
    } else {
        options_str
            .split(',')
            .map(|s| s.trim().to_string())
            .collect()
    };

    let tokens = proto_tokenise(line);
    let ws = words_from_tokens(&tokens);

    // Proto2 / proto3 field patterns:
    // [optional|required|repeated] type name = number
    // type name = number  (proto3)
    match ws.len() {
        4 => {
            // label type name number
            let label = ws[0].clone();
            if matches!(label.as_str(), "optional" | "required" | "repeated") {
                let number: u32 = ws[3].parse().ok()?;
                Some(Field {
                    label,
                    field_type: ws[1].clone(),
                    name: ws[2].clone(),
                    number,
                    options,
                })
            } else {
                // No label: type name = number (but ws[3] would be the number after '=')
                // Actually ws still only has words; '=' is a Punct so we skip it
                let number: u32 = ws[3].parse().ok()?;
                Some(Field {
                    label: String::new(),
                    field_type: ws[0].clone(),
                    name: ws[1].clone(),
                    number,
                    options,
                })
            }
        }
        3 => {
            // type name number (no '=' in tokens since it was filtered as Punct)
            let number: u32 = ws[2].parse().ok()?;
            Some(Field {
                label: String::new(),
                field_type: ws[0].clone(),
                name: ws[1].clone(),
                number,
                options,
            })
        }
        5 => {
            // optional type name = number → label already in ws[0]
            let label = ws[0].clone();
            if matches!(label.as_str(), "optional" | "required" | "repeated") {
                let number: u32 = ws[4].parse().ok()?;
                Some(Field {
                    label,
                    field_type: ws[1].clone(),
                    name: ws[2].clone(),
                    number,
                    options,
                })
            } else {
                None
            }
        }
        _ => None,
    }
}

fn parse_enum(name: &str, body: &str) -> ProtoEnum {
    let mut e = ProtoEnum {
        name: name.to_string(),
        values: Vec::new(),
    };
    for line in body.lines() {
        let trimmed = line.trim().trim_end_matches(';').trim();
        if trimmed.is_empty()
            || trimmed.starts_with("//")
            || trimmed.starts_with("option")
            || trimmed == "{"
            || trimmed == "}"
        {
            continue;
        }
        let tokens = proto_tokenise(trimmed);
        let ws = words_from_tokens(&tokens);
        if ws.len() >= 2 {
            if let Ok(num) = ws[1].parse::<i32>() {
                e.values.push(EnumValue {
                    name: ws[0].clone(),
                    number: num,
                });
            }
        }
    }
    e
}

fn parse_service(name: &str, body: &str) -> Service {
    let mut svc = Service {
        name: name.to_string(),
        methods: Vec::new(),
    };
    // rpc MethodName (InputType) returns (OutputType) {}
    let blocks = extract_rpc_methods(body);
    for m in blocks {
        svc.methods.push(m);
    }
    svc
}

fn extract_rpc_methods(body: &str) -> Vec<RpcMethod> {
    let mut methods = Vec::new();
    let chars: Vec<char> = body.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        // Skip whitespace
        if chars[i].is_whitespace() {
            i += 1;
            continue;
        }
        // Read word
        if chars[i].is_alphabetic() || chars[i] == '_' {
            let ws = i;
            while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') {
                i += 1;
            }
            let word: String = chars[ws..i].iter().collect();
            if word == "rpc" {
                // Parse: rpc Name (InputType) returns (OutputType)
                // Skip whitespace
                while i < chars.len() && chars[i].is_whitespace() {
                    i += 1;
                }
                // Method name
                let ns = i;
                while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') {
                    i += 1;
                }
                let method_name: String = chars[ns..i].iter().collect();

                // Read input type from (...)
                while i < chars.len() && chars[i] != '(' {
                    i += 1;
                }
                i += 1; // skip '('
                let (input_raw, client_streaming) = read_type_from_parens(&chars, &mut i);

                // Skip 'returns'
                while i < chars.len() && chars[i] != '(' {
                    i += 1;
                }
                i += 1; // skip '('
                let (output_raw, server_streaming) = read_type_from_parens(&chars, &mut i);

                // Skip body {} or ;
                while i < chars.len() && chars[i] != '{' && chars[i] != ';' {
                    i += 1;
                }
                if i < chars.len() && chars[i] == '{' {
                    let mut depth = 1i32;
                    i += 1;
                    while i < chars.len() && depth > 0 {
                        if chars[i] == '{' {
                            depth += 1;
                        } else if chars[i] == '}' {
                            depth -= 1;
                        }
                        i += 1;
                    }
                } else if i < chars.len() {
                    i += 1;
                }

                if !method_name.is_empty() {
                    methods.push(RpcMethod {
                        name: method_name,
                        input: input_raw,
                        output: output_raw,
                        client_streaming,
                        server_streaming,
                    });
                }
            }
        } else {
            i += 1;
        }
    }
    methods
}

fn read_type_from_parens(chars: &[char], i: &mut usize) -> (String, bool) {
    let mut buf = String::new();
    while *i < chars.len() && chars[*i] != ')' {
        buf.push(chars[*i]);
        *i += 1;
    }
    if *i < chars.len() {
        *i += 1;
    }
    let trimmed = buf.trim().to_string();
    let streaming = trimmed.to_lowercase().starts_with("stream");
    let type_name = if streaming {
        trimmed
            .trim_start_matches("stream")
            .trim_start_matches("STREAM")
            .trim()
            .to_string()
    } else {
        trimmed
    };
    (type_name, streaming)
}

// ── Actions ──────────────────────────────────────────────────────────────────

fn info_action(args: &Value) -> Result<String, String> {
    let text = get_text(args)?;
    let pf = parse_proto(&text);

    let mut out = format!("Protobuf File\n{}\n\n", "=".repeat(44));
    out += &format!("Syntax:   {}\n", pf.syntax);
    if let Some(ref pkg) = pf.package {
        out += &format!("Package:  {}\n", pkg);
    }
    if !pf.imports.is_empty() {
        out += &format!("Imports:  {} file(s)\n", pf.imports.len());
        for imp in pf.imports.iter().take(5) {
            out += &format!("  {}\n", imp);
        }
        if pf.imports.len() > 5 {
            out += &format!("  ... ({} more)\n", pf.imports.len() - 5);
        }
    }
    if !pf.options.is_empty() {
        out += "\nFile options:\n";
        for (k, v) in pf.options.iter().take(6) {
            out += &format!("  {:<32} {}\n", k, v);
        }
    }
    out += "\n";
    out += &format!("Messages: {}\n", pf.messages.len());
    out += &format!("Enums:    {}\n", pf.enums.len());
    out += &format!("Services: {}\n", pf.services.len());

    if !pf.messages.is_empty() {
        out += "\nMessages:\n";
        for msg in &pf.messages {
            out += &format!("  {} ({} fields)\n", msg.name, msg.fields.len());
        }
    }
    if !pf.enums.is_empty() {
        out += "\nEnums:\n";
        for e in &pf.enums {
            out += &format!("  {} ({} values)\n", e.name, e.values.len());
        }
    }
    if !pf.services.is_empty() {
        out += "\nServices:\n";
        for svc in &pf.services {
            out += &format!("  {} ({} method(s))\n", svc.name, svc.methods.len());
        }
    }

    Ok(out)
}

fn messages_action(args: &Value) -> Result<String, String> {
    let text = get_text(args)?;
    let pf = parse_proto(&text);

    if pf.messages.is_empty() {
        return Ok("No message definitions found.\n".to_string());
    }

    let filter = args
        .get("filter")
        .or_else(|| args.get("message"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_lowercase());

    let mut out = format!("Messages  [{}]\n{}\n\n", pf.messages.len(), "=".repeat(44));

    for msg in &pf.messages {
        if let Some(ref f) = filter {
            if !msg.name.to_lowercase().contains(f.as_str()) {
                continue;
            }
        }
        out += &format!("message {} {{\n", msg.name);
        if !msg.nested_messages.is_empty() {
            out += &format!("  // nested messages: {}\n", msg.nested_messages.join(", "));
        }
        if !msg.nested_enums.is_empty() {
            out += &format!("  // nested enums: {}\n", msg.nested_enums.join(", "));
        }
        for field in &msg.fields {
            let label = if field.label.is_empty() {
                String::new()
            } else {
                format!("{} ", field.label)
            };
            let opts = if field.options.is_empty() {
                String::new()
            } else {
                format!(" [{}]", field.options.join(", "))
            };
            out += &format!(
                "  {}{} {} = {}{}\n",
                label, field.field_type, field.name, field.number, opts
            );
        }
        if !pf.enums.is_empty() {
            for e in &pf.enums {
                if msg.nested_enums.contains(&e.name) {
                    out += &format!("  enum {} {{\n", e.name);
                    for val in &e.values {
                        out += &format!("    {} = {}\n", val.name, val.number);
                    }
                    out += "  }\n";
                }
            }
        }
        out += "}\n\n";
    }

    if !pf.enums.is_empty() {
        let top_level: Vec<&ProtoEnum> = pf
            .enums
            .iter()
            .filter(|e| !pf.messages.iter().any(|m| m.nested_enums.contains(&e.name)))
            .collect();
        if !top_level.is_empty() {
            for e in top_level {
                if let Some(ref f) = filter {
                    if !e.name.to_lowercase().contains(f.as_str()) {
                        continue;
                    }
                }
                out += &format!("enum {} {{\n", e.name);
                for val in &e.values {
                    out += &format!("  {} = {}\n", val.name, val.number);
                }
                out += "}\n\n";
            }
        }
    }

    Ok(out)
}

fn services_action(args: &Value) -> Result<String, String> {
    let text = get_text(args)?;
    let pf = parse_proto(&text);

    if pf.services.is_empty() {
        return Ok("No service definitions found.\n".to_string());
    }

    let mut out = format!("Services  [{}]\n{}\n\n", pf.services.len(), "=".repeat(44));

    for svc in &pf.services {
        out += &format!("service {} {{\n", svc.name);
        for method in &svc.methods {
            let client_s = if method.client_streaming {
                "stream "
            } else {
                ""
            };
            let server_s = if method.server_streaming {
                "stream "
            } else {
                ""
            };
            let kind = match (method.client_streaming, method.server_streaming) {
                (false, false) => "unary",
                (true, false) => "client-streaming",
                (false, true) => "server-streaming",
                (true, true) => "bidirectional-streaming",
            };
            out += &format!(
                "  rpc {}({}{}) returns ({}{})  // {}\n",
                method.name, client_s, method.input, server_s, method.output, kind
            );
        }
        out += "}\n\n";
    }

    Ok(out)
}

fn validate_action(args: &Value) -> Result<String, String> {
    let text = get_text(args)?;
    let pf = parse_proto(&text);
    let mut warnings: Vec<String> = Vec::new();

    // Syntax check
    if pf.syntax != "proto3" && pf.syntax != "proto2" {
        warnings.push(format!(
            "Unrecognised syntax '{}' — expected 'proto3' or 'proto2'",
            pf.syntax
        ));
    }

    // Package is recommended
    if pf.package.is_none() {
        warnings.push(
            "No 'package' declaration — add one to avoid name collisions across schemas"
                .to_string(),
        );
    }

    // java_package / go_package options are good practice
    let has_go_pkg = pf.options.iter().any(|(k, _)| k == "go_package");
    let has_java_pkg = pf.options.iter().any(|(k, _)| k == "java_package");
    if !has_go_pkg && !has_java_pkg && !pf.options.is_empty() {
        // Only warn if there are other options but these are missing (common mistake)
    }

    // Messages: check for field number conflicts and missing fields
    for msg in &pf.messages {
        if msg.fields.is_empty() && msg.nested_messages.is_empty() {
            warnings.push(format!(
                "Message '{}' has no fields — is this intentional?",
                msg.name
            ));
        }
        // Duplicate field numbers
        let mut nums: Vec<u32> = msg.fields.iter().map(|f| f.number).collect();
        nums.sort();
        let mut prev = 0u32;
        for &n in &nums {
            if n == prev && n != 0 {
                warnings.push(format!(
                    "Message '{}' has duplicate field number {}",
                    msg.name, n
                ));
            }
            prev = n;
        }
        // Field number 0 is invalid
        for field in &msg.fields {
            if field.number == 0 {
                warnings.push(format!(
                    "Message '{}', field '{}': field number 0 is reserved and invalid",
                    msg.name, field.name
                ));
            }
            // Field numbers 19000-19999 are reserved by Google
            if field.number >= 19000 && field.number <= 19999 {
                warnings.push(format!(
                    "Message '{}', field '{}': field number {} is in the reserved range 19000–19999",
                    msg.name, field.name, field.number
                ));
            }
        }
        // proto2: required fields are considered bad practice
        if pf.syntax == "proto2" {
            for field in &msg.fields {
                if field.label == "required" {
                    warnings.push(format!(
                        "Message '{}', field '{}': 'required' fields are brittle in proto2 — prefer 'optional'",
                        msg.name, field.name
                    ));
                }
            }
        }
    }

    // Enums: first value should be 0 (proto3)
    if pf.syntax == "proto3" {
        for e in &pf.enums {
            if let Some(first) = e.values.first() {
                if first.number != 0 {
                    warnings.push(format!(
                        "Enum '{}': first value must be 0 in proto3, got {} = {}",
                        e.name, first.name, first.number
                    ));
                }
            }
        }
    }

    // Services: check for empty services
    for svc in &pf.services {
        if svc.methods.is_empty() {
            warnings.push(format!("Service '{}' has no RPC methods defined", svc.name));
        }
    }

    let mut out = format!("Protobuf Validation\n{}\n\n", "=".repeat(44));
    out += &format!(
        "Result: {}\n\n",
        if warnings.is_empty() {
            "VALID"
        } else {
            "VALID with warnings"
        }
    );
    out += &format!(
        "Syntax: {}, {} message(s), {} enum(s), {} service(s).\n",
        pf.syntax,
        pf.messages.len(),
        pf.enums.len(),
        pf.services.len()
    );
    if warnings.is_empty() {
        out += "No issues found.\n";
    } else {
        out += &format!("\n{} warning(s):\n", warnings.len());
        for w in &warnings {
            out += &format!("  [WARN] {}\n", w);
        }
    }
    Ok(out)
}
