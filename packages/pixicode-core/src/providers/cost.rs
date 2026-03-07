//! Cost calculation — Token usage cost tracking

use crate::providers::types::{ModelInfo, PricingInfo, Usage};

/// Calculate cost for a given usage and pricing.
pub fn calculate_cost(usage: &Usage, pricing: &PricingInfo) -> f64 {
    let input_cost = (usage.input_tokens as f64 / 1000.0) * pricing.input_per_1k;
    let output_cost = (usage.output_tokens as f64 / 1000.0) * pricing.output_per_1k;
    input_cost + output_cost
}

/// Calculate cost with model info.
pub fn calculate_cost_with_model(usage: &Usage, model: &ModelInfo) -> Option<f64> {
    model.pricing.as_ref().map(|p| calculate_cost(usage, p))
}

/// Cost tracker for a session.
#[derive(Debug, Default)]
pub struct CostTracker {
    total_input_tokens: u32,
    total_output_tokens: u32,
    total_cost: f64,
    requests: Vec<RequestCost>,
}

impl CostTracker {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_request(&mut self, usage: &Usage, pricing: &PricingInfo) {
        let cost = calculate_cost(usage, pricing);
        self.total_input_tokens += usage.input_tokens;
        self.total_output_tokens += usage.output_tokens;
        self.total_cost += cost;
        
        self.requests.push(RequestCost {
            input_tokens: usage.input_tokens,
            output_tokens: usage.output_tokens,
            cost,
        });
    }

    pub fn total_tokens(&self) -> u32 {
        self.total_input_tokens + self.total_output_tokens
    }

    pub fn total_cost(&self) -> f64 {
        self.total_cost
    }

    pub fn request_count(&self) -> usize {
        self.requests.len()
    }

    pub fn average_cost(&self) -> Option<f64> {
        if self.requests.is_empty() {
            None
        } else {
            Some(self.total_cost / self.requests.len() as f64)
        }
    }

    pub fn reset(&mut self) {
        self.total_input_tokens = 0;
        self.total_output_tokens = 0;
        self.total_cost = 0.0;
        self.requests.clear();
    }
}

/// Cost for a single request.
#[derive(Debug, Clone)]
pub struct RequestCost {
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub cost: f64,
}

/// Budget tracker with alerts.
pub struct BudgetTracker {
    budget: f64,
    alert_threshold: f64,
    tracker: CostTracker,
}

impl BudgetTracker {
    pub fn new(budget: f64) -> Self {
        Self {
            budget,
            alert_threshold: 0.8, // Alert at 80% of budget
            tracker: CostTracker::new(),
        }
    }

    pub fn with_alert_threshold(mut self, threshold: f64) -> Self {
        self.alert_threshold = threshold.clamp(0.0, 1.0);
        self
    }

    pub fn add_request(&mut self, usage: &Usage, pricing: &PricingInfo) {
        self.tracker.add_request(usage, pricing);
    }

    pub fn remaining(&self) -> f64 {
        self.budget - self.tracker.total_cost()
    }

    pub fn is_over_budget(&self) -> bool {
        self.tracker.total_cost() >= self.budget
    }

    pub fn should_alert(&self) -> bool {
        let usage_ratio = self.tracker.total_cost() / self.budget;
        usage_ratio >= self.alert_threshold && usage_ratio < 1.0
    }

    pub fn get_tracker(&self) -> &CostTracker {
        &self.tracker
    }
}

/// Known model pricing (USD per 1K tokens).
pub mod pricing {
    use super::PricingInfo;

    pub fn gpt_4() -> PricingInfo {
        PricingInfo {
            input_per_1k: 0.03,
            output_per_1k: 0.06,
        }
    }

    pub fn gpt_4_turbo() -> PricingInfo {
        PricingInfo {
            input_per_1k: 0.01,
            output_per_1k: 0.03,
        }
    }

    pub fn gpt_3_5_turbo() -> PricingInfo {
        PricingInfo {
            input_per_1k: 0.0005,
            output_per_1k: 0.0015,
        }
    }

    pub fn claude_3_opus() -> PricingInfo {
        PricingInfo {
            input_per_1k: 0.015,
            output_per_1k: 0.075,
        }
    }

    pub fn claude_3_sonnet() -> PricingInfo {
        PricingInfo {
            input_per_1k: 0.003,
            output_per_1k: 0.015,
        }
    }

