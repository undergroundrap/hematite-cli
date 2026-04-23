use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::agent::config::WorkspaceTrustConfig;
use crate::agent::trust_resolver::{resolve_workspace_trust, WorkspaceTrustPolicy};

pub const PROJECT_GUIDANCE_FILES: &[&str] = &[
    "AGENTS.md",
    "agents.md",
    "CLAUDE.md",
    ".claude.md",
    "CLAUDE.local.md",
    "HEMATITE.md",
    "HEMATITE.local.md",
    ".hematite/rules.md",
    ".hematite/rules.local.md",
    "SKILLS.md",
    "SKILL.md",
    ".hematite/instructions.md",
];

pub const AGENT_SKILL_DIRS: &[&str] = &[".agents/skills", ".hematite/skills"];

#[derive(Debug, Clone)]
pub struct InstructionFile {
    pub path: PathBuf,
    pub content: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkillScope {
    User,
    Project,
}

impl SkillScope {
    pub fn label(self) -> &'static str {
        match self {
            SkillScope::User => "user",
            SkillScope::Project => "project",
        }
    }
}

#[derive(Debug, Clone)]
pub struct AgentSkill {
    pub name: String,
    pub description: String,
    pub compatibility: Option<String>,
    /// Glob patterns that auto-activate this skill based on file context.
    /// Examples: `["*.py"]`, `["*.rs"]`, `["Cargo.toml"]`
    pub triggers: Vec<String>,
    pub skill_md_path: PathBuf,
    pub scope: SkillScope,
    pub body: String,
}

#[derive(Debug, Clone)]
pub struct SkillDiscovery {
    pub skills: Vec<AgentSkill>,
    pub project_skills_loaded: bool,
    pub project_skills_note: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
struct SkillFrontmatter {
    name: Option<String>,
    description: Option<String>,
    compatibility: Option<String>,
    // Comma-separated glob patterns, e.g. "*.py, *.pyx" or "Cargo.toml"
    triggers: Option<String>,
}

pub fn resolve_guidance_path(dir: &Path, candidate_name: &str) -> PathBuf {
    candidate_name
        .split('/')
        .fold(dir.to_path_buf(), |acc, part| acc.join(part))
}

pub fn guidance_section_title(candidate_name: &str) -> &'static str {
    match candidate_name {
        "SKILLS.md" | "SKILL.md" => "PROJECT GUIDANCE",
        _ => "PROJECT RULES",
    }
}

pub fn guidance_status_label(candidate_name: &str) -> &'static str {
    match candidate_name {
        "SKILLS.md" | "SKILL.md" => "(workspace guidance)",
        _ if candidate_name.contains(".local") || candidate_name.ends_with(".local.md") => {
            "(local override)"
        }
        _ => "(shared asset)",
    }
}

pub fn discover_agent_skills(
    workspace_root: &Path,
    trust_config: &WorkspaceTrustConfig,
) -> SkillDiscovery {
    let mut discovered: Vec<AgentSkill> = Vec::new();

    if let Some(home) = dirs::home_dir() {
        let user_roots = AGENT_SKILL_DIRS
            .iter()
            .map(|relative| resolve_guidance_path(&home, relative))
            .collect::<Vec<_>>();
        load_skills_from_roots(&mut discovered, &user_roots, SkillScope::User);
    }

    let trust = resolve_workspace_trust(workspace_root, trust_config);
    let (project_skills_loaded, project_skills_note) = match trust.policy {
        WorkspaceTrustPolicy::Trusted => {
            let project_roots = AGENT_SKILL_DIRS
                .iter()
                .map(|relative| resolve_guidance_path(workspace_root, relative))
                .collect::<Vec<_>>();
            load_skills_from_roots(&mut discovered, &project_roots, SkillScope::Project);
            (true, None)
        }
        WorkspaceTrustPolicy::RequireApproval => (
            false,
            Some(format!(
                "Project skill directories were skipped because `{}` is not trust-allowlisted.",
                trust.workspace_display
            )),
        ),
        WorkspaceTrustPolicy::Denied => (
            false,
            Some(format!(
                "Project skill directories were skipped because `{}` is denied by trust policy.",
                trust.workspace_display
            )),
        ),
    };

    SkillDiscovery {
        skills: dedupe_skills(discovered),
        project_skills_loaded,
        project_skills_note,
    }
}

