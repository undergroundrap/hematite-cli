use serde_json::Value;

pub async fn execute(args: &Value) -> Result<String, String> {
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("info");

    let text = get_text(args)?;

    match action {
        "info" => action_info(&text),
        "types" => action_types(&text, args),
        "queries" => action_queries(&text),
        "validate" => action_validate(&text),
        other => Err(format!(
            "Unknown action '{}'. Valid: info, types, queries, validate",
            other
        )),
    }
}

fn get_text(args: &Value) -> Result<String, String> {
    args.get("text")
        .or_else(|| args.get("schema"))
        .or_else(|| args.get("query"))
        .or_else(|| args.get("graphql"))
        .or_else(|| args.get("content"))
        .or_else(|| args.get("input"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| {
            "Missing 'text' — pass GraphQL schema or query content as a string".to_string()
        })
}

// ── Tokeniser ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
enum GTok {
    Name(String),
    Str(String),
    Punct(char),
    Int(String),
    Float(String),
}

fn tokenise(src: &str) -> Vec<GTok> {
    let chars: Vec<char> = src.chars().collect();
    let mut i = 0;
    let mut tokens = Vec::new();

    while i < chars.len() {
        match chars[i] {
            c if c.is_whitespace() || c == ',' => i += 1,
            '#' => {
                while i < chars.len() && chars[i] != '\n' {
                    i += 1;
                }
            }
            '"' if chars.get(i + 1) == Some(&'"') && chars.get(i + 2) == Some(&'"') => {
                i += 3;
                let mut s = String::new();
                while i + 2 < chars.len()
                    && !(chars[i] == '"' && chars[i + 1] == '"' && chars[i + 2] == '"')
                {
                    s.push(chars[i]);
                    i += 1;
                }
                i += 3;
                tokens.push(GTok::Str(s.trim().to_string()));
            }
            '"' => {
                i += 1;
                let mut s = String::new();
                while i < chars.len() && chars[i] != '"' {
                    if chars[i] == '\\' {
                        i += 1;
                    }
                    if i < chars.len() {
                        s.push(chars[i]);
                        i += 1;
                    }
                }
                i += 1;
                tokens.push(GTok::Str(s));
            }
            c if c.is_ascii_digit()
                || (c == '-' && chars.get(i + 1).is_some_and(|n| n.is_ascii_digit())) =>
            {
                let start = i;
                let mut is_float = false;
                if chars[i] == '-' {
                    i += 1;
                }
                while i < chars.len() && chars[i].is_ascii_digit() {
                    i += 1;
                }
                if i < chars.len() && (chars[i] == '.' || chars[i] == 'e' || chars[i] == 'E') {
                    is_float = true;
                    i += 1;
                    while i < chars.len()
                        && (chars[i].is_ascii_digit() || chars[i] == '+' || chars[i] == '-')
                    {
                        i += 1;
                    }
                }
                let s: String = chars[start..i].iter().collect();
                if is_float {
                    tokens.push(GTok::Float(s));
                } else {
                    tokens.push(GTok::Int(s));
                }
            }
            c if c.is_alphabetic() || c == '_' => {
                let start = i;
                while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') {
                    i += 1;
                }
                let name: String = chars[start..i].iter().collect();
                tokens.push(GTok::Name(name));
            }
            '.' if chars.get(i + 1) == Some(&'.') && chars.get(i + 2) == Some(&'.') => {
                tokens.push(GTok::Punct('.'));
                i += 3;
            }
            c => {
                tokens.push(GTok::Punct(c));
                i += 1;
            }
        }
    }
    tokens
}

// ── AST types ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
struct FieldDef {
    name: String,
    field_type: String,
    #[allow(dead_code)]
    non_null: bool,
    args: Vec<ArgDef>,
    #[allow(dead_code)]
    description: Option<String>,
    deprecated: bool,
}

