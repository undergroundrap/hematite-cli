use crate::tools::file_ops::{hematite_dir, is_project_workspace, workspace_root};
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const EXEC_PLANS_DIR: &str = "docs/exec-plans";
const ACTIVE_EXEC_PLANS_DIR: &str = "active";
const COMPLETED_EXEC_PLANS_DIR: &str = "completed";
const ACTIVE_EXEC_PLAN_MARKER: &str = "ACTIVE_EXEC_PLAN";

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

fn plan_path() -> PathBuf {
    hematite_dir().join("PLAN.md")
}

fn plan_path_for_root(root: &Path) -> PathBuf {
    root.join(".hematite").join("PLAN.md")
}

fn task_path_for_root(root: &Path) -> PathBuf {
    root.join(".hematite").join("TASK.md")
}

fn walkthrough_path() -> PathBuf {
    hematite_dir().join("WALKTHROUGH.md")
}

fn teleport_resume_marker_path() -> PathBuf {
    hematite_dir().join("TELEPORT_RESUME")
}

fn teleport_resume_marker_path_for_root(root: &Path) -> PathBuf {
    root.join(".hematite").join("TELEPORT_RESUME")
}

fn exec_plans_root_for_root(root: &Path) -> PathBuf {
    root.join(EXEC_PLANS_DIR)
}

fn active_exec_plans_dir_for_root(root: &Path) -> PathBuf {
    exec_plans_root_for_root(root).join(ACTIVE_EXEC_PLANS_DIR)
}

fn completed_exec_plans_dir_for_root(root: &Path) -> PathBuf {
    exec_plans_root_for_root(root).join(COMPLETED_EXEC_PLANS_DIR)
}

fn active_exec_plan_marker_path_for_root(root: &Path) -> PathBuf {
    root.join(".hematite").join(ACTIVE_EXEC_PLAN_MARKER)
}

fn tech_debt_tracker_path_for_root(root: &Path) -> PathBuf {
    exec_plans_root_for_root(root).join("tech-debt-tracker.md")
}

fn exec_plans_readme_path_for_root(root: &Path) -> PathBuf {
    exec_plans_root_for_root(root).join("README.md")
}

fn active_exec_plan_path_for_root(root: &Path, slug: &str) -> PathBuf {
    active_exec_plans_dir_for_root(root).join(format!("{slug}.md"))
}

fn completed_exec_plan_path_for_root(root: &Path, slug: &str) -> PathBuf {
    completed_exec_plans_dir_for_root(root).join(format!("{slug}.md"))
}

fn should_sync_current_workspace_exec_plans() -> bool {
    is_project_workspace()
}

fn default_exec_plans_readme() -> String {
    "# Execution Plans\n\n\
Active plans in this directory are the long-lived system of record for larger multi-step work.\n\n\
- `active/` holds the current execution plan Hematite is driving.\n\
- `completed/` holds archived plans with final walkthrough notes.\n\
- `tech-debt-tracker.md` captures unfinished or follow-up cleanup discovered during execution.\n\n\
`.hematite/PLAN.md` remains the fast local handoff. Hematite mirrors meaningful plans here so a repository can carry forward intent across sessions, worktrees, and reviewers.\n"
        .to_string()
}

fn default_tech_debt_tracker() -> String {
    "# Tech Debt Tracker\n\n\
Use this file for cleanup, refactors, and follow-up work that should survive beyond a single interactive session.\n\n\
Add concrete unchecked items. Prefer specific debt with enough context for a future agent run.\n"
        .to_string()
}

fn ensure_exec_plan_layout_for_root(root: &Path) -> Result<(), String> {
    fs::create_dir_all(active_exec_plans_dir_for_root(root)).map_err(|e| e.to_string())?;
    fs::create_dir_all(completed_exec_plans_dir_for_root(root)).map_err(|e| e.to_string())?;
    fs::create_dir_all(root.join(".hematite")).map_err(|e| e.to_string())?;

    let readme_path = exec_plans_readme_path_for_root(root);
    if !readme_path.exists() {
        fs::write(&readme_path, default_exec_plans_readme())
            .map_err(|e| format!("Failed to write exec plan README: {e}"))?;
    }

    let debt_path = tech_debt_tracker_path_for_root(root);
    if !debt_path.exists() {
        fs::write(&debt_path, default_tech_debt_tracker())
            .map_err(|e| format!("Failed to write tech debt tracker: {e}"))?;
    }

    Ok(())
}