pub fn render_skill_catalog(discovery: &SkillDiscovery, max_chars: usize) -> Option<String> {
    if discovery.skills.is_empty() && discovery.project_skills_note.is_none() {
        return None;
    }

    let mut output = Vec::new();
    output.push("# Agent Skills Catalog".to_string());
    output.push(
        "These skills use progressive disclosure. Read a skill's SKILL.md before following it; only load scripts, references, or assets when the skill calls for them.".to_string(),
    );
    if let Some(note) = &discovery.project_skills_note {
        output.push(format!("- {}", note));
    }

    let mut remaining = max_chars;
    for skill in &discovery.skills {
        if remaining < 150 {
            output.push("\n... [further skills omitted due to context limit]".to_string());
            break;
        }
        let mut line = format!(
            "- {} [{}] — {} | SKILL.md: {}",
            skill.name,
            skill.scope.label(),
            skill.description,
            skill.skill_md_path.display()
        );
        if !skill.triggers.is_empty() {
            line.push_str(&format!(" | auto-activates: {}", skill.triggers.join(", ")));
        }
        if let Some(compatibility) = &skill.compatibility {
            line.push_str(&format!(" | compatibility: {}", compatibility));
        }
        remaining = remaining.saturating_sub(line.len());
        output.push(line);
    }

    Some(output.join("\n"))
}

/// Returns skills whose names (or hyphenated name parts) appear in `query`,
/// or whose `triggers` glob patterns match files referenced in the query or
/// the active workspace stack.
pub fn activate_matching_skills<'a>(
    discovery: &'a SkillDiscovery,
    query: &str,
) -> Vec<&'a AgentSkill> {
    let q = query.to_lowercase();
    let workspace_root = crate::tools::file_ops::workspace_root();
    let ws_exts = workspace_stack_extensions(&workspace_root);
    let query_paths = extract_query_paths(query);

    let mut matched = Vec::new();
    for skill in &discovery.skills {
        // 1. Direct name match (e.g. "use the pdf-processing skill")
        let name_lower = skill.name.to_lowercase();
        if q.contains(&name_lower) {
            matched.push(skill);
            continue;
        }

        // 2. All significant hyphen/underscore parts appear in query
        let parts: Vec<&str> = skill
            .name
            .split(['-', '_', ' '])
            .filter(|p| p.len() > 3)
            .collect();
        if parts.len() >= 2 && parts.iter().all(|p| q.contains(&p.to_lowercase())) {
            matched.push(skill);
            continue;
        }

        // 3. Trigger glob matches a file path mentioned in the query
        if !skill.triggers.is_empty() {
            let trigger_hit = skill
                .triggers
                .iter()
                .any(|pattern| query_paths.iter().any(|path| glob_matches(pattern, path)));
            if trigger_hit {
                matched.push(skill);
                continue;
            }

            // 4. Trigger glob matches the workspace stack (e.g. "*.rs" when Cargo.toml exists)
            let ws_hit = skill
                .triggers
                .iter()
                .any(|pattern| ws_exts.iter().any(|ext| glob_matches(pattern, ext)));
            if ws_hit {
                matched.push(skill);
            }
        }
    }
    matched
}

/// Simple glob matcher supporting `*.ext`, `prefix*`, and exact-name patterns.
fn glob_matches(pattern: &str, name: &str) -> bool {
    if let Some(ext_pattern) = pattern.strip_prefix("*.") {
        // *.ext — match the file extension
        name.ends_with(&format!(".{}", ext_pattern)) || name == ext_pattern
    } else if let Some(prefix) = pattern.strip_suffix('*') {
        name.starts_with(prefix)
    } else if pattern.contains('*') {
        // mid-pattern wildcard: split on first * and check prefix + suffix
        let (pre, suf) = pattern.split_once('*').unwrap();
        name.starts_with(pre) && name.ends_with(suf)
    } else {
        // exact match (e.g. "Cargo.toml")
        name == pattern
    }
}

