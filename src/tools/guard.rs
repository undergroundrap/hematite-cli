use super::tool::RiskLevel;
use std::path::{Path, PathBuf};

#[allow(dead_code)]
pub const PROTECTED_FILES: &[&str] = &[
    // Windows System
    "C:\\Windows",
    "C:\\Program Files",
    "C:\\$Recycle.Bin",
    "System Volume Information",
    "C:\\Users\\Default",
    // Linux/Unix System
    "/etc",
    "/dev",
    "/proc",
    "/sys",
    "/root",
    "/var/log",
    "/boot",
    // User Sensitives
    ".bashrc",
    ".zshrc",
    ".bash_history",
    ".gitconfig",
    ".ssh/",
    ".aws/",
    ".env",
    "credentials.json",
    "auth.json",
    "id_rsa",
    // Hematite Internal
    ".mcp.json",
    "hematite_memory.db",
];

/// Enforces the absolute Canonical Traversal lock on the LLM, rendering directory climbing (`../`) obsolete
/// and blocking any OS-critical reads aggressively by cross-referencing global blacklists.
#[allow(dead_code)]
pub fn path_is_safe(workspace_root: &Path, target: &Path) -> Result<PathBuf, String> {
    // 1) Evaluate target string explicitly normalizing unicode and backslash injection vectors!
    let mut target_str = target.to_string_lossy().to_string().to_lowercase();
    target_str = target_str
        .replace("\\", "/")
        .replace("\u{005c}", "/")
        .replace("%5c", "/");

    // Early evaluation covering read-only "Ghosting" on target secrets explicitly
    for protected in PROTECTED_FILES {
        let prot_lower = protected.to_lowercase().replace("\\", "/");
        if target_str.contains(&prot_lower) {
            return Err(format!(
                "AccessDenied: Path {} hits the Hematite Security Blacklist natively: {}",
                target_str, protected
            ));
        }
    }

    // 2) Native Canonicalization - Forcing OS Reality Context over LLM hallucinations
    let resolved_path = match std::fs::canonicalize(target) {
        Ok(p) => p,
        Err(_) => {
            // If creating a brand new isolated file, physically trace the parent node
            let parent = target.parent().unwrap_or(Path::new(""));
            let mut resolved_parent = std::fs::canonicalize(parent)
                .map_err(|_| "AccessDenied: Invalid directory ancestry inside sandbox root. Path traversing halted!".to_string())?;
            if let Some(name) = target.file_name() {
                resolved_parent.push(name);
            }
            resolved_parent
        }
    };

    // Hard check against hallucinated drive letters that resolved cleanly across symlinks natively
    let resolved_str = resolved_path
        .to_string_lossy()
        .to_string()
        .to_lowercase()
        .replace("\\", "/");
    for protected in PROTECTED_FILES {
        let prot_lower = protected.to_lowercase().replace("\\", "/");
        if resolved_str.contains(&prot_lower) {
            return Err(format!(
                "AccessDenied: Canonicalized Sandbox resolution natively hits Blacklist bounds: {}",
                protected
            ));
        }
    }

    let resolved_workspace = std::fs::canonicalize(workspace_root).unwrap_or_default();

    // 3) Assess Physical Traversal Limits strictly against the Root Environment Prefix
    if !resolved_path.starts_with(&resolved_workspace) {
        // RELAXED SANDBOX: Allow absolute paths IF they passed the blacklist checks above.
        if target.is_absolute() {
            return Ok(resolved_path);
        }
        return Err(format!("AccessDenied: ⛔ SANDBOX BREACHED ⛔ Attempted directory traversal outside project bounds: {:?}", resolved_path));
    }

    Ok(resolved_path)
}

/// Hard-blocks Bash payloads unconditionally if they attempt to reference OS-critical locations
#[allow(dead_code)]
pub fn bash_is_safe(cmd: &str) -> Result<(), String> {
    let lower = cmd
        .to_lowercase()
        .replace("\\", "/")
        .replace("\u{005c}", "/")
        .replace("%5c", "/");
    for protected in PROTECTED_FILES {
        let prot_lower = protected.to_lowercase().replace("\\", "/");
        if lower.contains(&prot_lower) {
            return Err(format!("AccessDenied: Bash command structurally attempts to manipulate blacklisted system area: {}", protected));
        }
    }

    // Block using shell as a substitute for run_code.
    // The model should use run_code directly — shell is the wrong tool for this.
    let sandbox_redirects = [
        "deno run",
        "deno --version",
        "deno -v",
        "python -c ",
        "python3 -c ",
        "node -e ",
        "node --eval",
    ];
    for pattern in sandbox_redirects {
        if lower.contains(pattern) {
            return Err(format!(
                "Use the run_code tool instead of shell for executing {} code. \
                 Shell is blocked for sandbox-style execution.",
                pattern.split_whitespace().next().unwrap_or("code")
            ));
        }
    }

    Ok(())
}

