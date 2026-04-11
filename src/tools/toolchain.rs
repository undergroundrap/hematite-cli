use serde_json::Value;

pub async fn describe_toolchain(args: &Value) -> Result<String, String> {
    let topic = args.get("topic").and_then(|v| v.as_str()).unwrap_or("all");
    let question =
        normalize_question_label(args.get("question").and_then(|v| v.as_str()).unwrap_or(""));

    match topic {
        "read_only_codebase" => Ok(describe_read_only_codebase_tools()),
        "user_turn_plan" => Ok(describe_user_turn_plan(question)),
        "voice_latency_plan" => Ok(describe_voice_latency_plan(question)),
        "host_inspection_plan" => Ok(describe_host_inspection_plan(question)),
        "all" => Ok(format!(
            "{}\n\n{}",
            describe_read_only_codebase_tools(),
            describe_best_plan_for_question(question)
        )),
        other => Err(format!(
            "Unknown topic '{}'. Use one of: read_only_codebase, user_turn_plan, voice_latency_plan, host_inspection_plan, all.",
            other
        )),
    }
}

fn describe_best_plan_for_question(question: &str) -> String {
    if is_voice_latency_question(question) {
        describe_voice_latency_plan(question)
    } else if is_host_inspection_question(question) {
        describe_host_inspection_plan(question)
    } else {
        describe_user_turn_plan(question)
    }
}

fn is_voice_latency_question(question: &str) -> bool {
    let lower = question.to_lowercase();
    (lower.contains("voice output") || lower.contains("voice"))
        && (lower.contains("lag")
            || lower.contains("behind visible text")
            || lower.contains("latency"))
}

fn is_host_inspection_question(question: &str) -> bool {
    let lower = question.to_lowercase();
    let host_terms = [
        "path",
        "package manager",
        "package managers",
        "environment",
        "env doctor",
        "network",
        "adapter",
        "dns",
        "gateway",
        "ip address",
        "service",
        "services",
        "daemon",
        "startup type",
        "desktop",
        "downloads",
        "toolchain",
        "installed",
        "version",
        "directory",
        "folder",
        "computer",
        "machine",
        "port",
        "process",
        "environment",
    ];
    host_terms.iter().any(|needle| lower.contains(needle))
}

fn normalize_question_label(question: &str) -> &str {
    let trimmed = question.trim();
    if trimmed.is_empty() {
        return trimmed;
    }

    if let Some(idx) = trimmed.find("Question:") {
        let after = trimmed[idx + "Question:".len()..].trim();
        if !after.is_empty() {
            let requirement_markers = [
                "Requirements:",
                "Requirement:",
                "Initial Investigation Order",
            ];
            let mut end = after.len();
            for marker in requirement_markers {
                if let Some(marker_idx) = after.find(marker) {
                    end = end.min(marker_idx);
                }
            }
            return after[..end].trim();
        }
    }

    trimmed
}

