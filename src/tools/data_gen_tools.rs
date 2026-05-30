use serde_json::Value;

pub async fn execute(args: &Value) -> Result<String, String> {
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("lorem");
    match action {
        "lorem" => action_lorem(args),
        "name" => action_name(args),
        "email" => action_email(args),
        "numbers" => action_numbers(args),
        "dates" => action_dates(args),
        "id" => action_id(args),
        other => Err(format!(
            "Unknown action '{other}'. Use: lorem, name, email, numbers, dates, id"
        )),
    }
}

// Minimal LCG for deterministic pseudo-random output
struct Lcg(u64);
impl Lcg {
    fn new(seed: u64) -> Self {
        Lcg(seed ^ 0xdeadbeef_cafebabe)
    }
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.0
    }
    fn range(&mut self, lo: u64, hi: u64) -> u64 {
        lo + self.next() % (hi - lo)
    }
    fn usize_range(&mut self, lo: usize, hi: usize) -> usize {
        self.range(lo as u64, hi as u64) as usize
    }
    fn pick<'a, T>(&mut self, slice: &'a [T]) -> &'a T {
        &slice[self.usize_range(0, slice.len())]
    }
}

// ── Lorem ipsum ──────────────────────────────────────────────────────────────

const LOREM_WORDS: &[&str] = &[
    "lorem",
    "ipsum",
    "dolor",
    "sit",
    "amet",
    "consectetur",
    "adipiscing",
    "elit",
    "sed",
    "do",
    "eiusmod",
    "tempor",
    "incididunt",
    "ut",
    "labore",
    "et",
    "dolore",
    "magna",
    "aliqua",
    "enim",
    "ad",
    "minim",
    "veniam",
    "quis",
    "nostrud",
    "exercitation",
    "ullamco",
    "laboris",
    "nisi",
    "aliquip",
    "ex",
    "ea",
    "commodo",
    "consequat",
    "duis",
    "aute",
    "irure",
    "in",
    "reprehenderit",
    "voluptate",
    "velit",
    "esse",
    "cillum",
    "eu",
    "fugiat",
    "nulla",
    "pariatur",
    "excepteur",
    "sint",
    "occaecat",
    "cupidatat",
    "non",
    "proident",
    "sunt",
    "culpa",
    "qui",
    "officia",
    "deserunt",
    "mollit",
    "anim",
    "id",
    "est",
    "laborum",
    "perspiciatis",
    "unde",
    "omnis",
    "iste",
    "natus",
    "error",
    "voluptatem",
    "accusantium",
    "doloremque",
    "laudantium",
    "totam",
    "rem",
    "aperiam",
    "eaque",
    "ipsa",
    "quae",
    "ab",
    "inventore",
    "veritatis",
    "quasi",
    "architecto",
    "beatae",
    "vitae",
    "dicta",
    "explicabo",
    "nemo",
    "ipsam",
    "quia",
    "voluptas",
    "aspernatur",
    "odit",
    "fugit",
    "consequuntur",
    "magni",
    "dolores",
    "eos",
    "ratione",
    "sequi",
    "nesciunt",
    "neque",
    "porro",
    "quisquam",
    "eius",
    "modi",
    "tempora",
    "incidunt",
    "magnam",
    "quaerat",
    "soluta",
    "nobis",
    "eligendi",
    "optio",
    "cumque",
    "nihil",
    "impedit",
    "quo",
    "minus",
    "maxime",
    "placeat",
    "facere",
];

fn make_sentence(rng: &mut Lcg, word_count: usize) -> String {
    let mut words: Vec<String> = (0..word_count)
        .map(|_| rng.pick(LOREM_WORDS).to_string())
        .collect();
    if let Some(first) = words.first_mut() {
        let mut chars = first.chars();
        *first = chars
            .next()
            .map(|c| c.to_uppercase().collect::<String>() + chars.as_str())
            .unwrap_or_default();
    }
    words.join(" ") + "."
}

