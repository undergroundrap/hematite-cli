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

/// Takes a "Ghost Snapshot" by saving the current state to a hidden ref.
/// This allows for easy rollbacks without creating visible commits or branches.
pub fn create_ghost_snapshot(repo_path: &Path) -> io::Result<()> {
    // 1. Stage all changes (so we don't lose the original state of the file we're about to edit)
    let add_status = Command::new("git")
        .arg("-C")
        .arg(repo_path)
        .arg("add")
        .arg("-A")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()?;

    if !add_status.success() {
        return Err(io::Error::new(io::ErrorKind::Other, "Git add failed"));
    }

    // 2. Create a tree from the index
    let tree_output = Command::new("git")
        .arg("-C")
        .arg(repo_path)
        .arg("write-tree")
        .stderr(Stdio::null())
        .output()?;

    if !tree_output.status.success() {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            "Git write-tree failed",
        ));
    }
    let tree_sha = String::from_utf8_lossy(&tree_output.stdout)
        .trim()
        .to_string();

    // 3. Create a commit object (parent is HEAD)
    let commit_output = Command::new("git")
        .arg("-C")
        .arg(repo_path)
        .arg("commit-tree")
        .arg(&tree_sha)
        .arg("-p")
        .arg("HEAD")
        .arg("-m")
        .arg("Hematite Ghost Snapshot")
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

    // 4. Update the hidden ghost ref
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