/// Returns synthetic "file extension" strings that represent the active workspace stack,
/// derived from presence of stack marker files. Used to match trigger patterns like `*.rs`.
fn workspace_stack_extensions(root: &std::path::Path) -> Vec<String> {
    let mut exts: Vec<String> = Vec::new();
    let markers: &[(&str, &[&str])] = &[
        ("Cargo.toml", &["x.rs"]),
        ("go.mod", &["x.go"]),
        ("CMakeLists.txt", &["x.cpp", "x.c", "x.h"]),
        ("package.json", &["x.ts", "x.js", "x.tsx", "x.jsx"]),
        ("tsconfig.json", &["x.ts", "x.tsx"]),
        ("pyproject.toml", &["x.py"]),
        ("setup.py", &["x.py"]),
        ("requirements.txt", &["x.py"]),
        ("Gemfile", &["x.rb"]),
        ("pom.xml", &["x.java"]),
        ("build.gradle", &["x.java", "x.kt"]),
        ("composer.json", &["x.php"]),
    ];
    for (marker, file_exts) in markers {
        if root.join(marker).exists() {
            exts.extend(file_exts.iter().map(|s| s.to_string()));
        }
    }
    exts
}

/// Extracts token-like file paths from the query (words containing a `.` and a known extension,
/// plus `@mention` paths).
fn extract_query_paths(query: &str) -> Vec<String> {
    let known_exts = [
        "rs", "py", "ts", "js", "tsx", "jsx", "go", "cpp", "c", "h", "java", "kt", "rb", "php",
        "swift", "cs", "md", "toml", "yaml", "yml", "json", "html", "css", "scss", "sh", "pdf",
        "txt",
    ];
    let mut paths = Vec::new();
    for token in query.split_whitespace() {
        let token = token.trim_matches(|c: char| {
            !c.is_alphanumeric() && c != '.' && c != '/' && c != '_' && c != '-' && c != '@'
        });
        let effective = if token.starts_with('@') {
            &token[1..]
        } else {
            token
        };
        if let Some(ext) = effective.rsplit('.').next() {
            if known_exts.contains(&ext.to_lowercase().as_str()) {
                paths.push(effective.to_string());
            }
        }
    }
    paths
}

/// Renders the full body text of every skill that matches `query`.
/// Returns `None` when no skills are activated or all bodies are empty.
pub fn render_active_skill_bodies(
    discovery: &SkillDiscovery,
    query: &str,
    max_chars: usize,
) -> Option<String> {
    let matches = activate_matching_skills(discovery, query);
    if matches.is_empty() {
        return None;
    }
    let mut sections: Vec<String> = vec!["# Active Skill Instructions".to_string()];
    let mut remaining = max_chars;
    for skill in matches {
        if remaining < 200 {
            sections.push("... [further skill bodies omitted — context limit]".to_string());
            break;
        }
        let body = skill.body.trim();
        if body.is_empty() {
            continue;
        }
        let section = format!("## Skill: {}\n{}", skill.name, body);
        let entry = if section.len() > remaining {
            format!(
                "{}\n... [skill body truncated]",
                &section[..remaining.saturating_sub(30)]
            )
        } else {
            section
        };
        remaining = remaining.saturating_sub(entry.len());
        sections.push(entry);
    }
    if sections.len() <= 1 {
        return None;
    }
    Some(sections.join("\n\n"))
}

pub fn render_skills_report(discovery: &SkillDiscovery) -> String {
    let mut report = String::from("## Agent Skills\n\n");
    report.push_str(&format!(
        "Project skill directories: {}\n\n",
        if discovery.project_skills_loaded {
            "loaded"
        } else {
            "skipped"
        }
    ));
    if let Some(note) = &discovery.project_skills_note {
        report.push_str(note);
        report.push_str("\n\n");
    }
    if discovery.skills.is_empty() {
        report.push_str("No Agent Skills were discovered.\n\n");
        report.push_str("Scanned locations:\n");
        report.push_str("- `<project>/.agents/skills/`\n");
        report.push_str("- `<project>/.hematite/skills/`\n");
        report.push_str("- `~/.agents/skills/`\n");
        report.push_str("- `~/.hematite/skills/`\n");
        report.push_str(
            "\nAgent Skills are directory-based and require a `SKILL.md` file at the skill root.",
        );
        return report;
    }

    report.push_str("Discovered skills:\n");
    for skill in &discovery.skills {
        report.push_str(&format!(
            "- `{}` [{}] — {}\n  SKILL.md: {}\n",
            skill.name,
            skill.scope.label(),
            skill.description,
            skill.skill_md_path.display()
        ));
        if !skill.triggers.is_empty() {
            report.push_str(&format!(
                "  auto-activates: {}\n",
                skill.triggers.join(", ")
            ));
        }
        if let Some(compatibility) = &skill.compatibility {
            report.push_str(&format!("  compatibility: {}\n", compatibility));
        }
    }
    report
}

