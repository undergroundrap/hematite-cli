use serde_json::{json, Value};

pub fn cite_tools_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "action": {
                "type": "string",
                "enum": ["format", "bibtex", "parse_doi", "parse_isbn", "validate"],
                "description": "format: format citation in a style | bibtex: generate BibTeX entry | parse_doi: extract DOI metadata | parse_isbn: validate ISBN | validate: check citation fields"
            },
            "style": {
                "type": "string",
                "enum": ["apa", "mla", "chicago", "ieee", "harvard"],
                "description": "Citation style (default: apa)"
            },
            "type": {
                "type": "string",
                "enum": ["article", "book", "chapter", "website", "conference", "thesis", "report"],
                "description": "Source type (default: article)"
            },
            "authors": {
                "description": "Author(s) as a string ('Last, First and Last, First') or array of strings",
                "oneOf": [{"type": "string"}, {"type": "array", "items": {"type": "string"}}]
            },
            "title": {"type": "string", "description": "Title of the work"},
            "journal": {"type": "string", "description": "Journal or magazine name"},
            "book": {"type": "string", "description": "Book title (for chapters)"},
            "publisher": {"type": "string", "description": "Publisher name"},
            "year": {"type": "string", "description": "Publication year"},
            "volume": {"type": "string", "description": "Volume number"},
            "issue": {"type": "string", "description": "Issue number"},
            "pages": {"type": "string", "description": "Page range (e.g. '123-145')"},
            "doi": {"type": "string", "description": "DOI identifier"},
            "isbn": {"type": "string", "description": "ISBN-10 or ISBN-13"},
            "url": {"type": "string", "description": "URL for website or online sources"},
            "accessed": {"type": "string", "description": "Date accessed for websites (e.g. '15 June 2025')"},
            "city": {"type": "string", "description": "Publication city"},
            "edition": {"type": "string", "description": "Edition number"},
            "editors": {"type": "string", "description": "Editors (for chapters in edited books)"},
            "institution": {"type": "string", "description": "Institution (for theses/reports)"},
            "degree": {"type": "string", "description": "Degree type (e.g. 'PhD', 'Master')"},
            "key": {"type": "string", "description": "BibTeX citation key (auto-generated if omitted)"}
        },
        "required": []
    })
}

struct Citation<'a> {
    authors: Vec<String>,
    title: &'a str,
    journal: &'a str,
    book: &'a str,
    publisher: &'a str,
    year: &'a str,
    volume: &'a str,
    issue: &'a str,
    pages: &'a str,
    doi: &'a str,
    url: &'a str,
    accessed: &'a str,
    city: &'a str,
    edition: &'a str,
    editors: &'a str,
    institution: &'a str,
    degree: &'a str,
    source_type: &'a str,
}

fn parse_authors(args: &Value) -> Vec<String> {
    match args.get("authors") {
        Some(Value::Array(arr)) => arr
            .iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect(),
        Some(Value::String(s)) => s.split(" and ").map(|a| a.trim().to_string()).collect(),
        _ => vec![],
    }
}

fn extract<'a>(args: &'a Value, key: &str) -> &'a str {
    args.get(key).and_then(|v| v.as_str()).unwrap_or("")
}

fn author_last_first(name: &str) -> String {
    // Accepts "Last, First" or "First Last"
    if name.contains(',') {
        name.trim().to_string()
    } else {
        let parts: Vec<&str> = name.trim().splitn(2, ' ').collect();
        if parts.len() == 2 {
            format!("{}, {}", parts[1], parts[0])
        } else {
            name.to_string()
        }
    }
}

fn author_first_last(name: &str) -> String {
    if name.contains(',') {
        let parts: Vec<&str> = name.splitn(2, ',').collect();
        if parts.len() == 2 {
            format!("{} {}", parts[1].trim(), parts[0].trim())
        } else {
            name.to_string()
        }
    } else {
        name.trim().to_string()
    }
}

fn initials(first: &str) -> String {
    first
        .split_whitespace()
        .map(|w| format!("{}.", w.chars().next().unwrap_or(' ')))
        .collect::<Vec<_>>()
        .join(" ")
}

