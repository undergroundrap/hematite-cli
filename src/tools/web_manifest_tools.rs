use serde_json::{json, Value};

pub fn web_manifest_tools_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "action": {
                "type": "string",
                "enum": ["parse", "validate", "icons", "screenshots", "info"],
                "description": "parse: default full manifest summary | validate: check required/recommended fields | icons: list all icons | screenshots: list screenshots | info: concise display/orientation/features"
            },
            "manifest": {
                "description": "Manifest as a JSON object or a JSON string",
                "oneOf": [
                    {"type": "object"},
                    {"type": "string"}
                ]
            },
            "file": {"type": "string", "description": "Path to manifest.json or .webmanifest file"}
        },
        "required": []
    })
}

fn load_manifest(args: &Value) -> Result<Value, String> {
    if let Some(f) = args.get("file").and_then(|v| v.as_str()) {
        let content =
            std::fs::read_to_string(f).map_err(|e| format!("Cannot read '{}': {}", f, e))?;
        serde_json::from_str(&content).map_err(|e| format!("JSON parse error: {}", e))
    } else if let Some(m) = args.get("manifest") {
        match m {
            Value::Object(_) => Ok(m.clone()),
            Value::String(s) => {
                serde_json::from_str(s).map_err(|e| format!("JSON parse error: {}", e))
            }
            _ => Err("'manifest' must be a JSON object or JSON string".to_string()),
        }
    } else {
        Err(
            "Provide 'manifest' (JSON object or string) or 'file' (path to manifest.json)"
                .to_string(),
        )
    }
}

fn str_field<'a>(m: &'a Value, key: &str) -> Option<&'a str> {
    m.get(key).and_then(|v| v.as_str())
}

fn display_val(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Array(arr) => arr
            .iter()
            .map(|x| x.as_str().unwrap_or("?"))
            .collect::<Vec<_>>()
            .join(", "),
        other => other.to_string(),
    }
}

fn action_parse(m: &Value) -> String {
    let mut out = String::from("WEB APP MANIFEST\n================\n\n");

    let fields = [
        ("name", "Name"),
        ("short_name", "Short name"),
        ("description", "Description"),
        ("start_url", "Start URL"),
        ("scope", "Scope"),
        ("display", "Display mode"),
        ("display_override", "Display override"),
        ("orientation", "Orientation"),
        ("theme_color", "Theme color"),
        ("background_color", "Background color"),
        ("lang", "Language"),
        ("dir", "Text direction"),
        ("id", "App ID"),
        ("categories", "Categories"),
        ("iarc_rating_id", "IARC rating"),
        ("related_applications", "Related apps"),
        ("prefer_related_applications", "Prefer related apps"),
    ];

    for (key, label) in &fields {
        if let Some(v) = m.get(*key) {
            out.push_str(&format!("{:<26}: {}\n", label, display_val(v)));
        }
    }

    // Icons summary
    if let Some(icons) = m.get("icons").and_then(|v| v.as_array()) {
        out.push_str(&format!("{:<26}: {} icon(s)\n", "Icons", icons.len()));
    }

    // Screenshots summary
    if let Some(ss) = m.get("screenshots").and_then(|v| v.as_array()) {
        out.push_str(&format!(
            "{:<26}: {} screenshot(s)\n",
            "Screenshots",
            ss.len()
        ));
    }

    // Shortcuts
    if let Some(sc) = m.get("shortcuts").and_then(|v| v.as_array()) {
        out.push_str(&format!("{:<26}: {} shortcut(s)\n", "Shortcuts", sc.len()));
        for (i, s) in sc.iter().enumerate() {
            let name = s.get("name").and_then(|v| v.as_str()).unwrap_or("?");
            let url = s.get("url").and_then(|v| v.as_str()).unwrap_or("?");
            out.push_str(&format!("  [{}] {} → {}\n", i + 1, name, url));
        }
    }

    // File handlers
    if let Some(fh) = m.get("file_handlers").and_then(|v| v.as_array()) {
        out.push_str(&format!(
            "{:<26}: {} file handler(s)\n",
            "File handlers",
            fh.len()
        ));
    }

    // Protocol handlers
    if let Some(ph) = m.get("protocol_handlers").and_then(|v| v.as_array()) {
        out.push_str(&format!(
            "{:<26}: {} protocol handler(s)\n",
            "Protocol handlers",
            ph.len()
        ));
    }

    // Share target
    if m.get("share_target").is_some() {
        out.push_str(&format!("{:<26}: yes\n", "Share target"));
    }

    out.trim_end().to_string()
}