/// Discovers project guidance files from the current directory up to the root.
pub fn discover_instruction_files(cwd: &Path) -> Vec<InstructionFile> {
    let mut directories = Vec::new();
    let mut cursor = Some(cwd);
    while let Some(dir) = cursor {
        directories.push(dir.to_path_buf());
        cursor = dir.parent();
    }
    directories.reverse();

    let mut files = Vec::new();
    let mut seen_hashes = HashSet::new();

    for dir in directories {
        for candidate_name in PROJECT_GUIDANCE_FILES {
            let candidate_path = resolve_guidance_path(&dir, candidate_name);

            if let Ok(content) = fs::read_to_string(&candidate_path) {
                let trimmed = content.trim();
                if !trimmed.is_empty() {
                    // Simple hash/dedupe based on content to ignore shadowed files.
                    let hash = stable_hash(trimmed);
                    if seen_hashes.contains(&hash) {
                        continue;
                    }
                    seen_hashes.insert(hash);
                    files.push(InstructionFile {
                        path: candidate_path,
                        content: trimmed.to_string(),
                    });
                }
            }
        }
    }
    files
}

fn stable_hash(s: &str) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    s.hash(&mut hasher);
    hasher.finish()
}

/// Renders instruction files into a prompt section with a limit on characters.
pub fn render_instructions(files: &[InstructionFile], max_chars: usize) -> Option<String> {
    if files.is_empty() {
        return None;
    }

    let mut output = Vec::new();
    output.push("# Project Instructions And Skills".to_string());
    output.push(
        "These guidance files were discovered in the directory tree for the current repository:"
            .to_string(),
    );

    let mut remaining = max_chars;
    for file in files {
        if remaining < 100 {
            output.push("\n... [further instructions omitted due to context limit]".to_string());
            break;
        }

        let content = if file.content.len() > remaining {
            format!("{}\n... [truncated]", &file.content[..remaining - 20])
        } else {
            file.content.clone()
        };

        remaining = remaining.saturating_sub(content.len());
        output.push(format!("\n## Source: {}\n{}", file.path.display(), content));
    }

    Some(output.join("\n"))
}

fn load_skills_from_roots(into: &mut Vec<AgentSkill>, roots: &[PathBuf], scope: SkillScope) {
    for root in roots {
        if !root.exists() || !root.is_dir() {
            continue;
        }
        for skill_md in discover_skill_markdown_files(root) {
            if let Some(skill) = parse_agent_skill(&skill_md, scope) {
                into.push(skill);
            }
        }
    }
}

fn discover_skill_markdown_files(root: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    for entry in walkdir::WalkDir::new(root)
        .min_depth(2)
        .max_depth(4)
        .into_iter()
        .filter_map(Result::ok)
    {
        if !entry.file_type().is_file() {
            continue;
        }
        if entry.file_name() != "SKILL.md" {
            continue;
        }
        files.push(entry.into_path());
    }
    files
}

fn parse_agent_skill(skill_md_path: &Path, scope: SkillScope) -> Option<AgentSkill> {
    let content = fs::read_to_string(skill_md_path).ok()?;
    let (frontmatter, body) = split_frontmatter(&content)?;
    let parsed = parse_frontmatter(&frontmatter)?;
    let name = parsed.name?.trim().to_string();
    let description = parsed.description?.trim().to_string();
    if name.is_empty() || description.is_empty() {
        return None;
    }
    let triggers = parsed
        .triggers
        .map(|t| {
            t.split(',')
                .map(|p| p.trim().to_string())
                .filter(|p| !p.is_empty())
                .collect()
        })
        .unwrap_or_default();
    Some(AgentSkill {
        name,
        description,
        compatibility: parsed
            .compatibility
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty()),
        triggers,
        skill_md_path: skill_md_path.to_path_buf(),
        scope,
        body: body.trim().to_string(),
    })
}

