use serde_json::Value;

pub async fn describe_toolchain(args: &Value) -> Result<String, String> {
    let topic = args.get("topic").and_then(|v| v.as_str()).unwrap_or("all");
    let question = args.get("question").and_then(|v| v.as_str()).unwrap_or("");

    match topic {
        "read_only_codebase" => Ok(describe_read_only_codebase_tools()),
        "user_turn_plan" => Ok(describe_user_turn_plan(question)),
        "all" => Ok(format!(
            "{}\n\n{}",
            describe_read_only_codebase_tools(),
            describe_user_turn_plan(question)
        )),
        other => Err(format!(
            "Unknown topic '{}'. Use one of: read_only_codebase, user_turn_plan, all.",
            other
        )),
    }
}

fn describe_read_only_codebase_tools() -> String {
    "Verified Hematite read-only toolchain\n\n\
Text search and file inspection\n\
- `map_project`\n\
  Good for: first-pass spatial awareness of the repository layout.\n\
  Bad for: symbol-level detail or exact control flow.\n\
  Choose it over another tool when: you need the top-level shape of the codebase before diving into files.\n\
- `list_files`\n\
  Good for: enumerating files in a directory, optionally narrowed by extension.\n\
  Bad for: content search or semantic understanding.\n\
  Choose it over another tool when: you know the directory area but need concrete file candidates.\n\
- `grep_files`\n\
  Good for: fast textual search across many files, including regex and context lines.\n\
  Bad for: exact symbol definitions, types, or call relationships.\n\
  Choose it over another tool when: you know a string pattern but not the owning symbol.\n\
- `read_file`\n\
  Good for: reading a full file or a large chunk once you know the target path.\n\
  Bad for: precise line-range inspection in very large files.\n\
  Choose it over another tool when: you already know the file and need broad local context.\n\
- `inspect_lines`\n\
  Good for: tight, line-ranged inspection after you know the relevant window.\n\
  Bad for: first-pass exploration or cross-file search.\n\
  Choose it over another tool when: you want exact nearby lines without rereading the whole file.\n\n\
Semantic and LSP tools\n\
- `lsp_search_symbol`\n\
  Good for: jumping to a named symbol quickly across the workspace.\n\
  Bad for: fuzzy textual patterns or unknown names.\n\
  Choose it over another tool when: you know the symbol name and want the fastest semantic entry point.\n\
- `lsp_definitions`\n\
  Good for: confirming the exact definition site of a symbol at a position.\n\
  Bad for: finding every caller or usage.\n\
  Choose it over another tool when: you already have a coordinate and need the true definition.\n\
- `lsp_references`\n\
  Good for: tracing who uses a symbol across the project.\n\
  Bad for: initial discovery when you do not know the symbol yet.\n\
  Choose it over another tool when: you need impact analysis or call-flow expansion.\n\
- `lsp_hover`\n\
  Good for: quick type and documentation context at a position.\n\
  Bad for: ownership mapping or full call graphs.\n\
  Choose it over another tool when: you need a compact semantic summary before deeper reading.\n\
- `lsp_get_diagnostics`\n\
  Good for: current compiler and analysis errors on a file.\n\
  Bad for: architecture understanding.\n\
  Choose it over another tool when: you need to validate file health or check active breakage.\n\
  Conditional: usefulness depends on the language server being available and healthy.\n\n\
Runtime and control-flow tools\n\
- `trace_runtime_flow`\n\
  Good for: authoritative runtime/control-flow questions such as user turns, startup, session reset, and reasoning separation.\n\
  Bad for: arbitrary feature ownership outside the built-in runtime reports.\n\
  Choose it over another tool when: the user asks how data or events move through Hematite.\n\n\
Web research and docs\n\
- `research_web`\n\
  Good for: external technical search when repo context is not enough.\n\
  Bad for: internal code truth.\n\
  Choose it over another tool when: you need current docs, standards, or API changes outside the repo.\n\
  Conditional: only relevant when external information is needed.\n\
- `fetch_docs`\n\
  Good for: reading a specific documentation URL found elsewhere.\n\
  Bad for: discovery.\n\
  Choose it over another tool when: you already have the URL and want readable docs.\n\
  Conditional: usually paired with `research_web`.\n\n\
Vision\n\
- `vision_analyze`\n\
  Good for: screenshots, diagrams, and visual state confirmation.\n\
  Bad for: source-of-truth code tracing.\n\
  Choose it over another tool when: the input is visual rather than textual.\n\
  Conditional: only relevant when an image is available and the vision path is enabled.\n\n\
Shell and context management\n\
- `shell`\n\
  Good for: builds, tests, environment checks, and OS-level read-only inspection.\n\
  Bad for: precise code understanding when built-in file and LSP tools are available.\n\
  Choose it over another tool when: you need runtime verification or information that only the host system can provide.\n\
- `auto_pin_context`\n\
  Good for: keeping 1-3 critical files in active memory during a complex investigation.\n\
  Bad for: discovery by itself.\n\
  Choose it over another tool when: the task spans several important files and you need them held stable.\n\
- `list_pinned`\n\
  Good for: confirming what is pinned right now.\n\
  Bad for: learning anything new about the codebase.\n\
  Choose it over another tool when: you want to inspect or audit the current pinned set.\n\n\
Optional external surface\n\
- `mcp__*` tools\n\
  Good for: optional external capabilities from configured MCP servers.\n\
  Bad for: baseline assumptions about Hematite's built-in tool surface.\n\
  Choose them over another tool when: a configured MCP server is active and directly relevant.\n\
  Conditional: they only exist when MCP servers are configured and loaded.\n\n\
Best Read-Only Toolchain\n\
- Start with `trace_runtime_flow` for runtime wiring questions.\n\
- Use `map_project` only when ownership or structure is still unclear.\n\
- Use `grep_files` for textual discovery, then switch to `read_file` or `inspect_lines` for exact local context.\n\
- Use `lsp_search_symbol`, `lsp_definitions`, `lsp_references`, and `lsp_hover` for semantic confirmation once you know the area.\n\
- Use `shell` only when the answer requires runtime verification or host-state information.\n\
- Use `research_web`, `fetch_docs`, and `vision_analyze` only when the question truly depends on external docs or images."
        .to_string()
}

