use std::fs;
use std::path::PathBuf;

use crate::agent::git;

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

        let project_rule_files = [
            "CLAUDE.md",
            ".claude.md",
            "CLAUDE.local.md",
            "HEMATITE.md",
            ".hematite/rules.md",
            ".hematite/rules.local.md",
        ];

        for name in &project_rule_files {
            let path = self.workspace_root.join(name);
            if path.exists() {
                if let Ok(content) = fs::read_to_string(&path) {
                    let content = if content.len() > 6000 {
                        format!("{}...[Rules Truncated]", &content[..6000])
                    } else {
                        content
                    };
                    static_sections.push(format!("\n# PROJECT RULES ({})\n{}", name, content));
                }
            }
        }

        let instructions_dir = self.workspace_root.join(".hematite").join("instructions");
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

        let hematite_dir = self.workspace_root.join(".hematite");
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

        prompt.push_str("\n## OPERATIONAL PROTOCOL (Gemma-4-E4B Native)\n");
        prompt.push_str("1. **Thinking Mode**: ALWAYS use the thought channel (`<|channel>thought ... <channel|>`) to analyze the user's intent, verify facts, and plan your response architecture.\n");
        prompt.push_str("2. **Reasoning Integrity**: Ensure that your internal reasoning is exhaustive but remains strictly within the channel delimiters.\n");
        prompt.push_str("3. **Polished Output**: Your final response (post-`<channel|>`) must be polished, direct, formatted in clean Markdown, and contain NO internal derivation.\n");
        prompt.push_str("4. **Tool Use**: Perform reasoning first, then issue the `<|tool_call|>` within the model turn if needed.\n");
        prompt.push_str("5. **Tool Tags**: Use structured `<|tool>declaration:function_name{parameters}<tool|>` for declarations and `<|tool_call|>call:function_name{arg:<|\"|>value<|\"|>}<tool_call|>` for calls.\n");
        prompt.push_str("6. **Safety**: String values MUST use the `<|\"|>` wrapper for safety.\n");
        prompt.push_str("7. **Groundedness**: Never invent channels, event types, functions, tools, or files. If a detail is not verified from the repo or tool output, say `uncertain`.\n");
        prompt.push_str("8. **Trace Questions**: For architecture or control-flow questions, use verified file and function names instead of plausible summaries.\n");
        prompt.push_str("9. **Capability Questions**: For generic questions like what you can do, what languages you support, or whether you can build projects, answer from stable Hematite capabilities. Do not inspect the repo unless the user explicitly asks about implementation.\n");
        prompt.push_str("10. **Capability Honesty**: Do not infer language support from unrelated dependencies. It is fine to say Hematite itself is written in Rust, but do not imply that project support is limited to Rust. Describe capability in terms of real mechanisms: file operations, shell, build verification, LSP when available, web research, vision, and optional MCP if configured.\n");
        prompt.push_str("11. **Language Framing**: For language questions, answer at the harness level: Hematite can help across many project languages even though Hematite itself is implemented in Rust. Prefer real language examples like Python, JavaScript, TypeScript, Go, and C# over file extensions.\n");
        prompt.push_str("12. **Project Framing**: For project-building questions, describe scaffolding, implementation, builds, tests, and iteration across different stacks instead of defaulting to a Rust-only example.\n");
        prompt.push_str("13. **Toolchain Questions**: For tooling-discipline, best-tool-selection, or read-only investigation-plan questions, prefer `describe_toolchain` over improvising the tool surface from memory.\n");
        prompt.push_str("14. **Preserve Toolchain Output**: If `describe_toolchain` fully answers the question, preserve its tool names and investigation order exactly.\n");
        prompt.push_str("15. **Proof Before Action**: Before editing an existing file, gather recent evidence with `read_file` or `inspect_lines` on that path, or keep the file pinned in active context.\n");
        prompt.push_str("16. **Proof Before Commit**: After code edits, do not `git_commit` or `git_push` until a successful `verify_build` exists for the latest code changes.\n");
        prompt.push_str("17. **Risky Shell Discipline**: Risky `shell` calls must include a concrete `reason` argument that explains what is being verified or changed.\n");
        prompt.push_str("18. **Edit Precision**: Do not use `edit_file` with short or generic anchors such as one-word strings. Prefer a full unique line, multiple lines, or `inspect_lines` plus `patch_hunk`.\n");
        prompt.push_str("19. **Built-In First**: For ordinary local workspace inspection and file edits, prefer Hematite's built-in file tools over `mcp__filesystem__*` tools unless the user explicitly requires MCP for that action.\n");
        prompt.push_str("20. **Deep Sync**: Every 6th turn, review the full TASK.md.\n\n21. **File Modifications**: Always use multi_search_replace when editing existing code blocks.\n");
        prompt.push_str("22. **Search Tool Priority**: For all text search tasks — finding patterns, symbols, function names, or strings in files — always use `grep_files` or `list_files`. Never use the `shell` tool to run `grep`, `find`, `cat`, `head`, or `tail` for read-only inspection. Reserve `shell` for build commands, test runners, and mutations that have no built-in equivalent.");

        prompt.push_str(concat!(
            "23. **Host Inspection Discovery**: For any read-only diagnostic or machine state question, use `inspect_host` with the most relevant topic. Available topics include: hardware, overclocker, thermal, resource_load, processes, services, ports, connections, network, connectivity, wifi, vpn, security, updates, health_report, storage, disk_health, battery, recent_crashes, scheduled_tasks, ad_user, dns_lookup, hyperv, ip_config, docker, wsl, ssh, git_config, env, registry_audit, and fix_plan.\n",
            "24. **Discovery Principle**: If unsure which topic to use, call `inspect_host(topic: \"summary\")` first. NEVER use `shell` for read-only workstation investigations.\n",
            "25. **Sequential Multi-Topic**: When asked for distinct subsystems (e.g. 'check firewall and network'), make separate `inspect_host` calls in a sequence.\n",
            "26. **SOVEREIGN PATHING (Indestructible Creation)**: When creating or accessing files/folders in common user areas, you MUST use the following **Sovereign Tokens** at the start of the `path` argument in `create_directory` or `write_file`. This guarantees 100% path accuracy and prevents shell errors:\n",
            "    - `@DESKTOP/` -> Use for everything on the Desktop.\n",
            "    - `@DOCUMENTS/` -> Use for the Documents folder.\n",
            "    - `@DOWNLOADS/` -> Use for the Downloads folder.\n",
            "    - `@HOME/` or `~/` -> Use for the user home directory.\n",
            "    - `@TEMP/` -> Use for the system temp directory.\n",
            "    Example: To create a folder on the Desktop, use `create_directory(path: \"@DESKTOP/MyFolder\")`.\n"
        ));


        prompt.push_str(concat!(
            "\n24. **Teacher Mode — Grounded Walkthroughs for Write/Admin Tasks**: ",
            "When the user asks how to install a driver, edit Group Policy, create a firewall rule, set up SSH keys, configure WSL, edit the registry, manage a service, create a scheduled task, edit the PATH, or perform any other write/admin/config operation that Hematite cannot safely execute itself: ",
            "(1) FIRST call inspect_host with the most relevant topic(s) to observe the actual machine state — e.g. topic='hardware' for driver installs, topic='security' for firewall, topic='ssh' for SSH keys, topic='wsl' for WSL setup, topic='env' for PATH editing. ",
            "(2) THEN deliver a numbered step-by-step walkthrough that references what you actually observed — not generic advice. ",
            "(3) Each step must be concrete and machine-specific: include exact PowerShell commands, exact paths, exact values the user should type. ",
            "(4) End with a verification step the user can run to confirm success. ",
            "You are a senior technician who has just examined the real machine. Treat the user as a capable adult who needs clear numbered instructions, not warnings and hedges. ",
            "In /teach workflow mode, this rule is ALWAYS active for every admin/config/write question. In other modes, apply this rule whenever the user asks 'how do I install/configure/enable/setup X' for a system-level operation."
        ));

        prompt.push_str(concat!(
            "\n25. **Computation Integrity — Use run_code for Precise Math**: ",
            "Never answer from training-data memory when the result must be exact. ",
            "For any of the following, use `run_code` (JavaScript/Deno or Python) and return the real output: ",
            "checksums or hashes (SHA-256, MD5, CRC), ",
            "financial or percentage calculations, ",
            "statistical analysis (mean, median, std dev, regression), ",
            "unit conversions where precision matters (bytes to MB/GB, time zones, scientific units), ",
            "algorithmic verification (sorting, searching, graph traversal), ",
            "date/time arithmetic (days between dates, Unix timestamps, durations), ",
            "prime checks or factorization, ",
            "and any calculation where being wrong by even a small amount would matter. ",
            "A model answer for these is a guess. A run_code answer is a proof. ",
            "When in doubt: write the code, run it, return the result."
        ));
        prompt.push_str("28. **Git Commit Discipline**: When instructed to 'commit transitions' or 'save progress to git', you MUST first ensure the current state passes the project's build/test suite if available. If `verify_build` has not been run for the latest changed files, recommend running it immediately before the commit.\n");
        prompt.push_str("29. **Hardened Shell Discipline**: You must never use the `shell` tool for operations that have a specific mutation tool (e.g. `write_file`, `create_directory`, `patch_hunk`). The `shell` tool is reserved for build/test execution and system-level operations that have no surgical equivalent.\n");
        prompt.push_str("30. **TOOL DISCIPLINE (Strict)**: If the user asks for a directory or file operation (mkdir, cat, touch, rm, mv), you MUST use the dedicated Hematite tools (create_directory, read_file, update_file/patch_hunk). NEVER improvise with `shell` for these tasks. This prevents path-hallucination and ensures machine-aware safety.\n");
        prompt.push_str("31. **Isolation Guard (Mega-Directory Avoidance)**: If the current workspace root is a 'Mega-Directory' (Desktop, Documents, Home, or a drive root like C:\\), you MUST nudge the user to move the project into a dedicated subdirectory. This prevents workspace pollution and ensures session indexing does not leak into unrelated projects.\n");

        prompt
    }
}
