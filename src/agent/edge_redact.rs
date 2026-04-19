// Edge Redaction — privacy-preserving filter for MCP server mode.
//
// Runs after inspect_host() and before the response crosses the wire to any
// cloud agent. Strips identifiers that should never leave the machine:
//   - Usernames embedded in file paths
//   - MAC addresses
//   - Hardware serial numbers
//   - Hostnames / computer names
//   - Credential-shaped values (API keys, tokens, passwords)
//   - AWS access key IDs
//
// Each category is tracked separately so the cloud model receives a clear
// redaction receipt explaining what was sanitized and how much, without
// revealing the original values.
//
// Enable: hematite --mcp-server --edge-redact

use lazy_static::lazy_static;
use regex::Regex;
use std::collections::BTreeMap;

pub struct RedactResult {
    /// Sanitized text safe to send to a cloud model.
    pub text: String,
    /// Total number of individual substitutions made.
    pub redaction_count: usize,
    /// Human-readable summary line prepended to the text.
    pub summary_header: String,
}

struct Pattern {
    re: Regex,
    label: &'static str,
    replacement: &'static str,
}

lazy_static! {
    static ref PATTERNS: Vec<Pattern> = vec![
        // Windows username in paths: C:\Users\<name>\ or C:/Users/<name>/
        Pattern {
            re: Regex::new(r"(?i)(C:[/\\]Users[/\\])([^/\\\r\n\t ]+)([/\\])").unwrap(),
            label: "username-path",
            replacement: "${1}[USER]${3}",
        },
        // Linux/macOS home paths: /home/<name>/ or /Users/<name>/
        Pattern {
            re: Regex::new(r"(/(?:home|Users)/)([^/\r\n\t ]+)(/)").unwrap(),
            label: "username-path",
            replacement: "${1}[USER]${3}",
        },
        // MAC addresses (colon or hyphen-separated)
        Pattern {
            re: Regex::new(r"\b([0-9A-Fa-f]{2}[:\-]){5}[0-9A-Fa-f]{2}\b").unwrap(),
            label: "mac-address",
            replacement: "[MAC]",
        },
        // Hardware / disk serial numbers
        Pattern {
            re: Regex::new(r"(?i)(serial\s*(?:number)?[:=]\s*)([^\s\r\n]{4,})").unwrap(),
            label: "serial-number",
            replacement: "${1}[SERIAL]",
        },
        // Computer / hostname labels
        Pattern {
            re: Regex::new(
                r"(?i)((?:hostname|computer\s*name|machine\s*name|device\s*name|netbios\s*name)\s*[:=]\s*)([^\s\r\n]+)"
            ).unwrap(),
            label: "hostname",
            replacement: "${1}[HOSTNAME]",
        },
        // AWS access key IDs
        Pattern {
            re: Regex::new(r"\bAKIA[0-9A-Z]{16}\b").unwrap(),
            label: "aws-key",
            replacement: "[AWS-KEY]",
        },
        // Generic credential values: KEY=xxx, TOKEN=xxx, PASSWORD=xxx, etc.
        // Only fires when the label looks credential-shaped and value is ≥8 chars.
        Pattern {
            re: Regex::new(
                r"(?i)((?:api[_\-]?key|secret[_\-]?key|access[_\-]?token|auth[_\-]?token|password|passwd|pwd|private[_\-]?key|client[_\-]?secret)[^\s=:]*\s*[:=]\s*)(\S{8,})"
            ).unwrap(),
            label: "credential",
            replacement: "${1}[REDACTED]",
        },
    ];
}

/// Apply all redaction patterns to `input`.
/// Returns the sanitized text plus a receipt of what was removed.
pub fn redact(input: &str) -> RedactResult {
    let mut text = input.to_string();
    let mut counts: BTreeMap<&'static str, usize> = BTreeMap::new();

    for pattern in PATTERNS.iter() {
        let hits = pattern.re.find_iter(&text).count();
        if hits > 0 {
            *counts.entry(pattern.label).or_insert(0) += hits;
            text = pattern
                .re
                .replace_all(&text, pattern.replacement)
                .into_owned();
        }
    }

    let total: usize = counts.values().sum();

    let summary_header = if total == 0 {
        String::from("[edge-redact: no sensitive patterns detected]")
    } else {
        let detail: Vec<String> = counts
            .iter()
            .map(|(label, n)| format!("{label} \u{00d7}{n}"))
            .collect();
        format!(
            "[edge-redact: {total} substitution(s) — {} — values replaced before leaving this machine]",
            detail.join(", ")
        )
    };

    RedactResult {
        text,
        redaction_count: total,
        summary_header,
    }
}

/// Wrap a tool result with the edge-redact header so the cloud model
/// always sees a clear privacy receipt at the top of the response.
pub fn apply(raw: &str) -> String {
    let result = redact(raw);
    format!("{}\n\n{}", result.summary_header, result.text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacts_windows_username_path() {
        let input = "path: C:\\Users\\johndoe\\Documents\\project";
        let r = redact(input);
        assert!(r.text.contains("[USER]"), "should redact username");
        assert!(!r.text.contains("johndoe"), "should not contain raw username");
        assert!(r.redaction_count > 0);
    }

    #[test]
    fn redacts_mac_address() {
        let input = "MAC: 00:1A:2B:3C:4D:5E adapter connected";
        let r = redact(input);
        assert!(r.text.contains("[MAC]"), "should redact MAC");
        assert!(!r.text.contains("00:1A:2B:3C:4D:5E"), "raw MAC must not appear");
    }

    #[test]
    fn redacts_serial_number() {
        let input = "SerialNumber: WD-WX12345678";
        let r = redact(input);
        assert!(r.text.contains("[SERIAL]"), "should redact serial");
        assert!(!r.text.contains("WD-WX12345678"), "raw serial must not appear");
    }

    #[test]
    fn redacts_hostname_label() {
        let input = "ComputerName: CORP-LAPTOP-007";
        let r = redact(input);
        assert!(r.text.contains("[HOSTNAME]"), "should redact hostname");
        assert!(!r.text.contains("CORP-LAPTOP-007"), "raw hostname must not appear");
    }

    #[test]
    fn redacts_aws_key() {
        let input = "AWS_ACCESS_KEY_ID=AKIAIOSFODNN7EXAMPLE found in env";
        let r = redact(input);
        assert!(r.text.contains("[AWS-KEY]"), "should redact AWS key");
        assert!(!r.text.contains("AKIAIOSFODNN7EXAMPLE"), "raw key must not appear");
    }

    #[test]
    fn redacts_credential_value() {
        let input = "API_KEY=sk-supersecretvalue123 exported";
        let r = redact(input);
        assert!(r.text.contains("[REDACTED]"), "should redact credential value");
        assert!(!r.text.contains("sk-supersecretvalue123"), "raw secret must not appear");
    }

    #[test]
    fn clean_input_passes_through_unchanged() {
        let input = "Processes: 42 running\nCPU: 12%\nRAM: 8.1 GB / 32 GB";
        let r = redact(input);
        assert_eq!(r.redaction_count, 0);
        assert_eq!(r.text, input);
        assert!(r.summary_header.contains("no sensitive patterns"));
    }

    #[test]
    fn apply_always_prepends_header() {
        let out = apply("CPU: 15%");
        assert!(out.starts_with("[edge-redact:"), "header must be first");
    }
}
