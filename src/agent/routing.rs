use super::conversation::WorkflowMode;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum QueryIntentClass {
    ProductTruth,
    RuntimeDiagnosis,
    RepoArchitecture,
    Toolchain,
    Capability,
    Implementation,
    Unknown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum DirectAnswerKind {
    LanguageCapability,
    SessionMemory,
    RecoveryRecipes,
    McpLifecycle,
    AuthorizationPolicy,
    ToolClasses,
    ToolRegistryOwnership,
    SessionResetSemantics,
    ProductSurface,
    ReasoningSplit,
    Identity,
    WorkflowModes,
    GemmaNative,
    GemmaNativeSettings,
    VerifyProfiles,
    Toolchain,
    ArchitectSessionResetPlan,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct QueryIntent {
    pub(crate) primary_class: QueryIntentClass,
    pub(crate) direct_answer: Option<DirectAnswerKind>,
    pub(crate) stabilize_session_reset_inspection: bool,
    pub(crate) stabilize_product_surface_inspection: bool,
    pub(crate) grounded_trace_mode: bool,
    pub(crate) capability_mode: bool,
    pub(crate) capability_needs_repo: bool,
    pub(crate) toolchain_mode: bool,
    pub(crate) preserve_project_map_output: bool,
    pub(crate) architecture_overview_mode: bool,
}

fn contains_any(haystack: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| haystack.contains(needle))
}

fn contains_all(haystack: &str, needles: &[&str]) -> bool {
    needles.iter().all(|needle| haystack.contains(needle))
}

fn mentions_reset_commands(lower: &str) -> bool {
    contains_all(lower, &["/clear", "/new", "/forget"])
}

fn mentions_repo_inspection_request(lower: &str) -> bool {
    contains_any(
        lower,
        &[
            "inspect",
            "repository file tools",
            "then continue",
            "src/agent/conversation.rs",
            "src/ui/tui.rs",
        ],
    )
}

fn mentions_stable_product_surface(lower: &str) -> bool {
    contains_any(
        lower,
        &[
            "stable product-surface question",
            "stable product surface question",
            "stable product-surface questions",
            "stable product surface questions",
        ],
    )
}

fn mentions_product_truth_routing(lower: &str) -> bool {
    let asks_decision_policy = contains_any(
        lower,
        &[
            "how hematite decides",
            "how does hematite decide",
            "decides whether",
            "decide whether",
        ],
    );
    let asks_direct_vs_inspect_split = contains_any(
        lower,
        &[
            "answered as stable product truth",
            "stable product truth",
            "stable product behavior",
            "answer directly",
            "direct answer",
            "inspect the repository",
            "inspect repository",
            "repository implementation",
            "repo implementation",
        ],
    );
    asks_decision_policy && asks_direct_vs_inspect_split
}

fn mentions_capability_question(lower: &str) -> bool {
    contains_any(
        lower,
        &[
            "what can you do",
            "what are you capable",
            "can you make projects",
            "can you build projects",
            "do you know other coding languages",
            "other coding languages",
            "what languages",
            "can you use the internet",
            "internet research capabilities",
            "what tools do you have",
        ],
    )
}

fn capability_question_requires_repo_inspection(lower: &str) -> bool {
    contains_any(
        lower,
        &[
            "this repo",
            "this repository",
            "codebase",
            "which files",
            "implementation",
            "in this project",
        ],
    )
}

pub(crate) fn looks_like_mutation_request(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    [
        "fix ",
        "change ",
        "edit ",
        "modify ",
        "update ",
        "rename ",
        "refactor ",
        "patch ",
        "rewrite ",
        "implement ",
        "create a file",
        "create file",
        "add a file",
        "delete ",
        "remove ",
        "make the change",
    ]
    .iter()
    .any(|needle| lower.contains(needle))
}

pub(crate) fn classify_query_intent(workflow_mode: WorkflowMode, user_input: &str) -> QueryIntent {
    let lower = user_input.to_lowercase();
    let trimmed = user_input.trim().to_ascii_lowercase();

    let mentions_runtime_trace = contains_any(
        &lower,
        &[
            "trace",
            "how does",
            "what are the main runtime subsystems",
            "how does a user message move",
            "separate normal assistant output",
            "session reset behavior",
            "file references",
            "event types",
            "channels",
        ],
    );
    let anti_guess = contains_any(&lower, &["do not guess", "if you are unsure"]);
    let capability_mode = mentions_capability_question(&lower);
    let capability_needs_repo =
        capability_mode && capability_question_requires_repo_inspection(&lower);
    let toolchain_mode = contains_any(
        &lower,
        &[
            "tooling discipline",
            "best read-only toolchain",
            "identify the best tools you actually have",
            "concrete read-only investigation plan",
            "do not execute the plan",
            "available repo-inspection tools",
            "tool choice discipline",
            "what tools would you choose first",
        ],
    ) || (lower.contains("which tools") && lower.contains("why"))
        || (lower.contains("when would you choose") && lower.contains("tool"));
    let preserve_project_map_output = lower.contains("map_project")
        || lower.contains("entrypoint")
        || lower.contains("owner file")
        || lower.contains("owner files")
        || lower.contains("project structure")
        || lower.contains("repository structure")
        || (lower.contains("architecture")
            && (lower.contains("repo") || lower.contains("repository")));
    let architecture_overview_mode = {
        let architecture_signals = contains_any(
            &lower,
            &[
                "architecture walkthrough",
                "full architecture",
                "runtime walkthrough",
                "control flow",
                "tool routing",
                "workflow modes",
                "repo map behavior",
                "mcp policy",
                "prompt budgeting",
                "compaction",
                "file ownership",
                "owner files",
            ],
        );
        let broad = contains_any(
            &lower,
            &["full detailed", "all in one answer", "concrete file ownership"],
        );
        (architecture_signals && broad)
            || (lower.contains("runtime")
                && lower.contains("workflow")
                && (lower.contains("architecture") || lower.contains("tool routing")))
    };

    let direct_answer = if matches!(
        trimmed.as_str(),
        "who are you" | "who are you?" | "what are you" | "what are you?"
    ) || (lower.contains("what is hematite") && !lower.contains("lm studio"))
    {
        Some(DirectAnswerKind::Identity)
    } else if (mentions_stable_product_surface(&lower)
        || mentions_product_truth_routing(&lower))
        && contains_any(
            &lower,
            &[
                "how hematite answers",
                "how does hematite answer",
                "how hematite handles",
                "how does hematite handle",
                "how hematite decides",
                "how does hematite decide",
                "decides whether",
                "decide whether",
            ],
        )
    {
        Some(DirectAnswerKind::ProductSurface)
    } else if mentions_reset_commands(&lower)
        && contains_any(
            &lower,
            &[
                "exact difference",
                "difference between",
                "explain the exact difference",
                "what is the difference",
            ],
        )
    {
        Some(DirectAnswerKind::SessionResetSemantics)
    } else if (lower.contains("reasoning output") || lower.contains("reasoning"))
        && contains_any(&lower, &["visible chat output", "visible chat", "chat output"])
    {
        Some(DirectAnswerKind::ReasoningSplit)
    } else if lower.contains("/ask")
        && lower.contains("/code")
        && lower.contains("/architect")
        && lower.contains("/read-only")
        && lower.contains("/auto")
        && contains_any(&lower, &["difference", "differences", "what are"])
    {
        Some(DirectAnswerKind::WorkflowModes)
    } else if lower.contains(".hematite/settings.json")
        && lower.contains("gemma_native_auto")
        && lower.contains("gemma_native_formatting")
    {
        Some(DirectAnswerKind::GemmaNativeSettings)
    } else if contains_any(&lower, &["/gemma-native", "gemma native"])
        && contains_any(&lower, &["what does", "what is", "how does", "what do"])
    {
        Some(DirectAnswerKind::GemmaNative)
    } else if lower.contains("verify_build")
        && lower.contains(".hematite/settings.json")
        && contains_any(&lower, &["build", "test", "lint", "fix", "verification commands"])
    {
        Some(DirectAnswerKind::VerifyProfiles)
    } else if (lower.contains("carry forward by default")
        || lower.contains("session memory should you carry forward")
        || (lower.contains("carry forward")
            && contains_any(
                &lower,
                &[
                    "besides the active task",
                    "blocker",
                    "compacts",
                    "recovers from a blocker",
                    "session state",
                ],
            )))
        && contains_any(
            &lower,
            &[
                "restarted hematite",
                "restarted",
                "avoid carrying forward",
                "session state",
                "active task",
                "blocker",
                "compacts",
                "recovers from a blocker",
            ],
        )
    {
        Some(DirectAnswerKind::SessionMemory)
    } else if contains_any(
        &lower,
        &[
            "recovery recipe",
            "recovery recipes",
            "recovery step",
            "recovery steps",
        ],
    ) && contains_any(
        &lower,
        &[
            "blocker",
            "runtime failure",
            "degrades",
            "context window",
            "context-window",
            "operator",
        ],
    ) {
        Some(DirectAnswerKind::RecoveryRecipes)
    } else if contains_any(
        &lower,
        &[
            "mcp server health",
            "mcp runtime state",
            "mcp lifecycle",
            "mcp state",
            "mcp healthy",
            "mcp degraded",
            "mcp failed",
        ],
    ) {
        Some(DirectAnswerKind::McpLifecycle)
    } else if contains_any(
        &lower,
        &[
            "allowed, denied, or require approval",
            "allowed denied or require approval",
            "allow, ask, or deny",
            "tool call should be allowed",
            "authorization logic",
            "workspace trust",
            "trust-allowlisted",
        ],
    ) {
        Some(DirectAnswerKind::AuthorizationPolicy)
    } else if contains_any(
        &lower,
        &[
            "tool classes",
            "tool class",
            "flat tool list",
            "runtime tool classes",
            "different runtime tool classes",
        ],
    ) || (lower.contains("repo reads")
        && lower.contains("repo writes")
        && contains_any(
            &lower,
            &[
                "verification tools",
                "git tools",
                "external mcp tools",
                "different runtime",
            ],
        ))
    {
        Some(DirectAnswerKind::ToolClasses)
    } else if contains_any(
        &lower,
        &[
            "built-in tool catalog",
            "builtin tool catalog",
            "builtin-tool dispatch",
            "built-in tool dispatch",
            "tool registry ownership",
            "which file now owns",
        ],
    ) && contains_any(
        &lower,
        &["tool catalog", "dispatch path", "dispatch", "tool registry", "owns"],
    ) {
        Some(DirectAnswerKind::ToolRegistryOwnership)
    } else if (lower.contains("other coding languages")
        || lower.contains("what languages")
        || lower.contains("know other languages"))
        && contains_any(
            &lower,
            &[
                "capable of making projects",
                "can you make projects",
                "can you build projects",
            ],
        )
    {
        Some(DirectAnswerKind::LanguageCapability)
    } else if workflow_mode == WorkflowMode::Architect
        && (lower.contains("session reset")
            || (lower.contains("/clear") && lower.contains("/new") && lower.contains("/forget")))
        && contains_any(&lower, &["redesign", "clearer", "easier", "understand"])
    {
        Some(DirectAnswerKind::ArchitectSessionResetPlan)
    } else if toolchain_mode
        && lower.contains("read-only")
        && contains_any(
            &lower,
            &[
                "tooling discipline",
                "investigation plan",
                "best read-only toolchain",
                "tool choice discipline",
                "what tools would you choose first",
            ],
        )
    {
        Some(DirectAnswerKind::Toolchain)
    } else {
        None
    };

    let primary_class = if direct_answer.is_some()
        || mentions_stable_product_surface(&lower)
        || mentions_product_truth_routing(&lower)
    {
        QueryIntentClass::ProductTruth
    } else if architecture_overview_mode || preserve_project_map_output {
        QueryIntentClass::RepoArchitecture
    } else if toolchain_mode {
        QueryIntentClass::Toolchain
    } else if capability_mode {
        QueryIntentClass::Capability
    } else if mentions_runtime_trace || anti_guess || lower.contains("read-only") {
        QueryIntentClass::RuntimeDiagnosis
    } else if looks_like_mutation_request(user_input) {
        QueryIntentClass::Implementation
    } else {
        QueryIntentClass::Unknown
    };

    QueryIntent {
        primary_class,
        direct_answer,
        stabilize_session_reset_inspection: mentions_reset_commands(&lower)
            && mentions_repo_inspection_request(&lower),
        stabilize_product_surface_inspection: (mentions_stable_product_surface(&lower)
            || mentions_product_truth_routing(&lower))
            && mentions_repo_inspection_request(&lower),
        grounded_trace_mode: mentions_runtime_trace || lower.contains("read-only") || anti_guess,
        capability_mode,
        capability_needs_repo,
        toolchain_mode,
        preserve_project_map_output,
        architecture_overview_mode,
    }
}

pub(crate) fn is_capability_probe_tool(name: &str) -> bool {
    matches!(
        name,
        "map_project"
            | "read_file"
            | "inspect_lines"
            | "list_files"
            | "grep_files"
            | "lsp_definitions"
            | "lsp_references"
            | "lsp_hover"
            | "lsp_search_symbol"
            | "lsp_get_diagnostics"
            | "trace_runtime_flow"
            | "auto_pin_context"
            | "list_pinned"
    )
}