fn author_with_initials(name: &str) -> String {
    // Produce "Smith, J. A."
    let lf = author_last_first(name);
    if lf.contains(',') {
        let parts: Vec<&str> = lf.splitn(2, ',').collect();
        let last = parts[0].trim();
        let first = parts[1].trim();
        format!("{}, {}", last, initials(first))
    } else {
        lf
    }
}

fn format_apa(c: &Citation) -> String {
    let mut out = String::new();
    // Authors
    let authors_str = match c.authors.len() {
        0 => String::new(),
        1 => format!("{}.", author_with_initials(&c.authors[0])),
        2..=7 => {
            let parts: Vec<String> = c.authors.iter().map(|a| author_with_initials(a)).collect();
            let (last, rest) = parts.split_last().unwrap();
            format!("{}, & {}.", rest.join(", "), last)
        }
        _ => {
            let first6: Vec<String> = c.authors[..6]
                .iter()
                .map(|a| author_with_initials(a))
                .collect();
            format!(
                "{}, . . . {}.",
                first6.join(", "),
                author_with_initials(c.authors.last().unwrap())
            )
        }
    };
    if !authors_str.is_empty() {
        out.push_str(&authors_str);
        out.push(' ');
    }
    // Year
    if !c.year.is_empty() {
        out.push_str(&format!("({}). ", c.year));
    }
    // Title
    if !c.title.is_empty() {
        match c.source_type {
            "article" | "chapter" | "conference" => out.push_str(&format!("{}. ", c.title)),
            _ => out.push_str(&format!("*{}*. ", c.title)),
        }
    }
    match c.source_type {
        "article" => {
            if !c.journal.is_empty() {
                out.push_str(&format!("*{}*", c.journal));
            }
            if !c.volume.is_empty() {
                out.push_str(&format!(", *{}*", c.volume));
            }
            if !c.issue.is_empty() {
                out.push_str(&format!("({})", c.issue));
            }
            if !c.pages.is_empty() {
                out.push_str(&format!(", {}", c.pages));
            }
            out.push('.');
        }
        "book" => {
            if !c.edition.is_empty() {
                out.push_str(&format!("({} ed.). ", c.edition));
            }
            if !c.publisher.is_empty() {
                out.push_str(&format!("{}.", c.publisher));
            }
        }
        "chapter" => {
            if !c.editors.is_empty() {
                out.push_str(&format!("In {} (Eds.), ", c.editors));
            }
            if !c.book.is_empty() {
                out.push_str(&format!("*{}*", c.book));
            }
            if !c.pages.is_empty() {
                out.push_str(&format!(" (pp. {})", c.pages));
            }
            out.push_str(". ");
            if !c.publisher.is_empty() {
                out.push_str(&format!("{}.", c.publisher));
            }
        }
        "website" => {
            if !c.url.is_empty() {
                out.push_str(c.url);
            }
        }
        _ => {
            if !c.publisher.is_empty() {
                out.push_str(&format!("{}.", c.publisher));
            }
        }
    }
    if !c.doi.is_empty() {
        out.push_str(&format!(" https://doi.org/{}", c.doi));
    }
    out.trim().to_string()
}

fn format_mla(c: &Citation) -> String {
    let mut out = String::new();
    // First author Last, First; rest First Last
    if !c.authors.is_empty() {
        out.push_str(&author_last_first(&c.authors[0]));
        for a in &c.authors[1..] {
            out.push_str(&format!(", and {}", author_first_last(a)));
        }
        out.push_str(". ");
    }
    if !c.title.is_empty() {
        match c.source_type {
            "article" | "chapter" | "conference" => out.push_str(&format!("\"{}\" ", c.title)),
            _ => out.push_str(&format!("*{}* ", c.title)),
        }
    }
    match c.source_type {
        "article" => {
            if !c.journal.is_empty() {
                out.push_str(&format!("*{}*, ", c.journal));
            }
            if !c.volume.is_empty() {
                out.push_str(&format!("vol. {}, ", c.volume));
            }
            if !c.issue.is_empty() {
                out.push_str(&format!("no. {}, ", c.issue));
            }
            if !c.year.is_empty() {
                out.push_str(&format!("{}, ", c.year));
            }
            if !c.pages.is_empty() {
                out.push_str(&format!("pp. {}.", c.pages));
            }
        }
        "book" => {
            if !c.publisher.is_empty() {
                out.push_str(&format!("{}, ", c.publisher));
            }
            if !c.year.is_empty() {
                out.push_str(&format!("{}.", c.year));
            }
        }
        "website" => {
            if !c.url.is_empty() {
                out.push_str(c.url);
            }
            if !c.accessed.is_empty() {
                out.push_str(&format!(" Accessed {}", c.accessed));
            }
        }
        _ => {
            if !c.publisher.is_empty() {
                out.push_str(&format!("{}, ", c.publisher));
            }
            if !c.year.is_empty() {
                out.push_str(&format!("{}.", c.year));
            }
        }
    }
    if !c.doi.is_empty() {
        out.push_str(&format!(" doi:{}", c.doi));
    }
    out.trim().to_string()
}

