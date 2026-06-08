use quick_xml::events::Event;
use quick_xml::Reader;
use serde_json::Value;

pub async fn execute(args: &Value) -> Result<String, String> {
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("parse");
    match action {
        "parse" | "read" => action_parse(args),
        "get" => action_get(args),
        "keys" => action_keys(args),
        "validate" => action_validate(args),
        "to-json" => action_to_json(args),
        other => Err(format!(
            "plist_tools: unknown action '{other}'. Valid: parse, get, keys, validate, to-json"
        )),
    }
}

// ── Input resolution ───────────────────────────────────────────────────────────

fn resolve_input(args: &Value) -> Result<String, String> {
    if let Some(s) = args
        .get("text")
        .or_else(|| args.get("plist"))
        .or_else(|| args.get("xml"))
        .and_then(|v| v.as_str())
    {
        return Ok(s.to_string());
    }
    if let Some(path) = args.get("file").and_then(|v| v.as_str()) {
        return std::fs::read_to_string(path)
            .map_err(|e| format!("plist_tools: cannot read '{path}': {e}"));
    }
    Err("plist_tools: provide 'text'/'plist'/'xml' (inline plist XML) or 'file' (path)".into())
}

fn file_hint(args: &Value) -> String {
    args.get("file")
        .and_then(|v| v.as_str())
        .and_then(|p| std::path::Path::new(p).file_name())
        .and_then(|n| n.to_str())
        .unwrap_or("plist")
        .to_string()
}

// ── Plist value tree ───────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
enum PlistValue {
    String(String),
    Integer(i64),
    Real(f64),
    Bool(bool),
    Date(String),
    Data(usize),
    Array(Vec<PlistValue>),
    Dict(Vec<(String, PlistValue)>),
}

impl PlistValue {
    fn type_name(&self) -> &'static str {
        match self {
            PlistValue::String(_) => "string",
            PlistValue::Integer(_) => "integer",
            PlistValue::Real(_) => "real",
            PlistValue::Bool(_) => "bool",
            PlistValue::Date(_) => "date",
            PlistValue::Data(_) => "data",
            PlistValue::Array(_) => "array",
            PlistValue::Dict(_) => "dict",
        }
    }
}

// ── Parser ─────────────────────────────────────────────────────────────────────

// State for the stack-based parser
#[derive(Debug)]
enum StackFrame {
    Root,
    Dict(Vec<(String, PlistValue)>, Option<String>), // entries, pending key
    Array(Vec<PlistValue>),
}

