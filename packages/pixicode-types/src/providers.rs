//! Provider types for shared usage

use serde::{Deserialize, Serialize};
use schemars::JsonSchema;

/// Provider summary for UI.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ProviderSummary {
    /// Provider name
    pub name: String,
    /// Display name
    pub display_name: String,
    /// Whether provider is configured
    pub configured: bool,
    /// Provider status
    pub status: ProviderStatus,
}

/// Provider status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ProviderStatus {
    Unknown,
    Configured,
    NotConfigured,
    Error,
}

/// Model summary for UI.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ModelSummary {
    /// Model ID
    pub id: String,
    /// Model name
    pub name: Option<String>,
    /// Provider name
    pub provider: String,
    /// Context window size
    pub context_window: Option<u32>,
    /// Whether model supports streaming
    pub supports_streaming: bool,
    /// Whether model supports function calling
    pub supports_functions: bool,
}

/// Chat request for UI.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ChatRequest {
    /// Model to use
    pub model: String,
    /// Messages
    pub messages: Vec<ChatMessage>,
    /// Whether to stream response
    pub stream: bool,
    /// Temperature
    pub temperature: Option<f32>,
    /// Max tokens
    pub max_tokens: Option<u32>,
}

/// Chat message for UI.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ChatMessage {
    /// Message role
    pub role: String,
    /// Message content
    pub content: String,
}

/// Chat response chunk for streaming.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ChatResponseChunk {
    /// Model generating response
    pub model: String,
    /// Content delta
    pub delta: String,
    /// Whether response is complete
    pub done: bool,
    /// Finish reason
    pub finish_reason: Option<String>,
}

/// Chat response for non-streaming.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ChatResponse {
    /// Model used
    pub model: String,
    /// Response content
    pub content: String,
    /// Token usage
    pub usage: Option<TokenUsage>,
    /// Finish reason
    pub finish_reason: Option<String>,
}

/// Token usage.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TokenUsage {
    /// Input tokens
    pub input_tokens: u32,
    /// Output tokens
    pub output_tokens: u32,
    /// Total tokens
    pub total_tokens: u32,
    /// Estimated cost
    pub estimated_cost: Option<f64>,
}