fn format_chicago(c: &Citation) -> String {
    let mut out = String::new();
    if !c.authors.is_empty() {
        out.push_str(&author_last_first(&c.authors[0]));
        for a in &c.authors[1..] {
            out.push_str(&format!(", and {}", author_first_last(a)));
        }
        out.push_str(". ");
    }
    if !c.title.is_empty() {
        match c.source_type {
            "article" | "chapter" | "conference" => out.push_str(&format!("\"{}\" ", c.title)),
            _ => out.push_str(&format!("*{}* ", c.title)),
        }
    }
    match c.source_type {
        "article" => {
            if !c.journal.is_empty() {
                out.push_str(&format!("*{}* ", c.journal));
            }
            if !c.volume.is_empty() {
                out.push_str(&format!("{}, ", c.volume));
            }
            if !c.issue.is_empty() {
                out.push_str(&format!("no. {} ", c.issue));
            }
            if !c.year.is_empty() {
                out.push_str(&format!("({}): ", c.year));
            }
            if !c.pages.is_empty() {
                out.push_str(&format!("{}.", c.pages));
            }
        }
        "book" => {
            if !c.city.is_empty() {
                out.push_str(&format!("{}: ", c.city));
            }
            if !c.publisher.is_empty() {
                out.push_str(&format!("{}, ", c.publisher));
            }
            if !c.year.is_empty() {
                out.push_str(&format!("{}.", c.year));
            }
        }
        "website" => {
            if !c.year.is_empty() {
                out.push_str(c.year);
            }
            if !c.url.is_empty() {
                out.push_str(&format!(" {}", c.url));
            }
            if !c.accessed.is_empty() {
                out.push_str(&format!(" (accessed {}).", c.accessed));
            }
        }
        _ => {
            if !c.publisher.is_empty() {
                out.push_str(&format!("{}, ", c.publisher));
            }
            if !c.year.is_empty() {
                out.push_str(&format!("{}.", c.year));
            }
        }
    }
    if !c.doi.is_empty() {
        out.push_str(&format!(" https://doi.org/{}", c.doi));
    }
    out.trim().to_string()
}

fn format_ieee(c: &Citation) -> String {
    let mut out = String::new();
    // IEEE: initials first, then last
    let fmt_ieee_author = |name: &str| -> String {
        let lf = author_last_first(name);
        if lf.contains(',') {
            let parts: Vec<&str> = lf.splitn(2, ',').collect();
            format!("{} {}", initials(parts[1].trim()), parts[0].trim())
        } else {
            lf.to_string()
        }
    };
    if !c.authors.is_empty() {
        let parts: Vec<String> = c.authors.iter().map(|a| fmt_ieee_author(a)).collect();
        out.push_str(&parts.join(", "));
        out.push_str(", ");
    }
    if !c.title.is_empty() {
        out.push_str(&format!("\"{}\" ", c.title));
    }
    match c.source_type {
        "article" => {
            if !c.journal.is_empty() {
                out.push_str(&format!("*{}*, ", c.journal));
            }
            if !c.volume.is_empty() {
                out.push_str(&format!("vol. {}, ", c.volume));
            }
            if !c.issue.is_empty() {
                out.push_str(&format!("no. {}, ", c.issue));
            }
            if !c.pages.is_empty() {
                out.push_str(&format!("pp. {}, ", c.pages));
            }
            if !c.year.is_empty() {
                out.push_str(&format!("{}.", c.year));
            }
        }
        "book" => {
            if !c.city.is_empty() {
                out.push_str(&format!("{}:", c.city));
            }
            if !c.publisher.is_empty() {
                out.push_str(&format!(" {}, ", c.publisher));
            }
            if !c.year.is_empty() {
                out.push_str(&format!("{}.", c.year));
            }
        }
        "conference" => {
            if !c.book.is_empty() {
                out.push_str(&format!("in *{}*, ", c.book));
            }
            if !c.year.is_empty() {
                out.push_str(&format!("{}, ", c.year));
            }
            if !c.pages.is_empty() {
                out.push_str(&format!("pp. {}.", c.pages));
            }
        }
        _ => {
            if !c.publisher.is_empty() {
                out.push_str(&format!("{}, ", c.publisher));
            }
            if !c.year.is_empty() {
                out.push_str(&format!("{}.", c.year));
            }
        }
    }
    if !c.doi.is_empty() {
        out.push_str(&format!(" doi: {}", c.doi));
    }
    out.trim().to_string()
}