fn parse_plist(src: &str) -> Result<PlistValue, String> {
    let mut reader = Reader::from_str(src);
    reader.config_mut().trim_text(true);

    // Stack of frames. We start with a Root frame.
    let mut stack: Vec<StackFrame> = vec![StackFrame::Root];
    // Text accumulator for leaf tags
    let mut text_buf = String::new();

    // The finished root value, set when the <plist> end tag closes.
    let mut root_value: Option<PlistValue> = None;

    loop {
        match reader.read_event() {
            Err(e) => {
                return Err(format!(
                    "plist_tools: XML parse error at byte {}: {e}",
                    reader.buffer_position()
                ))
            }
            Ok(Event::Eof) => break,
            Ok(Event::Start(e)) => {
                let tag = String::from_utf8_lossy(e.local_name().as_ref()).to_string();
                match tag.as_str() {
                    "plist" | "?xml" => {} // structural wrapper
                    "dict" => {
                        stack.push(StackFrame::Dict(Vec::new(), None));
                    }
                    "array" => {
                        stack.push(StackFrame::Array(Vec::new()));
                    }
                    _other => {
                        text_buf.clear();
                    }
                }
            }
            Ok(Event::Empty(e)) => {
                // Self-closing: <true/> or <false/>
                let tag = String::from_utf8_lossy(e.local_name().as_ref()).to_string();
                let val = match tag.as_str() {
                    "true" => PlistValue::Bool(true),
                    "false" => PlistValue::Bool(false),
                    _ => continue,
                };
                push_value(&mut stack, val)?;
            }
            Ok(Event::End(e)) => {
                let tag = String::from_utf8_lossy(e.local_name().as_ref()).to_string();
                match tag.as_str() {
                    "plist" => {
                        // Pop the root frame if it holds a completed value
                        if let Some(StackFrame::Root) = stack.last() {
                            // root_value already set via push_value into Root
                        }
                    }
                    "dict" => {
                        if let Some(StackFrame::Dict(entries, _)) = stack.pop() {
                            let val = PlistValue::Dict(entries);
                            let at_root =
                                stack.is_empty() || matches!(stack.last(), Some(StackFrame::Root));
                            if at_root {
                                root_value = Some(val);
                            } else {
                                push_value(&mut stack, val)?;
                            }
                        }
                    }
                    "array" => {
                        if let Some(StackFrame::Array(items)) = stack.pop() {
                            let val = PlistValue::Array(items);
                            let at_root =
                                stack.is_empty() || matches!(stack.last(), Some(StackFrame::Root));
                            if at_root {
                                root_value = Some(val);
                            } else {
                                push_value(&mut stack, val)?;
                            }
                        }
                    }
                    "key" => {
                        // End of a <key> tag — set pending key on enclosing dict
                        if let Some(StackFrame::Dict(_, ref mut pending_key)) = stack.last_mut() {
                            *pending_key = Some(text_buf.clone());
                        }
                        text_buf.clear();
                    }
                    leaf => {
                        let text = text_buf.clone();
                        text_buf.clear();
                        let val = match leaf {
                            "string" => PlistValue::String(text),
                            "integer" => {
                                let n: i64 = text.trim().parse().map_err(|_| {
                                    format!("plist_tools: invalid integer '{text}'")
                                })?;
                                PlistValue::Integer(n)
                            }
                            "real" => {
                                let f: f64 = text
                                    .trim()
                                    .parse()
                                    .map_err(|_| format!("plist_tools: invalid real '{text}'"))?;
                                PlistValue::Real(f)
                            }
                            "true" => PlistValue::Bool(true),
                            "false" => PlistValue::Bool(false),
                            "date" => PlistValue::Date(text),
                            "data" => {
                                // base64 content — count raw bytes
                                let stripped: String =
                                    text.chars().filter(|c| !c.is_whitespace()).collect();
                                // Each base64 char represents 6 bits; 4 chars = 3 bytes
                                let byte_count = stripped.len() * 3 / 4;
                                PlistValue::Data(byte_count)
                            }
                            _ => continue, // ignore unknown closing tags
                        };
                        let at_root =
                            stack.is_empty() || matches!(stack.last(), Some(StackFrame::Root));
                        if at_root {
                            root_value = Some(val);
                        } else {
                            push_value(&mut stack, val)?;
                        }
                    }
                }
            }
            Ok(Event::Text(e)) => {
                if let Ok(s) = e.unescape() {
                    text_buf.push_str(&s);
                }
            }
            Ok(Event::CData(e)) => {
                let s = String::from_utf8_lossy(e.as_ref());
                text_buf.push_str(&s);
            }
            Ok(Event::DocType(_)) | Ok(Event::PI(_)) | Ok(Event::Comment(_)) => {}
            _ => {}
        }
    }

    root_value.ok_or_else(|| "plist_tools: no root value found in plist XML".into())
}

fn push_value(stack: &mut [StackFrame], val: PlistValue) -> Result<(), String> {
    match stack.last_mut() {
        Some(StackFrame::Dict(entries, pending_key)) => {
            if let Some(key) = pending_key.take() {
                entries.push((key, val));
            } else {
                return Err("plist_tools: dict value without preceding <key>".into());
            }
        }
        Some(StackFrame::Array(items)) => {
            items.push(val);
        }
        Some(StackFrame::Root) | None => {
            // The dict/array end handlers deal with this; for bare scalar roots we'd
            // need to store it separately — but valid plists have a single root dict.
            // Just ignore the root frame push and let parse_plist handle it via
            // the dict/array end arms.
        }
    }
    Ok(())
}

// ── Display helpers ────────────────────────────────────────────────────────────