#[derive(Debug, Clone)]
struct ArgDef {
    name: String,
    arg_type: String,
    has_default: bool,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
enum GqlDef {
    Type {
        name: String,
        fields: Vec<FieldDef>,
        implements: Vec<String>,
        description: Option<String>,
    },
    Enum {
        name: String,
        values: Vec<String>,
        description: Option<String>,
    },
    Union {
        name: String,
        members: Vec<String>,
        description: Option<String>,
    },
    Input {
        name: String,
        fields: Vec<FieldDef>,
        description: Option<String>,
    },
    Interface {
        name: String,
        fields: Vec<FieldDef>,
        description: Option<String>,
    },
    Scalar {
        name: String,
        description: Option<String>,
    },
    Schema {
        query: Option<String>,
        mutation: Option<String>,
        subscription: Option<String>,
    },
    Directive {
        name: String,
    },
    Operation {
        kind: String,
        name: Option<String>,
        fields: Vec<String>,
    },
    Fragment {
        name: String,
        on_type: String,
        fields: Vec<String>,
    },
}

// ── Parser ────────────────────────────────────────────────────────────────────
// next() returns an owned (cloned) GTok so we never hold a reference to self
// while also needing to borrow self mutably.

struct Parser {
    tokens: Vec<GTok>,
    pos: usize,
}

impl Parser {
    fn new(tokens: Vec<GTok>) -> Self {
        Parser { tokens, pos: 0 }
    }

    fn peek_cloned(&self) -> Option<GTok> {
        self.tokens.get(self.pos).cloned()
    }

    fn next(&mut self) -> Option<GTok> {
        let t = self.tokens.get(self.pos).cloned();
        if t.is_some() {
            self.pos += 1;
        }
        t
    }

    fn expect_name(&mut self) -> Option<String> {
        match self.next() {
            Some(GTok::Name(s)) => Some(s),
            _ => None,
        }
    }

    fn maybe_string(&mut self) -> Option<String> {
        if matches!(self.peek_cloned(), Some(GTok::Str(_))) {
            match self.next() {
                Some(GTok::Str(s)) => Some(s),
                _ => None,
            }
        } else {
            None
        }
    }

    fn skip_until_close(&mut self, open: char, close: char) {
        // Called with the opening delimiter at the current position (peeked, not consumed).
        // Consumes the opening delimiter, then balances open/close until depth returns to 0.
        let mut depth = 0i32;
        while let Some(tok) = self.next() {
            match tok {
                GTok::Punct(c) if c == open => depth += 1,
                GTok::Punct(c) if c == close => {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                }
                _ => {}
            }
        }
    }

    fn parse_type_string(&mut self) -> String {
        let mut s = String::new();
        match self.peek_cloned() {
            Some(GTok::Punct('[')) => {
                self.next();
                let inner = self.parse_type_string();
                s.push('[');
                s.push_str(&inner);
                s.push(']');
                if matches!(self.peek_cloned(), Some(GTok::Punct('!'))) {
                    self.next();
                    s.push('!');
                }
                return s;
            }
            Some(GTok::Name(n)) => {
                self.next();
                s.push_str(&n);
            }
            _ => {}
        }
        if matches!(self.peek_cloned(), Some(GTok::Punct('!'))) {
            self.next();
            s.push('!');
        }
        s
    }

    fn parse_args_def(&mut self) -> Vec<ArgDef> {
        let mut args = Vec::new();
        if !matches!(self.peek_cloned(), Some(GTok::Punct('('))) {
            return args;
        }
        self.next(); // consume (
        loop {
            match self.peek_cloned() {
                None | Some(GTok::Punct(')')) => break,
                _ => {}
            }
            let _desc = self.maybe_string();
            let name = match self.peek_cloned() {
                Some(GTok::Name(n)) => {
                    self.next();
                    n
                }
                _ => break,
            };
            if matches!(self.peek_cloned(), Some(GTok::Punct(':'))) {
                self.next();
            }
            let arg_type = self.parse_type_string();
            let has_default = if matches!(self.peek_cloned(), Some(GTok::Punct('='))) {
                self.next();
                match self.peek_cloned() {
                    Some(GTok::Punct('[')) => self.skip_until_close('[', ']'),
                    Some(GTok::Punct('{')) => self.skip_until_close('{', '}'),
                    _ => {
                        self.next();
                    }
                }
                true
            } else {
                false
            };
            self.skip_directives();
            args.push(ArgDef {
                name,
                arg_type,
                has_default,
            });
        }
        if matches!(self.peek_cloned(), Some(GTok::Punct(')'))) {
            self.next();
        }
        args
    }