fn action_lorem(args: &Value) -> Result<String, String> {
    let unit = args.get("unit").and_then(|v| v.as_str()).unwrap_or("words");
    let count = args.get("count").and_then(|v| v.as_u64()).unwrap_or(50) as usize;
    let seed = args.get("seed").and_then(|v| v.as_u64()).unwrap_or(42);

    if count == 0 || count > 10_000 {
        return Err("'count' must be 1–10000".to_string());
    }

    let mut rng = Lcg::new(seed);
    let mut out = String::from("data_gen_tools — lorem\n\n");

    match unit {
        "words" => {
            let words: Vec<&str> = (0..count).map(|_| *rng.pick(LOREM_WORDS)).collect();
            out.push_str(&words.join(" "));
        }
        "sentences" => {
            for i in 0..count {
                let wc = rng.usize_range(8, 18);
                out.push_str(&make_sentence(&mut rng, wc));
                if i + 1 < count {
                    out.push(' ');
                }
            }
        }
        "paragraphs" => {
            for i in 0..count {
                let sent_count = rng.usize_range(3, 7);
                for j in 0..sent_count {
                    let wc = rng.usize_range(8, 18);
                    out.push_str(&make_sentence(&mut rng, wc));
                    if j + 1 < sent_count {
                        out.push(' ');
                    }
                }
                if i + 1 < count {
                    out.push_str("\n\n");
                }
            }
        }
        other => {
            return Err(format!(
                "Unknown unit '{other}'. Use: words, sentences, paragraphs"
            ))
        }
    }

    out.push('\n');
    Ok(out)
}

// ── Names ────────────────────────────────────────────────────────────────────

const FIRST_NAMES: &[&str] = &[
    "Alice", "Bob", "Carol", "David", "Emma", "Frank", "Grace", "Henry", "Isabel", "James",
    "Karen", "Liam", "Maya", "Nathan", "Olivia", "Peter", "Quinn", "Rachel", "Sam", "Taylor",
    "Uma", "Victor", "Wendy", "Xavier", "Yuki", "Zara", "Aaron", "Bella", "Chris", "Diana",
    "Ethan", "Fiona", "George", "Hana", "Ivan", "Julia", "Kevin", "Laura", "Mike", "Nina", "Oscar",
    "Paula", "Raj", "Sofia", "Tom", "Uma", "Vera", "Will", "Xia", "Yuna",
];

const LAST_NAMES: &[&str] = &[
    "Adams",
    "Brown",
    "Clark",
    "Davis",
    "Evans",
    "Foster",
    "Green",
    "Harris",
    "Ito",
    "Jones",
    "Kim",
    "Lee",
    "Miller",
    "Nguyen",
    "Owens",
    "Patel",
    "Quinn",
    "Rivera",
    "Smith",
    "Taylor",
    "Ueda",
    "Vance",
    "Wilson",
    "Xu",
    "Young",
    "Zhang",
    "Anderson",
    "Baker",
    "Campbell",
    "Dixon",
    "Edwards",
    "Fisher",
    "Garcia",
    "Hall",
    "Ingram",
    "Jackson",
    "Khan",
    "Lambert",
    "Morgan",
    "Nelson",
    "Ortega",
    "Parker",
    "Reed",
    "Santos",
    "Thomas",
    "Underwood",
    "Vasquez",
    "Walker",
    "Xavier",
    "Yamamoto",
    "Zhou",
];

fn action_name(args: &Value) -> Result<String, String> {
    let count = args.get("count").and_then(|v| v.as_u64()).unwrap_or(5) as usize;
    let seed = args.get("seed").and_then(|v| v.as_u64()).unwrap_or(42);

    if count == 0 || count > 1000 {
        return Err("'count' must be 1–1000".to_string());
    }

    let mut rng = Lcg::new(seed);
    let mut out = String::from("data_gen_tools — name\n\n");
    for i in 0..count {
        let first = rng.pick(FIRST_NAMES);
        let last = rng.pick(LAST_NAMES);
        out.push_str(&format!("{} {}", first, last));
        if i + 1 < count {
            out.push('\n');
        }
    }
    out.push('\n');
    Ok(out)
}

// ── Email ────────────────────────────────────────────────────────────────────

const DOMAINS: &[&str] = &[
    "example.com",
    "test.org",
    "demo.net",
    "sample.io",
    "mock.dev",
    "placeholder.co",
    "fake.example",
    "testdata.net",
];