fn format_harvard(c: &Citation) -> String {
    let mut out = String::new();
    if !c.authors.is_empty() {
        let parts: Vec<String> = c.authors.iter().map(|a| author_with_initials(a)).collect();
        out.push_str(&parts.join(", "));
        out.push_str(". ");
    }
    if !c.year.is_empty() {
        out.push_str(&format!("{}. ", c.year));
    }
    if !c.title.is_empty() {
        match c.source_type {
            "article" | "chapter" | "conference" => out.push_str(&format!("'{}' ", c.title)),
            _ => out.push_str(&format!("*{}* ", c.title)),
        }
    }
    match c.source_type {
        "article" => {
            if !c.journal.is_empty() {
                out.push_str(&format!("*{}*, ", c.journal));
            }
            if !c.volume.is_empty() {
                out.push_str(&format!("{}(", c.volume));
            }
            if !c.issue.is_empty() {
                out.push_str(c.issue);
            }
            if !c.volume.is_empty() {
                out.push_str("), ");
            }
            if !c.pages.is_empty() {
                out.push_str(&format!("pp. {}.", c.pages));
            }
        }
        "book" => {
            if !c.edition.is_empty() {
                out.push_str(&format!("{} edn, ", c.edition));
            }
            if !c.city.is_empty() {
                out.push_str(&format!("{}: ", c.city));
            }
            if !c.publisher.is_empty() {
                out.push_str(&format!("{}.", c.publisher));
            }
        }
        _ => {
            if !c.publisher.is_empty() {
                out.push_str(&format!("{}.", c.publisher));
            }
        }
    }
    if !c.doi.is_empty() {
        out.push_str(&format!(" https://doi.org/{}", c.doi));
    }
    out.trim().to_string()
}

fn action_format(args: &Value) -> String {
    let style = args.get("style").and_then(|v| v.as_str()).unwrap_or("apa");
    let source_type = args
        .get("type")
        .and_then(|v| v.as_str())
        .unwrap_or("article");
    let c = Citation {
        authors: parse_authors(args),
        title: extract(args, "title"),
        journal: extract(args, "journal"),
        book: extract(args, "book"),
        publisher: extract(args, "publisher"),
        year: extract(args, "year"),
        volume: extract(args, "volume"),
        issue: extract(args, "issue"),
        pages: extract(args, "pages"),
        doi: extract(args, "doi"),
        url: extract(args, "url"),
        accessed: extract(args, "accessed"),
        city: extract(args, "city"),
        edition: extract(args, "edition"),
        editors: extract(args, "editors"),
        institution: extract(args, "institution"),
        degree: extract(args, "degree"),
        source_type,
    };
    let formatted = match style {
        "apa" => format_apa(&c),
        "mla" => format_mla(&c),
        "chicago" => format_chicago(&c),
        "ieee" => format_ieee(&c),
        "harvard" => format_harvard(&c),
        other => {
            return format!(
                "Unknown style '{}'. Use: apa, mla, chicago, ieee, harvard",
                other
            )
        }
    };
    let mut out = format!(
        "CITATION ({})\n{}\n\n",
        style.to_uppercase(),
        "=".repeat(20)
    );
    out.push_str(&formatted);
    out
}

