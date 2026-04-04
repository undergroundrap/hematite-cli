use std::fs;
use std::path::PathBuf;

use crate::agent::git;

pub struct SystemPromptBuilder {
    pub workspace_root: PathBuf,
}

impl SystemPromptBuilder {
    pub fn new(root: PathBuf) -> Self {
        Self { workspace_root: root }
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
        static_sections.push("You are Hematite, an advanced AI collaborator modeled after a helpful, concise, and highly intelligent strategist. \
                             You are an advanced software architect designed for deep reasoning and precision code manipulation.".to_string());

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
                                static_sections.push(format!("\n# DEEP CONTEXT RULES ({}.md)\n{}", stem, content));
                            }
                        }
                    }
                }
            }
        }

        let mut prompt = static_sections.join("\n");
        prompt.push_str("\n\n###############################################################################\n");
        prompt.push_str("# DYNAMIC CONTEXT (Changes every turn)                                        #\n");
        prompt.push_str("###############################################################################\n");

        if let Some(s) = summary {
            prompt.push_str(&format!("\n# COMPACTED HISTORY SUMMARY\n{}\nRecent messages are preserved below.", s));
        }

        if let Some(mem) = memory {
            prompt.push_str(&format!("\n# SESSION MEMORY\n{mem}"));
        }

        prompt.push_str("\n# ENVIRONMENT");
        prompt.push_str(&format!("\n- Local Time: {}", chrono::Local::now().format("%Y-%m-%d %H:%M:%S")));
        prompt.push_str("\n- Operating System: Windows (User workspace)");

        if git::is_git_repo(&self.workspace_root) {
            if let Ok(branch) = git::get_active_branch(&self.workspace_root) {
                prompt.push_str(&format!("\n- Git Branch: {branch}"));
            }
        }

        let hematite_dir = self.workspace_root.join(".hematite");
        for (name, path) in [("TASK", hematite_dir.join("TASK.md")), ("PLAN", hematite_dir.join("PLAN.md"))] {
            if path.exists() {
                if let Ok(content) = fs::read_to_string(&path) {
                    if !content.trim().is_empty() {
                        let content = if content.len() > 3000 {
                            format!("{}...[Truncated]", &content[..3000])
                        } else {
                            content
                        };
                        prompt.push_str(&format!("\n\n# ACTIVE TASK {} (.hematite/)\n{}", name, content));
                    }
                }
            }
        }

        if !mcp_tools.is_empty() {
            prompt.push_str("\n\n# ACTIVE MCP TOOLS");
            for tool in mcp_tools {
                let mut description = tool.description.clone().unwrap_or_else(|| "No description provided.".to_string());
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
        prompt.push_str("7. **Deep Sync**: Every 6th turn, perform a 'deep sync' by reviewing the full TASK.md.\n\n8. **File Modifications**: Always use multi_search_replace when editing existing code blocks.");

        prompt
    }
}
