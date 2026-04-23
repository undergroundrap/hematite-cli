use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

pub const PROJECT_GUIDANCE_FILES: &[&str] = &[
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

#[derive(Debug, Clone)]
pub struct InstructionFile {
    pub path: PathBuf,
    pub content: String,
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