fn slugify_fragment(input: &str) -> String {
    let mut slug = String::new();

    for ch in input.chars() {
        let mapped = if ch.is_ascii_alphanumeric() {
            Some(ch.to_ascii_lowercase())
        } else if ch.is_whitespace() || matches!(ch, '-' | '_' | '/' | '\\' | ':') {
            Some('-')
        } else {
            None
        };

        match mapped {
            Some('-') if !slug.is_empty() && !slug.ends_with('-') => {
                slug.push('-');
            }
            Some('-') => {}
            Some(c) => {
                slug.push(c);
            }
            None => {}
        }
    }

    let trimmed = slug.trim_matches('-');
    if trimmed.is_empty() {
        "plan".to_string()
    } else {
        trimmed.chars().take(48).collect()
    }
}

fn fresh_plan_slug(goal: &str) -> String {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    format!("{stamp}-{}", slugify_fragment(goal))
}

fn read_active_plan_slug_for_root(root: &Path) -> Option<String> {
    let slug = fs::read_to_string(active_exec_plan_marker_path_for_root(root)).ok()?;
    let trimmed = slug.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn write_active_plan_slug_for_root(root: &Path, slug: &str) -> Result<(), String> {
    let path = active_exec_plan_marker_path_for_root(root);
    fs::create_dir_all(path.parent().unwrap()).map_err(|e| e.to_string())?;
    fs::write(path, slug).map_err(|e| format!("Failed to write active exec plan marker: {e}"))
}

fn clear_active_plan_slug_for_root(root: &Path) {
    let _ = fs::remove_file(active_exec_plan_marker_path_for_root(root));
}

fn current_or_new_active_plan_slug_for_root(root: &Path, title_hint: &str) -> String {
    read_active_plan_slug_for_root(root).unwrap_or_else(|| fresh_plan_slug(title_hint))
}

fn render_structured_execution_plan(plan: &PlanHandoff, slug: &str, status: &str) -> String {
    let mut out = String::new();
    out.push_str(&format!("# Execution Plan: {}\n\n", plan.summary_line()));
    out.push_str(&format!("- Plan ID: `{slug}`\n"));
    out.push_str(&format!("- Status: {status}\n"));
    out.push_str("- Source: `.hematite/PLAN.md`\n\n");
    out.push_str(&plan.to_markdown());
    out
}

fn render_blueprint_execution_plan(blueprint: &str, slug: &str, status: &str) -> String {
    let title = blueprint
        .lines()
        .find(|line| !line.trim().is_empty())
        .map(|line| line.trim().trim_start_matches('#').trim())
        .filter(|line| !line.is_empty())
        .unwrap_or("Strategic Blueprint");

    let mut out = String::new();
    out.push_str(&format!("# Execution Plan: {title}\n\n"));
    out.push_str(&format!("- Plan ID: `{slug}`\n"));
    out.push_str(&format!("- Status: {status}\n"));
    out.push_str("- Source: `.hematite/PLAN.md`\n\n");
    out.push_str("## Blueprint\n");
    out.push_str(blueprint.trim());
    out.push('\n');
    out
}

fn sync_structured_execution_plan_for_root(
    root: &Path,
    plan: &PlanHandoff,
) -> Result<PathBuf, String> {
    ensure_exec_plan_layout_for_root(root)?;
    let slug = current_or_new_active_plan_slug_for_root(root, &plan.summary_line());
    let path = active_exec_plan_path_for_root(root, &slug);
    fs::write(
        &path,
        render_structured_execution_plan(plan, &slug, "active"),
    )
    .map_err(|e| format!("Failed to write active execution plan: {e}"))?;
    write_active_plan_slug_for_root(root, &slug)?;
    Ok(path)
}

fn sync_blueprint_execution_plan_for_root(root: &Path, blueprint: &str) -> Result<PathBuf, String> {
    ensure_exec_plan_layout_for_root(root)?;
    let title_hint = parse_plan_handoff(blueprint)
        .map(|plan| plan.summary_line())
        .unwrap_or_else(|| {
            blueprint
                .lines()
                .find(|line| !line.trim().is_empty())
                .map(|line| line.trim().to_string())
                .unwrap_or_else(|| "strategic-blueprint".to_string())
        });
    let slug = current_or_new_active_plan_slug_for_root(root, &title_hint);
    let path = active_exec_plan_path_for_root(root, &slug);
    fs::write(
        &path,
        render_blueprint_execution_plan(blueprint, &slug, "active"),
    )
    .map_err(|e| format!("Failed to write active execution plan: {e}"))?;
    write_active_plan_slug_for_root(root, &slug)?;
    Ok(path)
}

pub fn sync_plan_blueprint_for_path(plan_file: &Path, blueprint: &str) -> Result<PathBuf, String> {
    let Some(dir) = plan_file.parent() else {
        return Err("PLAN.md path has no parent directory".to_string());
    };
    if dir.file_name().and_then(|s| s.to_str()) != Some(".hematite") {
        return Err("PLAN.md sync requires a .hematite parent directory".to_string());
    }
    let Some(root) = dir.parent() else {
        return Err("PLAN.md sync requires a project root above .hematite".to_string());
    };
    sync_blueprint_execution_plan_for_root(root, blueprint)
}

fn unchecked_task_items_for_root(root: &Path) -> Vec<String> {
    let Ok(content) = fs::read_to_string(task_path_for_root(root)) else {
        return Vec::new();
    };

    content
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            let stripped = trimmed
                .strip_prefix("- [ ] ")
                .or_else(|| trimmed.strip_prefix("* [ ] "))
                .or_else(|| trimmed.strip_prefix("+ [ ] "))?;
            if stripped.trim().is_empty() {
                None
            } else {
                Some(stripped.trim().to_string())
            }
        })
        .collect()
}

