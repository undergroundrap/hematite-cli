use std::io;
use std::path::Path;
use std::process::{Command, Stdio};

pub fn is_git_repo(path: &Path) -> bool {
    Command::new("git")
        .arg("-C")
        .arg(path)
        .arg("rev-parse")
        .arg("--is-inside-work-tree")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Takes an "Isolated Ghost Snapshot" by saving the current state to a hidden ref
/// using a temporary Git index. This prevents pollution of the user's staged changes.
pub fn create_ghost_snapshot(repo_path: &Path) -> io::Result<()> {
    // 1. Create a temporary index file to avoid touching the user's actual index.
    let (temp_file, index_path) = match tempfile::NamedTempFile::new() {
        Ok(t) => {
            let (file, path) = t.into_parts();
            (file, path)
        }
        Err(e) => {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                format!("Failed to create temp index: {}", e),
            ))
        }
    };
    // Close the file handle immediately so Git can own it.
    drop(temp_file);

    // 2. Pre-populate the temporary index with HEAD so unchanged tracked files
    // are included in the snapshot tree.
    let _ = Command::new("git")
        .arg("-C")
        .arg(repo_path)
        .env("GIT_INDEX_FILE", &index_path)
        .arg("read-tree")
        .arg("HEAD")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();

    // 3. Stage all current working directory changes (tracked + untracked) into the temp index.
    let add_status = Command::new("git")
        .arg("-C")
        .arg(repo_path)
        .env("GIT_INDEX_FILE", &index_path)
        .arg("add")
        .arg("--all")
        .stderr(Stdio::piped())
        .status()?;

    if !add_status.success() {
        // Cleanup on failure
        let _ = std::fs::remove_file(&index_path);
        return Err(io::Error::new(
            io::ErrorKind::Other,
            "Git add to temp index failed",
        ));
    }

    // 4. Create a tree object from the temporary index state.
    let tree_output = Command::new("git")
        .arg("-C")
        .arg(repo_path)
        .env("GIT_INDEX_FILE", &index_path)
        .arg("write-tree")
        .stderr(Stdio::null())
        .output()?;

    // Cleanup temp index now that we have the tree SHA
    let _ = std::fs::remove_file(&index_path);

    if !tree_output.status.success() {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            "Git write-tree failed",
        ));
    }
    let tree_sha = String::from_utf8_lossy(&tree_output.stdout)
        .trim()
        .to_string();

    // 5. Create a commit object (parent is HEAD).
    let commit_output = Command::new("git")
        .arg("-C")
        .arg(repo_path)
        .arg("commit-tree")
        .arg(&tree_sha)
        .arg("-p")
        .arg("HEAD")
        .arg("-m")
        .arg("Hematite Ghost Snapshot [Isolated]")
        .stderr(Stdio::null())
        .output()?;

    if !commit_output.status.success() {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            "Git commit-tree failed",
        ));
    }
    let commit_sha = String::from_utf8_lossy(&commit_output.stdout)
        .trim()
        .to_string();

    // 6. Update the hidden ghost ref.
    let update_status = Command::new("git")
        .arg("-C")
        .arg(repo_path)
        .arg("update-ref")
        .arg("refs/hematite/ghost")
        .arg(&commit_sha)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()?;

    if !update_status.success() {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            "Git update-ref failed",
        ));
    }

    Ok(())
}

/// Reverts a file to its state in the last Ghost Snapshot.
pub fn revert_from_ghost(repo_path: &Path, file_path: &str) -> io::Result<String> {
    let status = Command::new("git")
        .arg("-C")
        .arg(repo_path)
        .arg("checkout")
        .arg("refs/hematite/ghost")
        .arg("--")
        .arg(file_path)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()?;

    if !status.success() {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            "Git checkout from ghost ref failed",
        ));
    }

    Ok(format!("Restored {} from Git Ghost ref", file_path))
}

pub fn get_active_branch(repo_path: &Path) -> io::Result<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo_path)
        .arg("rev-parse")
        .arg("--abbrev-ref")
        .arg("HEAD")
        .stderr(Stdio::null())
        .output()?;
    if !output.status.success() {
        return Err(io::Error::new(io::ErrorKind::Other, "Git rev-parse failed"));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_ghost_snapshot_isolation() {
        let dir = tempdir().unwrap();
        let repo_path = dir.path();

        // Initialize a fake repo
        Command::new("git")
            .arg("-C")
            .arg(repo_path)
            .arg("init")
            .status()
            .unwrap();
        Command::new("git")
            .arg("-C")
            .arg(repo_path)
            .arg("config")
            .arg("user.email")
            .arg("test@example.com")
            .status()
            .unwrap();
        Command::new("git")
            .arg("-C")
            .arg(repo_path)
            .arg("config")
            .arg("user.name")
            .arg("Test")
            .status()
            .unwrap();

        // Create initial commit
        fs::write(repo_path.join("file1.txt"), "hello").unwrap();
        Command::new("git")
            .arg("-C")
            .arg(repo_path)
            .arg("add")
            .arg(".")
            .status()
            .unwrap();
        Command::new("git")
            .arg("-C")
            .arg(repo_path)
            .arg("commit")
            .arg("-m")
            .arg("first")
            .status()
            .unwrap();

        // Make an unstaged change
        fs::write(repo_path.join("file2.txt"), "untracked").unwrap();
        fs::write(repo_path.join("file1.txt"), "modified").unwrap();

        // Pre-condition: git status should show changes
        let status_before = Command::new("git")
            .arg("-C")
            .arg(repo_path)
            .arg("status")
            .arg("--porcelain")
            .output()
            .unwrap();
        let status_before_str = String::from_utf8_lossy(&status_before.stdout).to_string();

        // Take ghost snapshot
        create_ghost_snapshot(repo_path).unwrap();

        // Post-condition: git status should be IDENTICAL (nothing extra staged in real index)
        let status_after = Command::new("git")
            .arg("-C")
            .arg(repo_path)
            .arg("status")
            .arg("--porcelain")
            .output()
            .unwrap();
        let status_after_str = String::from_utf8_lossy(&status_after.stdout).to_string();

        assert_eq!(
            status_before_str, status_after_str,
            "Ghost snapshot should not pollute the user's Git index"
        );

        // Verify the ghost ref exists
        let ref_check = Command::new("git")
            .arg("-C")
            .arg(repo_path)
            .arg("rev-parse")
            .arg("refs/hematite/ghost")
            .status()
            .unwrap();
        assert!(ref_check.success());
    }
}
