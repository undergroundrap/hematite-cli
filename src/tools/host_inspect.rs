use serde_json::Value;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

const DEFAULT_MAX_ENTRIES: usize = 10;
const MAX_ENTRIES_CAP: usize = 25;
const DIRECTORY_SCAN_NODE_BUDGET: usize = 25_000;

pub async fn inspect_host(args: &Value) -> Result<String, String> {
    let topic = args
        .get("topic")
        .and_then(|v| v.as_str())
        .unwrap_or("summary");
    let max_entries = parse_max_entries(args);

    match topic {
        "summary" => inspect_summary(max_entries),
        "toolchains" => inspect_toolchains(),
        "path" => inspect_path(max_entries),
        "desktop" => inspect_known_directory("Desktop", desktop_dir(), max_entries).await,
        "downloads" => inspect_known_directory("Downloads", downloads_dir(), max_entries).await,
        "directory" => {
            let raw_path = args
                .get("path")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    "Missing required argument: 'path' for inspect_host(topic: \"directory\")"
                        .to_string()
                })?;
            let resolved = resolve_path(raw_path)?;
            inspect_directory("Directory", resolved, max_entries).await
        }
        other => Err(format!(
            "Unknown inspect_host topic '{}'. Use one of: summary, toolchains, path, desktop, downloads, directory.",
            other
        )),
    }
}

fn parse_max_entries(args: &Value) -> usize {
    args.get("max_entries")
        .and_then(|v| v.as_u64())
        .map(|n| n as usize)
        .unwrap_or(DEFAULT_MAX_ENTRIES)
        .clamp(1, MAX_ENTRIES_CAP)
}

fn inspect_summary(max_entries: usize) -> Result<String, String> {
    let current_dir =
        std::env::current_dir().map_err(|e| format!("Failed to get current directory: {e}"))?;
    let workspace_root = crate::tools::file_ops::workspace_root();
    let workspace_mode = workspace_mode_label(&workspace_root);
    let path_stats = analyze_path_env();
    let toolchains = collect_toolchains();

    let mut out = String::from("Host inspection: summary\n\n");
    out.push_str(&format!("- OS: {}\n", std::env::consts::OS));
    out.push_str(&format!("- Current directory: {}\n", current_dir.display()));
    out.push_str(&format!("- Workspace root: {}\n", workspace_root.display()));
    out.push_str(&format!("- Workspace mode: {}\n", workspace_mode));
    out.push_str(&format!("- Preferred shell: {}\n", preferred_shell_label()));
    out.push_str(&format!(
        "- PATH entries: {} total, {} unique, {} duplicates, {} missing\n",
        path_stats.total_entries,
        path_stats.unique_entries,
        path_stats.duplicate_entries.len(),
        path_stats.missing_entries.len()
    ));

    if toolchains.found.is_empty() {
        out.push_str(
            "- Toolchains found: none of the common developer tools were detected on PATH\n",
        );
    } else {
        out.push_str("- Toolchains found:\n");
        for (label, version) in toolchains.found.iter().take(max_entries.min(8)) {
            out.push_str(&format!("  - {}: {}\n", label, version));
        }
        if toolchains.found.len() > max_entries.min(8) {
            out.push_str(&format!(
                "  - ... {} more found tools omitted\n",
                toolchains.found.len() - max_entries.min(8)
            ));
        }
    }

    if !toolchains.missing.is_empty() {
        out.push_str(&format!(
            "- Common tools not detected on PATH: {}\n",
            toolchains.missing.join(", ")
        ));
    }

    for (label, path) in [("Desktop", desktop_dir()), ("Downloads", downloads_dir())] {
        match path {
            Some(path) if path.exists() => match count_top_level_items(&path) {
                Ok(count) => out.push_str(&format!(
                    "- {}: {} top-level items at {}\n",
                    label,
                    count,
                    path.display()
                )),
                Err(e) => out.push_str(&format!(
                    "- {}: exists at {} but could not inspect ({})\n",
                    label,
                    path.display(),
                    e
                )),
            },
            Some(path) => out.push_str(&format!(
                "- {}: expected at {} but not found\n",
                label,
                path.display()
            )),
            None => out.push_str(&format!("- {}: location unavailable on this host\n", label)),
        }
    }

    Ok(out.trim_end().to_string())
}

fn inspect_toolchains() -> Result<String, String> {
    let report = collect_toolchains();
    let mut out = String::from("Host inspection: toolchains\n\n");

    if report.found.is_empty() {
        out.push_str("- No common developer tools were detected on PATH.");
    } else {
        out.push_str("Detected developer tools:\n");
        for (label, version) in report.found {
            out.push_str(&format!("- {}: {}\n", label, version));
        }
    }

    if !report.missing.is_empty() {
        out.push_str("\nNot detected on PATH:\n");
        for label in report.missing {
            out.push_str(&format!("- {}\n", label));
        }
    }

    Ok(out.trim_end().to_string())
}