fn append_unchecked_tasks_to_tech_debt_tracker(
    root: &Path,
    slug: &str,
    unchecked_tasks: &[String],
) -> Result<(), String> {
    if unchecked_tasks.is_empty() {
        return Ok(());
    }

    ensure_exec_plan_layout_for_root(root)?;
    let debt_path = tech_debt_tracker_path_for_root(root);
    let mut content =
        fs::read_to_string(&debt_path).unwrap_or_else(|_| default_tech_debt_tracker());
    if !content.ends_with('\n') {
        content.push('\n');
    }
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    content.push_str(&format!("\n## Carry Forward from `{slug}` ({stamp})\n"));
    for task in unchecked_tasks {
        content.push_str(&format!("- [ ] {task}\n"));
    }

    fs::write(&debt_path, content).map_err(|e| format!("Failed to update tech debt tracker: {e}"))
}

fn archive_active_execution_plan_for_root(
    root: &Path,
    summary: &str,
) -> Result<Option<PathBuf>, String> {
    let Some(slug) = read_active_plan_slug_for_root(root) else {
        return Ok(None);
    };

    let active_path = active_exec_plan_path_for_root(root, &slug);
    if !active_path.exists() {
        clear_active_plan_slug_for_root(root);
        return Ok(None);
    }

    ensure_exec_plan_layout_for_root(root)?;

    let active_content = fs::read_to_string(&active_path)
        .map_err(|e| format!("Failed to read active execution plan: {e}"))?;
    let mut archived = if active_content.contains("- Status: active") {
        active_content.replacen("- Status: active", "- Status: completed", 1)
    } else {
        active_content
    };
    archived.push_str("\n## Walkthrough\n");
    archived.push_str(summary.trim());
    archived.push('\n');

    let unchecked_tasks = unchecked_task_items_for_root(root);
    if !unchecked_tasks.is_empty() {
        archived.push_str("\n## Carry Forward\n");
        for task in &unchecked_tasks {
            archived.push_str(&format!("- [ ] {task}\n"));
        }
    }

    let completed_path = completed_exec_plan_path_for_root(root, &slug);
    fs::write(&completed_path, archived)
        .map_err(|e| format!("Failed to write completed execution plan: {e}"))?;
    let _ = fs::remove_file(&active_path);
    clear_active_plan_slug_for_root(root);
    append_unchecked_tasks_to_tech_debt_tracker(root, &slug, &unchecked_tasks)?;
    Ok(Some(completed_path))
}