fn split_frontmatter(content: &str) -> Option<(String, String)> {
    let mut lines = content.lines();
    if lines.next()?.trim() != "---" {
        return None;
    }
    let mut frontmatter = Vec::new();
    let mut body = Vec::new();
    let mut in_frontmatter = true;
    for line in lines {
        if in_frontmatter && line.trim() == "---" {
            in_frontmatter = false;
            continue;
        }
        if in_frontmatter {
            frontmatter.push(line);
        } else {
            body.push(line);
        }
    }
    if in_frontmatter {
        return None;
    }
    Some((frontmatter.join("\n"), body.join("\n")))
}

fn parse_frontmatter(frontmatter: &str) -> Option<SkillFrontmatter> {
    serde_yaml::from_str::<SkillFrontmatter>(frontmatter)
        .ok()
        .or_else(|| parse_frontmatter_fallback(frontmatter))
}

fn parse_frontmatter_fallback(frontmatter: &str) -> Option<SkillFrontmatter> {
    let mut parsed = SkillFrontmatter::default();
    for line in frontmatter.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let Some((key, value)) = trimmed.split_once(':') else {
            continue;
        };
        let value = value.trim();
        let value = strip_matching_quotes(value);
        match key.trim() {
            "name" => parsed.name = Some(value.to_string()),
            "description" => parsed.description = Some(value.to_string()),
            "compatibility" => parsed.compatibility = Some(value.to_string()),
            "triggers" => parsed.triggers = Some(value.to_string()),
            _ => {}
        }
    }
    (parsed.name.is_some() || parsed.description.is_some()).then_some(parsed)
}

fn strip_matching_quotes(value: &str) -> &str {
    if value.len() >= 2 {
        let bytes = value.as_bytes();
        let first = bytes[0] as char;
        let last = bytes[value.len() - 1] as char;
        if (first == '"' && last == '"') || (first == '\'' && last == '\'') {
            return &value[1..value.len() - 1];
        }
    }
    value
}

