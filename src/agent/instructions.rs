use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct InstructionFile {
    pub _path: PathBuf,
    pub content: String,
}

/// Discovers instruction files from the current directory up to the root.
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
        for candidate_name in [
            "HEMATITE.md",
            "HEMATITE.local.md",
            ".hematite/rules.md",
            ".hematite/instructions.md",
        ] {
            let candidate_path = if candidate_name.contains('/') {
                let parts: Vec<&str> = candidate_name.split('/').collect();
                dir.join(parts[0]).join(parts[1])
            } else {
                dir.join(candidate_name)
            };

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
                        _path: candidate_path,
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
    output.push("# Project Instructions".to_string());
    output.push(
        "These rules were discovered in the directory tree for the current repository:".to_string(),
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
        output.push(format!("\n## Source: HEMATITE FILE\n{}", content));
    }

    Some(output.join("\n"))
}