fn render_value(val: &PlistValue, indent: usize, out: &mut String) {
    let pad = "  ".repeat(indent);
    match val {
        PlistValue::Dict(entries) => {
            for (k, v) in entries {
                match v {
                    PlistValue::Dict(inner) => {
                        out.push_str(&format!(
                            "{}{}:  (dict, {} key{})\n",
                            pad,
                            k,
                            inner.len(),
                            if inner.len() == 1 { "" } else { "s" }
                        ));
                        render_value(v, indent + 1, out);
                    }
                    PlistValue::Array(items) => {
                        out.push_str(&format!(
                            "{}{}:  (array, {} item{})\n",
                            pad,
                            k,
                            items.len(),
                            if items.len() == 1 { "" } else { "s" }
                        ));
                        render_value(v, indent + 1, out);
                    }
                    PlistValue::String(s) => out.push_str(&format!("{}{}: {}\n", pad, k, s)),
                    PlistValue::Integer(n) => {
                        out.push_str(&format!("{}{}: {} [integer]\n", pad, k, n))
                    }
                    PlistValue::Real(f) => out.push_str(&format!("{}{}: {} [real]\n", pad, k, f)),
                    PlistValue::Bool(b) => out.push_str(&format!("{}{}: {} [bool]\n", pad, k, b)),
                    PlistValue::Date(d) => out.push_str(&format!("{}{}: {} [date]\n", pad, k, d)),
                    PlistValue::Data(n) => {
                        out.push_str(&format!("{}{}: [data: {} bytes]\n", pad, k, n))
                    }
                }
            }
        }
        PlistValue::Array(items) => {
            for (i, item) in items.iter().enumerate() {
                match item {
                    PlistValue::Dict(inner) => {
                        out.push_str(&format!(
                            "{}[{}] (dict, {} key{})\n",
                            pad,
                            i,
                            inner.len(),
                            if inner.len() == 1 { "" } else { "s" }
                        ));
                        render_value(item, indent + 1, out);
                    }
                    PlistValue::Array(inner) => {
                        out.push_str(&format!(
                            "{}[{}] (array, {} item{})\n",
                            pad,
                            i,
                            inner.len(),
                            if inner.len() == 1 { "" } else { "s" }
                        ));
                        render_value(item, indent + 1, out);
                    }
                    PlistValue::String(s) => out.push_str(&format!("{}[{}] {}\n", pad, i, s)),
                    PlistValue::Integer(n) => {
                        out.push_str(&format!("{}[{}] {} [integer]\n", pad, i, n))
                    }
                    PlistValue::Real(f) => out.push_str(&format!("{}[{}] {} [real]\n", pad, i, f)),
                    PlistValue::Bool(b) => out.push_str(&format!("{}[{}] {} [bool]\n", pad, i, b)),
                    PlistValue::Date(d) => out.push_str(&format!("{}[{}] {} [date]\n", pad, i, d)),
                    PlistValue::Data(n) => {
                        out.push_str(&format!("{}[{}] [data: {} bytes]\n", pad, i, n))
                    }
                }
            }
        }
        _ => {} // bare scalar root — unlikely in real plists
    }
}

// ── Dot-path navigation ────────────────────────────────────────────────────────

fn navigate<'a>(root: &'a PlistValue, path: &str) -> Result<&'a PlistValue, String> {
    let path = path.trim_start_matches('.');
    if path.is_empty() {
        return Ok(root);
    }
    // Split on first '.' or '[', handling array indexing
    let current = root;
    navigate_inner(current, path)
}

fn navigate_inner<'a>(val: &'a PlistValue, path: &str) -> Result<&'a PlistValue, String> {
    if path.is_empty() {
        return Ok(val);
    }
    // Consume a leading dot
    let path = path.strip_prefix('.').unwrap_or(path);
    if path.is_empty() {
        return Ok(val);
    }

    // Check for array index: starts with '['
    if let Some(rest) = path.strip_prefix('[') {
        let end = rest
            .find(']')
            .ok_or_else(|| "plist_tools: unmatched '[' in path".to_string())?;
        let idx: usize = rest[..end]
            .parse()
            .map_err(|_| format!("plist_tools: invalid array index '{}'", &rest[..end]))?;
        let after = &rest[end + 1..];
        if let PlistValue::Array(items) = val {
            let item = items.get(idx).ok_or_else(|| {
                format!(
                    "plist_tools: array index {idx} out of bounds (len {})",
                    items.len()
                )
            })?;
            return navigate_inner(item, after);
        } else {
            return Err(format!(
                "plist_tools: expected array for index [{idx}], got {}",
                val.type_name()
            ));
        }
    }

    // Key segment: read until '.', '[', or end
    let seg_end = path.find(['.', '[']).unwrap_or(path.len());
    let key = &path[..seg_end];
    let rest = &path[seg_end..];

    if let PlistValue::Dict(entries) = val {
        for (k, v) in entries {
            if k == key {
                return navigate_inner(v, rest);
            }
        }
        return Err(format!("plist_tools: key '{key}' not found"));
    }
    Err(format!(
        "plist_tools: expected dict for key '{key}', got {}",
        val.type_name()
    ))
}

