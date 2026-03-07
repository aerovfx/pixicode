//! Tool types for shared usage

use serde::{Deserialize, Serialize};
use schemars::JsonSchema;

/// Tool summary for UI.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ToolSummary {
    /// Tool name
    pub name: String,
    /// Tool description
    pub description: String,
    /// Whether tool is available
    pub available: bool,
}

/// Tool execution request.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ToolExecutionRequest {
    /// Tool name
    pub tool_name: String,
    /// Tool arguments
    pub arguments: serde_json::Value,
    /// Session ID for tracking
    pub session_id: Option<String>,
}

/// Tool execution result.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ToolExecutionResult {
    /// Tool name
    pub tool_name: String,
    /// Whether execution was successful
    pub success: bool,
    /// Output text
    pub output: String,
    /// Structured data
    pub data: Option<serde_json::Value>,
    /// Error message
    pub error: Option<String>,
    /// Execution time in ms
    pub execution_time_ms: Option<u64>,
}

/// Tool permission request.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ToolPermissionRequest {
    /// Tool name
    pub tool_name: String,
    /// Permission type
    pub permission: String,
    /// Whether to allow
    pub allow: bool,
    /// Whether to remember choice
    pub remember: bool,
}
