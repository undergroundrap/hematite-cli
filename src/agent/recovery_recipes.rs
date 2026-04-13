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
    PolicyCorrection,
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
            RecoveryScenario::PolicyCorrection => "policy_correction",
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
    SelfCorrectToolSelection,
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
            RecoveryStep::SelfCorrectToolSelection => "self_correct_tool_selection",
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
    /// Total transient provider retries consumed this turn across all inference calls.
    transient_retries_this_turn: u32,
}

/// Maximum transient provider retries allowed across an entire multi-step turn.
const MAX_TRANSIENT_RETRIES_PER_TURN: u32 = 3;

impl RecoveryContext {
    pub fn clear(&mut self) {
        self.attempts.clear();
        self.transient_retries_this_turn = 0;
    }

    pub fn attempt_count(&self, scenario: RecoveryScenario) -> u32 {
        self.attempts.get(&scenario).copied().unwrap_or(0)
    }

    /// Returns true and increments the turn-level transient retry budget if a retry
    /// is still available. Returns false when the budget is exhausted.
    pub fn consume_transient_retry(&mut self) -> bool {
        if self.transient_retries_this_turn < MAX_TRANSIENT_RETRIES_PER_TURN {
            self.transient_retries_this_turn += 1;
            // Reset the per-scenario counter so attempt_recovery allows the attempt.
            self.attempts.remove(&RecoveryScenario::ProviderDegraded);
            self.attempts.remove(&RecoveryScenario::EmptyModelResponse);
            true
        } else {
            false
        }
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
            steps: vec![
                RecoveryStep::StopRepeatingToolPattern,
                RecoveryStep::NarrowRequest,
            ],
            max_attempts: 1,
        },
        RecoveryScenario::VerificationFailed => RecoveryRecipe {
            scenario,
            steps: vec![RecoveryStep::FixVerificationFailure],
            max_attempts: 1,
        },
        RecoveryScenario::PolicyCorrection => RecoveryRecipe {
            scenario,
            steps: vec![RecoveryStep::SelfCorrectToolSelection],
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

pub fn attempt_recovery(scenario: RecoveryScenario, ctx: &mut RecoveryContext) -> RecoveryDecision {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn context_window_recipe_matches_expected_local_recovery_flow() {
        let recipe = recipe_for(RecoveryScenario::ContextWindow);
        assert_eq!(recipe.max_attempts, 1);
        assert_eq!(
            recipe.steps,
            vec![
                RecoveryStep::RefreshRuntimeProfile,
                RecoveryStep::ReducePromptBudget,
                RecoveryStep::CompactHistory,
                RecoveryStep::NarrowRequest,
            ]
        );
        assert_eq!(
            recipe.steps_summary(),
            "refresh_runtime_profile -> reduce_prompt_budget -> compact_history -> narrow_request"
        );
    }

    #[test]
    fn provider_degraded_attempts_once_then_escalates() {
        let mut ctx = RecoveryContext::default();

        let first = attempt_recovery(RecoveryScenario::ProviderDegraded, &mut ctx);
        match first {
            RecoveryDecision::Attempt(plan) => {
                assert_eq!(plan.recipe.scenario, RecoveryScenario::ProviderDegraded);
                assert_eq!(plan.next_attempt, 1);
            }
            other => panic!("expected attempt, got {:?}", other),
        }

        let second = attempt_recovery(RecoveryScenario::ProviderDegraded, &mut ctx);
        match second {
            RecoveryDecision::Escalate {
                recipe,
                attempts_made,
                reason,
            } => {
                assert_eq!(recipe.scenario, RecoveryScenario::ProviderDegraded);
                assert_eq!(attempts_made, 1);
                assert!(reason.contains("max recovery attempts"));
            }
            other => panic!("expected escalate, got {:?}", other),
        }
    }

    #[test]
    fn tool_loop_recipe_stops_repetition_before_narrowing() {
        let recipe = recipe_for(RecoveryScenario::ToolLoop);
        assert_eq!(
            recipe.steps,
            vec![
                RecoveryStep::StopRepeatingToolPattern,
                RecoveryStep::NarrowRequest,
            ]
        );
    }
}