fn bibtex_key(authors: &[String], year: &str) -> String {
    let last = authors
        .first()
        .map(|a| {
            author_last_first(a)
                .split(',')
                .next()
                .unwrap_or("author")
                .trim()
                .to_lowercase()
                .replace(' ', "_")
        })
        .unwrap_or_else(|| "author".to_string());
    format!("{}{}", last, year)
}

fn action_bibtex(args: &Value) -> String {
    let source_type = args
        .get("type")
        .and_then(|v| v.as_str())
        .unwrap_or("article");
    let authors = parse_authors(args);
    let year = extract(args, "year");
    let key = if extract(args, "key").is_empty() {
        bibtex_key(&authors, year)
    } else {
        extract(args, "key").to_string()
    };
    let entry_type = match source_type {
        "article" => "article",
        "book" => "book",
        "chapter" => "incollection",
        "conference" => "inproceedings",
        "thesis" => "phdthesis",
        "report" => "techreport",
        _ => "misc",
    };
    let authors_str = authors
        .iter()
        .map(|a| author_last_first(a))
        .collect::<Vec<_>>()
        .join(" and ");
    let mut fields: Vec<(String, String)> = Vec::new();
    if !authors_str.is_empty() {
        fields.push(("author".into(), authors_str));
    }
    if !extract(args, "title").is_empty() {
        fields.push(("title".into(), extract(args, "title").to_string()));
    }
    if !extract(args, "journal").is_empty() {
        fields.push(("journal".into(), extract(args, "journal").to_string()));
    }
    if !extract(args, "book").is_empty() {
        fields.push(("booktitle".into(), extract(args, "book").to_string()));
    }
    if !extract(args, "year").is_empty() {
        fields.push(("year".into(), extract(args, "year").to_string()));
    }
    if !extract(args, "volume").is_empty() {
        fields.push(("volume".into(), extract(args, "volume").to_string()));
    }
    if !extract(args, "issue").is_empty() {
        fields.push(("number".into(), extract(args, "issue").to_string()));
    }
    if !extract(args, "pages").is_empty() {
        fields.push(("pages".into(), extract(args, "pages").replace('-', "--")));
    }
    if !extract(args, "publisher").is_empty() {
        fields.push(("publisher".into(), extract(args, "publisher").to_string()));
    }
    if !extract(args, "city").is_empty() {
        fields.push(("address".into(), extract(args, "city").to_string()));
    }
    if !extract(args, "doi").is_empty() {
        fields.push(("doi".into(), extract(args, "doi").to_string()));
    }
    if !extract(args, "url").is_empty() {
        fields.push(("url".into(), extract(args, "url").to_string()));
    }
    if !extract(args, "institution").is_empty() {
        fields.push((
            "institution".into(),
            extract(args, "institution").to_string(),
        ));
    }
    let body = fields
        .iter()
        .map(|(k, v)| format!("  {} = {{{}}}", k, v))
        .collect::<Vec<_>>()
        .join(",\n");
    format!("@{}{{{},\n{}\n}}", entry_type, key, body)
}

fn validate_doi(doi: &str) -> String {
    let doi = doi
        .trim()
        .trim_start_matches("https://doi.org/")
        .trim_start_matches("doi:");
    if doi.starts_with("10.") && doi.contains('/') {
        let parts: Vec<&str> = doi.splitn(2, '/').collect();
        format!(
            "DOI VALIDATION\n==============\n\nInput    : {}\nStatus   : VALID\nPrefix   : {}\nSuffix   : {}\nURL      : https://doi.org/{}",
            doi, parts[0], parts[1], doi
        )
    } else {
        format!(
            "DOI VALIDATION\n==============\n\nInput  : {}\nStatus : INVALID -- DOI must start with '10.' and contain '/'",
            doi
        )
    }
}

