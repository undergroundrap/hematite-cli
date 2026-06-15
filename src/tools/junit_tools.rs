use quick_xml::events::Event;
use quick_xml::Reader;
use serde_json::{json, Value};
use std::collections::HashMap;

pub fn make_schema() -> Value {
    json!({
        "name": "junit_tools",
        "description": "Parse and analyze JUnit/xUnit XML test result files without external utilities.",
        "parameters": {
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["parse", "failures", "summary", "list"],
                    "description": "parse=suite overview with failing tests highlighted, failures=only failing/erroring tests, summary=aggregate stats with pass-rate bar, list=all tests with status (filter with 'status')"
                },
                "xml": { "type": "string", "description": "JUnit XML content inline" },
                "file": { "type": "string", "description": "Path to .xml test result file" },
                "status": { "type": "string", "description": "For 'list': filter by passed/failed/error/skipped" }
            }
        }
    })
}

struct TestSuite {
    name: String,
    tests: u32,
    failures: u32,
    errors: u32,
    skipped: u32,
    time: f64,
    cases: Vec<TestCase>,
}

struct TestCase {
    name: String,
    classname: String,
    time: f64,
    status: String,
    message: Option<String>,
    body: Option<String>,
}

fn attrs_of(e: &quick_xml::events::BytesStart) -> HashMap<String, String> {
    e.attributes()
        .filter_map(|a| {
            let a = a.ok()?;
            let key = String::from_utf8_lossy(a.key.as_ref()).to_string();
            let val = a.unescape_value().ok()?.to_string();
            Some((key, val))
        })
        .collect()
}

