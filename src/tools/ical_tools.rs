use serde_json::Value;

pub async fn execute(args: &Value) -> Result<String, String> {
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("parse");
    let text = get_text(args)?;
    match action {
        "parse" | "list" => action_parse(&text),
        "events" => action_events(&text),
        "todos" => action_todos(&text),
        "info" => action_info(&text),
        "search" => {
            let q = args
                .get("query")
                .or_else(|| args.get("q"))
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_lowercase();
            action_search(&text, &q)
        }
        other => Err(format!(
            "ical_tools: unknown action '{other}'. Valid: parse, events, todos, info, search"
        )),
    }
}

// ── input ─────────────────────────────────────────────────────────────────────

fn get_text(args: &Value) -> Result<String, String> {
    for key in &["text", "ical", "ics", "content", "input"] {
        if let Some(v) = args.get(key).and_then(|v| v.as_str()) {
            return Ok(v.to_string());
        }
    }
    if let Some(path) = args.get("file").and_then(|v| v.as_str()) {
        return std::fs::read_to_string(path).map_err(|e| format!("cannot read '{}': {}", path, e));
    }
    Err(
        "ical_tools: pass 'text' with iCalendar content or 'file' with a path to a .ics file"
            .into(),
    )
}

// ── parser ────────────────────────────────────────────────────────────────────

#[derive(Debug)]
struct ICalComponent {
    kind: String,
    props: Vec<(String, String, String)>, // (name, params, value)
}

fn unfold(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\r' || c == '\n' {
            if c == '\r' {
                chars.next_if(|&x| x == '\n');
            }
            if matches!(chars.peek(), Some(' ') | Some('\t')) {
                chars.next();
            } else {
                out.push('\n');
            }
        } else {
            out.push(c);
        }
    }
    out
}

fn parse_ical(text: &str) -> Vec<ICalComponent> {
    let unfolded = unfold(text);
    let mut stack: Vec<ICalComponent> = Vec::new();
    let mut done: Vec<ICalComponent> = Vec::new();

    for line in unfolded.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if let Some(kind) = line.strip_prefix("BEGIN:") {
            stack.push(ICalComponent {
                kind: kind.trim().to_uppercase(),
                props: Vec::new(),
            });
        } else if let Some(_kind) = line.strip_prefix("END:") {
            if let Some(comp) = stack.pop() {
                if stack.is_empty() {
                    done.push(comp);
                } else {
                    // nested — keep only top-level but record leaf info in parent
                    let parent = stack.last_mut().unwrap();
                    // merge leaf summary into parent
                    for prop in comp.props {
                        parent.props.push(prop);
                    }
                }
            }
        } else if let Some(comp) = stack.last_mut() {
            // parse property
            let colon = line.find(':').unwrap_or(line.len());
            let name_part = &line[..colon];
            let value = if colon < line.len() {
                line[colon + 1..].trim().to_string()
            } else {
                String::new()
            };
            let semi = name_part.find(';').unwrap_or(name_part.len());
            let name = name_part[..semi].to_uppercase();
            let params = if semi < name_part.len() {
                name_part[semi + 1..].to_string()
            } else {
                String::new()
            };
            comp.props.push((name, params, value));
        }
    }
    done
}

fn prop<'a>(comp: &'a ICalComponent, name: &str) -> Option<&'a str> {
    comp.props
        .iter()
        .find(|(n, _, _)| n == name)
        .map(|(_, _, v)| v.as_str())
}

fn fmt_dt(s: &str) -> String {
    let s = s.trim_end_matches('Z').replace('T', " ").replace("--", "-");
    if s.len() >= 15 {
        format!(
            "{}-{}-{} {}:{}",
            &s[0..4],
            &s[4..6],
            &s[6..8],
            &s[9..11],
            &s[11..13]
        )
    } else if s.len() >= 8 {
        format!("{}-{}-{}", &s[0..4], &s[4..6], &s[6..8])
    } else {
        s
    }
}

