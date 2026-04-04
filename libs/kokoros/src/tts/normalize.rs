use lazy_static::lazy_static;
use regex::Regex;

lazy_static! {
    static ref WHITESPACE_RE: Regex = Regex::new(r"[^\S \n]").unwrap();
    static ref MULTI_SPACE_RE: Regex = Regex::new(r"  +").unwrap();
    // Replacing look-arounds with capture groups for Rust Regex crate compatibility
    static ref NEWLINE_SPACE_RE: Regex = Regex::new(r"(\n) +(\n)").unwrap();
    static ref DOCTOR_RE: Regex = Regex::new(r"\bD[Rr]\. ([A-Z])").unwrap();
    static ref MISTER_RE: Regex = Regex::new(r"\b(Mr\.|MR\.) ([A-Z])").unwrap();
    static ref MISS_RE: Regex = Regex::new(r"\b(Ms\.|MS\.) ([A-Z])").unwrap();
    static ref MRS_RE: Regex = Regex::new(r"\b(Mrs\.|MRS\.) ([A-Z])").unwrap();
    static ref ETC_RE: Regex = Regex::new(r"\betc\.([^A-Z])").unwrap();
    static ref YEAH_RE: Regex = Regex::new(r"(?i)\b(y)eah?\b").unwrap();
    static ref COMMA_NUM_RE: Regex = Regex::new(r"(\d),(\d)").unwrap();
    static ref RANGE_RE: Regex = Regex::new(r"(\d)-(\d)").unwrap();
    static ref S_AFTER_NUM_RE: Regex = Regex::new(r"(\d)S").unwrap();
    static ref POSSESSIVE_RE: Regex = Regex::new(r"([BCDFGHJ-NP-TV-Z])'?s\b").unwrap();
    static ref X_POSSESSIVE_RE: Regex = Regex::new(r"(X')S\b").unwrap();
    static ref INITIALS_RE: Regex = Regex::new(r"((?:[A-Za-z]\.){2,}) ([a-z])").unwrap();
    static ref ACRONYM_RE: Regex = Regex::new(r"([A-Z])\.([A-Z])").unwrap();
}

pub fn normalize_text(text: &str) -> String {
    let mut text = text.to_string();

    // Replace special quotes and brackets
    text = text.replace('\u{2018}', "'").replace('\u{2019}', "'");
    text = text.replace('«', "\u{201C}").replace('»', "\u{201D}");
    text = text.replace('\u{201C}', "\"").replace('\u{201D}', "\"");
    text = text.replace('(', "«").replace(')', "»");

    // Replace Chinese/Japanese punctuation
    let from_chars = ['、', '。', '！', '，', '：', '；', '？'];
    let to_chars = [',', '.', '!', ',', ':', ';', '?'];

    for (from, to) in from_chars.iter().zip(to_chars.iter()) {
        text = text.replace(*from, &format!("{} ", to));
    }

    // Apply regex replacements using standard groups (Regex crate doesn't support look-around)
    text = WHITESPACE_RE.replace_all(&text, " ").to_string();
    text = MULTI_SPACE_RE.replace_all(&text, " ").to_string();
    text = NEWLINE_SPACE_RE.replace_all(&text, "$1$2").to_string();
    text = DOCTOR_RE.replace_all(&text, "Doctor $1").to_string();
    text = MISTER_RE.replace_all(&text, "Mister $2").to_string();
    text = MISS_RE.replace_all(&text, "Miss $2").to_string();
    text = MRS_RE.replace_all(&text, "Mrs $2").to_string();
    text = ETC_RE.replace_all(&text, "etc$1").to_string();
    text = YEAH_RE.replace_all(&text, "${1}e'a").to_string();
    text = COMMA_NUM_RE.replace_all(&text, "$1$2").to_string();
    text = RANGE_RE.replace_all(&text, "$1 to $2").to_string();
    text = S_AFTER_NUM_RE.replace_all(&text, "$1 S").to_string();
    text = POSSESSIVE_RE.replace_all(&text, "$1'S").to_string();
    text = X_POSSESSIVE_RE.replace_all(&text, "$1s").to_string();

    // Handle initials and acronyms
    text = INITIALS_RE
        .replace_all(&text, |caps: &regex::Captures| {
            format!("{} {}", caps[1].replace('.', "-"), &caps[2])
        })
        .to_string();
    text = ACRONYM_RE.replace_all(&text, "$1-$2").to_string();

    text.trim().to_string()
}
