use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkspaceProfile {
    pub workspace_mode: String,
    pub primary_stack: Option<String>,
    #[serde(default)]
    pub stack_signals: Vec<String>,
    #[serde(default)]
    pub package_managers: Vec<String>,
    #[serde(default)]
    pub important_paths: Vec<String>,
    #[serde(default)]
    pub ignored_paths: Vec<String>,
    pub verify_profile: Option<String>,
    pub build_hint: Option<String>,
    pub test_hint: Option<String>,
    pub summary: String,
}

pub fn workspace_profile_path(root: &Path) -> PathBuf {
    root.join(".hematite").join("workspace_profile.json")
}

pub fn ensure_workspace_profile(root: &Path) -> Result<WorkspaceProfile, String> {
    let profile = detect_workspace_profile(root);
    let path = workspace_profile_path(root);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }

    let json = serde_json::to_string_pretty(&profile).map_err(|e| e.to_string())?;
    let existing = std::fs::read_to_string(&path).ok();
    if existing.as_deref() != Some(json.as_str()) {
        std::fs::write(&path, json).map_err(|e| e.to_string())?;
    }

    Ok(profile)
}

pub fn load_workspace_profile(root: &Path) -> Option<WorkspaceProfile> {
    let path = workspace_profile_path(root);
    std::fs::read_to_string(path)
        .ok()
        .and_then(|raw| serde_json::from_str(&raw).ok())
}

pub fn profile_prompt_block(root: &Path) -> Option<String> {
    let profile = load_workspace_profile(root).unwrap_or_else(|| detect_workspace_profile(root));
    if profile.summary.trim().is_empty() {
        return None;
    }

    let mut lines = vec![format!("Summary: {}", profile.summary)];
    if let Some(stack) = &profile.primary_stack {
        lines.push(format!("Primary stack: {}", stack));
    }
    if !profile.package_managers.is_empty() {
        lines.push(format!(
            "Package managers: {}",
            profile.package_managers.join(", ")
        ));
    }
    if let Some(profile_name) = &profile.verify_profile {
        lines.push(format!("Verify profile: {}", profile_name));
    }
    if let Some(build_hint) = &profile.build_hint {
        lines.push(format!("Build hint: {}", build_hint));
    }
    if let Some(test_hint) = &profile.test_hint {
        lines.push(format!("Test hint: {}", test_hint));
    }
    if !profile.important_paths.is_empty() {
        lines.push(format!(
            "Important paths: {}",
            profile.important_paths.join(", ")
        ));
    }
    if !profile.ignored_paths.is_empty() {
        lines.push(format!(
            "Ignore noise from: {}",
            profile.ignored_paths.join(", ")
        ));
    }

    Some(format!(
        "# Workspace Profile (auto-generated)\n{}",
        lines.join("\n")
    ))
}

pub fn profile_report(root: &Path) -> String {
    let profile = load_workspace_profile(root).unwrap_or_else(|| detect_workspace_profile(root));
    let path = workspace_profile_path(root);

    let mut out = String::new();
    out.push_str("Workspace Profile\n");
    out.push_str(&format!("Path: {}\n", path.display()));
    out.push_str(&format!("Mode: {}\n", profile.workspace_mode));
    out.push_str(&format!(
        "Primary stack: {}\n",
        profile.primary_stack.as_deref().unwrap_or("unknown")
    ));
    if !profile.stack_signals.is_empty() {
        out.push_str(&format!(
            "Stack signals: {}\n",
            profile.stack_signals.join(", ")
        ));
    }
    if !profile.package_managers.is_empty() {
        out.push_str(&format!(
            "Package managers: {}\n",
            profile.package_managers.join(", ")
        ));
    }
    if let Some(profile_name) = &profile.verify_profile {
        out.push_str(&format!("Verify profile: {}\n", profile_name));
    }
    if let Some(build_hint) = &profile.build_hint {
        out.push_str(&format!("Build hint: {}\n", build_hint));
    }
    if let Some(test_hint) = &profile.test_hint {
        out.push_str(&format!("Test hint: {}\n", test_hint));
    }
    if !profile.important_paths.is_empty() {
        out.push_str(&format!(
            "Important paths: {}\n",
            profile.important_paths.join(", ")
        ));
    }
    if !profile.ignored_paths.is_empty() {
        out.push_str(&format!(
            "Ignored noise: {}\n",
            profile.ignored_paths.join(", ")
        ));
    }
    out.push_str(&format!("Summary: {}", profile.summary));
    out
}