fn dict_at<'a>(root: &'a PlistValue, path: &str) -> Result<&'a Vec<(String, PlistValue)>, String> {
    let val = navigate(root, path)?;
    if let PlistValue::Dict(entries) = val {
        Ok(entries)
    } else {
        Err(format!(
            "plist_tools: path '{}' is a {}, not a dict",
            path,
            val.type_name()
        ))
    }
}

// ── Notable key helpers ────────────────────────────────────────────────────────

fn dict_string<'a>(entries: &'a [(String, PlistValue)], key: &str) -> Option<&'a str> {
    for (k, v) in entries {
        if k == key {
            if let PlistValue::String(s) = v {
                return Some(s.as_str());
            }
        }
    }
    None
}

fn dict_bool(entries: &[(String, PlistValue)], key: &str) -> Option<bool> {
    for (k, v) in entries {
        if k == key {
            if let PlistValue::Bool(b) = v {
                return Some(*b);
            }
        }
    }
    None
}

fn dict_sub_bool(
    entries: &[(String, PlistValue)],
    outer_key: &str,
    inner_key: &str,
) -> Option<bool> {
    for (k, v) in entries {
        if k == outer_key {
            if let PlistValue::Dict(inner) = v {
                return dict_bool(inner, inner_key);
            }
        }
    }
    None
}

fn dict_has_key(entries: &[(String, PlistValue)], key: &str) -> bool {
    entries.iter().any(|(k, _)| k == key)
}

fn notable_section(entries: &[(String, PlistValue)]) -> String {
    let mut notes: Vec<String> = Vec::new();

    if let Some(id) = dict_string(entries, "CFBundleIdentifier") {
        notes.push(format!("Bundle ID:        {id}"));
    }
    if let Some(v) = dict_string(entries, "CFBundleVersion") {
        notes.push(format!("Version:          {v}"));
    }
    if let Some(v) = dict_string(entries, "CFBundleShortVersionString") {
        notes.push(format!("Short version:    {v}"));
    }
    if let Some(v) = dict_string(entries, "LSMinimumSystemVersion") {
        notes.push(format!("Min macOS:        {v}"));
    }
    if let Some(v) = dict_string(entries, "MinimumOSVersion") {
        notes.push(format!("Min iOS:          {v}"));
    }
    if let Some(v) = dict_string(entries, "NSPrincipalClass") {
        notes.push(format!("Main class:       {v}"));
    }
    if dict_sub_bool(entries, "NSAppTransportSecurity", "NSAllowsArbitraryLoads") == Some(true) {
        notes.push("⚠ ATS disabled (NSAllowsArbitraryLoads = true — allows HTTP traffic)".into());
    }
    if let Some(v) = dict_string(entries, "NSCameraUsageDescription") {
        notes.push(format!("Camera perm:      {v}"));
    } else if dict_bool(entries, "NSCamera") == Some(true) {
        notes.push("Camera:           true [no UsageDescription]".into());
    }
    if let Some(v) = dict_string(entries, "NSMicrophoneUsageDescription") {
        notes.push(format!("Microphone perm:  {v}"));
    }
    if let Some(v) = dict_string(entries, "NSLocationWhenInUseUsageDescription") {
        notes.push(format!("Location perm:    {v}"));
    }

    if notes.is_empty() {
        return String::new();
    }
    let mut out = "\nNotable Keys\n────────────────────────────────\n".to_string();
    for n in &notes {
        out.push_str(n);
        out.push('\n');
    }
    out
}

// ── JSON conversion ────────────────────────────────────────────────────────────

