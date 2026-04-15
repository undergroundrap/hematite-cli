use crate::agent::conversation::shell_looks_like_structured_host_inspection;
use crate::agent::inference::ToolCallResponse;
use crate::agent::routing::preferred_host_inspection_topic;
use serde_json::Value;

fn prompt_mentions_specific_repo_path(user_input: &str) -> bool {
    let lower = user_input.to_lowercase();
    lower.contains("src/")
        || lower.contains("cargo.toml")
        || lower.contains("readme.md")
        || lower.contains("memory.md")
        || lower.contains("claude.md")
        || lower.contains(".rs")
        || lower.contains(".py")
        || lower.contains(".ts")
        || lower.contains(".js")
        || lower.contains(".go")
        || lower.contains(".cs")
}

fn is_broad_repo_read_tool(name: &str) -> bool {
    matches!(
        name,
        "read_file"
            | "inspect_lines"
            | "grep_files"
            | "list_files"
            | "auto_pin_context"
            | "lsp_definitions"
            | "lsp_references"
            | "lsp_hover"
            | "lsp_search_symbol"
            | "lsp_get_diagnostics"
    )
}

pub(crate) fn prune_read_only_context_bloat_batch(
    calls: Vec<ToolCallResponse>,
    read_only_mode: bool,
    architecture_overview_mode: bool,
) -> (Vec<ToolCallResponse>, Option<String>) {
    if !read_only_mode || !architecture_overview_mode {
        return (calls, None);
    }

    let mut kept = Vec::new();
    let mut dropped = Vec::new();
    for call in calls {
        if matches!(
            call.function.name.as_str(),
            "auto_pin_context" | "list_pinned"
        ) {
            dropped.push(call.function.name.clone());
        } else {
            kept.push(call);
        }
    }

    if dropped.is_empty() {
        return (kept, None);
    }

    (
        kept,
        Some(format!(
            "Read-only architecture discipline: skipping context-bloat tools in analysis mode (dropped: {}). Use grounded tool output already gathered instead of pinning more files.",
            dropped.join(", ")
        )),
    )
}

fn trace_topic_priority_for_architecture(call: &ToolCallResponse) -> i32 {
    let args: Value = serde_json::from_str(&call.function.arguments).unwrap_or(Value::Null);
    match args.get("topic").and_then(|v| v.as_str()).unwrap_or("") {
        "runtime_subsystems" => 3,
        "user_turn" => 2,
        "startup" => 1,
        _ => 0,
    }
}

pub(crate) fn prune_architecture_trace_batch(
    calls: Vec<ToolCallResponse>,
    architecture_overview_mode: bool,
) -> (Vec<ToolCallResponse>, Option<String>) {
    if !architecture_overview_mode {
        return (calls, None);
    }

    let trace_calls: Vec<_> = calls
        .iter()
        .filter(|call| call.function.name == "trace_runtime_flow")
        .cloned()
        .collect();
    if trace_calls.len() <= 1 {
        return (calls, None);
    }

    let best_trace = trace_calls
        .iter()
        .max_by_key(|call| trace_topic_priority_for_architecture(call))
        .map(|call| call.id.clone());

    let mut kept = Vec::new();
    let mut dropped_topics = Vec::new();
    for call in calls {
        if call.function.name == "trace_runtime_flow" && Some(call.id.clone()) != best_trace {
            let args: Value = serde_json::from_str(&call.function.arguments).unwrap_or(Value::Null);
            let topic = args
                .get("topic")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            dropped_topics.push(topic.to_string());
        } else {
            kept.push(call);
        }
    }

    (
        kept,
        Some(format!(
            "Architecture overview discipline: keeping one runtime trace topic for this batch and dropping extra variants (dropped: {}).",
            dropped_topics.join(", ")
        )),
    )
}

