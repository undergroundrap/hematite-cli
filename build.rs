use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    emit_git_build_info();

    // Dynamically target standard Windows CUDA implementations
    let base_cuda_path = PathBuf::from("C:\\Program Files\\NVIDIA GPU Computing Toolkit\\CUDA");

    if base_cuda_path.exists() {
        if let Ok(entries) = fs::read_dir(&base_cuda_path) {
            let mut versions: Vec<PathBuf> = entries
                .filter_map(|e| e.ok())
                .map(|e| e.path())
                .filter(|p| {
                    p.is_dir()
                        && p.file_name()
                            .unwrap_or_default()
                            .to_string_lossy()
                            .starts_with("v12.")
                })
                .collect();

            // Sort to ensure highest version (v12.x) is strictly prioritized for RTX architecture
            versions.sort();

            if let Some(latest) = versions.last() {
                let lib_path = latest.join("lib").join("x64");
                if lib_path.exists() {
                    // Inject natively into the cargo compiler bounds evading pure CPU fallbacks!
                    println!("cargo:rustc-link-search=native={}", lib_path.display());
                    println!("cargo:rustc-link-lib=dylib=cudart");

                    #[cfg(target_os = "windows")]
                    println!("cargo:rustc-link-lib=dylib=cublas");
                }
            }
        }
    }

    // Embed application icon on Windows
    #[cfg(target_os = "windows")]
    {
        let mut res = winres::WindowsResource::new();
        res.set_icon("assets/hematite.ico");
        if let Err(e) = res.compile() {
            eprintln!("winres warning: {e}");
        }
    }

    // Auto-Rerun on build constraint updates
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=assets/hematite.ico");
}

fn emit_git_build_info() {
    let repo_root = std::env::var_os("CARGO_MANIFEST_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."));

    let git_dir = resolve_git_dir(&repo_root);
    if let Some(git_dir) = git_dir.as_ref() {
        println!("cargo:rerun-if-changed={}", git_dir.join("HEAD").display());
        println!("cargo:rerun-if-changed={}", git_dir.join("index").display());
        if let Ok(head) = fs::read_to_string(git_dir.join("HEAD")) {
            if let Some(reference) = head.trim().strip_prefix("ref: ") {
                println!(
                    "cargo:rerun-if-changed={}",
                    git_dir.join(reference).display()
                );
            }
        }
    }

    let commit = git_output(&repo_root, &["rev-parse", "--short", "HEAD"]).unwrap_or_default();
    let exact_tag =
        git_output(&repo_root, &["describe", "--tags", "--exact-match"]).unwrap_or_default();
    let dirty = git_output(&repo_root, &["status", "--porcelain"])
        .map(|out| (!out.trim().is_empty()).to_string())
        .unwrap_or_else(|| "false".to_string());

    println!("cargo:rustc-env=HEMATITE_GIT_COMMIT_SHORT={}", commit);
    println!("cargo:rustc-env=HEMATITE_GIT_EXACT_TAG={}", exact_tag);
    println!("cargo:rustc-env=HEMATITE_GIT_DIRTY={}", dirty);
}

fn resolve_git_dir(repo_root: &Path) -> Option<PathBuf> {
    let dot_git = repo_root.join(".git");
    if dot_git.is_dir() {
        return Some(dot_git);
    }
    let content = fs::read_to_string(&dot_git).ok()?;
    let gitdir = content.trim().strip_prefix("gitdir:")?.trim();
    let path = PathBuf::from(gitdir);
    if path.is_absolute() {
        Some(path)
    } else {
        Some(repo_root.join(path))
    }
}

fn git_output(repo_root: &PathBuf, args: &[&str]) -> Option<String> {
    let output = Command::new("git")
        .current_dir(repo_root)
        .args(args)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Some(text)
}