fn to_json_value(val: &PlistValue) -> serde_json::Value {
    match val {
        PlistValue::String(s) => serde_json::Value::String(s.clone()),
        PlistValue::Integer(n) => serde_json::Value::Number((*n).into()),
        PlistValue::Real(f) => serde_json::json!(*f),
        PlistValue::Bool(b) => serde_json::Value::Bool(*b),
        PlistValue::Date(d) => serde_json::Value::String(d.clone()),
        PlistValue::Data(n) => serde_json::Value::String(format!("<data: {n} bytes>")),
        PlistValue::Array(items) => {
            serde_json::Value::Array(items.iter().map(to_json_value).collect())
        }
        PlistValue::Dict(entries) => {
            let mut map = serde_json::Map::new();
            for (k, v) in entries {
                map.insert(k.clone(), to_json_value(v));
            }
            serde_json::Value::Object(map)
        }
    }
}

// ── Actions ────────────────────────────────────────────────────────────────────

fn action_parse(args: &Value) -> Result<String, String> {
    let src = resolve_input(args)?;
    let hint = file_hint(args);
    let root = parse_plist(&src)?;

    let mut out = format!("Parse Result — {hint}\n{}\n", "─".repeat(34));

    match &root {
        PlistValue::Dict(entries) => {
            let mut body = String::new();
            render_value(&root, 0, &mut body);
            out.push_str(&body);
            out.push_str(&format!(
                "\nTotal: {} top-level key{}\n",
                entries.len(),
                if entries.len() == 1 { "" } else { "s" }
            ));
            let notable = notable_section(entries);
            if !notable.is_empty() {
                out.push_str(&notable);
            }
        }
        PlistValue::Array(items) => {
            let mut body = String::new();
            render_value(&root, 0, &mut body);
            out.push_str(&body);
            out.push_str(&format!(
                "\nTotal: {} item{}\n",
                items.len(),
                if items.len() == 1 { "" } else { "s" }
            ));
        }
        _ => {
            out.push_str(&format!("Root value: {:?}\n", root));
        }
    }

    Ok(out)
}

fn action_get(args: &Value) -> Result<String, String> {
    let src = resolve_input(args)?;
    let path = args
        .get("path")
        .or_else(|| args.get("key"))
        .and_then(|v| v.as_str())
        .ok_or("plist_tools: 'get' requires a 'path' argument (e.g. \"CFBundleVersion\" or \"NSAppTransportSecurity.NSAllowsArbitraryLoads\")")?;

    let root = parse_plist(&src)?;
    let val = navigate(&root, path)?;

    let mut out = format!("Get: {path}\n{}\n\n", "─".repeat(34));
    match val {
        PlistValue::String(s) => out.push_str(&format!("Value: {s}\nType:  string\n")),
        PlistValue::Integer(n) => out.push_str(&format!("Value: {n}\nType:  integer\n")),
        PlistValue::Real(f) => out.push_str(&format!("Value: {f}\nType:  real\n")),
        PlistValue::Bool(b) => out.push_str(&format!("Value: {b}\nType:  bool\n")),
        PlistValue::Date(d) => out.push_str(&format!("Value: {d}\nType:  date\n")),
        PlistValue::Data(n) => out.push_str(&format!("Value: [data: {n} bytes]\nType:  data\n")),
        PlistValue::Array(items) => {
            out.push_str(&format!(
                "Type:  array ({} item{})\n\n",
                items.len(),
                if items.len() == 1 { "" } else { "s" }
            ));
            let mut body = String::new();
            render_value(val, 0, &mut body);
            out.push_str(&body);
        }
        PlistValue::Dict(entries) => {
            out.push_str(&format!(
                "Type:  dict ({} key{})\n\n",
                entries.len(),
                if entries.len() == 1 { "" } else { "s" }
            ));
            let mut body = String::new();
            render_value(val, 0, &mut body);
            out.push_str(&body);
        }
    }
    Ok(out)
}