pub fn detect_workspace_profile(root: &Path) -> WorkspaceProfile {
    let is_project = looks_like_project_root(root);
    let workspace_mode = if is_project {
        "project"
    } else if root.join(".hematite").join("docs").exists()
        || root.join(".hematite").join("imports").exists()
    {
        "docs_only"
    } else {
        "general"
    }
    .to_string();

    let mut stack_signals = BTreeSet::new();
    let mut package_managers = BTreeSet::new();

    if root.join("Cargo.toml").exists() {
        stack_signals.insert("rust".to_string());
        package_managers.insert("cargo".to_string());
    }
    if root.join("package.json").exists() {
        stack_signals.insert("node".to_string());
        package_managers.insert(detect_node_package_manager(root));
    }
    if root.join("pyproject.toml").exists() || root.join("setup.py").exists() {
        stack_signals.insert("python".to_string());
        package_managers.insert(detect_python_package_manager(root));
    }
    if root.join("go.mod").exists() {
        stack_signals.insert("go".to_string());
        package_managers.insert("go".to_string());
    }
    if root.join("pom.xml").exists() {
        stack_signals.insert("java".to_string());
        package_managers.insert("maven".to_string());
    }
    if root.join("build.gradle").exists() || root.join("build.gradle.kts").exists() {
        stack_signals.insert("java".to_string());
        package_managers.insert("gradle".to_string());
    }
    if root.join("CMakeLists.txt").exists() {
        stack_signals.insert("cpp".to_string());
        package_managers.insert("cmake".to_string());
    }
    if has_extension_in_dir(root, "sln") || has_extension_in_dir(root, "csproj") {
        stack_signals.insert("dotnet".to_string());
        package_managers.insert("dotnet".to_string());
    }
    if root.join(".git").exists() && stack_signals.is_empty() {
        stack_signals.insert("git".to_string());
    }

    let primary_stack = stack_signals
        .iter()
        .find(|stack| stack.as_str() != "git")
        .cloned()
        .or_else(|| stack_signals.iter().next().cloned());

    let important_paths = collect_existing_paths(
        root,
        &[
            "src",
            "tests",
            "docs",
            "installer",
            "scripts",
            ".github/workflows",
            ".hematite/docs",
            ".hematite/imports",
        ],
    );
    let ignored_paths = collect_existing_paths(
        root,
        &[
            "target",
            "node_modules",
            ".git",
            ".hematite/reports",
            ".hematite/scratch",
        ],
    );

    let verify = load_workspace_verify_config(root);
    let verify_profile = verify.default_profile.clone();
    let (build_hint, test_hint) = if let Some(profile_name) = verify_profile.as_deref() {
        if let Some(profile) = verify.profiles.get(profile_name) {
            (profile.build.clone(), profile.test.clone())
        } else {
            (
                default_build_hint(root, primary_stack.as_deref()),
                default_test_hint(root, primary_stack.as_deref()),
            )
        }
    } else {
        (
            default_build_hint(root, primary_stack.as_deref()),
            default_test_hint(root, primary_stack.as_deref()),
        )
    };

    let summary = build_summary(
        &workspace_mode,
        primary_stack.as_deref(),
        &important_paths,
        verify_profile.as_deref(),
        build_hint.as_deref(),
        test_hint.as_deref(),
    );

    WorkspaceProfile {
        workspace_mode,
        primary_stack,
        stack_signals: stack_signals.into_iter().collect(),
        package_managers: package_managers
            .into_iter()
            .filter(|entry| !entry.is_empty())
            .collect(),
        important_paths,
        ignored_paths,
        verify_profile,
        build_hint,
        test_hint,
        summary,
    }
}

fn looks_like_project_root(root: &Path) -> bool {
    root.join("Cargo.toml").exists()
        || root.join("package.json").exists()
        || root.join("pyproject.toml").exists()
        || root.join("go.mod").exists()
        || root.join("setup.py").exists()
        || root.join("pom.xml").exists()
        || root.join("build.gradle").exists()
        || root.join("build.gradle.kts").exists()
        || root.join("CMakeLists.txt").exists()
        || (root.join(".git").exists() && root.join("src").exists())
}