// ── actions ───────────────────────────────────────────────────────────────────

fn action_info(text: &str) -> Result<String, String> {
    let comps = parse_ical(text);
    let cal = comps.iter().find(|c| c.kind == "VCALENDAR");
    let events: Vec<_> = comps.iter().filter(|c| c.kind == "VEVENT").collect();
    let todos: Vec<_> = comps.iter().filter(|c| c.kind == "VTODO").collect();
    let journals: Vec<_> = comps.iter().filter(|c| c.kind == "VJOURNAL").collect();
    let freebusy: Vec<_> = comps.iter().filter(|c| c.kind == "VFREEBUSY").collect();
    let timezones: Vec<_> = comps.iter().filter(|c| c.kind == "VTIMEZONE").collect();

    let mut out = String::from("iCalendar File Info\n");
    out.push_str(&"─".repeat(40));
    out.push('\n');

    if let Some(c) = cal {
        if let Some(v) = prop(c, "VERSION") {
            out.push_str(&format!("iCal Version:  {}\n", v));
        }
        if let Some(p) = prop(c, "PRODID") {
            out.push_str(&format!("Producer:      {}\n", p));
        }
        if let Some(n) = prop(c, "X-WR-CALNAME") {
            out.push_str(&format!("Calendar Name: {}\n", n));
        }
        if let Some(d) = prop(c, "X-WR-CALDESC") {
            out.push_str(&format!("Description:   {}\n", d));
        }
        if let Some(tz) = prop(c, "X-WR-TIMEZONE") {
            out.push_str(&format!("Timezone:      {}\n", tz));
        }
    }

    out.push('\n');
    out.push_str(&format!("Events (VEVENT):       {}\n", events.len()));
    out.push_str(&format!("Todos (VTODO):         {}\n", todos.len()));
    out.push_str(&format!("Journals (VJOURNAL):   {}\n", journals.len()));
    out.push_str(&format!("Free/Busy (VFREEBUSY): {}\n", freebusy.len()));
    out.push_str(&format!("Timezones (VTIMEZONE): {}\n", timezones.len()));
    Ok(out)
}

fn action_parse(text: &str) -> Result<String, String> {
    let comps = parse_ical(text);
    let events: Vec<_> = comps.iter().filter(|c| c.kind == "VEVENT").collect();
    let todos: Vec<_> = comps.iter().filter(|c| c.kind == "VTODO").collect();

    if events.is_empty() && todos.is_empty() {
        return Ok("No VEVENT or VTODO components found.".into());
    }

    let mut out = String::new();
    if !events.is_empty() {
        out.push_str(&format!("Events ({})\n{}\n", events.len(), "─".repeat(50)));
        for (i, e) in events.iter().enumerate() {
            out.push_str(&format!(
                "\n[{}] {}\n",
                i + 1,
                prop(e, "SUMMARY").unwrap_or("(no title)")
            ));
            if let Some(s) = prop(e, "DTSTART") {
                out.push_str(&format!("    Start:    {}\n", fmt_dt(s)));
            }
            if let Some(s) = prop(e, "DTEND") {
                out.push_str(&format!("    End:      {}\n", fmt_dt(s)));
            }
            if let Some(s) = prop(e, "DURATION") {
                out.push_str(&format!("    Duration: {}\n", s));
            }
            if let Some(s) = prop(e, "RRULE") {
                out.push_str(&format!("    Recur:    {}\n", s));
            }
            if let Some(s) = prop(e, "LOCATION") {
                out.push_str(&format!("    Location: {}\n", s));
            }
            if let Some(s) = prop(e, "ORGANIZER").or_else(|| prop(e, "ATTENDEE")) {
                out.push_str(&format!(
                    "    Organizer:{}\n",
                    s.trim_start_matches("mailto:")
                ));
            }
            if let Some(s) = prop(e, "STATUS") {
                out.push_str(&format!("    Status:   {}\n", s));
            }
            if let Some(s) = prop(e, "DESCRIPTION") {
                let short: String = s.chars().take(120).collect();
                out.push_str(&format!("    Desc:     {}…\n", short));
            }
        }
    }

    if !todos.is_empty() {
        if !out.is_empty() {
            out.push('\n');
        }
        out.push_str(&format!("Todos ({})\n{}\n", todos.len(), "─".repeat(50)));
        for (i, t) in todos.iter().enumerate() {
            out.push_str(&format!(
                "\n[{}] {}\n",
                i + 1,
                prop(t, "SUMMARY").unwrap_or("(no title)")
            ));
            if let Some(s) = prop(t, "DUE") {
                out.push_str(&format!("    Due:      {}\n", fmt_dt(s)));
            }
            if let Some(s) = prop(t, "STATUS") {
                out.push_str(&format!("    Status:   {}\n", s));
            }
            if let Some(s) = prop(t, "PRIORITY") {
                out.push_str(&format!("    Priority: {}\n", s));
            }
        }
    }

    Ok(out)
}

