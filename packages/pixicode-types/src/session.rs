//! Session types for shared usage

use serde::{Deserialize, Serialize};
use schemars::JsonSchema;

/// Session summary for UI display.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SessionSummary {
    /// Session ID
    pub id: String,
    /// Session title
    pub title: Option<String>,
    /// Model used
    pub model: String,
    /// Message count
    pub message_count: usize,
    /// Total tokens used
    pub total_tokens: u32,
    /// Created timestamp
    pub created_at: String,
    /// Updated timestamp
    pub updated_at: String,
    /// Whether session is archived
    pub archived: bool,
}

/// Message summary for UI display.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct MessageSummary {
    /// Message ID
    pub id: String,
    /// Message role
    pub role: String,
    /// Preview of content
    pub preview: String,
    /// Token count
    pub token_count: Option<u32>,
    /// Timestamp
    pub created_at: String,
}

/// Session creation request.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CreateSessionRequest {
    /// Model to use
    pub model: String,
    /// Initial title
    pub title: Option<String>,
    /// System prompt
    pub system_prompt: Option<String>,
}

/// Session response.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SessionResponse {
    /// Whether operation was successful
    pub success: bool,
    /// Session data
    pub session: Option<SessionSummary>,
    /// Error message
    pub error: Option<String>,
}

/// Message request.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct MessageRequest {
    /// Message content
    pub content: String,
    /// Parent message ID (for threading)
    pub parent_id: Option<String>,
}
