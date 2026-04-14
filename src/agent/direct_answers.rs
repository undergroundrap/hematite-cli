pub(crate) fn build_language_capability_answer() -> String {
    "Hematite itself is written in Rust, but it is not limited to that language. I can help with projects in Python, JavaScript, TypeScript, Go, C#, and other languages.\n\nI can help create projects by scaffolding files and directories, implementing features, editing code precisely, running the appropriate local build or test commands for the target stack, and iterating on the project structure as it grows. The main limits are the local model, the available tooling on this machine, and how much context fits cleanly in session.".to_string()
}

pub(crate) fn build_unsafe_workflow_pressure_answer() -> String {
    "Hematite should not skip verification and commit blindly.\n\nIf you want a real code change, the first requirement is a concrete change target. After edits, Hematite should run the appropriate verification path before committing so it has proof that the tree is still healthy.\n\nThe right workflow is: scope the change, inspect the relevant files, make the edit with evidence, run verification, and only then commit if the result is clean. Pressure to skip verification or jump straight to commit should be treated as a workflow correction point, not as permission to rush past safeguards.".to_string()
}

pub(crate) fn build_session_memory_answer() -> String {
    "By default, Hematite should carry forward lightweight project and task signal, not full conversational residue.\n\nCarry forward: Vein-backed project memory, compact session summary, current task memory, working-set files, explicit pinned context when it is still relevant, and the latest typed session ledger entries such as the most recent checkpoint, blocker, recovery step, verification result, and compaction note.\n\nAvoid carrying forward: full chat history, stale reasoning chains, one-off conversational residue, and transient in-flight state from the previous turn.\n\nFor a local model, the right split is to save the project, active task signal, and recent runtime state, not replay old dialogue unless you explicitly want to continue the same thread.".to_string()
}

pub(crate) fn build_recovery_recipes_answer() -> String {
    "Hematite now treats recovery as typed runtime policy rather than loose prose.\n\nWhen a turn degrades or hits a blocker, it maps the situation to a named recovery scenario and a compact recipe of next steps. Typical recipes include `retry_once` for degraded or empty provider turns, `refresh_runtime_profile -> reduce_prompt_budget -> compact_history -> narrow_request` for context-window failures, `use_builtin_workspace_tools` for blocked MCP workspace reads, and `inspect_target_file` or `inspect_exact_line_window` for proof-before-edit blockers.\n\nThose recovery recipes are surfaced to the operator as compact runtime state, recorded in the session ledger, and kept distinct from the final user-facing error message. The point is to make Hematite's next move explicit instead of hiding it inside ad hoc retry code or long diagnostics.".to_string()
}

pub(crate) fn build_mcp_lifecycle_answer() -> String {
    "Hematite should treat MCP server health as typed runtime state, not just a side effect of tool discovery.\n\nThe useful operator states are: unconfigured when no MCP servers are present, healthy when configured servers connect cleanly, degraded when some servers or tool discovery steps fail but some MCP capacity remains, and failed when MCP is configured but nothing connects. That state should stay compact and operator-facing so MCP health is visible without spamming the main chat.\n\nThe practical point is to keep MCP lifecycle separate from the generic provider path: MCP availability changes which external tools exist this turn, but it should not be confused with LM Studio model health or with ordinary built-in workspace tools.".to_string()
}

pub(crate) fn build_authorization_policy_answer() -> String {
    "Hematite routes authorization through one typed runtime decision: allow, ask, or deny.\n\nThat decision is shaped by several inputs in order: permission mode, workspace trust state, MCP default approval, safe-path write bypasses, shell rules from `.hematite/settings.json`, and shell risk classification. In practice that means a tool call can be denied because the workflow is read-only, asked because the workspace is not trust-allowlisted or the command is risky, or allowed because it is inside a trusted workspace and passes the normal policy checks.\n\nWorkspace trust is part of that policy now. A trusted repo root can continue through normal approval logic, an unknown root can require approval for destructive or external actions, and a denied root can block them outright. The goal is to keep repo safety explicit instead of hiding it inside scattered heuristics.".to_string()
}

