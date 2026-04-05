use std::fs;
use std::path::PathBuf;

use crate::agent::git;

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

        static_sections.push("<|think|>".to_string());
        static_sections.push("# IDENTITY & TONE".to_string());
        static_sections.push("You are Hematite, a local coding system for the user's machine and repository. \
                             Hematite is more than the terminal UI: it is the full local harness for tool use, code editing, context management, voice, and orchestration. \
                             Be direct, practical, technically precise, and ASCII-first in ordinary prose. \
                             For simple questions, answer briefly in plain language. \
                             Do not expose internal tool names, hidden protocols, or planning jargon unless the user asks.".to_string());

        static_sections.push("\n# ARCHITECTURAL CONSTRAINTS".to_string());
        static_sections.push("- **Model**: Gemma-4-E4B (Native Multimodal Dense Agent). \n\
                             - **Context Baseline**: 32,768 (32K) tokens. Use this for snappy, high-quality reasoning. \n\
                             - **High-Capacity Mode**: 131,072 (128K) tokens. Available for full-repo analysis. \n\
                             - **Hybrid Attention**: You have a 512-token Sliding Window Attention (SWA). Prioritize immediate local context while relying on Global layers for structural coherence. \n\
                             - **Multimodal Interleaving**: Place descriptions of images/audio BEFORE your textual conclusions.".to_string());

        static_sections.push(format!("\n# BASE INSTRUCTIONS\n{base_instructions}"));

        if let Some(home) = std::env::var_os("USERPROFILE") {
            let global_path = PathBuf::from(home).join(".hematite").join("CLAUDE.md");
            if global_path.exists() {
                if let Ok(content) = fs::read_to_string(&global_path) {
                    static_sections.push(format!("\n# GLOBAL USER PREFERENCES\n{content}"));
                }
            }
        }

        for name in &["CLAUDE.md", ".claude.md", "CLAUDE.local.md"] {
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
        prompt.push_str("20. **Deep Sync**: Every 6th turn, review the full TASK.md.\n\n21. **File Modifications**: Always use multi_search_replace when editing existing code blocks.");

        prompt
    }
}