fn action_email(args: &Value) -> Result<String, String> {
    let count = args.get("count").and_then(|v| v.as_u64()).unwrap_or(5) as usize;
    let seed = args.get("seed").and_then(|v| v.as_u64()).unwrap_or(42);
    let domain = args.get("domain").and_then(|v| v.as_str()).unwrap_or("");

    if count == 0 || count > 1000 {
        return Err("'count' must be 1–1000".to_string());
    }

    let mut rng = Lcg::new(seed);
    let mut out = String::from("data_gen_tools — email\n\n");
    for i in 0..count {
        let first = rng.pick(FIRST_NAMES).to_lowercase();
        let last = rng.pick(LAST_NAMES).to_lowercase();
        let d = if domain.is_empty() {
            rng.pick(DOMAINS).to_string()
        } else {
            domain.to_string()
        };
        out.push_str(&format!("{}.{}@{}", first, last, d));
        if i + 1 < count {
            out.push('\n');
        }
    }
    out.push('\n');
    Ok(out)
}

// ── Numbers ──────────────────────────────────────────────────────────────────

fn action_numbers(args: &Value) -> Result<String, String> {
    let count = args.get("count").and_then(|v| v.as_u64()).unwrap_or(10) as usize;
    let min = args.get("min").and_then(|v| v.as_i64()).unwrap_or(1);
    let max = args.get("max").and_then(|v| v.as_i64()).unwrap_or(100);
    let seed = args.get("seed").and_then(|v| v.as_u64()).unwrap_or(42);
    let float = args.get("float").and_then(|v| v.as_bool()).unwrap_or(false);
    let decimals = args.get("decimals").and_then(|v| v.as_u64()).unwrap_or(2) as usize;
    let sep = args
        .get("separator")
        .and_then(|v| v.as_str())
        .unwrap_or("\n");

    if count == 0 || count > 10_000 {
        return Err("'count' must be 1–10000".to_string());
    }
    if min >= max {
        return Err("'min' must be less than 'max'".to_string());
    }

    let mut rng = Lcg::new(seed);
    let mut out = String::from("data_gen_tools — numbers\n\n");
    let nums: Vec<String> = (0..count)
        .map(|_| {
            if float {
                let range = (max - min) as f64;
                let v = min as f64 + (rng.next() as f64 / u64::MAX as f64) * range;
                format!("{:.prec$}", v, prec = decimals)
            } else {
                rng.range(min as u64, max as u64 + 1).to_string()
            }
        })
        .collect();
    out.push_str(&nums.join(sep));
    out.push('\n');
    Ok(out)
}

// ── Dates ────────────────────────────────────────────────────────────────────

fn is_leap(y: i64) -> bool {
    (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
}

fn days_in_month(y: i64, m: u8) -> u8 {
    match m {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if is_leap(y) {
                29
            } else {
                28
            }
        }
        _ => 30,
    }
}

// Days since year 1 day 1 (proleptic Gregorian)
fn to_epoch_day(y: i64, m: u8, d: u8) -> i64 {
    let y = y - 1;
    y * 365 + y / 4 - y / 100
        + y / 400
        + [0, 31, 59, 90, 120, 151, 181, 212, 243, 273, 304, 334][m as usize - 1] as i64
        + d as i64
}

fn from_epoch_day(n: i64) -> (i64, u8, u8) {
    let mut y = n / 365;
    loop {
        let start = to_epoch_day(y + 1, 1, 1);
        if start > n {
            break;
        }
        y += 1;
    }
    let doy = n - to_epoch_day(y, 1, 1) + 1;
    let mut m = 1u8;
    let mut rem = doy;
    loop {
        let dim = days_in_month(y, m) as i64;
        if rem <= dim {
            break;
        }
        rem -= dim;
        m += 1;
    }
    (y, m, rem as u8)
}

fn parse_date(s: &str) -> Result<i64, String> {
    let parts: Vec<&str> = s.splitn(3, '-').collect();
    if parts.len() != 3 {
        return Err(format!("Invalid date '{s}', expected YYYY-MM-DD"));
    }
    let y: i64 = parts[0].parse().map_err(|_| format!("Bad year in '{s}'"))?;
    let m: u8 = parts[1]
        .parse()
        .map_err(|_| format!("Bad month in '{s}'"))?;
    let d: u8 = parts[2].parse().map_err(|_| format!("Bad day in '{s}'"))?;
    Ok(to_epoch_day(y, m, d))
}