pub(crate) fn prune_authoritative_tool_batch(
    calls: Vec<ToolCallResponse>,
    grounded_trace_mode: bool,
    user_input: &str,
) -> (Vec<ToolCallResponse>, Option<String>) {
    if !grounded_trace_mode || prompt_mentions_specific_repo_path(user_input) {
        return (calls, None);
    }

    let has_trace = calls
        .iter()
        .any(|call| call.function.name == "trace_runtime_flow");
    if !has_trace {
        return (calls, None);
    }

    let mut kept = Vec::new();
    let mut dropped = Vec::new();
    for call in calls {
        if is_broad_repo_read_tool(&call.function.name) {
            dropped.push(call.function.name.clone());
        } else {
            kept.push(call);
        }
    }

    if dropped.is_empty() {
        return (kept, None);
    }

    (
        kept,
        Some(format!(
            "Runtime-trace discipline: preserving `trace_runtime_flow` as the authoritative runtime source and skipping extra repo reads in the same batch (dropped: {}).",
            dropped.join(", ")
        )),
    )
}

pub(crate) fn prune_redirected_shell_batch(
    calls: Vec<ToolCallResponse>,
) -> (Vec<ToolCallResponse>, Option<String>) {
    let mut redirected_topics = std::collections::HashSet::new();
    let mut kept = Vec::new();
    let mut dropped_count = 0;

    for call in calls {
        if call.function.name == "shell" {
            let args: Value = serde_json::from_str(&call.function.arguments).unwrap_or(Value::Null);
            let command = args.get("command").and_then(|v| v.as_str()).unwrap_or("");
            if shell_looks_like_structured_host_inspection(command) {
                let topic = preferred_host_inspection_topic(command).unwrap_or("summary");
                if !redirected_topics.contains(topic) {
                    redirected_topics.insert(topic);
                    kept.push(call);
                } else {
                    dropped_count += 1;
                }
                continue;
            }
        }
        kept.push(call);
    }

    if dropped_count == 0 {
        return (kept, None);
    }

    (
        kept,
        Some(format!(
            "Redirection discipline: pruning redundant auto-redirected diagnostic tool calls in the same batch (dropped: {}).",
            dropped_count
        )),
    )
}

pub(crate) fn summarize_runtime_trace_output(report: &str) -> String {
    let mut lines = Vec::new();
    let mut started = false;
    let mut kept = 0usize;

    for line in report.lines() {
        let trimmed = line.trim_end();
        if trimmed.is_empty() {
            if started && !lines.last().map(|s: &String| s.is_empty()).unwrap_or(false) {
                lines.push(String::new());
            }
            continue;
        }

        if !started {
            if trimmed.starts_with("Verified runtime trace")
                || trimmed.starts_with("Verified runtime subsystems")
                || trimmed.starts_with("Verified startup flow")
            {
                started = true;
                lines.push(trimmed.to_string());
            }
            continue;
        }

        if trimmed == "Possible weak points" {
            break;
        }

        if trimmed.trim_start().starts_with("File refs:") {
            continue;
        }

        lines.push(trimmed.to_string());
        kept += 1;

        if kept >= 24 {
            break;
        }
    }

    lines.join("\n")
}

pub(crate) fn build_architecture_overview_answer(runtime_trace_summary: &str) -> String {
    let mut out = String::new();
    out.push_str("Grounded architecture overview\n\n");
    out.push_str("\n\nRuntime control flow\n");
    out.push_str(runtime_trace_summary.trim());
    out.push_str("\n\nStable workflow contracts\n");
    out.push_str("- Workflow modes live in `src/agent/conversation.rs`: `/ask` is read-only analysis, `/code` allows implementation, `/architect` is plan-first, `/read-only` is hard no-mutation, and `/auto` chooses the narrowest effective path.\n");
    out.push_str("- Reset semantics split across `src/ui/tui.rs` and `src/agent/conversation.rs`: `/clear` is UI-only cleanup, `/new` is fresh task context, and `/forget` is the hard memory purge path.\n");
    out.push_str("- Gemma-native formatting is controlled by the Gemma 4 config/runtime path in `src/agent/config.rs`, `src/agent/inference.rs`, `src/agent/conversation.rs`, and `src/ui/tui.rs`.\n");
    out.push_str("- Prompt budgeting is split between provider preflight in `src/agent/inference.rs` and turn-level trimming/compaction in `src/agent/conversation.rs` plus `src/agent/compaction.rs`.\n");
    out.push_str("- MCP policy and tool routing are enforced in `src/agent/conversation.rs`: ordinary workspace inspection is pushed toward built-in file tools, MCP filesystem reads are blocked by default for local inspection, and tool execution is partitioned into parallel-safe reads vs serialized mutating calls.\n");
    out
}