pub(crate) fn build_tool_classes_answer() -> String {
    "Hematite does not treat its tools as one flat list because runtime policy depends on what kind of tool is being used.\n\nRepo reads, repo writes, verification tools, git tools, architecture tools, workflow helpers, and external MCP tools carry different metadata for mutability, trust sensitivity, read-only fit, plan fit, and parallel-safe execution. That lets Hematite treat a `read_file` call differently from a `write_file`, a `verify_build`, a `git_push`, or an external `mcp__*` tool even if they all look like just \"tools\" at the model layer.\n\nThe practical benefit is cleaner orchestration: read-only analysis can prefer safe repo-read and architecture tools, current-plan execution can stay scoped to plan-fit tools, destructive or external actions can flow through trust and approval policy, and parallel execution can stay limited to tools that are actually safe to batch. The point is to keep runtime behavior explicit instead of hiding it inside one undifferentiated tool list.".to_string()
}

pub(crate) fn build_tool_registry_ownership_answer() -> String {
    "Hematite's built-in tool catalog and builtin-tool dispatch path now live in `src/agent/tool_registry.rs`.\n\nThat file owns the built-in tool definitions and builtin dispatch path, while `src/agent/conversation.rs` consumes the registry during turn orchestration instead of acting as the primary owner of the tool surface. The point is to keep tool ownership, metadata, and builtin dispatch behind a cleaner runtime boundary instead of leaving more catalog glue in the conversation loop.".to_string()
}

pub(crate) fn build_session_reset_semantics_answer() -> String {
    "`/clear` is the UI-only cleanup path: it clears the visible dialogue buffer, SPECULAR side-panel state, and pending one-shot attachments in the TUI, but it does not run the deeper agent reset path.\n\n`/new` is the fresh-task reset: it clears in-memory chat history, resets session/task state, drops pinned context, wipes task files, and starts a fresh conversation context while leaving longer-lived saved memory available.\n\n`/forget` is the hard memory purge path: it does the `/new` reset and also purges saved memory artifacts plus the Vein index so the next turn starts from a much cleaner slate.\n\nIf the issue is only a pending one-shot file or image, use `/detach` instead of resetting the whole session.\n\nSo the practical split is: `/clear` = visual cleanup, `/detach` = drop pending attachments, `/new` = fresh task context, `/forget` = hard wipe semantics.".to_string()
}

pub(crate) fn build_product_surface_answer() -> String {
    "Hematite answers stable product-surface questions in the conversation loop with direct classifiers before it falls back to the normal model-and-tools path.\n\nFor stable command/config behavior like `/gemma-native`, reset semantics, workflow modes, verify profiles, and session-memory policy, it matches the prompt against dedicated direct-answer gates, returns a prebuilt verified answer, logs it into history, and skips repository inspection entirely.\n\nOnly when the prompt is asking about repository implementation details rather than stable product behavior should Hematite inspect files like `src/agent/conversation.rs` or call other tools. The practical rule is: stable product truth first, repo implementation second.".to_string()
}

pub(crate) fn build_identity_answer() -> String {
    crate::hematite_identity_answer()
}

pub(crate) fn build_about_answer() -> String {
    crate::hematite_about_report()
}

pub(crate) fn build_reasoning_split_answer() -> String {
    "Hematite separates reasoning output from visible chat output so the operator sees a clean final answer while the system can still expose its internal reasoning state separately.\n\nVisible chat output is the user-facing reply that belongs in the main transcript. Reasoning output is routed to the SPECULAR side panel and related internal state so Hematite can show its thought process without polluting the main conversation.\n\nThat separation matters for three reasons: cleaner chat logs, easier debugging of agent behavior, and better control over modes like `/ask`, `/architect`, and read-only analysis where internal thinking should not be confused with the final reply.".to_string()
}

pub(crate) fn build_workflow_modes_answer() -> String {
    "/ask is sticky read-only analysis mode: inspect, explain, and answer without making changes.\n\n/code is sticky implementation mode: Hematite can edit, verify, and carry out coding work with the normal proof-before-action safeguards.\n\n/architect is sticky plan-first mode: inspect the repo, shape the solution, and produce the implementation approach before editing. It should not mutate code unless you explicitly ask to implement.\n\n/read-only is the hard no-mutation workflow: analysis only, no file edits, no mutating shell commands, and no commits.\n\n/teach is teacher/guide mode: Hematite inspects the real machine state first, then delivers a grounded, numbered step-by-step walkthrough for any admin, config, or system task. Hematite does not execute write operations in TEACH mode — it shows you exactly how to do each step yourself. Best for driver installs, Group Policy, firewall rules, SSH keys, WSL setup, service config, registry edits, and other tasks that require manual system-level operations.\n\n/auto returns Hematite to the default behavior where it chooses the narrowest effective path for the request.".to_string()
}