fn validate_isbn(isbn: &str) -> String {
    let clean: String = isbn.chars().filter(|c| c.is_ascii_alphanumeric()).collect();
    let mut out = format!("ISBN VALIDATION\n===============\n\nInput  : {}\n", isbn);
    match clean.len() {
        10 => {
            let digits: Vec<u32> = clean
                .chars()
                .enumerate()
                .map(|(i, c)| {
                    if i == 9 && (c == 'X' || c == 'x') {
                        10
                    } else {
                        c.to_digit(10).unwrap_or(0)
                    }
                })
                .collect();
            let sum: u32 = digits
                .iter()
                .enumerate()
                .map(|(i, &d)| d * (10 - i as u32))
                .sum();
            let valid = sum % 11 == 0;
            out.push_str(&format!(
                "Format : ISBN-10\nStatus : {}\n",
                if valid {
                    "VALID"
                } else {
                    "INVALID (check digit error)"
                }
            ));
            out.push_str(&format!("ISBN-13: 978-{}\n", &clean[..9]));
        }
        13 => {
            let digits: Vec<u32> = clean.chars().filter_map(|c| c.to_digit(10)).collect();
            if digits.len() == 13 {
                let sum: u32 = digits
                    .iter()
                    .enumerate()
                    .map(|(i, &d)| if i % 2 == 0 { d } else { d * 3 })
                    .sum();
                let valid = sum % 10 == 0;
                out.push_str(&format!(
                    "Format : ISBN-13\nStatus : {}\n",
                    if valid {
                        "VALID"
                    } else {
                        "INVALID (check digit error)"
                    }
                ));
            } else {
                out.push_str("Format : ISBN-13 (contains non-digits)\nStatus : INVALID\n");
            }
        }
        _ => {
            out.push_str(&format!(
                "Status : INVALID -- expected 10 or 13 digits, got {}\n",
                clean.len()
            ));
        }
    }
    out.trim_end().to_string()
}

fn action_validate(args: &Value) -> String {
    let mut issues: Vec<String> = Vec::new();
    let mut present: Vec<String> = Vec::new();
    let source_type = args
        .get("type")
        .and_then(|v| v.as_str())
        .unwrap_or("article");
    let check = |key: &str| {
        args.get(key)
            .and_then(|v| v.as_str())
            .map(|s| !s.is_empty())
            .unwrap_or(false)
            || args
                .get(key)
                .and_then(|v| v.as_array())
                .map(|a| !a.is_empty())
                .unwrap_or(false)
    };
    for field in &["title", "authors", "year"] {
        if check(field) {
            present.push(field.to_string());
        } else {
            issues.push(format!("Missing required field: {}", field));
        }
    }
    match source_type {
        "article" => {
            for f in &["journal", "volume", "pages"] {
                if check(f) {
                    present.push(f.to_string());
                } else {
                    issues.push(format!("Missing recommended field for article: {}", f));
                }
            }
        }
        "book" => {
            for f in &["publisher"] {
                if check(f) {
                    present.push(f.to_string());
                } else {
                    issues.push(format!("Missing recommended field for book: {}", f));
                }
            }
        }
        _ => {}
    }
    let verdict = if issues.is_empty() {
        "VALID"
    } else {
        "INCOMPLETE"
    };
    let mut out = format!(
        "CITATION VALIDATION\n===================\n\nType    : {}\nVerdict : {}\n\n",
        source_type, verdict
    );
    for p in &present {
        out.push_str(&format!("  + {}\n", p));
    }
    for i in &issues {
        out.push_str(&format!("  ! {}\n", i));
    }
    out.trim_end().to_string()
}

pub async fn execute(args: &Value) -> Result<String, String> {
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("format");
    Ok(match action {
        "format" => action_format(args),
        "bibtex" => action_bibtex(args),
        "parse_doi" | "doi" => {
            let doi = args.get("doi").and_then(|v| v.as_str()).unwrap_or("");
            if doi.is_empty() {
                "Provide 'doi' field to validate (e.g. '10.1038/nature09210').".to_string()
            } else {
                validate_doi(doi)
            }
        }
        "parse_isbn" | "isbn" => {
            let isbn = args.get("isbn").and_then(|v| v.as_str()).unwrap_or("");
            if isbn.is_empty() {
                "Provide 'isbn' field to validate (e.g. '9780306406157').".to_string()
            } else {
                validate_isbn(isbn)
            }
        }
        "validate" => action_validate(args),
        other => format!(
            "Unknown action '{}'. Use: format, bibtex, parse_doi, parse_isbn, validate",
            other
        ),
    })
}
