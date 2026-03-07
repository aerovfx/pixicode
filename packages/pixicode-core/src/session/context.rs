//! Context Manager — Context window management and token budget calculation

use crate::session::types::{Session, Message, MessageRole, ContextConfig, CompactionStrategy};

/// Context manager for handling context window limits.
pub struct ContextManager {
    pub config: ContextConfig,
    pub strategy: CompactionStrategy,
}

impl ContextManager {
    pub fn new(config: ContextConfig) -> Self {
        Self {
            config,
            strategy: CompactionStrategy::default(),
        }
    }

    pub fn with_strategy(mut self, strategy: CompactionStrategy) -> Self {
        self.strategy = strategy;
        self
    }

    /// Get messages that fit within the context window.
    pub fn build_context(&self, session: &Session) -> Vec<Message> {
        let available = self.config.available_tokens();
        
        // Always include system prompt
        let system_tokens = self.estimate_tokens(&session.system_prompt.clone().unwrap_or_default());
        
        // Calculate remaining tokens for messages
        let remaining = if system_tokens > 0 {
            (available as f32 * (1.0 - self.config.system_priority)) as u32
        } else {
            available
        };

        // Select messages based on strategy
        match self.strategy {
            CompactionStrategy::DropOldest => self.select_drop_oldest(&session.messages, remaining),
            CompactionStrategy::Summarize => self.select_with_summarize(&session.messages, remaining),
            CompactionStrategy::RecentOnly => self.select_recent(&session.messages, remaining),
            CompactionStrategy::Smart => self.select_smart(&session.messages, remaining),
        }
    }

    /// Estimate token count for text.
    pub fn estimate_tokens(&self, text: &str) -> u32 {
        // Simple estimation: ~4 characters per token for English
        // In production, use tiktoken or similar
        (text.len() / 4) as u32
    }

    /// Estimate tokens for a message.
    pub fn estimate_message_tokens(&self, message: &Message) -> u32 {
        let content = message.content();
        let base = self.estimate_tokens(&content);
        
        // Add overhead for message structure
        base + 4
    }

    /// Drop oldest messages until we fit.
    fn select_drop_oldest(&self, messages: &[Message], max_tokens: u32) -> Vec<Message> {
        let mut selected = Vec::new();
        let mut total = 0u32;

        // Iterate from newest to oldest
        for msg in messages.iter().rev() {
            let tokens = self.estimate_message_tokens(msg);
            if total + tokens <= max_tokens {
                selected.push(msg.clone());
                total += tokens;
            } else {
                break;
            }
        }

        selected.reverse();
        selected
    }

    /// Keep only recent messages.
    fn select_recent(&self, messages: &[Message], max_tokens: u32) -> Vec<Message> {
        let mut selected = Vec::new();
        let mut total = 0u32;

        // Keep most recent messages
        for msg in messages.iter().rev() {
            let tokens = self.estimate_message_tokens(msg);
            if total + tokens <= max_tokens {
                selected.push(msg.clone());
                total += tokens;
            }
        }

        selected.reverse();
        selected
    }

    /// Smart selection based on importance.
    fn select_smart(&self, messages: &[Message], max_tokens: u32) -> Vec<Message> {
        // Score messages by importance
        let mut scored: Vec<(usize, u32, Message)> = messages.iter().enumerate().map(|(i, m)| {
            let importance = self.score_message_importance(m, i, messages.len());
            let tokens = self.estimate_message_tokens(m);
            (i, importance, m.clone())
        }).collect();

        // Sort by importance (descending)
        scored.sort_by(|a, b| b.1.cmp(&a.1));

        // Select messages by importance until we hit limit
        let mut selected = Vec::new();
        let mut total = 0u32;

        for (_, _, msg) in scored {
            let tokens = self.estimate_message_tokens(&msg);
            if total + tokens <= max_tokens {
                selected.push(msg);
                total += tokens;
            }
        }

        // Restore original order
        selected.sort_by_key(|m| {
            messages.iter().position(|orig| orig.id == m.id).unwrap_or(0)
        });

        selected
    }