fn inspect_path(max_entries: usize) -> Result<String, String> {
    let path_stats = analyze_path_env();
    let mut out = String::from("Host inspection: PATH\n\n");
    out.push_str(&format!("- Total entries: {}\n", path_stats.total_entries));
    out.push_str(&format!(
        "- Unique entries: {}\n",
        path_stats.unique_entries
    ));
    out.push_str(&format!(
        "- Duplicate entries: {}\n",
        path_stats.duplicate_entries.len()
    ));
    out.push_str(&format!(
        "- Missing paths: {}\n",
        path_stats.missing_entries.len()
    ));

    out.push_str("\nPATH entries:\n");
    for entry in path_stats.entries.iter().take(max_entries) {
        out.push_str(&format!("- {}\n", entry));
    }
    if path_stats.entries.len() > max_entries {
        out.push_str(&format!(
            "- ... {} more entries omitted\n",
            path_stats.entries.len() - max_entries
        ));
    }

    if !path_stats.duplicate_entries.is_empty() {
        out.push_str("\nDuplicate entries:\n");
        for entry in path_stats.duplicate_entries.iter().take(max_entries) {
            out.push_str(&format!("- {}\n", entry));
        }
        if path_stats.duplicate_entries.len() > max_entries {
            out.push_str(&format!(
                "- ... {} more duplicates omitted\n",
                path_stats.duplicate_entries.len() - max_entries
            ));
        }
    }

    if !path_stats.missing_entries.is_empty() {
        out.push_str("\nMissing directories:\n");
        for entry in path_stats.missing_entries.iter().take(max_entries) {
            out.push_str(&format!("- {}\n", entry));
        }
        if path_stats.missing_entries.len() > max_entries {
            out.push_str(&format!(
                "- ... {} more missing entries omitted\n",
                path_stats.missing_entries.len() - max_entries
            ));
        }
    }

    Ok(out.trim_end().to_string())
}

async fn inspect_known_directory(
    label: &str,
    path: Option<PathBuf>,
    max_entries: usize,
) -> Result<String, String> {
    let path = path.ok_or_else(|| format!("{} location is unavailable on this host.", label))?;
    inspect_directory(label, path, max_entries).await
}

async fn inspect_directory(
    label: &str,
    path: PathBuf,
    max_entries: usize,
) -> Result<String, String> {
    let label = label.to_string();
    tokio::task::spawn_blocking(move || inspect_directory_sync(&label, &path, max_entries))
        .await
        .map_err(|e| format!("inspect_host task failed: {e}"))?
}

fn inspect_directory_sync(label: &str, path: &Path, max_entries: usize) -> Result<String, String> {
    if !path.exists() {
        return Err(format!("Path does not exist: {}", path.display()));
    }
    if !path.is_dir() {
        return Err(format!("Path is not a directory: {}", path.display()));
    }

    let mut top_level_entries = Vec::new();
    for entry in fs::read_dir(path)
        .map_err(|e| format!("Failed to read directory {}: {e}", path.display()))?
    {
        match entry {
            Ok(entry) => top_level_entries.push(entry),
            Err(_) => continue,
        }
    }
    top_level_entries.sort_by_key(|entry| entry.file_name());

    let top_level_count = top_level_entries.len();
    let mut sample_names = Vec::new();
    let mut largest_entries = Vec::new();
    let mut aggregate = PathAggregate::default();
    let mut budget = DIRECTORY_SCAN_NODE_BUDGET;

    for entry in top_level_entries {
        let name = entry.file_name().to_string_lossy().to_string();
        if sample_names.len() < max_entries {
            sample_names.push(name.clone());
        }
        let kind = match entry.file_type() {
            Ok(ft) if ft.is_dir() => "dir",
            Ok(ft) if ft.is_symlink() => "symlink",
            _ => "file",
        };
        let stats = measure_path(&entry.path(), &mut budget);
        aggregate.merge(&stats);
        largest_entries.push(LargestEntry {
            name,
            kind,
            bytes: stats.total_bytes,
        });
    }

    largest_entries.sort_by(|a, b| b.bytes.cmp(&a.bytes).then_with(|| a.name.cmp(&b.name)));

    let mut out = format!("Directory inspection: {}\n\n", label);
    out.push_str(&format!("- Path: {}\n", path.display()));
    out.push_str(&format!("- Top-level items: {}\n", top_level_count));
    out.push_str(&format!("- Recursive files: {}\n", aggregate.file_count));
    out.push_str(&format!(
        "- Recursive directories: {}\n",
        aggregate.dir_count
    ));
    out.push_str(&format!(
        "- Total size: {}{}\n",
        human_bytes(aggregate.total_bytes),
        if aggregate.partial {
            " (partial scan)"
        } else {
            ""
        }
    ));
    if aggregate.skipped_entries > 0 {
        out.push_str(&format!(
            "- Skipped entries: {} (permissions, symlinks, or scan budget)\n",
            aggregate.skipped_entries
        ));
    }

    if !largest_entries.is_empty() {
        out.push_str("\nLargest top-level entries:\n");
        for entry in largest_entries.iter().take(max_entries) {
            out.push_str(&format!(
                "- {} [{}] - {}\n",
                entry.name,
                entry.kind,
                human_bytes(entry.bytes)
            ));
        }
    }

    if !sample_names.is_empty() {
        out.push_str("\nSample names:\n");
        for name in sample_names {
            out.push_str(&format!("- {}\n", name));
        }
    }

    Ok(out.trim_end().to_string())
}

