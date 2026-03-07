//! Provider types — Core data structures for AI provider interactions

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Role of a message sender.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    System,
    User,
    Assistant,
    Tool,
}

/// A single message in a conversation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    /// Role of the message sender
    pub role: MessageRole,
    /// Message content
    pub content: String,
    /// Optional name for the sender
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Tool call ID (for tool responses)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    /// Tool calls (for assistant messages)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
}

impl Message {
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::System,
            content: content.into(),
            name: None,
            tool_call_id: None,
            tool_calls: None,
        }
    }

    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::User,
            content: content.into(),
            name: None,
            tool_call_id: None,
            tool_calls: None,
        }
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::Assistant,
            content: content.into(),
            name: None,
            tool_call_id: None,
            tool_calls: None,
        }
    }

    pub fn tool_response(tool_call_id: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::Tool,
            content: content.into(),
            name: None,
            tool_call_id: Some(tool_call_id.into()),
            tool_calls: None,
        }
    }
}

/// A tool definition for the model to use.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    /// Tool name
    pub name: String,
    /// Tool description
    pub description: String,
    /// Tool parameters schema (JSON Schema)
    pub parameters: serde_json::Value,
}

/// A tool call from the model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    /// Unique tool call ID
    pub id: String,
    /// Tool name
    pub name: String,
    /// Tool arguments (JSON string or object)
    pub arguments: serde_json::Value,
}

/// Result from a tool execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    /// Tool call ID
    pub tool_call_id: String,
    /// Tool output
    pub output: String,
    /// Whether the tool execution was successful
    pub success: bool,
}

/// Model information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    /// Model ID
    pub id: String,
    /// Model name (human-readable)
    pub name: Option<String>,
    /// Model description
    pub description: Option<String>,
    /// Context window size (max tokens)
    pub context_window: Option<u32>,
    /// Maximum output tokens
    pub max_output_tokens: Option<u32>,
    /// Supported capabilities
    pub capabilities: ModelCapabilities,
    /// Pricing info (input/output per 1K tokens)
    pub pricing: Option<PricingInfo>,
}

/// Model capabilities.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ModelCapabilities {
    /// Supports function calling
    pub functions: bool,
    /// Supports vision/images
    pub vision: bool,
    /// Supports streaming
    pub streaming: bool,
    /// Supports JSON mode
    pub json_mode: bool,
}

/// Pricing information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PricingInfo {
    /// Input price per 1K tokens (USD)
    pub input_per_1k: f64,
    /// Output price per 1K tokens (USD)
    pub output_per_1k: f64,
}

/// Token usage information.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Usage {
    /// Input tokens
    pub input_tokens: u32,
    /// Output tokens
    pub output_tokens: u32,
    /// Total tokens
    pub total_tokens: u32,
    /// Input tokens broken down by type
    pub input_token_details: Option<InputTokenDetails>,
}

/// Input token details (for models that support it).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct InputTokenDetails {
    /// Cached tokens
    pub cached_tokens: Option<u32>,
}

impl Usage {
    pub fn new(input: u32, output: u32) -> Self {
        Self {
            input_tokens: input,
            output_tokens: output,
            total_tokens: input + output,
            input_token_details: None,
        }
    }
}

/// Reason why the model stopped generating.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FinishReason {
    Stop,
    Length,
    ToolCalls,
    ContentFilter,
    Error,
    Other,
}

/// Chat request to send to a provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatRequest {
    /// Optional cancellation token; when cancelled, streaming stops.
    #[serde(skip, default)]
    pub cancel: Option<std::sync::Arc<tokio_util::sync::CancellationToken>>,
    /// Model to use
    pub model: String,
    /// Messages in the conversation
    pub messages: Vec<Message>,
    /// Tools available for the model to call
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<ToolDefinition>>,
    /// Tool choice (none, auto, required, or specific)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<ToolChoice>,
    /// Temperature (0-2)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    /// Maximum tokens to generate
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    /// Top-p sampling
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    /// Frequency penalty
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frequency_penalty: Option<f32>,
    /// Presence penalty
    #[serde(skip_serializing_if = "Option::is_none")]
    pub presence_penalty: Option<f32>,
    /// Stop sequences
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop: Option<Vec<String>>,
    /// Response format (text or json)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_format: Option<ResponseFormat>,
    /// Stream the response
    #[serde(default)]
    pub stream: bool,
    /// User identifier for tracking
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,
}

/// Tool choice option.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ToolChoice {
    /// Don't use tools
    None,
    /// Let model decide
    Auto,
    /// Force tool use
    Required,
    /// Specific tool
    Specific {
        #[serde(rename = "type")]
        tool_type: String,
        function: FunctionChoice,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionChoice {
    pub name: String,
}

impl ToolChoice {
    pub fn none() -> Self { Self::None }
    pub fn auto() -> Self { Self::Auto }
    pub fn required() -> Self { Self::Required }
    pub fn function(name: impl Into<String>) -> Self {
        Self::Specific {
            tool_type: "function".to_string(),
            function: FunctionChoice { name: name.into() },
        }
    }
}

/// Response format option.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseFormat {
    #[serde(rename = "type")]
    pub format_type: String,
    /// JSON schema (for json_object type)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub json_schema: Option<serde_json::Value>,
}

impl ResponseFormat {
    pub fn text() -> Self {
        Self { format_type: "text".to_string(), json_schema: None }
    }
    pub fn json() -> Self {
        Self { format_type: "json_object".to_string(), json_schema: None }
    }
}

/// Chat response from a provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatResponse {
    /// Model that generated the response
    pub model: String,
    /// Generated message
    pub message: Message,
    /// Finish reason
    pub finish_reason: FinishReason,
    /// Token usage
    pub usage: Option<Usage>,
    /// Provider-specific ID
    pub id: Option<String>,
    /// Creation timestamp
    pub created_at: Option<u64>,
}

/// Streaming chunk from a provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatChunk {
    /// Model generating the response
    pub model: String,
    /// Delta content (incremental)
    pub delta: MessageDelta,
    /// Finish reason (if finished)
    pub finish_reason: Option<FinishReason>,
    /// Token usage (if available)
    pub usage: Option<Usage>,
    /// Chunk index
    pub index: Option<u32>,
}

/// Delta message for streaming.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MessageDelta {
    /// Role (if known)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<MessageRole>,
    /// Content delta
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    /// Tool calls delta
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCallDelta>>,
}

/// Tool call delta for streaming.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallDelta {
    /// Index in the tool calls list
    pub index: u32,
    /// Tool call ID (if known)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// Tool name (if known)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Arguments delta
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<String>,
}

/// Model alias for configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Model {
    /// Model ID
    pub id: String,
    /// Provider name
    pub provider: String,
    /// Display name
    pub name: Option<String>,
}
