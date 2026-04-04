use std::path::Path;
use std::process::Command;

/// Reads a short summary of the current git status (branch + changes).
pub fn read_git_status(cwd: &Path) -> Option<String> {
    let output = Command::new("git")
        .args(["--no-optional-locks", "status", "--short", "--branch"])
        .current_dir(cwd)
        .output()
        .ok()?;
        
    if !output.status.success() {
        return None;
    }
    
    let stdout = String::from_utf8(output.stdout).ok()?;
    let trimmed = stdout.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

/// Reads the current git diff (staged + unstaged) and returns it as a formatted string.
/// Includes capping to prevent token overflow.
pub fn read_git_diff(cwd: &Path, max_chars: usize) -> Option<String> {
    let mut sections = Vec::new();

    // 1. Staged changes
    if let Some(staged) = read_git_output(cwd, &["diff", "--cached"]) {
        if !staged.trim().is_empty() {
            sections.push(format!("Staged changes:\n{}", staged.trim_end()));
        }
    }

    // 2. Unstaged changes
    if let Some(unstaged) = read_git_output(cwd, &["diff"]) {
        if !unstaged.trim().is_empty() {
            sections.push(format!("Unstaged changes:\n{}", unstaged.trim_end()));
        }
    }

    if sections.is_empty() {
        None
    } else {
        let combined = sections.join("\n\n");
        if combined.len() > max_chars {
            Some(format!("{}\n... [diff capped at {} chars]", &combined[..max_chars], max_chars))
        } else {
            Some(combined)
        }
    }
}

fn read_git_output(cwd: &Path, args: &[&str]) -> Option<String> {
    let output = Command::new("git")
        .args(args)
        .current_dir(cwd)
        .output()
        .ok()?;
        
    if !output.status.success() {
        return None;
    }
    
    String::from_utf8(output.stdout).ok()
}
