use serde::{Deserialize, Serialize};
use serde_json::Value;
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
    #[serde(default)]
    pub runtime_contract: Option<RuntimeContract>,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuntimeContract {
    pub loop_family: String,
    pub app_kind: String,
    pub framework_hint: Option<String>,
    #[serde(default)]
    pub preferred_workflows: Vec<String>,
    #[serde(default)]
    pub delivery_phases: Vec<String>,
    #[serde(default)]
    pub verification_workflows: Vec<String>,
    #[serde(default)]
    pub quality_gates: Vec<String>,
    pub local_url_hint: Option<String>,
    #[serde(default)]
    pub route_hints: Vec<String>,
}

pub fn workspace_profile_path(root: &Path) -> PathBuf {
    // In OS shortcut directories (Desktop, Downloads, etc.) write to the global dir
    // so no .hematite/ folder is created there.
    if crate::tools::file_ops::is_os_shortcut_directory(root) {
        return crate::tools::file_ops::hematite_dir().join("workspace_profile.json");
    }
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
    if let Some(contract) = &profile.runtime_contract {
        lines.push(format!("Loop family: {}", contract.loop_family));
        lines.push(format!("App kind: {}", contract.app_kind));
        if let Some(framework) = &contract.framework_hint {
            lines.push(format!("Framework hint: {}", framework));
        }
        if let Some(url) = &contract.local_url_hint {
            lines.push(format!("Local URL hint: {}", url));
        }
        if !contract.preferred_workflows.is_empty() {
            lines.push(format!(
                "Preferred workflows: {}",
                contract.preferred_workflows.join(", ")
            ));
        }
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

pub fn profile_strategy_prompt_block(root: &Path) -> Option<String> {
    let profile = load_workspace_profile(root).unwrap_or_else(|| detect_workspace_profile(root));
    let contract = profile.runtime_contract?;
    let mut lines = Vec::new();
    lines.push(format!(
        "Treat this workspace as a `{}` control loop, not a blank slate.",
        contract.app_kind
    ));
    if !contract.delivery_phases.is_empty() {
        lines.push(format!(
            "Work in this order: {}.",
            contract.delivery_phases.join(" -> ")
        ));
    }
    if !contract.verification_workflows.is_empty() {
        lines.push(format!(
            "Automatic proof should come from: {}.",
            contract.verification_workflows.join(", ")
        ));
    }
    if !contract.quality_gates.is_empty() {
        lines.push(format!(
            "Do not consider the task complete until these gates hold: {}.",
            contract.quality_gates.join("; ")
        ));
    }
    if let Some(url) = contract.local_url_hint {
        lines.push(format!("Local runtime hint: {}.", url));
    }
    if !contract.route_hints.is_empty() {
        lines.push(format!(
            "High-signal routes: {}.",
            contract.route_hints.join(", ")
        ));
    }
    Some(format!(
        "# Stack Delivery Contract (auto-generated)\n{}",
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
    if let Some(contract) = &profile.runtime_contract {
        out.push_str(&format!("Loop family: {}\n", contract.loop_family));
        out.push_str(&format!("App kind: {}\n", contract.app_kind));
        if let Some(framework) = &contract.framework_hint {
            out.push_str(&format!("Framework hint: {}\n", framework));
        }
        if let Some(url) = &contract.local_url_hint {
            out.push_str(&format!("Local URL hint: {}\n", url));
        }
        if !contract.preferred_workflows.is_empty() {
            out.push_str(&format!(
                "Preferred workflows: {}\n",
                contract.preferred_workflows.join(", ")
            ));
        }
        if !contract.delivery_phases.is_empty() {
            out.push_str(&format!(
                "Delivery phases: {}\n",
                contract.delivery_phases.join(" -> ")
            ));
        }
        if !contract.verification_workflows.is_empty() {
            out.push_str(&format!(
                "Verification workflows: {}\n",
                contract.verification_workflows.join(", ")
            ));
        }
        if !contract.quality_gates.is_empty() {
            out.push_str(&format!(
                "Quality gates: {}\n",
                contract.quality_gates.join("; ")
            ));
        }
        if !contract.route_hints.is_empty() {
            out.push_str(&format!(
                "Route hints: {}\n",
                contract.route_hints.join(", ")
            ));
        }
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
        // Only fallback to docs_only if we haven't already identified a managed project
        "docs_only"
    } else if root.join(".hematite").exists() {
        // If managed but no markers, treat as general (more tools available)
        "general"
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
    if root.join("pyproject.toml").exists()
        || root.join("setup.py").exists()
        || root.join("requirements.txt").exists()
        || root.join(".venv").is_dir()
        || root.join("venv").is_dir()
        || root.join("env").is_dir()
    {
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
    if root.join("index.html").exists()
        || root.join("style.css").exists()
        || root.join("script.js").exists()
    {
        stack_signals.insert("static-web".to_string());
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
            ".venv",
            "venv",
            "env",
            "vendor",
            "__pycache__",
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
    let runtime_contract = detect_runtime_contract(root, &workspace_mode, primary_stack.as_deref());

    let summary = build_summary(
        &workspace_mode,
        primary_stack.as_deref(),
        &important_paths,
        verify_profile.as_deref(),
        build_hint.as_deref(),
        test_hint.as_deref(),
        Some(&runtime_contract),
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
        runtime_contract: Some(runtime_contract),
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
        || root.join("index.html").exists()
        || root.join("style.css").exists()
        || root.join("script.js").exists()
        || root.join("main.py").exists()
        || root.join("HEMATITE_HANDOFF.md").exists()
        || root.join(".hematite").join("PLAN.md").exists()
        || root.join(".hematite").join("plan.md").exists()
        || root.join(".hematite").join("TASK.md").exists()
        || root.join(".hematite").join("task.md").exists()
        || root.join(".hematite").join("settings.json").exists()
        || root.join(".hematite").join("ACTIVE_EXEC_PLAN").exists()
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

fn detect_runtime_contract(
    root: &Path,
    workspace_mode: &str,
    primary_stack: Option<&str>,
) -> RuntimeContract {
    if let Some(stack) = primary_stack {
        let contract = match stack {
            "node" => detect_node_runtime_contract(root),
            "rust" => detect_rust_runtime_contract(root),
            "python" => detect_python_runtime_contract(root),
            "static-web" => Some(detect_static_runtime_contract()),
            _ => None,
        };
        if let Some(c) = contract {
            return c;
        }
    }

    if workspace_mode == "docs_only" {
        return detect_docs_runtime_contract();
    }

    detect_general_runtime_contract()
}

fn detect_static_runtime_contract() -> RuntimeContract {
    RuntimeContract {
        loop_family: "website".to_string(),
        app_kind: "static-site".to_string(),
        framework_hint: Some("vanilla".to_string()),
        preferred_workflows: vec!["website_status".to_string()],
        delivery_phases: vec![
            "design layout and asset structure".to_string(),
            "implement semantic html".to_string(),
            "style with vanilla css".to_string(),
            "validate assets and responsive behavior".to_string(),
        ],
        verification_workflows: vec!["build".to_string()],
        quality_gates: vec![
            "index.html exists and is valid".to_string(),
            "all linked assets resolve (no 404s)".to_string(),
            "responsive on mobile and desktop".to_string(),
        ],
        local_url_hint: None,
        route_hints: vec!["/".to_string()],
    }
}

fn detect_docs_runtime_contract() -> RuntimeContract {
    RuntimeContract {
        loop_family: "docs".to_string(),
        app_kind: "technical-documentation".to_string(),
        framework_hint: Some("markdown".to_string()),
        preferred_workflows: vec!["inspect_host".to_string()],
        delivery_phases: vec![
            "research and outline".to_string(),
            "draft core content".to_string(),
            "proofread and verify technical accuracy".to_string(),
            "check internal links and cross-references".to_string(),
        ],
        verification_workflows: vec!["build".to_string()],
        quality_gates: vec![
            "adheres to project voice".to_string(),
            "no placeholders or incomplete sections".to_string(),
            "all internal file links resolve".to_string(),
        ],
        local_url_hint: None,
        route_hints: vec![],
    }
}

fn detect_general_runtime_contract() -> RuntimeContract {
    RuntimeContract {
        loop_family: "general".to_string(),
        app_kind: "workstation-automation".to_string(),
        framework_hint: None,
        preferred_workflows: vec!["inspect_host".to_string(), "verify_build".to_string()],
        delivery_phases: vec![
            "research and environment discovery".to_string(),
            "planned surgical implementation".to_string(),
            "automated verification".to_string(),
            "completion report".to_string(),
        ],
        verification_workflows: vec!["build".to_string()],
        quality_gates: vec![
            "implementation satisfies objective".to_string(),
            "no logic regressions".to_string(),
            "workspace remains clean and hygienic".to_string(),
        ],
        local_url_hint: None,
        route_hints: vec![],
    }
}

fn detect_node_runtime_contract(root: &Path) -> Option<RuntimeContract> {
    let package = read_package_json(root).ok()?;
    let scripts = package_scripts(&package);
    let framework = infer_node_framework(&package);
    let is_website = looks_like_node_website(root, &scripts, framework.as_deref());

    if is_website {
        let local_url_hint = infer_website_default_url(framework.as_deref(), &scripts);
        return Some(RuntimeContract {
            loop_family: "website".to_string(),
            app_kind: "website".to_string(),
            framework_hint: framework.clone(),
            preferred_workflows: vec![
                "website_start".to_string(),
                "website_validate".to_string(),
                "website_status".to_string(),
                "website_stop".to_string(),
            ],
            delivery_phases: vec![
                "design routes and boundaries".to_string(),
                "scaffold feature shell".to_string(),
                "implement UI and interaction logic".to_string(),
                "validate routes and assets".to_string(),
                "update docs and task ledger".to_string(),
            ],
            verification_workflows: vec!["build".to_string(), "website_validate".to_string()],
            quality_gates: vec![
                "build stays green".to_string(),
                "critical routes return HTTP 200".to_string(),
                "linked local assets resolve".to_string(),
            ],
            local_url_hint,
            route_hints: infer_website_route_hints(root),
        });
    }

    if looks_like_node_service(&package, &scripts) {
        return Some(RuntimeContract {
            loop_family: "service".to_string(),
            app_kind: "node-service".to_string(),
            framework_hint: framework,
            preferred_workflows: vec![
                "package_script".to_string(),
                "build".to_string(),
                "test".to_string(),
            ],
            delivery_phases: vec![
                "define service boundary and inputs".to_string(),
                "implement handlers and domain logic".to_string(),
                "wire config and runtime entrypoint".to_string(),
                "verify build and targeted tests".to_string(),
                "document operational assumptions".to_string(),
            ],
            verification_workflows: vec!["build".to_string()],
            quality_gates: vec![
                "build stays green".to_string(),
                "tests cover changed behavior".to_string(),
                "config and entrypoint stay explicit".to_string(),
            ],
            local_url_hint: None,
            route_hints: Vec::new(),
        });
    }

    None
}

fn detect_rust_runtime_contract(root: &Path) -> Option<RuntimeContract> {
    if root.join("src").join("main.rs").exists() {
        Some(RuntimeContract {
            loop_family: "cli".to_string(),
            app_kind: "rust-cli".to_string(),
            framework_hint: None,
            preferred_workflows: vec!["build".to_string(), "test".to_string(), "lint".to_string()],
            delivery_phases: vec![
                "shape command surface".to_string(),
                "implement core behavior".to_string(),
                "tighten errors and output".to_string(),
                "verify build tests and lint".to_string(),
                "document usage and follow-up debt".to_string(),
            ],
            verification_workflows: vec!["build".to_string()],
            quality_gates: vec![
                "build stays green".to_string(),
                "tests cover command behavior".to_string(),
                "lint stays clean".to_string(),
            ],
            local_url_hint: None,
            route_hints: Vec::new(),
        })
    } else {
        None
    }
}

fn detect_python_runtime_contract(root: &Path) -> Option<RuntimeContract> {
    let requirements = std::fs::read_to_string(root.join("requirements.txt")).unwrap_or_default();
    let pyproject = std::fs::read_to_string(root.join("pyproject.toml")).unwrap_or_default();
    let combined = format!(
        "{}\n{}",
        requirements.to_ascii_lowercase(),
        pyproject.to_ascii_lowercase()
    );
    if combined.contains("fastapi") || combined.contains("flask") || combined.contains("django") {
        Some(RuntimeContract {
            loop_family: "service".to_string(),
            app_kind: "python-web-service".to_string(),
            framework_hint: if combined.contains("fastapi") {
                Some("fastapi".to_string())
            } else if combined.contains("django") {
                Some("django".to_string())
            } else {
                Some("flask".to_string())
            },
            preferred_workflows: vec!["build".to_string(), "test".to_string()],
            delivery_phases: vec![
                "define API surface and schemas".to_string(),
                "implement service logic".to_string(),
                "wire runtime and config".to_string(),
                "verify build and tests".to_string(),
                "document operational assumptions".to_string(),
            ],
            verification_workflows: vec!["build".to_string()],
            quality_gates: vec![
                "module import/compile pass".to_string(),
                "tests cover changed behavior".to_string(),
                "runtime entrypoint remains explicit".to_string(),
            ],
            local_url_hint: Some("http://127.0.0.1:8000/".to_string()),
            route_hints: vec!["/".to_string()],
        })
    } else {
        None
    }
}

fn read_package_json(root: &Path) -> Result<Value, String> {
    let path = root.join("package.json");
    let raw = std::fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read {}: {}", path.display(), e))?;
    serde_json::from_str(&raw).map_err(|e| format!("Failed to parse {}: {}", path.display(), e))
}

fn package_scripts(package: &Value) -> serde_json::Map<String, Value> {
    package
        .get("scripts")
        .and_then(|value| value.as_object())
        .cloned()
        .unwrap_or_default()
}

fn package_script_text(scripts: &serde_json::Map<String, Value>) -> String {
    scripts
        .values()
        .filter_map(|value| value.as_str())
        .map(|value| value.to_ascii_lowercase())
        .collect::<Vec<_>>()
        .join("\n")
}

fn package_dependency_names(package: &Value) -> BTreeSet<String> {
    let mut deps = BTreeSet::new();
    for field in ["dependencies", "devDependencies", "peerDependencies"] {
        if let Some(map) = package.get(field).and_then(|value| value.as_object()) {
            for name in map.keys() {
                deps.insert(name.to_ascii_lowercase());
            }
        }
    }
    deps
}

fn infer_node_framework(package: &Value) -> Option<String> {
    let deps = package_dependency_names(package);
    let scripts = package_script_text(&package_scripts(package));
    if deps.contains("next") || scripts.contains("next ") {
        Some("next".to_string())
    } else if deps.contains("vite") || scripts.contains("vite") {
        Some("vite".to_string())
    } else if deps.contains("astro") || scripts.contains("astro ") {
        Some("astro".to_string())
    } else if deps.contains("@angular/core") || scripts.contains("ng serve") {
        Some("angular".to_string())
    } else if deps.contains("gatsby") || scripts.contains("gatsby ") {
        Some("gatsby".to_string())
    } else if deps.contains("react-scripts") || scripts.contains("react-scripts") {
        Some("react-scripts".to_string())
    } else if deps.contains("@sveltejs/kit") || scripts.contains("svelte-kit") {
        Some("sveltekit".to_string())
    } else if deps.contains("nuxt") || scripts.contains("nuxt ") {
        Some("nuxt".to_string())
    } else if deps.contains("express") {
        Some("express".to_string())
    } else {
        None
    }
}

fn looks_like_node_service(package: &Value, scripts: &serde_json::Map<String, Value>) -> bool {
    let deps = package_dependency_names(package);
    let script_text = package_script_text(scripts);
    deps.contains("express")
        || deps.contains("fastify")
        || deps.contains("koa")
        || script_text.contains("node server")
        || script_text.contains("tsx server")
        || script_text.contains("nest start")
}

fn looks_like_node_website(
    root: &Path,
    scripts: &serde_json::Map<String, Value>,
    framework: Option<&str>,
) -> bool {
    let script_text = package_script_text(scripts);
    matches!(
        framework,
        Some("vite")
            | Some("next")
            | Some("astro")
            | Some("gatsby")
            | Some("react-scripts")
            | Some("sveltekit")
            | Some("nuxt")
            | Some("angular")
    ) || scripts.contains_key("preview")
        || script_text.contains("vite")
        || script_text.contains("next ")
        || script_text.contains("astro ")
        || script_text.contains("gatsby ")
        || script_text.contains("react-scripts")
        || script_text.contains("ng serve")
        || script_text.contains("nuxt ")
        || root.join("public").exists()
        || root.join("static").exists()
        || root.join("pages").exists()
        || root.join("src").join("pages").exists()
        || root.join("app").exists()
        || root.join("src").join("app").exists()
}

fn infer_website_default_url(
    framework: Option<&str>,
    scripts: &serde_json::Map<String, Value>,
) -> Option<String> {
    let uses_preview = scripts.contains_key("preview") && !scripts.contains_key("dev");
    let port = match framework {
        Some("vite") | Some("sveltekit") => {
            if uses_preview {
                4173
            } else {
                5173
            }
        }
        Some("astro") => 4321,
        Some("gatsby") => 8000,
        Some("angular") => 4200,
        Some("next") | Some("react-scripts") | Some("nuxt") => 3000,
        _ => 3000,
    };
    Some(format!("http://127.0.0.1:{}/", port))
}

fn infer_website_route_hints(root: &Path) -> Vec<String> {
    let mut routes = BTreeSet::new();
    routes.insert("/".to_string());

    for public_dir in ["public", "static"] {
        let dir = root.join(public_dir);
        if let Ok(entries) = std::fs::read_dir(&dir) {
            for entry in entries.filter_map(Result::ok) {
                let path = entry.path();
                if path.extension().and_then(|value| value.to_str()) == Some("html") {
                    if let Some(stem) = path.file_stem().and_then(|value| value.to_str()) {
                        if stem.eq_ignore_ascii_case("index") {
                            routes.insert("/".to_string());
                        } else {
                            routes.insert(format!("/{}.html", stem));
                        }
                    }
                }
            }
        }
    }

    for pages_dir in ["pages", "src/pages"] {
        collect_pages_routes(&root.join(pages_dir), &mut routes);
    }
    for app_dir in ["app", "src/app"] {
        collect_app_routes(&root.join(app_dir), &mut routes);
    }

    routes.into_iter().collect()
}

fn collect_pages_routes(dir: &Path, routes: &mut BTreeSet<String>) {
    collect_routes_recursive(dir, dir, routes, false);
}

fn collect_app_routes(dir: &Path, routes: &mut BTreeSet<String>) {
    collect_routes_recursive(dir, dir, routes, true);
}

fn collect_routes_recursive(dir: &Path, base: &Path, routes: &mut BTreeSet<String>, app_dir: bool) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        if path.is_dir() {
            if name.starts_with('[')
                || name == "api"
                || name.starts_with('(')
                || name.starts_with('@')
            {
                continue;
            }
            collect_routes_recursive(&path, base, routes, app_dir);
            continue;
        }

        let is_page_file = if app_dir {
            name.starts_with("page.")
        } else {
            matches!(
                path.extension().and_then(|value| value.to_str()),
                Some("js" | "jsx" | "ts" | "tsx" | "mdx")
            )
        };
        if !is_page_file {
            continue;
        }
        if matches!(
            name.as_str(),
            "_app.tsx"
                | "_app.jsx"
                | "_document.tsx"
                | "_document.jsx"
                | "layout.tsx"
                | "layout.jsx"
                | "template.tsx"
                | "template.jsx"
                | "error.tsx"
                | "loading.tsx"
                | "not-found.tsx"
        ) {
            continue;
        }
        if let Ok(relative) = path.strip_prefix(base) {
            let mut segments: Vec<String> = relative
                .iter()
                .filter_map(|part| part.to_str().map(|value| value.to_string()))
                .collect();
            if app_dir {
                let _ = segments.pop();
            } else if let Some(last) = segments.last_mut() {
                if let Some(stem) = Path::new(last).file_stem().and_then(|value| value.to_str()) {
                    *last = stem.to_string();
                }
            }
            segments.retain(|segment| {
                !segment.is_empty() && segment != "index" && !segment.starts_with('[')
            });
            let route = if segments.is_empty() {
                "/".to_string()
            } else {
                format!("/{}", segments.join("/"))
            };
            routes.insert(route);
        }
    }
}

fn build_summary(
    workspace_mode: &str,
    primary_stack: Option<&str>,
    important_paths: &[String],
    verify_profile: Option<&str>,
    build_hint: Option<&str>,
    test_hint: Option<&str>,
    runtime_contract: Option<&RuntimeContract>,
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
    if let Some(contract) = runtime_contract {
        parts.push(format!(
            "control loop: {} {}",
            contract.loop_family, contract.app_kind
        ));
        if !contract.verification_workflows.is_empty() {
            parts.push(format!(
                "verify via: {}",
                contract.verification_workflows.join(" + ")
            ));
        }
        if let Some(url) = contract.local_url_hint.as_deref() {
            parts.push(format!("local url: {}", url));
        }
    }

    parts.join(" | ")
}

fn load_workspace_verify_config(root: &Path) -> crate::agent::config::VerifyProfilesConfig {
    let path = if crate::tools::file_ops::is_os_shortcut_directory(root) {
        crate::tools::file_ops::hematite_dir().join("settings.json")
    } else {
        root.join(".hematite").join("settings.json")
    };
    std::fs::read_to_string(path)
        .ok()
        .and_then(|raw| serde_json::from_str::<crate::agent::config::HematiteConfig>(&raw).ok())
        .map(|config| config.verify)
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_detects_static_site_contract() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("index.html"), "<html></html>").unwrap();

        let profile = detect_workspace_profile(dir.path());
        assert_eq!(profile.workspace_mode, "project");
        assert_eq!(profile.primary_stack.as_deref(), Some("static-web"));

        let contract = profile
            .runtime_contract
            .as_ref()
            .expect("Contract should exist");
        assert_eq!(contract.app_kind, "static-site");
        assert!(contract
            .delivery_phases
            .iter()
            .any(|p| p.contains("vanilla css")));
    }

    #[test]
    fn test_detects_docs_only_contract() {
        let dir = tempdir().unwrap();
        // Mock docs-only mode by creating .hematite/docs
        let hem = dir.path().join(".hematite");
        fs::create_dir_all(hem.join("docs")).unwrap();

        let profile = detect_workspace_profile(dir.path());
        assert_eq!(profile.workspace_mode, "docs_only");

        let contract = profile
            .runtime_contract
            .as_ref()
            .expect("Contract should exist");
        assert_eq!(contract.app_kind, "technical-documentation");
    }

    #[test]
    fn test_managed_workspace_is_not_docs_only() {
        let dir = tempdir().unwrap();
        // folder has a .hematite folder but no docs yet
        fs::create_dir_all(dir.path().join(".hematite")).unwrap();

        let profile = detect_workspace_profile(dir.path());
        assert_eq!(profile.workspace_mode, "general"); // Managed but unknown stack
    }

    #[test]
    fn test_plan_triggers_project_mode() {
        let dir = tempdir().unwrap();
        let hem = dir.path().join(".hematite");
        fs::create_dir_all(&hem).unwrap();
        fs::write(hem.join("PLAN.md"), "# The Plan").unwrap();

        let profile = detect_workspace_profile(dir.path());
        assert_eq!(profile.workspace_mode, "project");
    }
}