fn resolve_path(raw: &str) -> Result<PathBuf, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err("Path must not be empty.".to_string());
    }

    if let Some(rest) = trimmed
        .strip_prefix("~/")
        .or_else(|| trimmed.strip_prefix("~\\"))
    {
        let home = home::home_dir().ok_or_else(|| "Home directory is unavailable.".to_string())?;
        return Ok(home.join(rest));
    }

    let path = PathBuf::from(trimmed);
    if path.is_absolute() {
        Ok(path)
    } else {
        let cwd =
            std::env::current_dir().map_err(|e| format!("Failed to get current directory: {e}"))?;
        Ok(cwd.join(path))
    }
}

fn workspace_mode_label(workspace_root: &Path) -> &'static str {
    if crate::tools::file_ops::is_project_workspace() {
        "project"
    } else if workspace_root.join(".hematite").join("docs").exists() {
        "docs-only"
    } else {
        "general directory"
    }
}

fn preferred_shell_label() -> &'static str {
    #[cfg(target_os = "windows")]
    {
        "PowerShell"
    }
    #[cfg(not(target_os = "windows"))]
    {
        "sh"
    }
}

fn desktop_dir() -> Option<PathBuf> {
    home::home_dir().map(|home| home.join("Desktop"))
}

fn downloads_dir() -> Option<PathBuf> {
    home::home_dir().map(|home| home.join("Downloads"))
}

fn count_top_level_items(path: &Path) -> Result<usize, String> {
    let mut count = 0usize;
    for entry in
        fs::read_dir(path).map_err(|e| format!("Failed to read {}: {e}", path.display()))?
    {
        if entry.is_ok() {
            count += 1;
        }
    }
    Ok(count)
}

#[derive(Default)]
struct PathAggregate {
    total_bytes: u64,
    file_count: u64,
    dir_count: u64,
    skipped_entries: u64,
    partial: bool,
}

impl PathAggregate {
    fn merge(&mut self, other: &PathAggregate) {
        self.total_bytes += other.total_bytes;
        self.file_count += other.file_count;
        self.dir_count += other.dir_count;
        self.skipped_entries += other.skipped_entries;
        self.partial |= other.partial;
    }
}

struct LargestEntry {
    name: String,
    kind: &'static str,
    bytes: u64,
}

fn measure_path(path: &Path, budget: &mut usize) -> PathAggregate {
    if *budget == 0 {
        return PathAggregate {
            partial: true,
            skipped_entries: 1,
            ..PathAggregate::default()
        };
    }
    *budget -= 1;

    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(_) => {
            return PathAggregate {
                skipped_entries: 1,
                ..PathAggregate::default()
            }
        }
    };

    let file_type = metadata.file_type();
    if file_type.is_symlink() {
        return PathAggregate {
            skipped_entries: 1,
            ..PathAggregate::default()
        };
    }

    if metadata.is_file() {
        return PathAggregate {
            total_bytes: metadata.len(),
            file_count: 1,
            ..PathAggregate::default()
        };
    }

    if !metadata.is_dir() {
        return PathAggregate::default();
    }

    let mut aggregate = PathAggregate {
        dir_count: 1,
        ..PathAggregate::default()
    };

    let read_dir = match fs::read_dir(path) {
        Ok(read_dir) => read_dir,
        Err(_) => {
            aggregate.skipped_entries += 1;
            return aggregate;
        }
    };

    for child in read_dir {
        match child {
            Ok(child) => {
                let child_stats = measure_path(&child.path(), budget);
                aggregate.merge(&child_stats);
            }
            Err(_) => aggregate.skipped_entries += 1,
        }
    }

    aggregate
}

struct PathAnalysis {
    total_entries: usize,
    unique_entries: usize,
    entries: Vec<String>,
    duplicate_entries: Vec<String>,
    missing_entries: Vec<String>,
}

