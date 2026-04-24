use std::fs;
use std::path::PathBuf;

use crate::agent::git;
use crate::agent::instructions::{
    guidance_section_title, resolve_guidance_path, PROJECT_GUIDANCE_FILES,
};

enum WorkspaceMode {
    Coding,
    Document,
    General,
}

fn detect_workspace_mode(root: &PathBuf) -> WorkspaceMode {
    // Strong coding signals — any of these present means it's a coding workspace
    let coding_markers = [
        "Cargo.toml",
        "package.json",
        "pyproject.toml",
        "setup.py",
        "go.mod",
        "pom.xml",
        "build.gradle",
        "CMakeLists.txt",
        "index.html",
        "style.css",
        "script.js",
        ".git",
        "src",
        "lib",
    ];
    for marker in &coding_markers {
        if root.join(marker).exists() {
            return WorkspaceMode::Coding;
        }
    }

    // No strong coding signal — check file extensions
    let code_exts = [
        "rs", "py", "ts", "js", "go", "cpp", "c", "java", "cs", "rb", "swift", "kt",
    ];
    let doc_exts = ["pdf", "md", "txt", "docx", "epub", "rst"];
    let mut code_count = 0usize;
    let mut doc_count = 0usize;

    if let Ok(entries) = fs::read_dir(root) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                    let ext = ext.to_lowercase();
                    if code_exts.contains(&ext.as_str()) {
                        code_count += 1;
                    }
                    if doc_exts.contains(&ext.as_str()) {
                        doc_count += 1;
                    }
                }
            }
        }
    }

    if code_count > 0 {
        WorkspaceMode::Coding
    } else if doc_count > 0 {
        WorkspaceMode::Document
    } else {
        WorkspaceMode::General
    }
}

pub struct SystemPromptBuilder {
    pub workspace_root: PathBuf,
}

impl SystemPromptBuilder {
    pub fn new(root: PathBuf) -> Self {
        Self {
            workspace_root: root,
        }
    }

    /// Build the full system prompt with Rule Hierarchy and Gemma-4 Optimization.
    /// Hierarchy: Global ($HOME) -> Project (Root) -> Local (Ignored).
    pub fn build(
        &self,
        base_instructions: &str,
        memory: Option<&str>,
        summary: Option<&str>,
        mcp_tools: &[crate::agent::mcp::McpTool],
    ) -> String {
        let config = crate::agent::config::load_config();
        let mut static_sections = Vec::new();

        let workspace_framing = match detect_workspace_mode(&self.workspace_root) {
            WorkspaceMode::Coding => "- **Authoritative Identity**: You are a Senior SysAdmin, Network Admin, and Software Engineer. Deliver grounded, expert diagnostics without generic assistant boilerplate. You have 100% workstation visibility via native tools.\n\
                                       - **Hardware Truth & Tool Discipline**: For any hardware, silicon, or performance query (GPU Vitals, CPU Thermals, Throttling), you MUST use `inspect_host` (topic=\"overclocker\", \"thermal\", \"hardware\").\n\
                                       - **Forbidden Regressions**: NEVER call raw shell commands like `nvidia-smi`, `wmic`, or `tasklist` for telemetry if a native `inspect_host` topic covers it.\n\
                                       - **Session History Awareness**: Use the RAM-only Silicon Historian trends reported by `inspect_host` to identify anomalies since the start of the session.\n\
                                       The current directory is a software project — lean into code editing, build verification, and repo-aware tooling.",
            WorkspaceMode::Document => "- **Authoritative Identity**: You are a Senior SysAdmin, Network Admin, and Software Engineer. Deliver grounded, expert diagnostics without generic assistant boilerplate. You have 100% workstation visibility via native tools.\n\
                                         - **Hardware Truth & Tool Discipline**: For any hardware, silicon, or performance query (GPU Vitals, CPU Thermals, Throttling), you MUST use `inspect_host` (topic=\"overclocker\", \"thermal\", \"hardware\").\n\
                                         - **Forbidden Regressions**: NEVER call raw shell commands like `nvidia-smi`, `wmic`, or `tasklist` for telemetry if a native `inspect_host` topic covers it.\n\
                                         - **Session History Awareness**: Use the RAM-only Silicon Historian trends reported by `inspect_host` to identify anomalies since the start of the session.\n\
                                         The current directory contains documents and files — lean into reading, summarizing, and hardware/network diagnostics.",
            WorkspaceMode::General => "- **Authoritative Identity**: You are a Senior SysAdmin, Network Admin, and Software Engineer. Deliver grounded, expert diagnostics without generic assistant boilerplate. You have 100% workstation visibility via native tools.\n\
                                       - **Hardware Truth & Tool Discipline**: For any hardware, silicon, or performance query (GPU Vitals, CPU Thermals, Throttling), you MUST use `inspect_host` (topic=\"overclocker\", \"thermal\", \"hardware\").\n\
                                       - **Forbidden Regressions**: NEVER call raw shell commands like `nvidia-smi`, `wmic`, or `tasklist` for telemetry if a native `inspect_host` topic covers it.\n\
                                       - **Session History Awareness**: Use the RAM-only Silicon Historian trends reported by `inspect_host` to identify anomalies since the start of the session.\n\
                                       No specific project or document context is loaded — focus on general machine health, system diagnostics, and shell-based tasks.",
        };

        static_sections.push("# IDENTITY & TONE".to_string());
        static_sections.push(format!("{} \
                             Be direct, practical, technically precise, and ASCII-first in ordinary prose. \
                             You provide 100% workstation visibility across 81+ read-only diagnostic topics (Hardware, Network, Security, OS). \
                             For simple questions, answer briefly in plain language. \
                             Do not expose internal tool names, hidden protocols, or planning jargon unless the user asks.", workspace_framing));
        static_sections.push(format!(
            "- Running Hematite build: {}",
            crate::hematite_version_display()
        ));
        static_sections.push(format!(
            "- Hematite author and maintainer: {}",
            crate::HEMATITE_AUTHOR
        ));
        static_sections.push(format!(
            "- Hematite repository: {}",
            crate::HEMATITE_REPOSITORY_URL
        ));

        static_sections.push(format!("\n# BASE INSTRUCTIONS\n{base_instructions}"));

        if let Some(home) = std::env::var_os("USERPROFILE") {
            let global_path = PathBuf::from(home).join(".hematite").join("CLAUDE.md");
            if global_path.exists() {
                if let Ok(content) = fs::read_to_string(&global_path) {
                    static_sections.push(format!("\n# GLOBAL USER PREFERENCES\n{content}"));
                }
            }
        }

        for name in PROJECT_GUIDANCE_FILES {
            let path = resolve_guidance_path(&self.workspace_root, name);
            if path.exists() {
                if let Ok(content) = fs::read_to_string(&path) {
                    let content = if content.len() > 6000 {
                        format!("{}...[Guidance Truncated]", &content[..6000])
                    } else {
                        content
                    };
                    static_sections.push(format!(
                        "\n# {} ({})\n{}",
                        guidance_section_title(name),
                        name,
                        content
                    ));
                }
            }
        }

        if let Some(skill_catalog) = crate::agent::instructions::render_skill_catalog(
            &crate::agent::instructions::discover_agent_skills(&self.workspace_root, &config.trust),
            6_000,
        ) {
            static_sections.push(format!("\n{}", skill_catalog));
        }

        let instructions_dir = crate::tools::file_ops::hematite_dir().join("instructions");
        if instructions_dir.exists() && instructions_dir.is_dir() {
            if let Ok(entries) = fs::read_dir(instructions_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().map(|e| e == "md").unwrap_or(false) {
                        let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
                        let include = if let Some(mem) = memory {
                            mem.to_lowercase().contains(&stem.to_lowercase())
                        } else {
                            false
                        };

                        if include {
                            if let Ok(content) = fs::read_to_string(&path) {
                                static_sections.push(format!(
                                    "\n# DEEP CONTEXT RULES ({}.md)\n{}",
                                    stem, content
                                ));
                            }
                        }
                    }
                }
            }
        }

