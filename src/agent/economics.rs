// ── Session Economics Tracking ───────────────────────────────────────────────

use serde::Serialize;

// ── Per-turn context budget ledger ────────────────────────────────────────────

/// Token cost of a single tool result within a turn.
#[derive(Debug, Clone)]
pub struct ToolCost {
    pub name: String,
    /// Estimated tokens (result_chars / 4).
    pub tokens: usize,
}

/// Per-turn breakdown of context consumed.
/// Populated at turn end and surfaced in the SPECULAR panel.
#[derive(Debug, Clone)]
pub struct TurnBudget {
    /// Actual input tokens charged this turn (precise — from API usage delta).
    pub input_tokens: usize,
    /// Actual output tokens generated this turn (precise — from API usage delta).
    pub output_tokens: usize,
    /// Estimated prior-history tokens (chars / 4) — context already present before this turn.
    pub history_est: usize,
    /// Per-tool result costs (estimated tokens from result length).
    pub tool_costs: Vec<ToolCost>,
    /// Context window fill percentage at turn end.
    pub context_pct: u8,
}

impl TurnBudget {
    /// Compact ledger string for the SPECULAR panel and /budget command.
    pub fn render(&self) -> String {
        let total = self.input_tokens + self.output_tokens;
        let mut parts = Vec::new();

        if self.history_est > 0 {
            parts.push(format!("prior hist ~{}t", self.history_est));
        }
        for tc in &self.tool_costs {
            parts.push(format!("{} ~{}t", tc.name, tc.tokens));
        }
        if self.output_tokens > 0 {
            parts.push(format!("model out {}t", self.output_tokens));
        }

        let breakdown = if parts.is_empty() {
            String::new()
        } else {
            format!("\n  {}", parts.join("  |  "))
        };

        format!(
            "Context budget: +{}t this turn  ({}% ctx){}\n  \
             Tip: large tool results are the most common cause of context pressure.",
            total, self.context_pct, breakdown
        )
    }
}

/// Tracks token usage and tool calls for a session.
pub struct SessionEconomics {
    /// Input tokens accumulated across all calls.
    pub input_tokens: usize,
    /// Output tokens accumulated across all calls.
    pub output_tokens: usize,
    /// List of tool calls with name and success/fail status.
    pub tools_used: Vec<ToolRecord>,
}

impl SessionEconomics {
    /// Create a new empty economics tracker.
    pub fn new() -> Self {
        Self {
            input_tokens: 0,
            output_tokens: 0,
            tools_used: Vec::new(),
        }
    }

    /// Record a tool call.
    pub fn record_tool(&mut self, name: &str, success: bool) {
        self.tools_used.push(ToolRecord {
            name: name.to_string(),
            success,
        });
    }
}

impl Default for SessionEconomics {
    fn default() -> Self {
        Self {
            input_tokens: 0,
            output_tokens: 0,
            tools_used: Vec::new(),
        }
    }
}

/// A record of a tool call.
#[derive(Serialize, Clone, Debug)]
pub struct ToolRecord {
    pub name: String,
    pub success: bool,
}

// ── Pricing constants ─────────────────────────────────────────────────────────

/// Input token price: $0.002 per 1K tokens.
pub const INPUT_PRICE_PER_1K: f64 = 0.002;

/// Output token price: $0.006 per 1K tokens.
pub const OUTPUT_PRICE_PER_1K: f64 = 0.006;

// ── Report generation ────────────────────────────────────────────────────────

impl SessionEconomics {
    /// Calculate simulated cost based on token usage.
    pub fn simulated_cost(&self) -> f64 {
        let input_cost = (self.input_tokens as f64 / 1000.0) * INPUT_PRICE_PER_1K;
        let output_cost = (self.output_tokens as f64 / 1000.0) * OUTPUT_PRICE_PER_1K;
        input_cost + output_cost
    }

    /// Generate a JSON report of the session economics.
    pub fn to_json(&self) -> String {
        use serde_json::json;
        json!({
            "session_economics": {
                "input_tokens": self.input_tokens,
                "output_tokens": self.output_tokens,
                "total_tokens": self.input_tokens + self.output_tokens,
                "tools_used": self.tools_used.iter().map(|t| {
                    json!({
                        "name": t.name,
                        "success": t.success
                    })
                }).collect::<Vec<_>>(),
                "simulated_cost_usd": self.simulated_cost()
            }
        })
        .to_string()
    }
}
