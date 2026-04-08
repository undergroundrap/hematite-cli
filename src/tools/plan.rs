use crate::tools::file_ops::workspace_root;
use serde_json::{json, Value};
use std::fs;

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct PlanHandoff {
    pub goal: String,
    #[serde(default)]
    pub target_files: Vec<String>,
    #[serde(default)]
    pub ordered_steps: Vec<String>,
    pub verification: String,
    #[serde(default)]
    pub risks: Vec<String>,
    #[serde(default)]
    pub open_questions: Vec<String>,
}

impl PlanHandoff {
    pub fn has_signal(&self) -> bool {
        !self.goal.trim().is_empty()
            || !self.target_files.is_empty()
            || !self.ordered_steps.is_empty()
            || !self.verification.trim().is_empty()
            || !self.risks.is_empty()
            || !self.open_questions.is_empty()
    }

    pub fn summary_line(&self) -> String {
        let goal = self.goal.trim();
        if goal.is_empty() {
            "Plan ready".to_string()
        } else if goal.chars().count() > 48 {
            let truncated: String = goal.chars().take(45).collect();
            format!("{truncated}...")
        } else {
            goal.to_string()
        }
    }

    pub fn to_prompt(&self) -> String {
        let mut out = String::new();
        if !self.goal.trim().is_empty() {
            out.push_str(&format!("  - Goal: {}\n", self.goal.trim()));
        }
        if !self.target_files.is_empty() {
            out.push_str(&format!(
                "  - Target Files: {}\n",
                self.target_files.join(", ")
            ));
        }
        if !self.ordered_steps.is_empty() {
            out.push_str("  - Ordered Steps:\n");
            for step in &self.ordered_steps {
                out.push_str(&format!("    - {}\n", step));
            }
        }
        if !self.verification.trim().is_empty() {
            out.push_str(&format!("  - Verification: {}\n", self.verification.trim()));
        }
        if !self.risks.is_empty() {
            out.push_str("  - Risks:\n");
            for risk in &self.risks {
                out.push_str(&format!("    - {}\n", risk));
            }
        }
        if !self.open_questions.is_empty() {
            out.push_str("  - Open Questions:\n");
            for question in &self.open_questions {
                out.push_str(&format!("    - {}\n", question));
            }
        }
        out
    }

    pub fn to_markdown(&self) -> String {
        let mut out = String::new();
        out.push_str("# Goal\n");
        out.push_str(self.goal.trim());
        out.push_str("\n\n# Target Files\n");
        if self.target_files.is_empty() {
            out.push_str("- none specified");
        } else {
            for path in &self.target_files {
                out.push_str(&format!("- {path}\n"));
            }
            if out.ends_with('\n') {
                out.pop();
            }
        }
        out.push_str("\n\n# Ordered Steps\n");
        if self.ordered_steps.is_empty() {
            out.push_str("1. clarify implementation steps");
        } else {
            for (idx, step) in self.ordered_steps.iter().enumerate() {
                out.push_str(&format!("{}. {}\n", idx + 1, step));
            }
            if out.ends_with('\n') {
                out.pop();
            }
        }
        out.push_str("\n\n# Verification\n");
        out.push_str(if self.verification.trim().is_empty() {
            "verify_build(action: \"build\")"
        } else {
            self.verification.trim()
        });
        out.push_str("\n\n# Risks\n");
        if self.risks.is_empty() {
            out.push_str("- none noted");
        } else {
            for risk in &self.risks {
                out.push_str(&format!("- {risk}\n"));
            }
            if out.ends_with('\n') {
                out.pop();
            }
        }
        out.push_str("\n\n# Open Questions\n");
        if self.open_questions.is_empty() {
            out.push_str("- none");
        } else {
            for question in &self.open_questions {
                out.push_str(&format!("- {question}\n"));
            }
            if out.ends_with('\n') {
                out.pop();
            }
        }
        out.push('\n');
        out
    }
}

fn plan_path() -> std::path::PathBuf {
    workspace_root().join(".hematite").join("PLAN.md")
}

pub fn save_plan_handoff(plan: &PlanHandoff) -> Result<(), String> {
    let path = plan_path();
    fs::create_dir_all(path.parent().unwrap()).map_err(|e| e.to_string())?;
    fs::write(&path, plan.to_markdown()).map_err(|e| format!("Failed to write plan: {e}"))
}

pub fn load_plan_handoff() -> Option<PlanHandoff> {
    let path = plan_path();
    let content = fs::read_to_string(path).ok()?;
    parse_plan_handoff(&content)
}