    fn skip_directives(&mut self) {
        while matches!(self.peek_cloned(), Some(GTok::Punct('@'))) {
            self.next();
            self.next(); // directive name
            if matches!(self.peek_cloned(), Some(GTok::Punct('('))) {
                self.skip_until_close('(', ')');
            }
        }
    }

    fn parse_fields(&mut self) -> Vec<FieldDef> {
        let mut fields = Vec::new();
        if !matches!(self.peek_cloned(), Some(GTok::Punct('{'))) {
            return fields;
        }
        self.next(); // consume {

        loop {
            match self.peek_cloned() {
                None | Some(GTok::Punct('}')) => break,
                _ => {}
            }
            let description = self.maybe_string();
            let name = match self.peek_cloned() {
                Some(GTok::Name(n)) => {
                    self.next();
                    n
                }
                _ => {
                    self.next();
                    continue;
                }
            };
            let args = self.parse_args_def();
            if matches!(self.peek_cloned(), Some(GTok::Punct(':'))) {
                self.next();
            }
            let field_type = self.parse_type_string();
            let non_null = field_type.ends_with('!');

            let mut deprecated = false;
            while matches!(self.peek_cloned(), Some(GTok::Punct('@'))) {
                self.next();
                if let Some(GTok::Name(n)) = self.next() {
                    if n == "deprecated" {
                        deprecated = true;
                    }
                }
                if matches!(self.peek_cloned(), Some(GTok::Punct('('))) {
                    self.skip_until_close('(', ')');
                }
            }

            fields.push(FieldDef {
                name,
                field_type,
                non_null,
                args,
                description,
                deprecated,
            });
        }
        if matches!(self.peek_cloned(), Some(GTok::Punct('}'))) {
            self.next();
        }
        fields
    }

    fn parse_implements(&mut self) -> Vec<String> {
        let mut impls = Vec::new();
        if !matches!(self.peek_cloned(), Some(GTok::Name(ref n)) if n == "implements") {
            return impls;
        }
        self.next(); // consume "implements"
        if matches!(self.peek_cloned(), Some(GTok::Punct('&'))) {
            self.next();
        }
        while let Some(GTok::Name(n)) = self.peek_cloned() {
            self.next();
            impls.push(n);
            if matches!(self.peek_cloned(), Some(GTok::Punct('&'))) {
                self.next();
            } else {
                break;
            }
        }
        impls
    }

    fn parse_selection_set_names(&mut self) -> Vec<String> {
        let mut names = Vec::new();
        if !matches!(self.peek_cloned(), Some(GTok::Punct('{'))) {
            return names;
        }
        self.next();
        let mut depth = 1i32;
        loop {
            match self.peek_cloned() {
                None => break,
                Some(GTok::Punct('{')) => {
                    depth += 1;
                    self.next();
                }
                Some(GTok::Punct('}')) => {
                    depth -= 1;
                    self.next();
                    if depth == 0 {
                        break;
                    }
                }
                Some(GTok::Name(n)) if depth == 1 => {
                    self.next();
                    // handle alias: name: field
                    if matches!(self.peek_cloned(), Some(GTok::Punct(':'))) {
                        self.next();
                        if let Some(GTok::Name(field)) = self.next() {
                            names.push(field);
                        }
                    } else {
                        names.push(n);
                    }
                    if matches!(self.peek_cloned(), Some(GTok::Punct('('))) {
                        self.skip_until_close('(', ')');
                    }
                    self.skip_directives();
                }
                _ => {
                    self.next();
                }
            }
        }
        names
    }

