/// Model Pricing tiers (USD per 1 million tokens).
pub struct ModelPricing {
    pub input: f64,
    pub output: f64,
}

/// Returns the pricing tier for a model based on its ID.
/// Benchmark rates based on standard cloud provider tiers.
pub fn get_pricing(model: &str) -> ModelPricing {
    let m = model.to_lowercase();

    // Gemma-4 / Gemini-1.5 Tier ($0.15 / $0.60)
    if m.contains("gemma-4") || m.contains("gemini-1.5") {
        return ModelPricing {
            input: 0.15,
            output: 0.60,
        };
    }

    // Opus / DeepSeek-V3 Tier ($15.00 / $75.00)
    if m.contains("opus") || m.contains("deepseek-v3") || m.contains("r1") {
        return ModelPricing {
            input: 15.0,
            output: 75.0,
        };
    }

    // Sonnet / GPT-4o / Qwen-72B Tier ($3.00 / $15.00)
    if m.contains("sonnet") || m.contains("gpt-4") || m.contains("72b") || m.contains("70b") {
        return ModelPricing {
            input: 3.0,
            output: 15.0,
        };
    }

    // Haiku / GPT-4o-mini / Qwen-7B Tier ($0.25 / $1.25)
    if m.contains("haiku")
        || m.contains("mini")
        || m.contains("7b")
        || m.contains("8b")
        || m.contains("9b")
        || m.contains("12b")
        || m.contains("14b")
    {
        return ModelPricing {
            input: 0.25,
            output: 1.25,
        };
    }

    // Default safe fallback (standard Haiku-like rate)
    ModelPricing {
        input: 0.25,
        output: 1.25,
    }
}

/// Calculates the cost in USD for a given token usage block.
/// Applies a 90% discount for cached input tokens.
pub fn calculate_cost(usage: &crate::agent::inference::TokenUsage, model: &str) -> f64 {
    let p = get_pricing(model);

    let cache_hits = usage.prompt_cache_hit_tokens + usage.cache_read_input_tokens;
    let fresh_input = usage.prompt_tokens.saturating_sub(cache_hits);

    let input_cost = (fresh_input as f64 / 1_000_000.0) * p.input;
    let cache_cost = (cache_hits as f64 / 1_000_000.0) * p.input * 0.10; // 90% discount
    let output_cost = (usage.completion_tokens as f64 / 1_000_000.0) * p.output;

    input_cost + cache_cost + output_cost
}

/// Calculates a rough estimate for non-streamed or partial data.
pub fn calculate_estimated_cost(tokens: usize, model: &str) -> f64 {
    let p = get_pricing(model);
    (tokens as f64 / 1_000_000.0) * p.input
}