pub fn parse_plan_handoff(input: &str) -> Option<PlanHandoff> {
    let sections = collect_sections(input);
    let goal = sections
        .get("goal")
        .map(|s| s.trim().to_string())
        .unwrap_or_default();
    let target_files = parse_bullets(
        sections
            .get("target files")
            .map(String::as_str)
            .unwrap_or(""),
    );
    let ordered_steps = parse_ordered(
        sections
            .get("ordered steps")
            .map(String::as_str)
            .unwrap_or(""),
    );
    let verification = sections
        .get("verification")
        .map(|s| s.trim().to_string())
        .unwrap_or_default();
    let risks = parse_bullets(sections.get("risks").map(String::as_str).unwrap_or(""));
    let open_questions = parse_bullets(
        sections
            .get("open questions")
            .map(String::as_str)
            .unwrap_or(""),
    );

    let plan = PlanHandoff {
        goal,
        target_files,
        ordered_steps,
        verification,
        risks,
        open_questions,
    };
    if plan.has_signal() && !plan.goal.trim().is_empty() && !plan.ordered_steps.is_empty() {
        Some(plan)
    } else {
        None
    }
}

fn collect_sections(input: &str) -> std::collections::BTreeMap<String, String> {
    let mut sections = std::collections::BTreeMap::new();
    let mut current: Option<String> = None;
    let mut buf = String::new();

    for line in input.lines() {
        let trimmed = line.trim();
        if let Some(name) = normalize_heading(trimmed) {
            if let Some(prev) = current.replace(name) {
                sections.insert(prev, buf.trim().to_string());
                buf.clear();
            }
            continue;
        }
        if current.is_some() {
            buf.push_str(line);
            buf.push('\n');
        }
    }

    if let Some(prev) = current {
        sections.insert(prev, buf.trim().to_string());
    }

    sections
}

fn normalize_heading(line: &str) -> Option<String> {
    let heading = line
        .trim_start_matches('#')
        .trim()
        .trim_end_matches(':')
        .trim();
    match heading.to_ascii_lowercase().as_str() {
        "goal" => Some("goal".to_string()),
        "target files" => Some("target files".to_string()),
        "ordered steps" => Some("ordered steps".to_string()),
        "verification" => Some("verification".to_string()),
        "risks" => Some("risks".to_string()),
        "open questions" => Some("open questions".to_string()),
        _ => None,
    }
}

fn parse_bullets(section: &str) -> Vec<String> {
    section
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            let stripped = trimmed
                .strip_prefix("- ")
                .or_else(|| trimmed.strip_prefix("* "))
                .map(str::trim)?;
            if stripped.is_empty()
                || stripped.eq_ignore_ascii_case("none")
                || stripped.eq_ignore_ascii_case("none specified")
            {
                None
            } else {
                Some(stripped.to_string())
            }
        })
        .collect()
}

fn parse_ordered(section: &str) -> Vec<String> {
    let mut out = Vec::new();
    for line in section.lines() {
        let trimmed = line.trim();
        let Some(dot_idx) = trimmed.find(". ") else {
            continue;
        };
        if trimmed[..dot_idx].chars().all(|c| c.is_ascii_digit()) {
            let step = trimmed[dot_idx + 2..].trim();
            if !step.is_empty() {
                out.push(step.to_string());
            }
        }
    }
    out
}

/// Manages a persistent mission plan for the agent in `.hematite/PLAN.md`.
pub async fn maintain_plan(args: &Value) -> Result<String, String> {
    let blueprint = args
        .get("blueprint")
        .and_then(|v| v.as_str())
        .ok_or("maintain_plan: 'blueprint' (markdown text) required")?;
    let plan_path = plan_path();

    fs::create_dir_all(plan_path.parent().unwrap()).map_err(|e| e.to_string())?;
    fs::write(&plan_path, blueprint).map_err(|e| format!("Failed to write plan: {e}"))?;

    Ok(format!(
        "Strategic Blueprint updated in .hematite/PLAN.md ({} bytes)",
        blueprint.len()
    ))
}

/// Generates a final walkthrough report for the current session.
pub async fn generate_walkthrough(args: &Value) -> Result<String, String> {
    let summary = args
        .get("summary")
        .and_then(|v| v.as_str())
        .ok_or("generate_walkthrough: 'summary' required")?;
    let path = workspace_root().join(".hematite").join("WALKTHROUGH.md");

    fs::write(&path, summary).map_err(|e| format!("Failed to save walkthrough: {e}"))?;

    Ok(format!(
        "Walkthrough report saved to .hematite/WALKTHROUGH.md. Session complete!"
    ))
}

pub fn get_plan_params() -> Value {
    json!({
        "type": "object",
        "properties": {
            "blueprint": {
                "type": "string",
                "description": "The full markdown content of the strategic blueprint."
            }
        },
        "required": ["blueprint"]
    })
}

pub fn get_walkthrough_params() -> Value {
    json!({
        "type": "object",
        "properties": {
            "summary": {
                "type": "string",
                "description": "The full markdown summary of accomplishments."
            }
        },
        "required": ["summary"]
    })
}