fn describe_user_turn_plan(question: &str) -> String {
    let label = if question.trim().is_empty() {
        "How does Hematite move a user message from the TUI to the model and back?"
    } else {
        question
    };

    format!(
        "Concrete read-only investigation plan for: {:?}\n\n\
1. `trace_runtime_flow`\n\
   Why first: it is the most authoritative built-in tool for runtime/control-flow questions and already knows the exact Hematite event path categories such as `user_turn`.\n\
   Use: request the `user_turn` report first so you get the verified top-level path before reading source.\n\
2. `read_file`\n\
   Why second: once the runtime trace identifies the owning files, read the specific owners directly instead of guessing from memory.\n\
   Use: inspect `src/main.rs`, `src/ui/tui.rs`, `src/agent/conversation.rs`, and `src/agent/inference.rs` in broad chunks.\n\
3. `inspect_lines`\n\
   Why third: after the broad read, narrow to the exact line windows that contain `run_app`, `run_agent_task`, `ConversationManager::run_turn`, and the relevant `InferenceEvent` handling.\n\
   Use: confirm the exact local flow without rereading unrelated code.\n\
4. `lsp_search_symbol`\n\
   Why fourth: if a specific symbol from the trace needs precise navigation, this is the fastest semantic jump.\n\
   Use: search for symbols like `run_app`, `run_agent_task`, `ConversationManager::run_turn`, `InferenceEvent`, `extract_think_block`, or `strip_think_blocks` only after the trace names them.\n\
5. `lsp_definitions`\n\
   Why fifth: confirm the true definition site when a symbol appears in several places or when the file read is ambiguous.\n\
   Use: anchor the investigation on the exact definition instead of a textual match.\n\
6. `lsp_references`\n\
   Why sixth: expand outward from a confirmed symbol to see who calls it and where the next handoff occurs.\n\
   Use: trace the path from TUI submit code into the agent loop and then into inference handling.\n\
7. `lsp_hover`\n\
   Why seventh: fill semantic gaps quickly without extra reading when a type or event payload is unclear.\n\
   Use: confirm what an enum variant or function signature carries at that point in the flow.\n\
8. `auto_pin_context`\n\
   Why eighth: once the 2-3 core files are obvious, pin them so a longer investigation does not drift.\n\
   Use: pin the owner files after the first pass, not before.\n\
9. `shell`\n\
   Why last and only if needed: runtime verification belongs after source truth, not before it.\n\
   Use: only when you need a build, a health check, or another host-level confirmation that the static code reading cannot provide.\n\n\
Tools I would not start with\n\
- `map_project`: useful for initial orientation, but unnecessary if `trace_runtime_flow` already identifies the owner files.\n\
- `grep_files`: useful for fuzzy discovery, but weaker than `trace_runtime_flow` plus LSP once the target path is a known runtime flow.\n\
- `research_web`, `fetch_docs`, `vision_analyze`: not first-choice tools for this repo-local runtime question.\n\
\nBest Read-Only Toolchain\n\
`trace_runtime_flow` -> `read_file` -> `inspect_lines` -> `lsp_search_symbol` -> `lsp_definitions` / `lsp_references` -> `lsp_hover` -> `auto_pin_context` -> optional `shell`",
        label
    )
}
