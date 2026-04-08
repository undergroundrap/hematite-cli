use lazy_static::lazy_static;
use regex::Regex;

lazy_static! {
    pub static ref ANSI_REGEX: Regex =
        Regex::new(r"[\u001b\u009b][\[()#;?]*([0-9]{1,4}(?:;[0-9]{0,4})*)?[0-9A-ORZcf-nqry=><]")
            .unwrap();
    pub static ref CRLF_REGEX: Regex = Regex::new(r"(?i)LF will be replaced by CRLF").unwrap();
}

pub fn strip_ansi(text: &str) -> String {
    let s = ANSI_REGEX.replace_all(text, "").to_string();
    CRLF_REGEX.replace_all(&s, "").to_string()
}