fn action_dates(args: &Value) -> Result<String, String> {
    let count = args.get("count").and_then(|v| v.as_u64()).unwrap_or(5) as usize;
    let from_str = args
        .get("from")
        .and_then(|v| v.as_str())
        .unwrap_or("2000-01-01");
    let to_str = args
        .get("to")
        .and_then(|v| v.as_str())
        .unwrap_or("2024-12-31");
    let seed = args.get("seed").and_then(|v| v.as_u64()).unwrap_or(42);
    let format = args.get("format").and_then(|v| v.as_str()).unwrap_or("iso");

    if count == 0 || count > 10_000 {
        return Err("'count' must be 1–10000".to_string());
    }

    let from_day = parse_date(from_str)?;
    let to_day = parse_date(to_str)?;
    if from_day >= to_day {
        return Err("'from' must be before 'to'".to_string());
    }

    let mut rng = Lcg::new(seed);
    let mut out = String::from("data_gen_tools — dates\n\n");

    let dates: Vec<String> = (0..count)
        .map(|_| {
            let day = rng.range(from_day as u64, to_day as u64) as i64;
            let (y, m, d) = from_epoch_day(day);
            match format {
                "us" => format!("{:02}/{:02}/{}", m, d, y),
                "eu" => format!("{:02}.{:02}.{}", d, m, y),
                "long" => {
                    let month_name = [
                        "January",
                        "February",
                        "March",
                        "April",
                        "May",
                        "June",
                        "July",
                        "August",
                        "September",
                        "October",
                        "November",
                        "December",
                    ][m as usize - 1];
                    format!("{} {}, {}", month_name, d, y)
                }
                _ => format!("{:04}-{:02}-{:02}", y, m, d),
            }
        })
        .collect();

    out.push_str(&dates.join("\n"));
    out.push('\n');
    Ok(out)
}

// ── Sequential IDs ────────────────────────────────────────────────────────────

fn action_id(args: &Value) -> Result<String, String> {
    let count = args.get("count").and_then(|v| v.as_u64()).unwrap_or(5) as usize;
    let prefix = args.get("prefix").and_then(|v| v.as_str()).unwrap_or("");
    let start = args.get("start").and_then(|v| v.as_u64()).unwrap_or(1);
    let pad = args.get("pad").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
    let kind = args.get("kind").and_then(|v| v.as_str()).unwrap_or("seq");
    let seed = args.get("seed").and_then(|v| v.as_u64()).unwrap_or(42);

    if count == 0 || count > 10_000 {
        return Err("'count' must be 1–10000".to_string());
    }

    let mut out = String::from("data_gen_tools — id\n\n");

    let ids: Vec<String> = match kind {
        "seq" => (0..count)
            .map(|i| {
                let n = start + i as u64;
                if pad > 0 {
                    format!("{}{:0>width$}", prefix, n, width = pad)
                } else {
                    format!("{}{}", prefix, n)
                }
            })
            .collect(),
        "hex" => {
            let mut rng = Lcg::new(seed);
            (0..count)
                .map(|_| {
                    let hi = rng.next();
                    let lo = rng.next();
                    format!("{}{:016x}{:016x}", prefix, hi, lo)
                        .chars()
                        .take(prefix.len() + 32)
                        .collect()
                })
                .collect()
        }
        "uuid" => {
            let mut rng = Lcg::new(seed);
            (0..count)
                .map(|_| {
                    let a = rng.next();
                    let b = rng.next();
                    // UUID v4 format: xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx
                    let b0 = (a >> 32) as u32;
                    let b1 = ((a >> 16) & 0xffff) as u16;
                    let b2 = (0x4000 | (a & 0x0fff)) as u16;
                    let b3 = (0x8000 | ((b >> 48) & 0x3fff)) as u16;
                    let b4 = b & 0xffffffffffff;
                    format!(
                        "{}{:08x}-{:04x}-{:04x}-{:04x}-{:012x}",
                        prefix, b0, b1, b2, b3, b4
                    )
                })
                .collect()
        }
        other => return Err(format!("Unknown kind '{other}'. Use: seq, hex, uuid")),
    };

    out.push_str(&ids.join("\n"));
    out.push('\n');
    Ok(out)
}