        let mut prompt = static_sections.join("\n");
        prompt.push_str("\n\n- **RECOVERY MANDATE**: If a tool returns 'Read discipline' or 'HALLUCINATION BLOCKED', do NOT repeat the failing thought or call. Pivot immediately to a different grounded tool (like `inspect_host` or `inspect_lines` on a different window) to break the loop.");
        prompt.push_str(
            "\n\n###############################################################################\n",
        );
        prompt.push_str(
            "# DYNAMIC CONTEXT (Changes every turn)                                        #\n",
        );
        prompt.push_str(
            "###############################################################################\n",
        );

        if let Some(s) = summary {
            prompt.push_str(&format!(
                "\n# COMPACTED HISTORY SUMMARY\n{}\nRecent messages are preserved below.",
                s
            ));
        }

        if let Some(mem) = memory {
            prompt.push_str(&format!("\n# SESSION MEMORY\n{mem}"));
        }

        prompt.push_str("\n# ENVIRONMENT");
        prompt.push_str(&format!(
            "\n- Local Time: {}",
            chrono::Local::now().format("%Y-%m-%d %H:%M:%S")
        ));
        prompt.push_str(&format!(
            "\n- Hematite Build: {}",
            crate::hematite_version_display()
        ));
        if let Ok(user) = std::env::var("USERPROFILE") {
            prompt.push_str(&format!("\n- USERPROFILE (Authoritative): {user}"));
        }
        if let Ok(comp) = std::env::var("COMPUTERNAME") {
            prompt.push_str(&format!("\n- COMPUTERNAME (Authoritative): {comp}"));
        }
        prompt.push_str("\n- Operating System: Windows (User workspace)");

        if git::is_git_repo(&self.workspace_root) {
            if let Ok(branch) = git::get_active_branch(&self.workspace_root) {
                prompt.push_str(&format!("\n- Git Branch: {branch}"));
            }
        }