    pub fn claude_3_haiku() -> PricingInfo {
        PricingInfo {
            input_per_1k: 0.00025,
            output_per_1k: 0.00125,
        }
    }

    pub fn gemini_pro() -> PricingInfo {
        PricingInfo {
            input_per_1k: 0.00025,
            output_per_1k: 0.0005,
        }
    }

    pub fn gemini_ultra() -> PricingInfo {
        PricingInfo {
            input_per_1k: 0.007, // Estimated
            output_per_1k: 0.021, // Estimated
        }
    }

    pub fn llama_3_70b() -> PricingInfo {
        PricingInfo {
            input_per_1k: 0.0008, // Groq pricing example
            output_per_1k: 0.0008,
        }
    }

    pub fn free() -> PricingInfo {
        PricingInfo {
            input_per_1k: 0.0,
            output_per_1k: 0.0,
        }
    }
}

/// Get pricing for a known model.
pub fn get_model_pricing(model_id: &str) -> Option<PricingInfo> {
    let model_lower = model_id.to_lowercase();
    
    // OpenAI
    if model_lower.contains("gpt-4-turbo") || model_lower.contains("gpt-4o") {
        return Some(pricing::gpt_4_turbo());
    } else if model_lower.contains("gpt-4") {
        return Some(pricing::gpt_4());
    } else if model_lower.contains("gpt-3.5") {
        return Some(pricing::gpt_3_5_turbo());
    }
    
    // Anthropic
    if model_lower.contains("claude-3-opus") || model_lower.contains("claude-3-5-sonnet") {
        return Some(pricing::claude_3_opus());
    } else if model_lower.contains("claude-3-sonnet") {
        return Some(pricing::claude_3_sonnet());
    } else if model_lower.contains("claude-3-haiku") {
        return Some(pricing::claude_3_haiku());
    }
    
    // Google
    if model_lower.contains("gemini-ultra") || model_lower.contains("gemini-1.5") {
        return Some(pricing::gemini_ultra());
    } else if model_lower.contains("gemini-pro") || model_lower.contains("gemini-1.0") {
        return Some(pricing::gemini_pro());
    }
    
    // Meta/Groq
    if model_lower.contains("llama-3-70b") || model_lower.contains("llama3-70b") {
        return Some(pricing::llama_3_70b());
    }
    
    // Ollama and other local models
    if model_lower.contains("ollama") || model_lower.contains("local") {
        return Some(pricing::free());
    }
    
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_cost() {
        let usage = Usage::new(1000, 500);
        let pricing = pricing::gpt_4();
        let cost = calculate_cost(&usage, &pricing);
        
        // Input: 1000 * 0.03 / 1000 = 0.03
        // Output: 500 * 0.06 / 1000 = 0.03
        // Total: 0.06
        assert!((cost - 0.06).abs() < 0.0001);
    }

    #[test]
    fn test_cost_tracker() {
        let mut tracker = CostTracker::new();
        let usage = Usage::new(1000, 500);
        let pricing = pricing::gpt_4();
        
        tracker.add_request(&usage, &pricing);
        tracker.add_request(&usage, &pricing);
        
        assert_eq!(tracker.total_tokens(), 3000);
        assert!((tracker.total_cost() - 0.12).abs() < 0.0001);
        assert_eq!(tracker.request_count(), 2);
    }

    #[test]
    fn test_budget_tracker() {
        // Budget 0.15 so two requests (0.06 each = 0.12) reach 80% and trigger alert
        let mut tracker = BudgetTracker::new(0.15);
        let usage = Usage::new(1000, 500);
        let pricing = pricing::gpt_4();
        tracker.add_request(&usage, &pricing);
        tracker.add_request(&usage, &pricing);
        assert!((tracker.remaining() - 0.03).abs() < 0.0001);
        assert!(!tracker.is_over_budget());
        assert!(tracker.should_alert()); // >= 80% of budget
    }

    #[test]
    fn test_get_model_pricing() {
        assert!(get_model_pricing("gpt-4").is_some());
        assert!(get_model_pricing("claude-3-sonnet-20240229").is_some());
        assert!(get_model_pricing("ollama/llama3").is_some());
        assert!(get_model_pricing("unknown-model").is_none());
    }
}