/// Three-tier risk classifier for shell commands.
///
/// Safe   → auto-approved (read-only, build, test, local git reads)
/// High   → always requires user approval (destructive, network, privilege)
/// Moderate → ask by default; can be configured to auto-approve
pub fn classify_bash_risk(cmd: &str) -> RiskLevel {
    let lower = cmd.to_lowercase();

    // ── HIGH: destructive / network / privilege ────────────────────────────
    let high = [
        // File destruction
        "rm -",
        "rm /",
        "del /",
        "del /f",
        "rmdir /s",
        "remove-item -r",
        // Network exfiltration
        "curl ",
        "wget ",
        "invoke-webrequest",
        "invoke-restmethod",
        "fetch ",
        // Privilege escalation
        "sudo ",
        "runas ",
        "su -",
        // Git remote writes
        "git push",
        "git force",
        "git reset --hard",
        "git clean -f",
        // System
        "shutdown",
        "restart-computer",
        "taskkill",
        "format-volume",
        "diskpart",
        "format c",
        "del c:\\",
        // Secrets
        ".ssh/",
        ".aws/",
        "credentials.json",
    ];
    if high.iter().any(|p| lower.contains(p)) {
        return RiskLevel::High;
    }

    // ── SAFE: read-only, build, test, local git reads ──────────────────────
    let safe_prefixes = [
        "cargo check",
        "cargo build",
        "cargo test",
        "cargo fmt",
        "cargo clippy",
        "cargo run",
        "cargo doc",
        "cargo tree",
        "rustc ",
        "rustfmt ",
        "git status",
        "git log",
        "git diff",
        "git branch",
        "git show",
        "git stash list",
        "git remote -v",
        "ls ",
        "ls\n",
        "dir ",
        "dir\n",
        "echo ",
        "pwd",
        "whoami",
        "cat ",
        "type ",
        "head ",
        "tail ",
        "get-childitem",
        "get-content",
        "get-location",
        "cargo --version",
        "rustc --version",
        "git --version",
        "node --version",
        "npm --version",
        "python --version",
        // Read-only search and inspection — must never require approval
        "grep ",
        "grep\n",
        "rg ",
        "rg\n",
        "find ",
        "find\n",
        "select-string",
        "select-object",
        "where-object",
        "sort ",
        "sort\n",
        "wc ",
        "uniq ",
        "cut ",
        "file ",
        "stat ",
        "du ",
        "df ",
        // PowerShell wrapped read-only commands (Select-String, Get-ChildItem inside powershell -Command)
        "powershell -command \"select-string",
        "powershell -command \"get-childitem",
        "powershell -command \"get-content",
        "powershell -command \"get-counter",
        "powershell -command 'select-string",
        "powershell -command 'get-childitem",
        "powershell -command 'get-counter",
        "get-counter",
    ];
    if safe_prefixes
        .iter()
        .any(|p| lower.starts_with(p) || lower == p.trim())
    {
        return RiskLevel::Safe;
    }

    // ── MODERATE: mutation ops that don't destroy data ─────────────────────
    RiskLevel::Moderate
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_blacklist_windows_system() {
        // Evaluate target string explicitly normalizing unicode and backslash injection vectors!
        let root = Path::new("C:\\Users\\ocean\\Project");
        let target = Path::new("C:\\Windows\\System32\\cmd.exe");
        let result = path_is_safe(root, target);
        assert!(
            result.is_err(),
            "Windows System directory should be blocked!"
        );
        assert!(result.unwrap_err().contains("Security Blacklist"));
    }

    #[test]
    fn test_relative_parent_traversal_is_blocked() {
        let root = std::env::current_dir().unwrap();
        let result = path_is_safe(&root, Path::new(".."));
        assert!(
            result.is_err(),
            "Relative traversal outside of workspace root should be blocked!"
        );
        assert!(result.unwrap_err().contains("SANDBOX BREACHED"));
    }

    #[test]
    fn test_absolute_outside_path_is_allowed_when_not_blacklisted() {
        let root = std::env::current_dir().unwrap();
        if let Some(parent) = root.parent() {
            let result = path_is_safe(&root, parent);
            assert!(
                result.is_ok(),
                "Absolute non-blacklisted paths should follow the relaxed sandbox policy."
            );
        }
    }

    #[test]
    fn test_bash_blacklist() {
        let cmd = "ls C:\\Windows";
        let result = bash_is_safe(cmd);
        assert!(
            result.is_err(),
            "Bash command touching Windows should be blocked!"
        );
        assert!(result.unwrap_err().contains("blacklisted system area"));
    }

    #[test]
    fn test_risk_classification() {
        assert_eq!(classify_bash_risk("cargo check"), RiskLevel::Safe);
        assert_eq!(classify_bash_risk("rm -rf /"), RiskLevel::High);
        assert_eq!(classify_bash_risk("mkdir new_dir"), RiskLevel::Moderate);
        assert_eq!(classify_bash_risk("get-counter '\\PhysicalDisk(_Total)\\Avg. Disk Queue Length'"), RiskLevel::Safe);
        assert_eq!(classify_bash_risk("powershell -command \"get-counter '\\PhysicalDisk(_Total)\\Avg. Disk Queue Length'\""), RiskLevel::Safe);
    }
}