fn action_validate(m: &Value) -> String {
    let mut errors: Vec<String> = Vec::new();
    let mut warnings: Vec<String> = Vec::new();
    let mut info: Vec<String> = Vec::new();

    // Required by browsers for installability
    if str_field(m, "name").is_none() {
        errors.push("Missing 'name' — required for installability".to_string());
    }
    if str_field(m, "icons").is_none() {
        errors.push("Missing 'icons' array — required for installability".to_string());
    } else if let Some(icons) = m.get("icons").and_then(|v| v.as_array()) {
        let has_192 = icons.iter().any(|ic| {
            ic.get("sizes")
                .and_then(|v| v.as_str())
                .map(|s| s.contains("192x192"))
                .unwrap_or(false)
        });
        let has_512 = icons.iter().any(|ic| {
            ic.get("sizes")
                .and_then(|v| v.as_str())
                .map(|s| s.contains("512x512"))
                .unwrap_or(false)
        });
        if !has_192 {
            warnings
                .push("No 192×192 icon found — recommended for Android home screen".to_string());
        }
        if !has_512 {
            warnings.push("No 512×512 icon found — recommended for splash screens".to_string());
        }
        let has_maskable = icons.iter().any(|ic| {
            ic.get("purpose")
                .and_then(|v| v.as_str())
                .map(|p| p.contains("maskable"))
                .unwrap_or(false)
        });
        if !has_maskable {
            warnings.push(
                "No maskable icon — add purpose: 'maskable' for adaptive icons on Android"
                    .to_string(),
            );
        }
    }
    if str_field(m, "start_url").is_none() {
        warnings.push("Missing 'start_url' — defaults to '/' but explicit is better".to_string());
    }
    if str_field(m, "display").is_none() {
        warnings.push(
            "Missing 'display' — add 'standalone' or 'minimal-ui' to enable app-like UI"
                .to_string(),
        );
    } else {
        let valid_display = ["fullscreen", "standalone", "minimal-ui", "browser"];
        if let Some(d) = str_field(m, "display") {
            if !valid_display.contains(&d) {
                errors.push(format!(
                    "Invalid display mode '{}' — use: fullscreen, standalone, minimal-ui, browser",
                    d
                ));
            }
        }
    }

    // Recommended
    if str_field(m, "short_name").is_none() {
        warnings
            .push("Missing 'short_name' — shown on home screen where space is limited".to_string());
    }
    if str_field(m, "description").is_none() {
        info.push("No 'description' — add for app store listings and discoverability".to_string());
    }
    if str_field(m, "theme_color").is_none() {
        info.push("No 'theme_color' — add to style the browser chrome/status bar".to_string());
    }
    if str_field(m, "background_color").is_none() {
        info.push("No 'background_color' — add to color the splash screen background".to_string());
    }
    if str_field(m, "scope").is_none() {
        info.push(
            "No 'scope' — defaults to start_url directory; set explicitly to control navigation"
                .to_string(),
        );
    }

    // Check theme_color is a valid hex
    if let Some(tc) = str_field(m, "theme_color") {
        if !tc.starts_with('#') && !tc.starts_with("rgb") {
            warnings.push(format!(
                "theme_color '{}' should be a CSS color value (e.g. #1a73e8)",
                tc
            ));
        }
    }

    let verdict = if errors.is_empty() && warnings.is_empty() {
        "VALID"
    } else if !errors.is_empty() {
        "INVALID"
    } else {
        "WARNINGS"
    };

    let mut out = format!(
        "WEB MANIFEST VALIDATION\n=======================\n\nVerdict: {}\n\n",
        verdict
    );
    for e in &errors {
        out.push_str(&format!("ERROR   : {}\n", e));
    }
    for w in &warnings {
        out.push_str(&format!("WARNING : {}\n", w));
    }
    for i in &info {
        out.push_str(&format!("INFO    : {}\n", i));
    }
    out.trim_end().to_string()
}

fn action_icons(m: &Value) -> String {
    let icons = match m.get("icons").and_then(|v| v.as_array()) {
        Some(i) => i,
        None => return "No 'icons' array found in manifest.".to_string(),
    };

    let mut out = format!("MANIFEST ICONS ({} total)\n", icons.len());
    out.push_str(&"=".repeat(out.trim_end().len()));
    out.push_str("\n\n");
    out.push_str(&format!(
        "{:<40} {:<20} {:<12} {}\n",
        "Source", "Sizes", "Type", "Purpose"
    ));
    out.push_str(&format!(
        "{:<40} {:<20} {:<12} {}\n",
        "-".repeat(38),
        "-".repeat(18),
        "-".repeat(10),
        "-".repeat(10)
    ));

    for icon in icons {
        let src = icon.get("src").and_then(|v| v.as_str()).unwrap_or("?");
        let sizes = icon.get("sizes").and_then(|v| v.as_str()).unwrap_or("any");
        let mime = icon.get("type").and_then(|v| v.as_str()).unwrap_or("-");
        let purpose = icon
            .get("purpose")
            .and_then(|v| v.as_str())
            .unwrap_or("any");

        // Truncate long src
        let src_display = if src.len() > 38 {
            format!("...{}", &src[src.len() - 35..])
        } else {
            src.to_string()
        };

        out.push_str(&format!(
            "{:<40} {:<20} {:<12} {}\n",
            src_display, sizes, mime, purpose
        ));
    }
    out.trim_end().to_string()
}