fn parse_xml(xml: &str) -> Result<Vec<TestSuite>, String> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);

    let mut suites: Vec<TestSuite> = Vec::new();
    let mut current_suite: Option<TestSuite> = None;
    let mut current_case: Option<TestCase> = None;
    let mut capture_text = false;
    let mut text_buf = String::new();
    let mut pending_status = String::new();
    let mut pending_msg: Option<String> = None;

    loop {
        match reader.read_event() {
            Ok(Event::Start(ref e)) | Ok(Event::Empty(ref e)) => {
                let tag = String::from_utf8_lossy(e.local_name().as_ref()).to_lowercase();
                let attrs = attrs_of(e);
                match tag.as_str() {
                    "testsuite" => {
                        let suite = TestSuite {
                            name: attrs.get("name").cloned().unwrap_or_default(),
                            tests: attrs.get("tests").and_then(|v| v.parse().ok()).unwrap_or(0),
                            failures: attrs
                                .get("failures")
                                .and_then(|v| v.parse().ok())
                                .unwrap_or(0),
                            errors: attrs
                                .get("errors")
                                .and_then(|v| v.parse().ok())
                                .unwrap_or(0),
                            skipped: attrs
                                .get("skipped")
                                .and_then(|v| v.parse().ok())
                                .unwrap_or(0),
                            time: attrs
                                .get("time")
                                .and_then(|v| v.parse().ok())
                                .unwrap_or(0.0),
                            cases: Vec::new(),
                        };
                        current_suite = Some(suite);
                    }
                    "testcase" => {
                        current_case = Some(TestCase {
                            name: attrs.get("name").cloned().unwrap_or_default(),
                            classname: attrs.get("classname").cloned().unwrap_or_default(),
                            time: attrs
                                .get("time")
                                .and_then(|v| v.parse().ok())
                                .unwrap_or(0.0),
                            status: "passed".to_string(),
                            message: None,
                            body: None,
                        });
                    }
                    "failure" | "error" => {
                        pending_status =
                            if tag == "failure" { "failed" } else { "error" }.to_string();
                        pending_msg = attrs.get("message").cloned();
                        capture_text = true;
                        text_buf.clear();
                    }
                    "skipped" => {
                        if let Some(ref mut tc) = current_case {
                            tc.status = "skipped".to_string();
                            tc.message = attrs.get("message").cloned();
                        }
                    }
                    _ => {}
                }
            }
            Ok(Event::Text(ref e)) => {
                if capture_text {
                    if let Ok(s) = e.unescape() {
                        text_buf.push_str(&s);
                    }
                }
            }
            Ok(Event::CData(ref e)) => {
                if capture_text {
                    text_buf.push_str(std::str::from_utf8(e).unwrap_or(""));
                }
            }
            Ok(Event::End(ref e)) => {
                let tag = String::from_utf8_lossy(e.local_name().as_ref()).to_lowercase();
                match tag.as_str() {
                    "failure" | "error" => {
                        if let Some(ref mut tc) = current_case {
                            tc.status = pending_status.clone();
                            if tc.message.is_none() {
                                tc.message = pending_msg.take();
                            } else {
                                pending_msg = None;
                            }
                            let body = text_buf.trim().to_string();
                            if !body.is_empty() {
                                tc.body = Some(body);
                            }
                        }
                        capture_text = false;
                        text_buf.clear();
                    }
                    "testcase" => {
                        if let Some(tc) = current_case.take() {
                            if let Some(ref mut suite) = current_suite {
                                suite.cases.push(tc);
                            }
                        }
                    }
                    "testsuite" => {
                        if let Some(suite) = current_suite.take() {
                            suites.push(suite);
                        }
                    }
                    _ => {}
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => {
                return Err(format!(
                    "XML parse error at position {}: {}",
                    reader.buffer_position(),
                    e
                ))
            }
            _ => {}
        }
    }

    Ok(suites)
}

pub async fn execute(args: &Value) -> Result<String, String> {
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("parse");

    let xml = if let Some(f) = args.get("file").and_then(|v| v.as_str()) {
        std::fs::read_to_string(f).map_err(|e| format!("Cannot read {}: {}", f, e))?
    } else if let Some(x) = args.get("xml").and_then(|v| v.as_str()) {
        x.to_string()
    } else {
        return Err(
            "Provide 'xml' (inline content) or 'file' (path to .xml test result).".to_string(),
        );
    };

    let suites = parse_xml(&xml)?;
    if suites.is_empty() {
        return Err("No test suites found. Is this JUnit/xUnit XML?".to_string());
    }

    match action {
        "parse" => action_parse(&suites),
        "failures" => action_failures(&suites),
        "summary" => action_summary(&suites),
        "list" => {
            let filter = args.get("status").and_then(|v| v.as_str());
            action_list(&suites, filter)
        }
        _ => Err(format!(
            "Unknown action '{}'. Use: parse, failures, summary, list",
            action
        )),
    }
}

fn totals(suites: &[TestSuite]) -> (u32, u32, u32, u32, u32, f64) {
    let tests: u32 = suites.iter().map(|s| s.tests).sum();
    let failures: u32 = suites.iter().map(|s| s.failures).sum();
    let errors: u32 = suites.iter().map(|s| s.errors).sum();
    let skipped: u32 = suites.iter().map(|s| s.skipped).sum();
    let passed = tests.saturating_sub(failures + errors + skipped);
    let time: f64 = suites.iter().map(|s| s.time).sum();
    (tests, passed, failures, errors, skipped, time)
}

fn action_parse(suites: &[TestSuite]) -> Result<String, String> {
    let (tests, passed, failures, errors, skipped, time) = totals(suites);
    let status = if failures + errors == 0 {
        "PASSED"
    } else {
        "FAILED"
    };
    let mut out = String::new();
    out.push_str(&format!(
        "## Test Results — {}\n\n{} suite(s) | {} tests | {} passed | {} failed | {} errors | {} skipped | {:.3}s\n\n",
        status, suites.len(), tests, passed, failures, errors, skipped, time
    ));
    for suite in suites {
        let s_passed = suite
            .tests
            .saturating_sub(suite.failures + suite.errors + suite.skipped);
        let icon = if suite.failures + suite.errors == 0 {
            "✓"
        } else {
            "✗"
        };
        out.push_str(&format!(
            "### {} {}\n{} tests, {} passed, {} failed, {} errors, {} skipped — {:.3}s\n",
            icon,
            suite.name,
            suite.tests,
            s_passed,
            suite.failures,
            suite.errors,
            suite.skipped,
            suite.time
        ));
        let bad: Vec<_> = suite
            .cases
            .iter()
            .filter(|c| c.status == "failed" || c.status == "error")
            .collect();
        if !bad.is_empty() {
            out.push('\n');
            for tc in &bad {
                out.push_str(&format!("  ✗ {} ({})\n", tc.name, tc.classname));
                if let Some(ref msg) = tc.message {
                    let first = msg.lines().next().unwrap_or("").trim();
                    let snippet: String = first.chars().take(120).collect();
                    if !snippet.is_empty() {
                        out.push_str(&format!("    {}\n", snippet));
                    }
                }
            }
        }
        out.push('\n');
    }
    Ok(out)
}

fn action_failures(suites: &[TestSuite]) -> Result<String, String> {
    let mut out = String::new();
    out.push_str("## Failing Tests\n\n");
    let mut count = 0;
    for suite in suites {
        for tc in suite
            .cases
            .iter()
            .filter(|c| c.status == "failed" || c.status == "error")
        {
            count += 1;
            out.push_str(&format!(
                "### {} / {} — {}\n**Status:** {}  **Time:** {:.3}s\n",
                suite.name,
                tc.classname,
                tc.name,
                tc.status.to_uppercase(),
                tc.time
            ));
            if let Some(ref msg) = tc.message {
                out.push_str(&format!(
                    "**Message:** {}\n",
                    msg.lines().next().unwrap_or("").trim()
                ));
            }
            if let Some(ref body) = tc.body {
                let lines: Vec<_> = body.lines().take(12).collect();
                out.push_str(&format!("```\n{}\n```\n", lines.join("\n")));
            }
            out.push('\n');
        }
    }
    if count == 0 {
        out.push_str("No failures or errors. All tests passed!\n");
    } else {
        out.push_str(&format!("**Total: {} failing test(s)**\n", count));
    }
    Ok(out)
}

fn action_summary(suites: &[TestSuite]) -> Result<String, String> {
    let (tests, passed, failures, errors, skipped, time) = totals(suites);
    let pass_pct = (passed * 100).checked_div(tests).unwrap_or(0);
    let status = if failures + errors == 0 {
        "PASSED ✓"
    } else {
        "FAILED ✗"
    };
    let filled = (pass_pct as usize * 40 / 100).min(40);
    let bar = format!("{}{}", "█".repeat(filled), "░".repeat(40 - filled));

    let mut out = String::new();
    out.push_str("## Test Summary\n\n");
    out.push_str(&format!("Status:        {}\n", status));
    out.push_str(&format!("Test Suites:   {}\n", suites.len()));
    out.push_str(&format!("Total Tests:   {}\n", tests));
    out.push_str(&format!("Passed:        {} ({}%)\n", passed, pass_pct));
    out.push_str(&format!("Failed:        {}\n", failures));
    out.push_str(&format!("Errors:        {}\n", errors));
    out.push_str(&format!("Skipped:       {}\n", skipped));
    out.push_str(&format!("Total Time:    {:.3}s\n\n", time));
    out.push_str(&format!("[{}] {}%\n", bar, pass_pct));
    Ok(out)
}

fn action_list(suites: &[TestSuite], filter: Option<&str>) -> Result<String, String> {
    let f = filter.unwrap_or("all");
    let mut out = String::new();
    out.push_str(&format!("## Test Cases ({})\n\n", f));
    let icon = |s: &str| match s {
        "passed" => "✓",
        "failed" => "✗",
        "error" => "!",
        "skipped" => "~",
        _ => "?",
    };
    let mut count = 0;
    for suite in suites {
        for tc in &suite.cases {
            if f != "all" && tc.status != f {
                continue;
            }
            count += 1;
            out.push_str(&format!(
                "{} [{:.3}s] {}::{}\n",
                icon(&tc.status),
                tc.time,
                tc.classname,
                tc.name
            ));
            if tc.status != "passed" {
                if let Some(ref msg) = tc.message {
                    let snippet: String = msg
                        .lines()
                        .next()
                        .unwrap_or("")
                        .trim()
                        .chars()
                        .take(100)
                        .collect();
                    if !snippet.is_empty() {
                        out.push_str(&format!("   {}\n", snippet));
                    }
                }
            }
        }
    }
    out.push_str(&format!("\n{} test(s)\n", count));
    Ok(out)
}