        // --- Intelligence Injection: Flat File Inventory ---
        if let Ok(entries) = fs::read_dir(&self.workspace_root) {
            let mut list = Vec::new();
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() {
                    if let Some(name) = path.file_name().and_then(|s| s.to_str()) {
                        if !name.starts_with('.') && name != "Cargo.lock" {
                            list.push(name.to_string());
                        }
                    }
                }
            }
            if !list.is_empty() {
                list.sort();
                prompt.push_str(&format!("\n- Workspace Files (Root): {}", list.join(", ")));
            }
        }

        let hematite_dir = crate::tools::file_ops::hematite_dir();
        for (name, path) in [
            ("TASK", hematite_dir.join("TASK.md")),
            ("PLAN", hematite_dir.join("PLAN.md")),
        ] {
            if path.exists() {
                if let Ok(content) = fs::read_to_string(&path) {
                    if !content.trim().is_empty() {
                        let content = if content.len() > 3000 {
                            format!("{}...[Truncated]", &content[..3000])
                        } else {
                            content
                        };
                        prompt.push_str(&format!(
                            "\n\n# ACTIVE TASK {} (.hematite/)\n{}",
                            name, content
                        ));
                    }
                }
            }
        }

        if !mcp_tools.is_empty() {
            prompt.push_str("\n\n# ACTIVE MCP TOOLS");
            for tool in mcp_tools {
                let mut description = tool
                    .description
                    .clone()
                    .unwrap_or_else(|| "No description provided.".to_string());
                if description.len() > 180 {
                    description.truncate(180);
                    description.push_str("...");
                }
                prompt.push_str(&format!("\n- {}: {}", tool.name, description));
            }
        }

        if let Some(hint) = &config.context_hint {
            prompt.push_str(&format!("\n## PROJECT CONTEXT HINT\n{}\n", hint));
        }

        prompt.push_str("\n## HEMATITE OPERATIONAL PROTOCOL\n");
        prompt.push_str("1. **Thinking Mode**: ALWAYS use the thought channel (`<|channel>thought ... <channel|>`) to plan your response.\n");
        prompt.push_str("2. **Direct Answer**: Unless hardware is specifically named (CPU, GPU, RAM, Disk), assume all performance questions are about the ACTIVE CODE/UI logic. DO NOT use `inspect_host` for code-vitals.\n");
        prompt.push_str("3. **Tool Format**: Use structured XML tags for tool calling. No natural language inside tool arguments.\n");
        prompt.push_str("4. **Identity**: You are a world-class Software Engineer. Answer from the codebase first.\n");
        prompt.push_str("5. **Continuous Goal**: Continue your task until you have fulfilled the user's intent. Stay grounded in results.\n");
        prompt.push_str("6. **Tool Discipline**: Use surgical file tools (`write_file`, `edit_file`, `grep_files`) instead of shell. Overwriting code is blocked; use hunk-patching.\n");
        prompt.push_str("7. **Workspace Efficiency**: Use `run_workspace_workflow` ONLY for project-level `build`, `test`, `lint`, or `fix`. Do NOT use it for general coding or autonomy.\n");
        prompt.push_str("8. **Host Inspection**: Use `inspect_host` ONLY for legitimate system diagnostics. Topics: hardware, security, network, updates, health_report, storage, storage_spaces, defender_quarantine.\n");
        prompt.push_str("9. **Proof Before Action**: ALWAYS `grep_files` for symbols and `read_file` to verify content before any edit.\n");
        prompt.push_str("10. **Proof Before Commit**: Run `verify_build` (or `workflow=build`) after all edits to confirm zero regressions.\n");
        prompt.push_str("11. **Edit Precision**: Match indentation and whitespace exactly in search/replace targets.\n");
        prompt.push_str("12. **Teacher Mode**: If asked how to perform an administrative task, provide a numbered walkthrough of exact PowerShell commands.\n");
        prompt.push_str("13. **Search Priority**: Use regex searches for complex patterns. Never assume a file exists without listing the directory.\n");
        prompt.push_str("14. **Communication**: Keep technical explanations concise. Focus on the 'what' and 'why' of the code change.\n");
        prompt.push_str("15. **Sovereign Safety**: If at a drive root or major system directory, ask to move to a project folder for better context.\n");
        prompt.push_str("16. **Proactive Research**: If you encounter a technical term, library version, or external API syntax you are not 100% certain about, do NOT guess. Use `research_web` to verify the latest authoritative facts. Double-check your own internal knowledge against current web reality when implementing modern tech stacks.\n");
        prompt.push_str("17. **Tool Precedence**: NEVER use the `shell` tool (e.g., `curl`, `wget`, or raw `grep` on URLs) to perform web research or fetch content if native precision tools like `research_web` or `fetch_docs` are available. Prioritize native tools for privacy and cleaner output.\n");
        prompt.push_str("18. **Entity Discovery**: For 'Who is', 'Who are', 'What is', or 'What was' queries about people, organizations, or concepts not explicitly defined in your local workspace context, ALWAYS use `research_web` to verify current facts. Do NOT guess or hallucinate identities from internal training data. If the user asks who you or your creator is, you may provide your identity from local context, but if they ask you to 'search' or 'google' that identity, you MUST use `research_web` as requested.\n");

        prompt
    }
}