fn action_events(text: &str) -> Result<String, String> {
    action_parse(text)
}

fn action_todos(text: &str) -> Result<String, String> {
    let comps = parse_ical(text);
    let todos: Vec<_> = comps.iter().filter(|c| c.kind == "VTODO").collect();
    if todos.is_empty() {
        return Ok("No VTODO components found.".into());
    }
    let mut out = format!("Todos ({})\n{}\n", todos.len(), "─".repeat(50));
    for (i, t) in todos.iter().enumerate() {
        out.push_str(&format!(
            "\n[{}] {}\n",
            i + 1,
            prop(t, "SUMMARY").unwrap_or("(no title)")
        ));
        if let Some(s) = prop(t, "DUE") {
            out.push_str(&format!("    Due:       {}\n", fmt_dt(s)));
        }
        if let Some(s) = prop(t, "STATUS") {
            out.push_str(&format!("    Status:    {}\n", s));
        }
        if let Some(s) = prop(t, "PRIORITY") {
            out.push_str(&format!("    Priority:  {}\n", s));
        }
        if let Some(s) = prop(t, "DESCRIPTION") {
            let short: String = s.chars().take(120).collect();
            out.push_str(&format!("    Desc:      {}…\n", short));
        }
    }
    Ok(out)
}

fn action_search(text: &str, query: &str) -> Result<String, String> {
    if query.is_empty() {
        return Err("ical_tools search: pass 'query' or 'q' with a search term".into());
    }
    let comps = parse_ical(text);
    let mut matches = Vec::new();
    for comp in &comps {
        if !matches!(comp.kind.as_str(), "VEVENT" | "VTODO" | "VJOURNAL") {
            continue;
        }
        let haystack = comp
            .props
            .iter()
            .map(|(_, _, v)| v.to_lowercase())
            .collect::<Vec<_>>()
            .join(" ");
        if haystack.contains(query) {
            matches.push(comp);
        }
    }
    if matches.is_empty() {
        return Ok(format!("No components matching '{query}' found."));
    }
    let mut out = format!(
        "{} match(es) for '{}'\n{}\n",
        matches.len(),
        query,
        "─".repeat(50)
    );
    for (i, c) in matches.iter().enumerate() {
        out.push_str(&format!(
            "\n[{}] {} — {}\n",
            i + 1,
            c.kind,
            prop(c, "SUMMARY").unwrap_or("(no title)")
        ));
        if let Some(s) = prop(c, "DTSTART") {
            out.push_str(&format!("    Start:  {}\n", fmt_dt(s)));
        }
        if let Some(s) = prop(c, "DTEND") {
            out.push_str(&format!("    End:    {}\n", fmt_dt(s)));
        }
        if let Some(s) = prop(c, "DUE") {
            out.push_str(&format!("    Due:    {}\n", fmt_dt(s)));
        }
        if let Some(s) = prop(c, "LOCATION") {
            out.push_str(&format!("    Loc:    {}\n", s));
        }
    }
    Ok(out)
}
