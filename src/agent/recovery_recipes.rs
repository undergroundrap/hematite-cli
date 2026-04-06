use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecoveryScenario {
    ProviderDegraded,
    EmptyModelResponse,
    ContextWindow,
    PromptBudgetPressure,
    HistoryPressure,
    McpWorkspaceReadBlocked,
    CurrentPlanScopeBlocked,
    RecentFileEvidenceMissing,
    ExactLineWindowRequired,
    ToolLoop,
    VerificationFailed,
}

impl RecoveryScenario {
    pub fn label(self) -> &'static str {
        match self {
            RecoveryScenario::ProviderDegraded => "provider_degraded",
            RecoveryScenario::EmptyModelResponse => "empty_model_response",
            RecoveryScenario::ContextWindow => "context_window",
            RecoveryScenario::PromptBudgetPressure => "prompt_budget_pressure",
            RecoveryScenario::HistoryPressure => "history_pressure",
            RecoveryScenario::McpWorkspaceReadBlocked => "mcp_workspace_read_blocked",
            RecoveryScenario::CurrentPlanScopeBlocked => "current_plan_scope_blocked",
            RecoveryScenario::RecentFileEvidenceMissing => "recent_file_evidence_missing",
            RecoveryScenario::ExactLineWindowRequired => "exact_line_window_required",
            RecoveryScenario::ToolLoop => "tool_loop",
            RecoveryScenario::VerificationFailed => "verification_failed",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecoveryStep {
    RetryOnce,
    RefreshRuntimeProfile,
    ReducePromptBudget,
    CompactHistory,
    NarrowRequest,
    UseBuiltinWorkspaceTools,
    StayOnPlannedFiles,
    InspectTargetFile,
    InspectExactLineWindow,
    StopRepeatingToolPattern,
    FixVerificationFailure,
}

impl RecoveryStep {
    pub fn label(self) -> &'static str {
        match self {
            RecoveryStep::RetryOnce => "retry_once",
            RecoveryStep::RefreshRuntimeProfile => "refresh_runtime_profile",
            RecoveryStep::ReducePromptBudget => "reduce_prompt_budget",
            RecoveryStep::CompactHistory => "compact_history",
            RecoveryStep::NarrowRequest => "narrow_request",
            RecoveryStep::UseBuiltinWorkspaceTools => "use_builtin_workspace_tools",
            RecoveryStep::StayOnPlannedFiles => "stay_on_planned_files",
            RecoveryStep::InspectTargetFile => "inspect_target_file",
            RecoveryStep::InspectExactLineWindow => "inspect_exact_line_window",
            RecoveryStep::StopRepeatingToolPattern => "stop_repeating_tool_pattern",
            RecoveryStep::FixVerificationFailure => "fix_verification_failure",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct RecoveryRecipe {
    pub scenario: RecoveryScenario,
    pub steps: Vec<RecoveryStep>,
    pub max_attempts: u32,
}

impl RecoveryRecipe {
    pub fn steps_summary(&self) -> String {
        self.steps
            .iter()
            .map(|step| step.label())
            .collect::<Vec<_>>()
            .join(" -> ")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecoveryPlan {
    pub recipe: RecoveryRecipe,
    pub next_attempt: u32,
}

impl RecoveryPlan {
    pub fn summary(&self) -> String {
        format!(
            "{} [{}/{}]: {}",
            self.recipe.scenario.label(),
            self.next_attempt,
            self.recipe.max_attempts.max(1),
            self.recipe.steps_summary()
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecoveryDecision {
    Attempt(RecoveryPlan),
    Escalate {
        recipe: RecoveryRecipe,
        attempts_made: u32,
        reason: String,
    },
}

impl RecoveryDecision {
    pub fn summary(&self) -> String {
        match self {
            RecoveryDecision::Attempt(plan) => format!("attempt {}", plan.summary()),
            RecoveryDecision::Escalate {
                recipe,
                attempts_made,
                reason,
        } => format!(
                "escalate {} after {}/{}: {} ({})",
                recipe.scenario.label(),
                attempts_made,
                recipe.max_attempts.max(1),
                recipe.steps_summary(),
                reason
            ),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct RecoveryContext {
    attempts: HashMap<RecoveryScenario, u32>,
}

impl RecoveryContext {
    pub fn clear(&mut self) {
        self.attempts.clear();
    }

    pub fn attempt_count(&self, scenario: RecoveryScenario) -> u32 {
        self.attempts.get(&scenario).copied().unwrap_or(0)
    }
}

pub fn recipe_for(scenario: RecoveryScenario) -> RecoveryRecipe {
    match scenario {
        RecoveryScenario::ProviderDegraded => RecoveryRecipe {
            scenario,
            steps: vec![RecoveryStep::RetryOnce],
            max_attempts: 1,
        },
        RecoveryScenario::EmptyModelResponse => RecoveryRecipe {
            scenario,
            steps: vec![RecoveryStep::RetryOnce],
            max_attempts: 1,
        },
        RecoveryScenario::ContextWindow => RecoveryRecipe {
            scenario,
            steps: vec![
                RecoveryStep::RefreshRuntimeProfile,
                RecoveryStep::ReducePromptBudget,
                RecoveryStep::CompactHistory,
                RecoveryStep::NarrowRequest,
            ],
            max_attempts: 1,
        },
        RecoveryScenario::PromptBudgetPressure => RecoveryRecipe {
            scenario,
            steps: vec![RecoveryStep::ReducePromptBudget],
            max_attempts: 1,
        },
        RecoveryScenario::HistoryPressure => RecoveryRecipe {
            scenario,
            steps: vec![RecoveryStep::CompactHistory],
            max_attempts: 1,
        },
        RecoveryScenario::McpWorkspaceReadBlocked => RecoveryRecipe {
            scenario,
            steps: vec![RecoveryStep::UseBuiltinWorkspaceTools],
            max_attempts: 1,
        },
        RecoveryScenario::CurrentPlanScopeBlocked => RecoveryRecipe {
            scenario,
            steps: vec![RecoveryStep::StayOnPlannedFiles],
            max_attempts: 1,
        },
        RecoveryScenario::RecentFileEvidenceMissing => RecoveryRecipe {
            scenario,
            steps: vec![RecoveryStep::InspectTargetFile],
            max_attempts: 1,
        },
        RecoveryScenario::ExactLineWindowRequired => RecoveryRecipe {
            scenario,
            steps: vec![RecoveryStep::InspectExactLineWindow],
            max_attempts: 1,
        },
        RecoveryScenario::ToolLoop => RecoveryRecipe {
            scenario,
            steps: vec![RecoveryStep::StopRepeatingToolPattern, RecoveryStep::NarrowRequest],
            max_attempts: 1,
        },
        RecoveryScenario::VerificationFailed => RecoveryRecipe {
            scenario,
            steps: vec![RecoveryStep::FixVerificationFailure],
            max_attempts: 1,
        },
    }
}

pub fn plan_recovery(scenario: RecoveryScenario, ctx: &RecoveryContext) -> RecoveryPlan {
    let recipe = recipe_for(scenario);
    RecoveryPlan {
        recipe,
        next_attempt: ctx.attempt_count(scenario).saturating_add(1),
    }
}

pub fn preview_recovery_decision(
    scenario: RecoveryScenario,
    ctx: &RecoveryContext,
) -> RecoveryDecision {
    let recipe = recipe_for(scenario);
    let attempts = ctx.attempt_count(scenario);
    if attempts >= recipe.max_attempts {
        let max_attempts = recipe.max_attempts.max(1);
        RecoveryDecision::Escalate {
            recipe,
            attempts_made: attempts,
            reason: format!("max recovery attempts ({}) exhausted", max_attempts),
        }
    } else {
        RecoveryDecision::Attempt(RecoveryPlan {
            recipe,
            next_attempt: attempts.saturating_add(1),
        })
    }
}

pub fn attempt_recovery(
    scenario: RecoveryScenario,
    ctx: &mut RecoveryContext,
) -> RecoveryDecision {
    match preview_recovery_decision(scenario, ctx) {
        RecoveryDecision::Attempt(plan) => {
            ctx.attempts.insert(scenario, plan.next_attempt);
            RecoveryDecision::Attempt(plan)
        }
        RecoveryDecision::Escalate {
            recipe,
            attempts_made,
            reason,
        } => RecoveryDecision::Escalate {
            recipe,
            attempts_made,
            reason,
        },
    }
}