fn action_keys(args: &Value) -> Result<String, String> {
    let src = resolve_input(args)?;
    let hint = file_hint(args);
    let root = parse_plist(&src)?;

    let path = args
        .get("path")
        .or_else(|| args.get("at"))
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let entries = dict_at(&root, path)?;

    let scope = if path.is_empty() {
        "root dict".to_string()
    } else {
        format!("'{path}'")
    };
    let mut out = format!("Keys — {hint} ({scope})\n{}\n\n", "─".repeat(34));

    let max_key = entries.iter().map(|(k, _)| k.len()).max().unwrap_or(0);

    for (k, v) in entries {
        let pad = max_key - k.len();
        let spaces = " ".repeat(pad + 2);
        let val_str = match v {
            PlistValue::String(s) => {
                let preview = if s.len() > 60 {
                    format!("{}…", &s[..57])
                } else {
                    s.clone()
                };
                format!("{preview}  [string]")
            }
            PlistValue::Integer(n) => format!("{n}  [integer]"),
            PlistValue::Real(f) => format!("{f}  [real]"),
            PlistValue::Bool(b) => format!("{b}  [bool]"),
            PlistValue::Date(d) => format!("{d}  [date]"),
            PlistValue::Data(n) => format!("[data: {n} bytes]"),
            PlistValue::Array(items) => format!(
                "(array, {} item{})",
                items.len(),
                if items.len() == 1 { "" } else { "s" }
            ),
            PlistValue::Dict(inner) => format!(
                "(dict, {} key{})",
                inner.len(),
                if inner.len() == 1 { "" } else { "s" }
            ),
        };
        out.push_str(&format!("{k}{spaces}{val_str}\n"));
    }
    out.push_str(&format!(
        "\n{} key{} total\n",
        entries.len(),
        if entries.len() == 1 { "" } else { "s" }
    ));
    Ok(out)
}

fn action_validate(args: &Value) -> Result<String, String> {
    let src = resolve_input(args)?;
    let hint = file_hint(args);
    let root = parse_plist(&src)?;

    let entries = match &root {
        PlistValue::Dict(e) => e,
        _ => return Ok(format!("Validate — {hint}\n{}\n\nWARNING: Root is not a dict — cannot validate as Info.plist\n", "─".repeat(34))),
    };

    let mut warnings: Vec<String> = Vec::new();

    // Required keys
    if !dict_has_key(entries, "CFBundleIdentifier") {
        warnings.push("Missing CFBundleIdentifier (required for iOS/macOS apps)".into());
    } else if let Some(id) = dict_string(entries, "CFBundleIdentifier") {
        if id.contains(' ') {
            warnings.push(format!("CFBundleIdentifier '{id}' contains spaces"));
        }
    }

    if !dict_has_key(entries, "CFBundleVersion") {
        warnings.push("Missing CFBundleVersion (required)".into());
    }

    if !dict_has_key(entries, "CFBundleShortVersionString") {
        warnings.push("Missing CFBundleShortVersionString (recommended)".into());
    }

    // ATS security
    if dict_sub_bool(entries, "NSAppTransportSecurity", "NSAllowsArbitraryLoads") == Some(true) {
        warnings.push(
            "NSAllowsArbitraryLoads = true — ATS disabled; app can make insecure HTTP connections"
                .into(),
        );
    }

    // Permission booleans without UsageDescription
    let perm_keys = [
        ("NSCamera", "NSCameraUsageDescription"),
        ("NSMicrophone", "NSMicrophoneUsageDescription"),
        ("NSLocation", "NSLocationWhenInUseUsageDescription"),
        ("NSPhotoLibrary", "NSPhotoLibraryUsageDescription"),
        ("NSContactsUsage", "NSContactsUsageDescription"),
        ("NSCalendarsUsage", "NSCalendarsUsageDescription"),
    ];
    for (flag_key, desc_key) in &perm_keys {
        if dict_bool(entries, flag_key) == Some(true) && !dict_has_key(entries, desc_key) {
            warnings.push(format!("{flag_key} = true but {desc_key} is missing"));
        }
    }

    let mut out = format!("Validate — {hint}\n{}\n\n", "─".repeat(34));
    if warnings.is_empty() {
        out.push_str("VALID — no issues found\n");
    } else {
        out.push_str(&format!(
            "{} WARNING{}\n\n",
            warnings.len(),
            if warnings.len() == 1 { "" } else { "S" }
        ));
        for w in &warnings {
            out.push_str(&format!("  ⚠  {w}\n"));
        }
    }
    Ok(out)
}

fn action_to_json(args: &Value) -> Result<String, String> {
    let src = resolve_input(args)?;
    let root = parse_plist(&src)?;
    let json_val = to_json_value(&root);
    serde_json::to_string_pretty(&json_val)
        .map_err(|e| format!("plist_tools: JSON serialization error: {e}"))
}