    fn parse_defs(&mut self) -> Vec<GqlDef> {
        let mut defs = Vec::new();

        while self.pos < self.tokens.len() {
            let description = self.maybe_string();

            let keyword = match self.peek_cloned() {
                Some(GTok::Name(n)) => n,
                Some(GTok::Punct('{')) => {
                    let fields = self.parse_selection_set_names();
                    defs.push(GqlDef::Operation {
                        kind: "query".to_string(),
                        name: None,
                        fields,
                    });
                    continue;
                }
                _ => {
                    self.next();
                    continue;
                }
            };

            match keyword.as_str() {
                "type" | "extend" => {
                    self.next();
                    if keyword == "extend" && matches!(self.peek_cloned(), Some(GTok::Name(_))) {
                        self.next(); // consume "type" or other keyword
                    }
                    let name = self.expect_name().unwrap_or_default();
                    let implements = self.parse_implements();
                    self.skip_directives();
                    let fields = self.parse_fields();
                    defs.push(GqlDef::Type {
                        name,
                        fields,
                        implements,
                        description,
                    });
                }
                "interface" => {
                    self.next();
                    let name = self.expect_name().unwrap_or_default();
                    self.skip_directives();
                    let fields = self.parse_fields();
                    defs.push(GqlDef::Interface {
                        name,
                        fields,
                        description,
                    });
                }
                "input" => {
                    self.next();
                    let name = self.expect_name().unwrap_or_default();
                    self.skip_directives();
                    let fields = self.parse_fields();
                    defs.push(GqlDef::Input {
                        name,
                        fields,
                        description,
                    });
                }
                "enum" => {
                    self.next();
                    let name = self.expect_name().unwrap_or_default();
                    self.skip_directives();
                    let mut values = Vec::new();
                    if matches!(self.peek_cloned(), Some(GTok::Punct('{'))) {
                        self.next();
                        loop {
                            match self.peek_cloned() {
                                None | Some(GTok::Punct('}')) => break,
                                _ => {}
                            }
                            let _d = self.maybe_string();
                            match self.peek_cloned() {
                                Some(GTok::Name(v)) => {
                                    self.next();
                                    values.push(v);
                                    self.skip_directives();
                                }
                                _ => {
                                    self.next();
                                }
                            }
                        }
                        if matches!(self.peek_cloned(), Some(GTok::Punct('}'))) {
                            self.next();
                        }
                    }
                    defs.push(GqlDef::Enum {
                        name,
                        values,
                        description,
                    });
                }
                "union" => {
                    self.next();
                    let name = self.expect_name().unwrap_or_default();
                    self.skip_directives();
                    let mut members = Vec::new();
                    if matches!(self.peek_cloned(), Some(GTok::Punct('='))) {
                        self.next();
                        if matches!(self.peek_cloned(), Some(GTok::Punct('|'))) {
                            self.next();
                        }
                        while let Some(GTok::Name(m)) = self.peek_cloned() {
                            self.next();
                            members.push(m);
                            if matches!(self.peek_cloned(), Some(GTok::Punct('|'))) {
                                self.next();
                            } else {
                                break;
                            }
                        }
                    }
                    defs.push(GqlDef::Union {
                        name,
                        members,
                        description,
                    });
                }
                "scalar" => {
                    self.next();
                    let name = self.expect_name().unwrap_or_default();
                    self.skip_directives();
                    defs.push(GqlDef::Scalar { name, description });
                }
                "schema" => {
                    self.next();
                    self.skip_directives();
                    let mut query = None;
                    let mut mutation = None;
                    let mut subscription = None;
                    if matches!(self.peek_cloned(), Some(GTok::Punct('{'))) {
                        self.next();
                        loop {
                            match self.peek_cloned() {
                                None | Some(GTok::Punct('}')) => break,
                                _ => {}
                            }
                            let op = self.expect_name().unwrap_or_default();
                            if matches!(self.peek_cloned(), Some(GTok::Punct(':'))) {
                                self.next();
                            }
                            let type_name = self.expect_name().unwrap_or_default();
                            match op.as_str() {
                                "query" => query = Some(type_name),
                                "mutation" => mutation = Some(type_name),
                                "subscription" => subscription = Some(type_name),
                                _ => {}
                            }
                        }
                        if matches!(self.peek_cloned(), Some(GTok::Punct('}'))) {
                            self.next();
                        }
                    }
                    defs.push(GqlDef::Schema {
                        query,
                        mutation,
                        subscription,
                    });
                }
                "directive" => {
                    self.next();
                    if matches!(self.peek_cloned(), Some(GTok::Punct('@'))) {
                        self.next();
                    }
                    let name = self.expect_name().unwrap_or_default();
                    // skip until next top-level keyword or end
                    loop {
                        match self.peek_cloned() {
                            None => break,
                            Some(GTok::Str(_)) => break,
                            Some(GTok::Name(ref n)) if is_top_level(n) => break,
                            _ => {
                                self.next();
                            }
                        }
                    }
                    defs.push(GqlDef::Directive { name });
                }
                "query" | "mutation" | "subscription" => {
                    self.next();
                    // optional operation name (but not a '{' or top-level keyword)
                    let name = match self.peek_cloned() {
                        Some(GTok::Name(ref n)) if !is_top_level(n) => {
                            let n = n.clone();
                            self.next();
                            Some(n)
                        }
                        _ => None,
                    };
                    if matches!(self.peek_cloned(), Some(GTok::Punct('('))) {
                        self.skip_until_close('(', ')');
                    }
                    self.skip_directives();
                    let fields = self.parse_selection_set_names();
                    defs.push(GqlDef::Operation {
                        kind: keyword,
                        name,
                        fields,
                    });
                }
                "fragment" => {
                    self.next();
                    let name = self.expect_name().unwrap_or_default();
                    // "on" keyword
                    if matches!(self.peek_cloned(), Some(GTok::Name(ref n)) if n == "on") {
                        self.next();
                    }
                    let on_type = self.expect_name().unwrap_or_default();
                    self.skip_directives();
                    let fields = self.parse_selection_set_names();
                    defs.push(GqlDef::Fragment {
                        name,
                        on_type,
                        fields,
                    });
                }
                _ => {
                    self.next();
                }
            }
        }
        defs
    }
}

fn is_top_level(name: &str) -> bool {
    matches!(
        name,
        "type"
            | "interface"
            | "input"
            | "enum"
            | "union"
            | "scalar"
            | "schema"
            | "directive"
            | "query"
            | "mutation"
            | "subscription"
            | "fragment"
            | "extend"
    )
}

fn parse(text: &str) -> Vec<GqlDef> {
    let tokens = tokenise(text);
    let mut parser = Parser::new(tokens);
    parser.parse_defs()
}

// ── Actions ───────────────────────────────────────────────────────────────────

fn action_info(text: &str) -> Result<String, String> {
    let defs = parse(text);

    let mut type_count = 0usize;
    let mut iface_count = 0usize;
    let mut input_count = 0usize;
    let mut enum_count = 0usize;
    let mut union_count = 0usize;
    let mut scalar_count = 0usize;
    let mut directive_count = 0usize;
    let mut operation_count = 0usize;
    let mut fragment_count = 0usize;
    let mut has_schema = false;
    let mut query_type = None::<String>;
    let mut mutation_type = None::<String>;
    let mut subscription_type = None::<String>;

    for def in &defs {
        match def {
            GqlDef::Type { .. } => type_count += 1,
            GqlDef::Interface { .. } => iface_count += 1,
            GqlDef::Input { .. } => input_count += 1,
            GqlDef::Enum { .. } => enum_count += 1,
            GqlDef::Union { .. } => union_count += 1,
            GqlDef::Scalar { .. } => scalar_count += 1,
            GqlDef::Directive { .. } => directive_count += 1,
            GqlDef::Operation { .. } => operation_count += 1,
            GqlDef::Fragment { .. } => fragment_count += 1,
            GqlDef::Schema {
                query,
                mutation,
                subscription,
            } => {
                has_schema = true;
                query_type = query.clone();
                mutation_type = mutation.clone();
                subscription_type = subscription.clone();
            }
        }
    }

    let is_schema = type_count > 0 || iface_count > 0 || enum_count > 0;
    let is_query_doc = operation_count > 0 || fragment_count > 0;

    let kind = match (is_schema, is_query_doc) {
        (true, false) => "Schema definition",
        (false, true) => "Query document",
        (true, true) => "Mixed (schema + operations)",
        _ if scalar_count > 0 || directive_count > 0 => "Schema extension",
        _ => "Empty or unrecognised",
    };

    let mut out = format!(
        "graphql_tools — info\n\
         Document kind:  {}\n\
         Types:          {}  Interfaces:  {}  Inputs:  {}\n\
         Enums:          {}  Unions:      {}  Scalars: {}  Directives: {}\n\
         Operations:     {}  Fragments:   {}\n",
        kind,
        type_count,
        iface_count,
        input_count,
        enum_count,
        union_count,
        scalar_count,
        directive_count,
        operation_count,
        fragment_count
    );

    if has_schema {
        out.push_str("\nSchema entry points:\n");
        if let Some(q) = &query_type {
            out.push_str(&format!("  query:        {}\n", q));
        }
        if let Some(m) = &mutation_type {
            out.push_str(&format!("  mutation:     {}\n", m));
        }
        if let Some(s) = &subscription_type {
            out.push_str(&format!("  subscription: {}\n", s));
        }
    }

    if !has_schema && is_schema {
        let root_names = ["Query", "Mutation", "Subscription"];
        let found_roots: Vec<String> = defs
            .iter()
            .filter_map(|d| match d {
                GqlDef::Type { name, .. } if root_names.contains(&name.as_str()) => {
                    Some(name.clone())
                }
                _ => None,
            })
            .collect();
        if !found_roots.is_empty() {
            out.push_str("\nConventional root types found:");
            for n in &found_roots {
                out.push_str(&format!(" {}", n));
            }
            out.push('\n');
        }
    }

    Ok(out)
}

fn action_types(text: &str, args: &Value) -> Result<String, String> {
    let defs = parse(text);
    let filter = args
        .get("filter")
        .and_then(|v| v.as_str())
        .map(|s| s.to_lowercase());

    let mut out = String::from("graphql_tools — types\n\n");
    let mut found = false;

    for def in &defs {
        let name_matches = |name: &str| {
            filter
                .as_deref()
                .is_none_or(|f| name.to_lowercase().contains(f))
        };

        match def {
            GqlDef::Type {
                name,
                fields,
                implements,
                description,
            } => {
                if !name_matches(name) {
                    continue;
                }
                found = true;
                if let Some(d) = description {
                    out.push_str(&format!("# {}\n", d));
                }
                let impl_str = if implements.is_empty() {
                    String::new()
                } else {
                    format!(" implements {}", implements.join(" & "))
                };
                out.push_str(&format!(
                    "type {}{} ({} fields)\n",
                    name,
                    impl_str,
                    fields.len()
                ));
                for f in fields {
                    let deprecated = if f.deprecated { " [DEPRECATED]" } else { "" };
                    let args_str = if f.args.is_empty() {
                        String::new()
                    } else {
                        let parts: Vec<String> = f
                            .args
                            .iter()
                            .map(|a| {
                                if a.has_default {
                                    format!("{}: {}=", a.name, a.arg_type)
                                } else {
                                    format!("{}: {}", a.name, a.arg_type)
                                }
                            })
                            .collect();
                        format!("({})", parts.join(", "))
                    };
                    out.push_str(&format!(
                        "  {}{}: {}{}\n",
                        f.name, args_str, f.field_type, deprecated
                    ));
                }
            }
            GqlDef::Interface {
                name,
                fields,
                description,
            } => {
                if !name_matches(name) {
                    continue;
                }
                found = true;
                if let Some(d) = description {
                    out.push_str(&format!("# {}\n", d));
                }
                out.push_str(&format!("interface {} ({} fields)\n", name, fields.len()));
                for f in fields {
                    out.push_str(&format!("  {}: {}\n", f.name, f.field_type));
                }
            }
            GqlDef::Input {
                name,
                fields,
                description,
            } => {
                if !name_matches(name) {
                    continue;
                }
                found = true;
                if let Some(d) = description {
                    out.push_str(&format!("# {}\n", d));
                }
                out.push_str(&format!("input {} ({} fields)\n", name, fields.len()));
                for f in fields {
                    out.push_str(&format!("  {}: {}\n", f.name, f.field_type));
                }
            }
            GqlDef::Enum {
                name,
                values,
                description,
            } => {
                if !name_matches(name) {
                    continue;
                }
                found = true;
                if let Some(d) = description {
                    out.push_str(&format!("# {}\n", d));
                }
                out.push_str(&format!(
                    "enum {} ({} values): {}\n",
                    name,
                    values.len(),
                    values.join(", ")
                ));
            }
            GqlDef::Union {
                name,
                members,
                description,
            } => {
                if !name_matches(name) {
                    continue;
                }
                found = true;
                if let Some(d) = description {
                    out.push_str(&format!("# {}\n", d));
                }
                out.push_str(&format!("union {} = {}\n", name, members.join(" | ")));
            }
            GqlDef::Scalar { name, description } => {
                if !name_matches(name) {
                    continue;
                }
                found = true;
                if let Some(d) = description {
                    out.push_str(&format!("# {}\n", d));
                }
                out.push_str(&format!("scalar {}\n", name));
            }
            _ => {}
        }
        out.push('\n');
    }

    if !found {
        out.push_str("No type definitions found");
        if filter.is_some() {
            out.push_str(" matching filter");
        }
        out.push('.');
    }

    Ok(out)
}

fn action_queries(text: &str) -> Result<String, String> {
    let defs = parse(text);
    let mut out = String::from("graphql_tools — queries\n\n");
    let mut found = false;

    for def in &defs {
        match def {
            GqlDef::Operation { kind, name, fields } => {
                found = true;
                let name_str = name.as_deref().unwrap_or("(anonymous)");
                out.push_str(&format!("{} {}\n", kind, name_str));
                if !fields.is_empty() {
                    out.push_str(&format!("  top-level fields: {}\n", fields.join(", ")));
                }
                out.push('\n');
            }
            GqlDef::Fragment {
                name,
                on_type,
                fields,
            } => {
                found = true;
                out.push_str(&format!("fragment {} on {}\n", name, on_type));
                if !fields.is_empty() {
                    out.push_str(&format!("  fields: {}\n", fields.join(", ")));
                }
                out.push('\n');
            }
            _ => {}
        }
    }

    if !found {
        out.push_str("No operations or fragments found. Pass a GraphQL query document.");
    }

    Ok(out)
}

fn action_validate(text: &str) -> Result<String, String> {
    let defs = parse(text);
    let mut warnings: Vec<String> = Vec::new();

    let defined_types: Vec<String> = defs
        .iter()
        .filter_map(|d| match d {
            GqlDef::Type { name, .. }
            | GqlDef::Interface { name, .. }
            | GqlDef::Input { name, .. }
            | GqlDef::Enum { name, .. }
            | GqlDef::Union { name, .. }
            | GqlDef::Scalar { name, .. } => Some(name.clone()),
            _ => None,
        })
        .collect();

    let builtin_scalars = ["String", "Int", "Float", "Boolean", "ID"];
    let all_known: Vec<&str> = defined_types
        .iter()
        .map(|s| s.as_str())
        .chain(builtin_scalars.iter().copied())
        .collect();

    let mut has_query_root = false;
    let mut schema_block = false;

    for def in &defs {
        match def {
            GqlDef::Schema { query, .. } => {
                schema_block = true;
                if query.is_some() {
                    has_query_root = true;
                }
            }
            GqlDef::Type { name, fields, .. } => {
                if name == "Query" || name == "Mutation" || name == "Subscription" {
                    if name == "Query" {
                        has_query_root = true;
                    }
                    if fields.is_empty() {
                        warnings.push(format!("Root type '{}' has no fields", name));
                    }
                }
                if fields.is_empty()
                    && !matches!(name.as_str(), "Query" | "Mutation" | "Subscription")
                {
                    warnings.push(format!("type '{}' has no fields", name));
                }
                for f in fields {
                    let base = base_type(&f.field_type);
                    if !base.is_empty() && !all_known.contains(&base) {
                        warnings.push(format!(
                            "type '{}' field '{}' references unknown type '{}'",
                            name, f.name, base
                        ));
                    }
                    for arg in &f.args {
                        let base_arg = base_type(&arg.arg_type);
                        if !base_arg.is_empty() && !all_known.contains(&base_arg) {
                            warnings.push(format!(
                                "type '{}' field '{}' arg '{}' references unknown type '{}'",
                                name, f.name, arg.name, base_arg
                            ));
                        }
                    }
                }
            }
            GqlDef::Interface { name, fields, .. } => {
                if fields.is_empty() {
                    warnings.push(format!("interface '{}' has no fields", name));
                }
            }
            GqlDef::Union { name, members, .. } => {
                if members.is_empty() {
                    warnings.push(format!("union '{}' has no members", name));
                }
                for m in members {
                    if !all_known.contains(&m.as_str()) {
                        warnings.push(format!(
                            "union '{}' member '{}' is not a defined type",
                            name, m
                        ));
                    }
                }
            }
            GqlDef::Enum { name, values, .. } => {
                if values.is_empty() {
                    warnings.push(format!("enum '{}' has no values", name));
                }
            }
            GqlDef::Input { name, fields, .. } => {
                if fields.is_empty() {
                    warnings.push(format!("input '{}' has no fields", name));
                }
                for f in fields {
                    let base = base_type(&f.field_type);
                    if base.chars().next().is_some_and(|c| c.is_uppercase())
                        && !builtin_scalars.contains(&base)
                    {
                        let is_output_only = defs
                            .iter()
                            .any(|d| matches!(d, GqlDef::Type { name: n, .. } if n == base));
                        if is_output_only {
                            warnings.push(format!(
                                "input '{}' field '{}' uses output type '{}' — inputs must use scalar, enum, or input types",
                                name, f.name, base
                            ));
                        }
                    }
                }
            }
            GqlDef::Operation { kind, name, fields } if fields.is_empty() => {
                let op_name = name.as_deref().unwrap_or("(anonymous)");
                warnings.push(format!("{} '{}' selects no fields", kind, op_name));
            }
            _ => {}
        }
    }

    let type_count = defined_types.len();
    if type_count > 0 && !has_query_root && !schema_block {
        warnings.push(
            "No Query root type — schema has no entry point (add 'type Query { ... }')".to_string(),
        );
    }

    // Duplicate type names
    let mut seen: Vec<String> = Vec::new();
    for name in &defined_types {
        if seen.contains(name) {
            warnings.push(format!("Duplicate type name '{}'", name));
        } else {
            seen.push(name.clone());
        }
    }

    let verdict = if warnings.is_empty() {
        "VALID"
    } else {
        "INVALID"
    };
    let mut out = format!(
        "graphql_tools — validate\n\
         Status: {} | {} type definitions | {} warnings\n",
        verdict,
        type_count,
        warnings.len()
    );

    if warnings.is_empty() {
        out.push_str("\nNo issues found.");
    } else {
        out.push('\n');
        for w in &warnings {
            out.push_str(&format!("  WARNING: {}\n", w));
        }
    }

    Ok(out)
}

fn base_type(type_str: &str) -> &str {
    type_str
        .trim_start_matches('[')
        .trim_end_matches(']')
        .trim_end_matches('!')
        .trim_start_matches('[')
        .trim_end_matches(']')
        .trim_end_matches('!')
}