pub fn save_plan_handoff(plan: &PlanHandoff) -> Result<(), String> {
    let path = plan_path();
    fs::create_dir_all(path.parent().unwrap()).map_err(|e| e.to_string())?;
    fs::write(&path, plan.to_markdown()).map_err(|e| format!("Failed to write plan: {e}"))?;

    if should_sync_current_workspace_exec_plans() {
        let root = workspace_root();
        let _ = sync_structured_execution_plan_for_root(&root, plan);
    }

    Ok(())
}

pub fn save_plan_handoff_for_root(root: &Path, plan: &PlanHandoff) -> Result<(), String> {
    let path = plan_path_for_root(root);
    fs::create_dir_all(path.parent().unwrap()).map_err(|e| e.to_string())?;
    fs::write(&path, plan.to_markdown()).map_err(|e| format!("Failed to write plan: {e}"))?;
    seed_plan_support_files_for_root(root, plan)?;
    let _ = sync_structured_execution_plan_for_root(root, plan);
    Ok(())
}

pub fn load_plan_handoff() -> Option<PlanHandoff> {
    let path = plan_path();
    let content = fs::read_to_string(path).ok()?;
    let plan = parse_plan_handoff(&content)?;
    let _ = seed_plan_support_files_for_root(&workspace_root(), &plan);
    Some(plan)
}

pub fn write_teleport_resume_marker_for_root(root: &Path) -> Result<(), String> {
    let path = teleport_resume_marker_path_for_root(root);
    fs::create_dir_all(path.parent().unwrap()).map_err(|e| e.to_string())?;
    fs::write(&path, b"implement-plan").map_err(|e| format!("Failed to write marker: {e}"))
}

pub fn consume_teleport_resume_marker() -> bool {
    let path = teleport_resume_marker_path();
    if !path.exists() {
        return false;
    }
    let _ = fs::remove_file(&path);
    true
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
                Some(clean_bullet_path(stripped))
            }
        })
        .filter(|s| !s.is_empty())
        .collect()
}

fn default_task_ledger_for_plan(plan: &PlanHandoff) -> String {
    let mut content = String::from("# Task Ledger\n\n");
    if plan.ordered_steps.is_empty() {
        content.push_str("- [ ] Clarify the next implementation step\n");
    } else {
        for step in &plan.ordered_steps {
            content.push_str("- [ ] ");
            content.push_str(step.trim());
            content.push('\n');
        }
    }
    content
}

fn seed_plan_support_files_for_root(root: &Path, plan: &PlanHandoff) -> Result<(), String> {
    let task_path = task_path_for_root(root);
    if !task_path.exists()
        || fs::read_to_string(&task_path)
            .map(|content| content.trim().is_empty())
            .unwrap_or(true)
    {
        fs::write(&task_path, default_task_ledger_for_plan(plan))
            .map_err(|e| format!("Failed to seed task ledger: {e}"))?;
    }

    let walkthrough_path = root.join(".hematite").join("WALKTHROUGH.md");
    if !walkthrough_path.exists() {
        fs::write(&walkthrough_path, "")
            .map_err(|e| format!("Failed to seed walkthrough file: {e}"))?;
    }

    Ok(())
}

/// Strip markdown formatting and parenthetical annotations from a bullet path.
/// e.g. "`src/runtime.rs` (startup greeting)" -> "src/runtime.rs"
fn clean_bullet_path(raw: &str) -> String {
    let no_backticks = raw.replace('`', "");
    let clean = if let Some(idx) = no_backticks.find(" (") {
        no_backticks[..idx].trim()
    } else {
        no_backticks.trim()
    };
    clean.to_string()
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

    let mut detail = format!(
        "Strategic Blueprint updated in .hematite/PLAN.md ({} bytes)",
        blueprint.len()
    );
    if should_sync_current_workspace_exec_plans() {
        let root = workspace_root();
        if let Ok(path) = sync_blueprint_execution_plan_for_root(&root, blueprint) {
            detail.push_str(&format!("\nMirrored to {}", path.display()));
        }
    }

    Ok(detail)
}