    /// Select with summarization (placeholder).
    fn select_with_summarize(&self, messages: &[Message], max_tokens: u32) -> Vec<Message> {
        // In production, this would summarize old conversations
        // For now, fall back to drop_oldest
        self.select_drop_oldest(messages, max_tokens)
    }

    /// Score message importance (0-100).
    fn score_message_importance(&self, message: &Message, index: usize, total: usize) -> u32 {
        let mut score = 50u32;

        // Recent messages are more important
        let recency = index as f32 / total as f32;
        score += (recency * 30.0) as u32;

        // User questions might be important
        if message.role == MessageRole::User {
            let content = message.content();
            if content.contains('?') || content.contains("how") || content.contains("what") {
                score += 10;
            }
        }

        // Assistant responses with code might be important
        if message.role == MessageRole::Assistant {
            let content = message.content();
            if content.contains("```") || content.contains("fn ") || content.contains("class ") {
                score += 10;
            }
        }

        score.min(100)
    }

    /// Check if context needs compaction.
    pub fn needs_compaction(&self, session: &Session) -> bool {
        let total_tokens: u32 = session.messages.iter()
            .map(|m| self.estimate_message_tokens(m))
            .sum();
        
        let system_tokens = self.estimate_tokens(&session.system_prompt.clone().unwrap_or_default());
        
        total_tokens + system_tokens > self.config.available_tokens()
    }

    /// Compact session messages.
    pub fn compact(&self, session: &mut Session) -> u32 {
        let original_count = session.messages.len();
        let compacted = self.build_context(session);
        let removed = original_count - compacted.len();
        
        session.messages = compacted;
        removed as u32
    }

    /// Calculate token budget usage.
    pub fn budget_usage(&self, session: &Session) -> TokenBudget {
        let system_tokens = self.estimate_tokens(&session.system_prompt.clone().unwrap_or_default());
        let message_tokens: u32 = session.messages.iter()
            .map(|m| self.estimate_message_tokens(m))
            .sum();
        
        let total_used = system_tokens + message_tokens;
        let available = self.config.available_tokens();

        TokenBudget {
            total_tokens: self.config.max_tokens,
            reserved_tokens: self.config.reserve_tokens,
            available_tokens: available,
            used_tokens: total_used,
            remaining_tokens: available.saturating_sub(total_used),
            usage_percent: (total_used as f32 / available as f32 * 100.0).min(100.0),
        }
    }
}

/// Token budget information.
#[derive(Debug, Clone)]
pub struct TokenBudget {
    pub total_tokens: u32,
    pub reserved_tokens: u32,
    pub available_tokens: u32,
    pub used_tokens: u32,
    pub remaining_tokens: u32,
    pub usage_percent: f32,
}

impl TokenBudget {
    pub fn is_near_limit(&self) -> bool {
        self.usage_percent >= 80.0
    }

    pub fn is_over_limit(&self) -> bool {
        self.remaining_tokens == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::types::Message;

    #[test]
    fn test_context_manager() {
        let config = ContextConfig {
            max_tokens: 8000,
            reserve_tokens: 1000,
            ..Default::default()
        };
        let manager = ContextManager::new(config);

        let mut session = Session::new("test".to_string(), "gpt-4".to_string());
        session.add_message(Message::user("Hello".to_string()));
        session.add_message(Message::assistant("Hi there!".to_string()));

        let context = manager.build_context(&session);
        assert_eq!(context.len(), 2);
    }

    #[test]
    fn test_token_estimation() {
        let manager = ContextManager::default();
        let tokens = manager.estimate_tokens("Hello, this is a test message!");
        assert!(tokens > 0);
    }

    #[test]
    fn test_budget() {
        let config = ContextConfig::default();
        let manager = ContextManager::new(config);
        
        let session = Session::new("test".to_string(), "gpt-4".to_string());
        let budget = manager.budget_usage(&session);
        
        assert_eq!(budget.total_tokens, 128000);
        assert_eq!(budget.reserved_tokens, 4096);
    }
}

impl Default for ContextManager {
    fn default() -> Self {
        Self::new(ContextConfig::default())
    }
}
