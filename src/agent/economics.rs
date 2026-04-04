// ── Session Economics Tracking ───────────────────────────────────────────────

use serde::Serialize;

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
        }).to_string()
    }
}