fn has_extension_in_dir(root: &Path, ext: &str) -> bool {
    std::fs::read_dir(root)
        .ok()
        .into_iter()
        .flat_map(|entries| entries.filter_map(|entry| entry.ok()))
        .any(|entry| {
            entry
                .path()
                .extension()
                .and_then(|value| value.to_str())
                .map(|value| value.eq_ignore_ascii_case(ext))
                .unwrap_or(false)
        })
}

fn detect_node_package_manager(root: &Path) -> String {
    if root.join("pnpm-lock.yaml").exists() {
        "pnpm".to_string()
    } else if root.join("yarn.lock").exists() {
        "yarn".to_string()
    } else if root.join("bun.lockb").exists() || root.join("bun.lock").exists() {
        "bun".to_string()
    } else {
        "npm".to_string()
    }
}

fn detect_python_package_manager(root: &Path) -> String {
    let pyproject = root.join("pyproject.toml");
    if let Ok(content) = std::fs::read_to_string(pyproject) {
        let lower = content.to_ascii_lowercase();
        if lower.contains("[tool.uv") {
            return "uv".to_string();
        }
        if lower.contains("[tool.poetry") {
            return "poetry".to_string();
        }
        if lower.contains("[project]") {
            return "pip/pyproject".to_string();
        }
    }
    "pip".to_string()
}

fn collect_existing_paths(root: &Path, candidates: &[&str]) -> Vec<String> {
    candidates
        .iter()
        .filter(|candidate| root.join(candidate).exists())
        .map(|candidate| candidate.replace('\\', "/"))
        .collect()
}

fn default_build_hint(root: &Path, primary_stack: Option<&str>) -> Option<String> {
    match primary_stack {
        Some("rust") => Some("cargo build".to_string()),
        Some("node") => {
            if root.join("package.json").exists() {
                Some(format!("{} run build", detect_node_package_manager(root)))
            } else {
                None
            }
        }
        Some("python") => None,
        Some("go") => Some("go build ./...".to_string()),
        Some("java") => {
            if root.join("pom.xml").exists() {
                Some("mvn -q -DskipTests package".to_string())
            } else if root.join("build.gradle").exists() || root.join("build.gradle.kts").exists() {
                Some("./gradlew build".to_string())
            } else {
                None
            }
        }
        Some("cpp") => Some("cmake --build build".to_string()),
        _ => None,
    }
}

fn default_test_hint(root: &Path, primary_stack: Option<&str>) -> Option<String> {
    match primary_stack {
        Some("rust") => Some("cargo test".to_string()),
        Some("node") => Some(format!("{} test", detect_node_package_manager(root))),
        Some("python") => {
            if root.join("tests").exists() || root.join("test").exists() {
                Some("pytest".to_string())
            } else {
                None
            }
        }
        Some("go") => Some("go test ./...".to_string()),
        Some("java") => {
            if root.join("pom.xml").exists() {
                Some("mvn test".to_string())
            } else if root.join("build.gradle").exists() || root.join("build.gradle.kts").exists() {
                Some("./gradlew test".to_string())
            } else {
                None
            }
        }
        _ => None,
    }
}

fn build_summary(
    workspace_mode: &str,
    primary_stack: Option<&str>,
    important_paths: &[String],
    verify_profile: Option<&str>,
    build_hint: Option<&str>,
    test_hint: Option<&str>,
) -> String {
    let mut parts = Vec::new();
    match workspace_mode {
        "project" => {
            if let Some(stack) = primary_stack {
                parts.push(format!("{stack} project workspace"));
            } else {
                parts.push("project workspace".to_string());
            }
        }
        "docs_only" => parts.push("docs-only workspace".to_string()),
        _ => parts.push("general local workspace".to_string()),
    }

    if !important_paths.is_empty() {
        parts.push(format!("key paths: {}", important_paths.join(", ")));
    }
    if let Some(profile) = verify_profile {
        parts.push(format!("verify profile: {}", profile));
    } else if let Some(build) = build_hint {
        parts.push(format!("suggested build: {}", build));
    }
    if let Some(test) = test_hint {
        parts.push(format!("suggested test: {}", test));
    }

    parts.join(" | ")
}

fn load_workspace_verify_config(root: &Path) -> crate::agent::config::VerifyProfilesConfig {
    let path = root.join(".hematite").join("settings.json");
    std::fs::read_to_string(path)
        .ok()
        .and_then(|raw| serde_json::from_str::<crate::agent::config::HematiteConfig>(&raw).ok())
        .map(|config| config.verify)
        .unwrap_or_default()
}