/// Generates a final walkthrough report for the current session.
pub async fn generate_walkthrough(args: &Value) -> Result<String, String> {
    let summary = args
        .get("summary")
        .and_then(|v| v.as_str())
        .ok_or("generate_walkthrough: 'summary' required")?;
    let path = walkthrough_path();

    fs::write(&path, summary).map_err(|e| format!("Failed to save walkthrough: {e}"))?;

    let mut detail =
        "Walkthrough report saved to .hematite/WALKTHROUGH.md. Session complete!".to_string();
    if should_sync_current_workspace_exec_plans() {
        let root = workspace_root();
        if let Ok(Some(archived)) = archive_active_execution_plan_for_root(&root, summary) {
            detail.push_str(&format!(
                "\nArchived active execution plan to {}",
                archived.display()
            ));
        }
    }

    Ok(detail)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugify_fragment_cleans_goal_text() {
        assert_eq!(
            slugify_fragment("Build Website: Landing Page / Hero Polish!"),
            "build-website-landing-page-hero-polish"
        );
        assert_eq!(slugify_fragment("###"), "plan");
    }

    #[test]
    fn sync_structured_execution_plan_writes_active_doc_and_marker() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        let plan = PlanHandoff {
            goal: "Ship the marketing landing page".to_string(),
            target_files: vec!["index.html".to_string(), "style.css".to_string()],
            ordered_steps: vec!["Build the hero".to_string()],
            verification: "Open index.html".to_string(),
            risks: vec!["Avoid endless polish".to_string()],
            open_questions: vec![],
        };

        let path = sync_structured_execution_plan_for_root(root, &plan).unwrap();
        let written = fs::read_to_string(&path).unwrap();
        let slug = fs::read_to_string(active_exec_plan_marker_path_for_root(root))
            .unwrap()
            .trim()
            .to_string();

        assert!(path.starts_with(active_exec_plans_dir_for_root(root)));
        assert!(written.contains("Status: active"));
        assert!(written.contains("Ship the marketing landing page"));
        assert!(!slug.is_empty());
        assert!(exec_plans_readme_path_for_root(root).exists());
        assert!(tech_debt_tracker_path_for_root(root).exists());
    }

    #[test]
    fn archive_active_execution_plan_moves_plan_and_captures_unchecked_tasks() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        let plan = PlanHandoff {
            goal: "Refine the docs".to_string(),
            target_files: vec!["README.md".to_string()],
            ordered_steps: vec!["Update docs".to_string()],
            verification: "Read the docs".to_string(),
            risks: vec![],
            open_questions: vec![],
        };
        let active = sync_structured_execution_plan_for_root(root, &plan).unwrap();
        fs::create_dir_all(root.join(".hematite")).unwrap();
        fs::write(
            task_path_for_root(root),
            "- [x] Update docs\n- [ ] Add reliability notes\n",
        )
        .unwrap();

        let archived = archive_active_execution_plan_for_root(root, "Docs walkthrough complete.")
            .unwrap()
            .unwrap();
        let archived_content = fs::read_to_string(&archived).unwrap();
        let tracker = fs::read_to_string(tech_debt_tracker_path_for_root(root)).unwrap();

        assert!(!active.exists());
        assert!(archived.exists());
        assert!(archived_content.contains("Status: completed"));
        assert!(archived_content.contains("Docs walkthrough complete."));
        assert!(archived_content.contains("Add reliability notes"));
        assert!(tracker.contains("Add reliability notes"));
        assert!(read_active_plan_slug_for_root(root).is_none());
    }

    #[test]
    fn save_plan_handoff_for_root_seeds_task_and_walkthrough_files() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        let plan = PlanHandoff {
            goal: "Document the findings".to_string(),
            target_files: vec!["index.html".to_string()],
            ordered_steps: vec![
                "Use `research_web` first to gather context.".to_string(),
                "Write the single index.html deliverable.".to_string(),
            ],
            verification: "Open index.html".to_string(),
            risks: vec![],
            open_questions: vec![],
        };

        save_plan_handoff_for_root(root, &plan).unwrap();

        let task = std::fs::read_to_string(task_path_for_root(root)).unwrap();
        let walkthrough =
            std::fs::read_to_string(root.join(".hematite").join("WALKTHROUGH.md")).unwrap();
        let written_plan = std::fs::read_to_string(plan_path_for_root(root)).unwrap();
        let parsed = parse_plan_handoff(&written_plan).unwrap();

        assert!(task.contains("Use `research_web` first to gather context."));
        assert!(task.contains("Write the single index.html deliverable."));
        assert!(walkthrough.is_empty());
        assert_eq!(parsed.target_files, vec!["index.html".to_string()]);
    }
}