fn describe_read_only_codebase_tools() -> String {
    "Verified Hematite read-only toolchain\n\n\
Text search and file inspection\n\
- `map_project`\n\
  Good for: first-pass spatial awareness of the repository layout, likely entrypoints, core owner files, and a small set of extracted top symbols.\n\
  Bad for: exact control flow, full call graphs, or precise line-level inspection.\n\
  Choose it over another tool when: you need a compact architecture map before diving into files or LSP.\n\
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
- `inspect_host`\n\
  Good for: structured read-only inspection of the current machine such as common developer tool versions, PATH analysis, environment/package-manager health, network snapshots, service snapshots, process snapshots, desktop items, Downloads summaries, listening ports, repo-doctor checks, and arbitrary directory or disk-size reports.\n\
  Bad for: custom build commands, arbitrary process control, or any mutation.\n\
  Choose it over another tool when: the user is asking about the host machine rather than repo internals and the question fits one of its built-in topics.\n\
- `shell`\n\
  Good for: builds, tests, environment checks, and OS-level read-only inspection.\n\
  Bad for: precise code understanding when built-in file and LSP tools are available.\n\
  Choose it over another tool when: you need runtime verification, a custom command, or host information that `inspect_host` cannot answer directly.\n\
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
- Use `inspect_host` before `shell` for read-only questions about PATH, installed tools, environment/package-manager health, network state, service state, running processes, desktop items, Downloads size, listening ports, repo-health summaries, or directory/disk summaries.\n\
- If `env_doctor` answers a PATH/package-manager sanity question, stop there unless the user explicitly asks for the raw PATH list.\n\
- Use `shell` only when the answer requires runtime verification or host-state information beyond `inspect_host`.\n\
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

fn describe_voice_latency_plan(question: &str) -> String {
    let label = if question.trim().is_empty() {
        "If I needed to understand why Hematite's voice output can lag behind visible text, what tools would I choose first, in order, and why?"
    } else {
        question
    };

    format!(
        "Concrete read-only investigation plan for: {:?}\n\n\
1. `trace_runtime_flow`\n\
   Why first: it is the only authoritative built-in runtime/control-flow report, and it already covers the visible text path and the voice path inside a normal `user_turn` trace.\n\
   Use: request the `user_turn` report first so you can see where visible `InferenceEvent::Token` handling and `app.voice_manager.speak(...)` diverge.\n\
2. `read_file`\n\
   Why second: once the high-level flow is confirmed, read the owner files directly instead of inventing helper layers.\n\
   Use: inspect `src/ui/tui.rs` for `InferenceEvent::Token`, `InferenceEvent::MutedToken`, and `InferenceEvent::Done` handling, then inspect `src/ui/voice.rs` for `VoiceManager::new`, `VoiceManager::speak`, and `VoiceManager::flush`.\n\
3. `inspect_lines`\n\
   Why third: narrow to the exact windows where visible text is appended and where voice work is queued or flushed.\n\
   Use: inspect the token-handling block in `src/ui/tui.rs` and the queueing / synthesis blocks in `src/ui/voice.rs` without rereading the full files.\n\
4. `lsp_search_symbol`\n\
   Why fourth: if you need precise navigation after the first file read, this is the fastest semantic jump.\n\
   Use: search for `VoiceManager`, `VoiceManager::speak`, `VoiceManager::flush`, and `run_app`.\n\
5. `lsp_references`\n\
   Why fifth: confirm every place where the TUI calls into the voice path and where the relevant voice methods are used.\n\
   Use: trace who calls `VoiceManager::speak` and `VoiceManager::flush` to see whether lag is created before queueing, during streaming, or at turn finalization.\n\
6. `lsp_hover`\n\
   Why sixth: quickly confirm type signatures and payload details for `InferenceEvent` handling and voice methods without extra full-file reading.\n\
   Use: inspect the event variants and the `VoiceManager` method surfaces when the control-flow meaning is still unclear.\n\
7. `lsp_definitions`\n\
   Why seventh: anchor the final understanding on the true definition sites if a search result or reference set is ambiguous.\n\
   Use: confirm exact definition coordinates for `VoiceManager` methods and the relevant `InferenceEvent` enum variants.\n\
8. `shell`\n\
   Why last and only if needed: shell is for runtime verification after the source investigation, not before it.\n\
   Use: only if you need to confirm host-level load or reproduce the lag under observation after the static code path is understood.\n\n\
Built-in authoritative tool note\n\
- `trace_runtime_flow` is authoritative for part of this question because it already describes the visible chat path and the voice path inside a `user_turn` trace.\n\
- It is not sufficient by itself to explain why lag happens inside `VoiceManager`, so the next step is direct file reading in `src/ui/tui.rs` and `src/ui/voice.rs`.\n\n\
Tools I would not start with\n\
- `mcp__*` tools: optional external surface, not the baseline for this built-in voice investigation.\n\
- `research_web`, `fetch_docs`, `vision_analyze`: not first-choice tools for a repo-local voice-latency question.\n\
- `map_project`: useful if ownership were unclear, but unnecessary here because the runtime trace and symbol names already point to the likely owners.\n\
\nInitial Investigation Order\n\
`trace_runtime_flow` -> `read_file` -> `inspect_lines` -> `lsp_search_symbol` -> `lsp_references` -> `lsp_hover` -> `lsp_definitions` -> optional `shell`",
        label
    )
}

fn describe_host_inspection_plan(question: &str) -> String {
    let label = if question.trim().is_empty() {
        "What is the best read-only tool order for checking my machine state, installed tools, PATH, environment/package-manager health, network adapters, services, desktop items, or folder sizes?"
    } else {
        question
    };

    format!(
        "Concrete read-only investigation plan for: {:?}\n\n\
1. `inspect_host`\n\
   Why first: it is the built-in structured host-inspection tool, so it can answer common machine-state questions without forcing the model to invent shell commands.\n\
   Use: start with the closest topic such as `summary`, `toolchains`, `path`, `env_doctor`, `network`, `services`, `processes`, `desktop`, `downloads`, `ports`, `repo_doctor`, `directory`, or `disk`.\n\
2. `shell`\n\
   Why second and only if needed: shell is still the fallback for custom host checks that go beyond `inspect_host`, but it should not be the first move for routine read-only inspection.\n\
   Use: confirm a special case, run a project-specific command, or inspect host state that has no structured built-in topic yet.\n\
3. `read_file` / `list_files`\n\
   Why third and conditional: if the question shifts from host state back into the workspace, move to file tools instead of staying in shell.\n\
   Use: inspect repo files, logs, or config once the machine-level question identifies the relevant path.\n\n\
Tools I would not start with\n\
- `grep_files`: useful for repo text search, but not the right first tool for PATH or desktop questions.\n\
- `trace_runtime_flow`: useful for Hematite runtime architecture, not machine-state inspection.\n\
- `research_web`, `fetch_docs`, `vision_analyze`: only relevant if the question expands beyond the local machine.\n\n\
Initial Investigation Order\n\
`inspect_host` -> optional `shell` -> optional repo/file tools",
        label
    )
}