fn analyze_path_env() -> PathAnalysis {
    let mut entries = Vec::new();
    let mut duplicate_entries = Vec::new();
    let mut missing_entries = Vec::new();
    let mut seen = HashSet::new();

    let raw_path = std::env::var_os("PATH").unwrap_or_default();
    for path in std::env::split_paths(&raw_path) {
        let display = path.display().to_string();
        if display.trim().is_empty() {
            continue;
        }

        let normalized = normalize_path_entry(&display);
        if !seen.insert(normalized) {
            duplicate_entries.push(display.clone());
        }
        if !path.exists() {
            missing_entries.push(display.clone());
        }
        entries.push(display);
    }

    let total_entries = entries.len();
    let unique_entries = seen.len();

    PathAnalysis {
        total_entries,
        unique_entries,
        entries,
        duplicate_entries,
        missing_entries,
    }
}

fn normalize_path_entry(value: &str) -> String {
    #[cfg(target_os = "windows")]
    {
        value
            .replace('/', "\\")
            .trim_end_matches(['\\', '/'])
            .to_ascii_lowercase()
    }
    #[cfg(not(target_os = "windows"))]
    {
        value.trim_end_matches('/').to_string()
    }
}

struct ToolchainReport {
    found: Vec<(String, String)>,
    missing: Vec<String>,
}

fn collect_toolchains() -> ToolchainReport {
    let checks = [
        ToolCheck::new("git", &[CommandProbe::new("git", &["--version"])]),
        ToolCheck::new("rustc", &[CommandProbe::new("rustc", &["--version"])]),
        ToolCheck::new("cargo", &[CommandProbe::new("cargo", &["--version"])]),
        ToolCheck::new("node", &[CommandProbe::new("node", &["--version"])]),
        ToolCheck::new(
            "npm",
            &[
                CommandProbe::new("npm", &["--version"]),
                CommandProbe::new("npm.cmd", &["--version"]),
            ],
        ),
        ToolCheck::new(
            "pnpm",
            &[
                CommandProbe::new("pnpm", &["--version"]),
                CommandProbe::new("pnpm.cmd", &["--version"]),
            ],
        ),
        ToolCheck::new(
            "python",
            &[
                CommandProbe::new("python", &["--version"]),
                CommandProbe::new("python3", &["--version"]),
                CommandProbe::new("py", &["-3", "--version"]),
                CommandProbe::new("py", &["--version"]),
            ],
        ),
        ToolCheck::new("deno", &[CommandProbe::new("deno", &["--version"])]),
        ToolCheck::new("go", &[CommandProbe::new("go", &["version"])]),
        ToolCheck::new("dotnet", &[CommandProbe::new("dotnet", &["--version"])]),
        ToolCheck::new("uv", &[CommandProbe::new("uv", &["--version"])]),
    ];

    let mut found = Vec::new();
    let mut missing = Vec::new();

    for check in checks {
        match check.detect() {
            Some(version) => found.push((check.label.to_string(), version)),
            None => missing.push(check.label.to_string()),
        }
    }

    ToolchainReport { found, missing }
}

#[derive(Clone)]
struct ToolCheck {
    label: &'static str,
    probes: Vec<CommandProbe>,
}

impl ToolCheck {
    fn new(label: &'static str, probes: &[CommandProbe]) -> Self {
        Self {
            label,
            probes: probes.to_vec(),
        }
    }

    fn detect(&self) -> Option<String> {
        for probe in &self.probes {
            if let Some(output) = capture_first_line(probe.program, probe.args) {
                return Some(output);
            }
        }
        None
    }
}

#[derive(Clone, Copy)]
struct CommandProbe {
    program: &'static str,
    args: &'static [&'static str],
}

impl CommandProbe {
    const fn new(program: &'static str, args: &'static [&'static str]) -> Self {
        Self { program, args }
    }
}

fn capture_first_line(program: &str, args: &[&str]) -> Option<String> {
    let output = std::process::Command::new(program)
        .args(args)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }

    let stdout = if output.stdout.is_empty() {
        String::from_utf8_lossy(&output.stderr).into_owned()
    } else {
        String::from_utf8_lossy(&output.stdout).into_owned()
    };

    stdout
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .map(|line| line.to_string())
}

fn human_bytes(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut value = bytes as f64;
    let mut unit_index = 0usize;

    while value >= 1024.0 && unit_index < UNITS.len() - 1 {
        value /= 1024.0;
        unit_index += 1;
    }

    if unit_index == 0 {
        format!("{} {}", bytes, UNITS[unit_index])
    } else {
        format!("{value:.1} {}", UNITS[unit_index])
    }
}
