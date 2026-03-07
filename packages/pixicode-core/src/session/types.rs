//! Session types — Core data structures for session management

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Helper module for RFC3339 datetime serialization.
mod datetime_rfc3339 {
    use chrono::{DateTime, Utc};
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S>(dt: &DateTime<Utc>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        dt.to_rfc3339().serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<DateTime<Utc>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        DateTime::parse_from_rfc3339(&s)
            .map(|dt| dt.into())
            .map_err(serde::de::Error::custom)
    }
}

/// Session metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMetadata {
    /// Session title
    pub title: Option<String>,
    /// Tags for organization
    pub tags: Vec<String>,
    /// Custom metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

impl Default for SessionMetadata {
    fn default() -> Self {
        Self {
            title: None,
            tags: Vec::new(),
            metadata: HashMap::new(),
        }
    }
}

/// A conversation session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    /// Unique session ID
    pub id: String,
    /// Session title
    pub title: Option<String>,
    /// Messages in the session
    pub messages: Vec<Message>,
    /// System prompt
    pub system_prompt: Option<String>,
    /// Model being used
    pub model: String,
    /// Session metadata
    pub metadata: SessionMetadata,
    /// Token usage
    pub usage: SessionUsage,
    /// Created timestamp
    #[serde(with = "datetime_rfc3339")]
    pub created_at: DateTime<Utc>,
    /// Updated timestamp
    #[serde(with = "datetime_rfc3339")]
    pub updated_at: DateTime<Utc>,
    /// Whether session is archived
    pub archived: bool,
}

impl Session {
    pub fn new(id: String, model: String) -> Self {
        let now = Utc::now();
        Self {
            id,
            title: None,
            messages: Vec::new(),
            system_prompt: None,
            model,
            metadata: SessionMetadata::default(),
            usage: SessionUsage::default(),
            created_at: now,
            updated_at: now,
            archived: false,
        }
    }

    pub fn add_message(&mut self, message: Message) {
        self.messages.push(message);
        self.updated_at = Utc::now();
    }

    pub fn message_count(&self) -> usize {
        self.messages.len()
    }

    pub fn total_tokens(&self) -> u32 {
        self.usage.total_tokens
    }
}

/// A single message in a session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    /// Unique message ID
    pub id: String,
    /// Message role
    pub role: MessageRole,
    /// Message content parts
    pub parts: Vec<MessagePart>,
    /// Timestamp
    #[serde(with = "datetime_rfc3339")]
    pub created_at: DateTime<Utc>,
    /// Token count for this message
    pub token_count: Option<u32>,
    /// Tool calls (for assistant messages)
    pub tool_calls: Option<Vec<ToolCallInfo>>,
    /// Tool call response (for tool messages)
    pub tool_call_id: Option<String>,
    /// Parent message ID (for threading)
    pub parent_id: Option<String>,
}

impl Message {
    pub fn new(id: String, role: MessageRole, content: String) -> Self {
        Self {
            id,
            role,
            parts: vec![MessagePart::Text(content)],
            created_at: Utc::now(),
            token_count: None,
            tool_calls: None,
            tool_call_id: None,
            parent_id: None,
        }
    }

    pub fn system(content: String) -> Self {
        Self::new(ulid::Ulid::new().to_string(), MessageRole::System, content)
    }

    pub fn user(content: String) -> Self {
        Self::new(ulid::Ulid::new().to_string(), MessageRole::User, content)
    }

    pub fn assistant(content: String) -> Self {
        Self::new(ulid::Ulid::new().to_string(), MessageRole::Assistant, content)
    }

    pub fn with_tool_calls(mut self, tool_calls: Vec<ToolCallInfo>) -> Self {
        self.tool_calls = Some(tool_calls);
        self
    }

    pub fn content(&self) -> String {
        self.parts.iter().filter_map(|p| {
            if let MessagePart::Text(t) = p {
                Some(t.clone())
            } else {
                None
            }
        }).collect::<Vec<_>>().join("\n")
    }
}

/// Message role.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    System,
    User,
    Assistant,
    Tool,
}

/// A part of a message (supports text, images, tool calls, etc.).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum MessagePart {
    /// Text content
    Text(String),
    /// Image content (URL or base64)
    Image { url: String, mime_type: Option<String> },
    /// Tool call
    ToolCall { name: String, input: serde_json::Value },
    /// Tool result
    ToolResult { tool_call_id: String, output: String, success: bool },
}

/// Tool call information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallInfo {
    /// Tool call ID
    pub id: String,
    /// Tool name
    pub name: String,
    /// Tool input
    pub input: serde_json::Value,
}

/// Session token usage.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SessionUsage {
    /// Total input tokens
    pub input_tokens: u32,
    /// Total output tokens
    pub output_tokens: u32,
    /// Total tokens
    pub total_tokens: u32,
    /// Total cost (USD)
    pub total_cost: f64,
    /// Token usage per message
    pub message_usage: Vec<MessageUsage>,
}

impl SessionUsage {
    pub fn add(&mut self, input: u32, output: u32, cost: f64) {
        self.input_tokens += input;
        self.output_tokens += output;
        self.total_tokens += input + output;
        self.total_cost += cost;
    }
}

/// Message-level token usage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageUsage {
    pub message_id: String,
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub cost: f64,
}

/// Context window configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextConfig {
    /// Maximum tokens for context
    pub max_tokens: u32,
    /// Reserve tokens for response
    pub reserve_tokens: u32,
    /// System prompt priority (0-1)
    pub system_priority: f32,
    /// Recent messages priority (0-1)
    pub recent_priority: f32,
}

impl Default for ContextConfig {
    fn default() -> Self {
        Self {
            max_tokens: 128000,
            reserve_tokens: 4096,
            system_priority: 0.1,
            recent_priority: 0.7,
        }
    }
}

impl ContextConfig {
    pub fn available_tokens(&self) -> u32 {
        self.max_tokens - self.reserve_tokens
    }
}

/// Compaction strategy for context window management.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompactionStrategy {
    /// Drop oldest messages first
    DropOldest,
    /// Summarize old conversations
    Summarize,
    /// Keep only recent messages
    RecentOnly,
    /// Smart compaction based on importance
    Smart,
}

impl Default for CompactionStrategy {
    fn default() -> Self {
        Self::Smart
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_creation() {
        let session = Session::new("test-id".to_string(), "gpt-4".to_string());
        assert_eq!(session.id, "test-id");
        assert_eq!(session.model, "gpt-4");
        assert_eq!(session.message_count(), 0);
    }

    #[test]
    fn test_message_creation() {
        let msg = Message::user("Hello".to_string());
        assert_eq!(msg.role, MessageRole::User);
        assert!(msg.content().contains("Hello"));
    }

    #[test]
    fn test_context_config() {
        let config = ContextConfig::default();
        assert_eq!(config.available_tokens(), 128000 - 4096);
    }
}