pub(crate) fn build_gemma_native_answer() -> String {
    "`/gemma-native` controls Hematite's Gemma 4 native-formatting mode from inside the TUI.\n\n`/gemma-native auto` restores the default behavior: if the loaded model is Gemma 4, Hematite enables the safer native-formatting path automatically at startup and on new turns.\n\n`/gemma-native on` force-enables that path for Gemma 4, `/gemma-native off` disables it, and `/gemma-native status` reports the current mode.\n\nThis setting matters only for Gemma 4 models. It does not change other model families.".to_string()
}

pub(crate) fn build_gemma_native_settings_answer() -> String {
    "For a Gemma 4 model, `gemma_native_auto` is the default startup behavior and `gemma_native_formatting` is the explicit forced-on override.\n\nIf `gemma_native_auto` is `true` and the loaded model is Gemma 4, Hematite enables the Gemma-native formatting path automatically at startup and on new turns.\n\nIf `gemma_native_formatting` is `true`, Hematite force-enables that path for Gemma 4 even if you are not relying on the automatic mode.\n\nIf both are `false`, Gemma-native formatting stays off. These settings do not activate for non-Gemma-4 models.".to_string()
}

pub(crate) fn build_verify_profiles_answer() -> String {
    "When a project defines verify profiles in `.hematite/settings.json`, `verify_build` should treat those profile commands as the first source of truth.\n\nEach action stays separate: `build` runs the profile's build command, `test` runs the test command, `lint` runs the lint command, and `fix` runs the fix command. `verify_build` should not run all of them at once unless you call those actions separately.\n\nIf you pass an explicit profile, Hematite should use that profile or fail clearly if it does not exist. If the project defines a default profile, Hematite should use it when no explicit profile is given. Only when no profile is configured should Hematite fall back to stack-aware auto-detection.".to_string()
}

pub(crate) fn build_architect_session_reset_plan() -> crate::tools::plan::PlanHandoff {
    crate::tools::plan::PlanHandoff {
        goal: "Redesign Hematite's session reset flow so `/clear`, `/new`, and `/forget` are easy for local-model users to distinguish at a glance.".to_string(),
        target_files: vec![
            "src/ui/tui.rs".to_string(),
            "src/agent/conversation.rs".to_string(),
            "README.md".to_string(),
            "evals/quick_smoke.md".to_string(),
        ],
        ordered_steps: vec![
            "Define one explicit reset contract for each command: `/clear` = UI-only cleanup, `/new` = fresh task context, `/forget` = hard memory purge.".to_string(),
            "Centralize the user-facing reset copy so the TUI and agent loop cannot drift on wording or intent.".to_string(),
            "Keep `/clear` visibly local to the TUI, and keep `/new` and `/forget` as agent-path resets with clearly different confirmation text.".to_string(),
            "Update help text and docs so operators can tell the difference without tracing the code.".to_string(),
            "Add or refresh eval coverage for exact reset semantics so future prompt or tool changes do not blur the three commands again.".to_string(),
        ],
        verification: "Run `trace_runtime_flow(topic: \"session_reset\", command: \"all\")` after the redesign and confirm the documented behavior still matches the real code path. Then rerun the session-reset smoke prompt.".to_string(),
        risks: vec![
            "Reset behavior is split across `src/ui/tui.rs` and `src/agent/conversation.rs`, so semantics can drift if only one side is updated.".to_string(),
            "Changing `/clear` too aggressively could accidentally erase more state than a user expects from a visual cleanup command.".to_string(),
            "If the confirmation strings stay too similar, local models and users will keep conflating `/new` with `/forget`.".to_string(),
        ],
        open_questions: vec![
            "Should `/new` preserve pinned context or always drop it?".to_string(),
            "Should the TUI expose a one-line reset legend in `/help` or the footer so users do not need to memorize the semantics?".to_string(),
        ],
    }
}