fn action_screenshots(m: &Value) -> String {
    let ss = match m.get("screenshots").and_then(|v| v.as_array()) {
        Some(s) if !s.is_empty() => s,
        _ => return "No 'screenshots' array found in manifest.".to_string(),
    };

    let mut out = format!("MANIFEST SCREENSHOTS ({} total)\n", ss.len());
    out.push_str(&"=".repeat(32));
    out.push_str("\n\n");

    for (i, s) in ss.iter().enumerate() {
        let src = s.get("src").and_then(|v| v.as_str()).unwrap_or("?");
        let sizes = s.get("sizes").and_then(|v| v.as_str()).unwrap_or("-");
        let mime = s.get("type").and_then(|v| v.as_str()).unwrap_or("-");
        let label = s.get("label").and_then(|v| v.as_str()).unwrap_or("");
        let form_factor = s.get("form_factor").and_then(|v| v.as_str()).unwrap_or("-");
        let platform = s.get("platform").and_then(|v| v.as_str()).unwrap_or("-");

        out.push_str(&format!("[{}] {}\n", i + 1, src));
        out.push_str(&format!("  Sizes       : {}\n", sizes));
        out.push_str(&format!("  Type        : {}\n", mime));
        out.push_str(&format!("  Form factor : {}\n", form_factor));
        out.push_str(&format!("  Platform    : {}\n", platform));
        if !label.is_empty() {
            out.push_str(&format!("  Label       : {}\n", label));
        }
        out.push('\n');
    }
    out.trim_end().to_string()
}

fn action_info(m: &Value) -> String {
    let mut out = String::from(
        "WEB MANIFEST — INSTALL & DISPLAY INFO\n======================================\n\n",
    );

    let display = str_field(m, "display").unwrap_or("browser (default)");
    let orientation = str_field(m, "orientation").unwrap_or("natural (default)");
    let start_url = str_field(m, "start_url").unwrap_or("/");
    let scope = str_field(m, "scope").unwrap_or("(same as start_url dir)");
    let theme = str_field(m, "theme_color").unwrap_or("(not set)");
    let bg = str_field(m, "background_color").unwrap_or("(not set)");

    out.push_str(&format!("Display mode    : {}\n", display));
    out.push_str(&format!("Orientation     : {}\n", orientation));
    out.push_str(&format!("Start URL       : {}\n", start_url));
    out.push_str(&format!("Scope           : {}\n", scope));
    out.push_str(&format!("Theme color     : {}\n", theme));
    out.push_str(&format!("Background color: {}\n", bg));

    // Display override
    if let Some(ov) = m.get("display_override").and_then(|v| v.as_array()) {
        let modes: Vec<&str> = ov.iter().filter_map(|v| v.as_str()).collect();
        out.push_str(&format!("Display override: {}\n", modes.join(", ")));
    }

    out.push_str("\n--- Advanced Features ---\n");

    let features = [
        ("share_target", "Web Share Target API"),
        ("file_handlers", "File Handling API"),
        ("protocol_handlers", "Protocol Handler API"),
        ("handle_links", "Handle Links API"),
        ("edge_side_panel", "Edge Side Panel"),
        ("launch_handler", "Launch Handler API"),
        ("capture_links", "Capture Links"),
        ("scope_extensions", "Scope Extensions"),
    ];

    let mut any_feature = false;
    for (key, label) in &features {
        if m.get(*key).is_some() {
            out.push_str(&format!("{:<26}: declared ✓\n", label));
            any_feature = true;
        }
    }
    if !any_feature {
        out.push_str("No advanced browser API declarations found.\n");
    }

    // Shortcuts summary
    if let Some(sc) = m.get("shortcuts").and_then(|v| v.as_array()) {
        out.push_str(&format!("\nShortcuts ({}):\n", sc.len()));
        for s in sc {
            let name = s.get("name").and_then(|v| v.as_str()).unwrap_or("?");
            let url = s.get("url").and_then(|v| v.as_str()).unwrap_or("?");
            out.push_str(&format!("  • {} → {}\n", name, url));
        }
    }

    out.trim_end().to_string()
}

pub async fn execute(args: &Value) -> Result<String, String> {
    let m = load_manifest(args)?;
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("parse");
    match action {
        "parse" => Ok(action_parse(&m)),
        "validate" => Ok(action_validate(&m)),
        "icons" => Ok(action_icons(&m)),
        "screenshots" => Ok(action_screenshots(&m)),
        "info" => Ok(action_info(&m)),
        other => Err(format!(
            "Unknown action '{}'. Use: parse, validate, icons, screenshots, info",
            other
        )),
    }
}