fn dedupe_skills(skills: Vec<AgentSkill>) -> Vec<AgentSkill> {
    let mut deduped = Vec::new();
    let mut indexes: HashMap<String, usize> = HashMap::new();
    for skill in skills {
        if let Some(index) = indexes.get(&skill.name).copied() {
            deduped[index] = skill;
        } else {
            indexes.insert(skill.name.clone(), deduped.len());
            deduped.push(skill);
        }
    }
    deduped.sort_by(|left, right| left.name.cmp(&right.name));
    deduped
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn fallback_frontmatter_handles_unquoted_colons() {
        let parsed = parse_frontmatter(
            "name: pdf-processing\ndescription: Use when: PDFs, forms, or extraction are involved\ncompatibility: Requires Python 3.11+: tested locally",
        )
        .unwrap();

        assert_eq!(parsed.name.as_deref(), Some("pdf-processing"));
        assert_eq!(
            parsed.description.as_deref(),
            Some("Use when: PDFs, forms, or extraction are involved")
        );
        assert_eq!(
            parsed.compatibility.as_deref(),
            Some("Requires Python 3.11+: tested locally")
        );
    }

    #[test]
    fn project_skill_overrides_user_skill_on_name_collision() {
        let temp = tempfile::tempdir().unwrap();
        let user_root = temp.path().join("user");
        let project_root = temp.path().join("project");

        fs::create_dir_all(user_root.join(".agents/skills/review")).unwrap();
        fs::create_dir_all(project_root.join(".agents/skills/review")).unwrap();

        fs::write(
            user_root.join(".agents/skills/review/SKILL.md"),
            "---\nname: review\ndescription: User skill.\n---\n",
        )
        .unwrap();
        fs::write(
            project_root.join(".agents/skills/review/SKILL.md"),
            "---\nname: review\ndescription: Project skill.\n---\n",
        )
        .unwrap();

        let mut discovered = Vec::new();
        load_skills_from_roots(
            &mut discovered,
            &[user_root.join(".agents/skills")],
            SkillScope::User,
        );
        load_skills_from_roots(
            &mut discovered,
            &[project_root.join(".agents/skills")],
            SkillScope::Project,
        );

        let deduped = dedupe_skills(discovered);
        assert_eq!(deduped.len(), 1);
        assert_eq!(deduped[0].description, "Project skill.");
        assert_eq!(deduped[0].scope, SkillScope::Project);
    }

    #[test]
    fn trusted_workspace_discovers_project_skill_dirs() {
        let temp = tempfile::tempdir().unwrap();
        let workspace = temp.path().join("workspace");
        let user_home = temp.path().join("home");

        fs::create_dir_all(workspace.join(".agents/skills/code-review")).unwrap();
        fs::create_dir_all(user_home.join(".agents/skills/global-review")).unwrap();
        fs::write(
            workspace.join(".agents/skills/code-review/SKILL.md"),
            "---\nname: code-review\ndescription: Review diffs.\n---\n",
        )
        .unwrap();
        fs::write(
            user_home.join(".agents/skills/global-review/SKILL.md"),
            "---\nname: global-review\ndescription: Global review skill.\n---\n",
        )
        .unwrap();

        let mut discovered = Vec::new();
        load_skills_from_roots(
            &mut discovered,
            &[user_home.join(".agents/skills")],
            SkillScope::User,
        );
        load_skills_from_roots(
            &mut discovered,
            &[workspace.join(".agents/skills")],
            SkillScope::Project,
        );
        let deduped = dedupe_skills(discovered);

        let names = deduped
            .into_iter()
            .map(|skill| skill.name)
            .collect::<Vec<_>>();
        assert_eq!(
            names,
            vec!["code-review".to_string(), "global-review".to_string()]
        );
    }

    #[test]
    fn activate_matching_skills_finds_by_name() {
        let discovery = SkillDiscovery {
            skills: vec![
                AgentSkill {
                    name: "pdf-processing".to_string(),
                    description: "Use when PDFs are involved.".to_string(),
                    compatibility: None,
                    triggers: vec![],
                    skill_md_path: PathBuf::from("/tmp/pdf-processing/SKILL.md"),
                    scope: SkillScope::User,
                    body: "Step 1: extract text.".to_string(),
                },
                AgentSkill {
                    name: "code-review".to_string(),
                    description: "Review diffs.".to_string(),
                    compatibility: None,
                    triggers: vec![],
                    skill_md_path: PathBuf::from("/tmp/code-review/SKILL.md"),
                    scope: SkillScope::Project,
                    body: "Review all changed files.".to_string(),
                },
            ],
            project_skills_loaded: true,
            project_skills_note: None,
        };

        // Direct name match
        let m = activate_matching_skills(&discovery, "please use the pdf-processing skill");
        assert_eq!(m.len(), 1);
        assert_eq!(m[0].name, "pdf-processing");

        // Part match (both "code" and "review" in query, each >3 chars)
        let m2 = activate_matching_skills(&discovery, "can you do a code review of this PR?");
        assert_eq!(m2.len(), 1);
        assert_eq!(m2[0].name, "code-review");

        // No match
        let m3 = activate_matching_skills(&discovery, "what is the weather today?");
        assert!(m3.is_empty());
    }

    #[test]
    fn activate_matching_skills_triggers_on_file_extension() {
        let discovery = SkillDiscovery {
            skills: vec![AgentSkill {
                name: "python-style".to_string(),
                description: "Python style guide.".to_string(),
                compatibility: None,
                triggers: vec!["*.py".to_string()],
                skill_md_path: PathBuf::from("/tmp/python-style/SKILL.md"),
                scope: SkillScope::User,
                body: "Use ruff for linting.".to_string(),
            }],
            project_skills_loaded: true,
            project_skills_note: None,
        };

        // File extension in query activates skill
        let m = activate_matching_skills(&discovery, "fix the type hints in src/parser.py");
        assert_eq!(m.len(), 1, "should activate via *.py trigger");

        // @mention path
        let m2 = activate_matching_skills(&discovery, "refactor @src/utils.py");
        assert_eq!(m2.len(), 1, "should activate via @mention .py path");

        // Unrelated query — no file extension match
        let m3 = activate_matching_skills(&discovery, "how does the network stack work?");
        assert!(m3.is_empty());
    }

    #[test]
    fn glob_matches_patterns() {
        assert!(glob_matches("*.rs", "main.rs"));
        assert!(glob_matches("*.rs", "src/lib.rs"));
        assert!(!glob_matches("*.rs", "main.py"));
        assert!(glob_matches("Cargo.toml", "Cargo.toml"));
        assert!(!glob_matches("Cargo.toml", "cargo.toml"));
        assert!(glob_matches("test*", "test_utils.rs"));
        assert!(!glob_matches("test*", "unit_test.rs"));
        assert!(glob_matches("*.py", "x.py")); // exact ext sentinel
    }

    #[test]
    fn triggers_parsed_from_frontmatter() {
        let temp = tempfile::tempdir().unwrap();
        fs::create_dir_all(temp.path().join("py-skill")).unwrap();
        fs::write(
            temp.path().join("py-skill/SKILL.md"),
            "---\nname: py-skill\ndescription: Python helper.\ntriggers: \"*.py, *.pyx\"\n---\n\nDo python things.\n",
        )
        .unwrap();

        let skill =
            parse_agent_skill(&temp.path().join("py-skill/SKILL.md"), SkillScope::User).unwrap();
        assert_eq!(skill.triggers, vec!["*.py", "*.pyx"]);
        assert!(skill.body.contains("Do python things."));
    }

    #[test]
    fn render_active_skill_bodies_injects_body() {
        let discovery = SkillDiscovery {
            skills: vec![AgentSkill {
                name: "pdf-processing".to_string(),
                description: "Use when PDFs are involved.".to_string(),
                compatibility: None,
                triggers: vec![],
                skill_md_path: PathBuf::from("/tmp/pdf-processing/SKILL.md"),
                scope: SkillScope::User,
                body: "## Instructions\nRun pdftotext first.".to_string(),
            }],
            project_skills_loaded: true,
            project_skills_note: None,
        };

        let rendered =
            render_active_skill_bodies(&discovery, "process this pdf-processing task", 8_000);
        assert!(rendered.is_some());
        let text = rendered.unwrap();
        assert!(text.contains("Active Skill Instructions"));
        assert!(text.contains("Skill: pdf-processing"));
        assert!(text.contains("pdftotext"));

        // No match → None
        let none = render_active_skill_bodies(&discovery, "unrelated query about network", 8_000);
        assert!(none.is_none());
    }

    #[test]
    fn skill_body_captured_from_skill_md() {
        let temp = tempfile::tempdir().unwrap();
        fs::create_dir_all(temp.path().join("my-skill")).unwrap();
        fs::write(
            temp.path().join("my-skill/SKILL.md"),
            "---\nname: my-skill\ndescription: A test skill.\n---\n\n## How to use\nDo the thing.\n",
        )
        .unwrap();

        let skill =
            parse_agent_skill(&temp.path().join("my-skill/SKILL.md"), SkillScope::User).unwrap();
        assert_eq!(skill.name, "my-skill");
        assert!(skill.body.contains("Do the thing."));
    }

    #[test]
    fn guidance_catalog_renders_skill_paths() {
        let discovery = SkillDiscovery {
            skills: vec![AgentSkill {
                name: "code-review".to_string(),
                description: "Review diffs.".to_string(),
                compatibility: Some("Requires git".to_string()),
                triggers: vec![],
                skill_md_path: PathBuf::from("/tmp/code-review/SKILL.md"),
                scope: SkillScope::Project,
                body: String::new(),
            }],
            project_skills_loaded: true,
            project_skills_note: None,
        };

        let rendered = render_skill_catalog(&discovery, 2_000).unwrap();
        assert!(rendered.contains("code-review"));
        assert!(rendered.contains("/tmp/code-review/SKILL.md"));
        assert!(rendered.contains("Requires git"));
    }
}
