//! Tool trait definition — core interface for all tools

use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

// ─────────────────────────────────────────────────────────────────────────────
//  Tool Error types
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Error)]
pub enum ToolError {
    #[error("Tool not found: {0}")]
    NotFound(String),

    #[error("Invalid parameters: {0}")]
    InvalidParams(String),

    #[error("Execution failed: {0}")]
    Execution(String),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Timeout after {0}ms")]
    Timeout(u64),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Internal error: {0}")]
    Internal(String),
}

pub type ToolResult<T> = Result<T, ToolError>;

// ─────────────────────────────────────────────────────────────────────────────
//  Tool Output
// ─────────────────────────────────────────────────────────────────────────────

/// Output from a tool execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolOutput {
    /// Success flag
    pub success: bool,
    /// Text output (for display)
    pub output: String,
    /// Structured data (if any)
    pub data: Option<serde_json::Value>,
    /// Error message (if failed)
    pub error: Option<String>,
}

impl ToolOutput {
    pub fn success(output: impl Into<String>) -> Self {
        Self {
            success: true,
            output: output.into(),
            data: None,
            error: None,
        }
    }

    pub fn failure(error: impl Into<String>) -> Self {
        Self {
            success: false,
            output: String::new(),
            data: None,
            error: Some(error.into()),
        }
    }

    pub fn with_data(mut self, data: serde_json::Value) -> Self {
        self.data = Some(data);
        self
    }
}

// ─────────────────────────────────────────────────────────────────────────────
//  Tool Schema
// ─────────────────────────────────────────────────────────────────────────────

/// JSON Schema for tool parameters.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ToolSchema {
    /// Schema type (usually "object")
    #[serde(rename = "type")]
    pub schema_type: String,
    /// Required parameter names
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub required: Vec<String>,
    /// Parameter definitions
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub properties: HashMap<String, ToolParameter>,
    /// Additional properties allowed
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub additional_properties: bool,
}

impl Default for ToolSchema {
    fn default() -> Self {
        Self {
            schema_type: "object".to_string(),
            required: Vec::new(),
            properties: HashMap::new(),
            additional_properties: false,
        }
    }
}

impl ToolSchema {
    /// Returns standard JSON Schema (OpenAPI-compatible) as a serde_json::Value.
    pub fn to_json_value(&self) -> serde_json::Value {
        let props: serde_json::Map<String, serde_json::Value> = self
            .properties
            .iter()
            .map(|(k, v)| (k.clone(), v.to_json_value()))
            .collect();
        let mut obj = serde_json::Map::new();
        obj.insert("type".into(), serde_json::Value::String(self.schema_type.clone()));
        if !self.required.is_empty() {
            obj.insert(
                "required".into(),
                serde_json::Value::Array(self.required.iter().map(|s| serde_json::Value::String(s.clone())).collect()),
            );
        }
        obj.insert("properties".into(), serde_json::Value::Object(props));
        obj.insert("additionalProperties".into(), serde_json::Value::Bool(self.additional_properties));
        serde_json::Value::Object(obj)
    }
}

/// A single tool parameter definition.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ToolParameter {
    /// Parameter type
    #[serde(rename = "type")]
    pub param_type: String,
    /// Parameter description
    pub description: String,
    /// Default value (if any)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<serde_json::Value>,
    /// Enum values (if applicable)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enum_values: Option<Vec<serde_json::Value>>,
    /// Minimum for numbers
    #[serde(skip_serializing_if = "Option::is_none")]
    pub minimum: Option<f64>,
    /// Maximum for numbers
    #[serde(skip_serializing_if = "Option::is_none")]
    pub maximum: Option<f64>,
    /// Minimum length for strings
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_length: Option<u32>,
    /// Maximum length for strings
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_length: Option<u32>,
    /// Items schema for arrays
    #[serde(skip_serializing_if = "Option::is_none")]
    pub items: Option<Box<ToolParameter>>,
}

impl ToolParameter {
    pub fn string(desc: impl Into<String>) -> Self {
        Self {
            param_type: "string".to_string(),
            description: desc.into(),
            default: None,
            enum_values: None,
            minimum: None,
            maximum: None,
            min_length: None,
            max_length: None,
            items: None,
        }
    }

    pub fn number(desc: impl Into<String>) -> Self {
        Self {
            param_type: "number".to_string(),
            description: desc.into(),
            default: None,
            enum_values: None,
            minimum: None,
            maximum: None,
            min_length: None,
            max_length: None,
            items: None,
        }
    }

    pub fn integer(desc: impl Into<String>) -> Self {
        Self {
            param_type: "integer".to_string(),
            description: desc.into(),
            default: None,
            enum_values: None,
            minimum: None,
            maximum: None,
            min_length: None,
            max_length: None,
            items: None,
        }
    }

    pub fn boolean(desc: impl Into<String>) -> Self {
        Self {
            param_type: "boolean".to_string(),
            description: desc.into(),
            default: None,
            enum_values: None,
            minimum: None,
            maximum: None,
            min_length: None,
            max_length: None,
            items: None,
        }
    }

    pub fn array(desc: impl Into<String>, items: ToolParameter) -> Self {
        Self {
            param_type: "array".to_string(),
            description: desc.into(),
            default: None,
            enum_values: None,
            minimum: None,
            maximum: None,
            min_length: None,
            max_length: None,
            items: Some(Box::new(items)),
        }
    }

    /// Returns standard JSON Schema fragment for this parameter.
    pub fn to_json_value(&self) -> serde_json::Value {
        let mut obj = serde_json::Map::new();
        obj.insert("type".into(), serde_json::Value::String(self.param_type.clone()));
        obj.insert("description".into(), serde_json::Value::String(self.description.clone()));
        if let Some(ref d) = self.default {
            obj.insert("default".into(), d.clone());
        }
        if let Some(ref e) = self.enum_values {
            obj.insert("enum".into(), serde_json::Value::Array(e.clone()));
        }
        if let Some(m) = self.minimum {
            if let Some(n) = serde_json::Number::from_f64(m) {
                obj.insert("minimum".into(), serde_json::Value::Number(n));
            }
        }
        if let Some(m) = self.maximum {
            if let Some(n) = serde_json::Number::from_f64(m) {
                obj.insert("maximum".into(), serde_json::Value::Number(n));
            }
        }
        if let Some(m) = self.min_length {
            obj.insert("minLength".into(), serde_json::Value::Number(m.into()));
        }
        if let Some(m) = self.max_length {
            obj.insert("maxLength".into(), serde_json::Value::Number(m.into()));
        }
        if let Some(ref it) = self.items {
            obj.insert("items".into(), it.to_json_value());
        }
        serde_json::Value::Object(obj)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
//  Tool Call
// ─────────────────────────────────────────────────────────────────────────────

/// A tool invocation request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    /// Tool name to invoke
    pub name: String,
    /// Tool arguments
    pub arguments: serde_json::Value,
    /// Unique call ID for tracking
    pub call_id: String,
}

impl ToolCall {
    pub fn new(name: impl Into<String>, arguments: serde_json::Value) -> Self {
        Self {
            name: name.into(),
            arguments,
            call_id: ulid::Ulid::new().to_string(),
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
//  Tool Trait
// ─────────────────────────────────────────────────────────────────────────────

/// Core trait that all tools must implement.
#[async_trait]
pub trait Tool: Send + Sync {
    /// Returns the tool's unique name (e.g., "read", "write", "bash").
    fn name(&self) -> &'static str;

    /// Returns a human-readable description of what the tool does.
    fn description(&self) -> &'static str;

    /// Returns the JSON Schema for the tool's parameters.
    fn schema(&self) -> ToolSchema;

    /// Executes the tool with the given parameters.
    ///
    /// # Arguments
    /// * `params` — Parsed parameters from the tool call
    /// * `context` — Execution context (working dir, permissions, etc.)
    async fn execute(&self, params: serde_json::Value, context: &ToolContext) -> ToolResult<ToolOutput>;
}

/// Callback type for executing tools from within other tools (e.g., batch).
pub type ToolExecutor = std::sync::Arc<
    dyn Fn(ToolCall, ToolContext) -> std::pin::Pin<Box<dyn std::future::Future<Output = ToolResult<ToolOutput>> + Send>>
        + Send
        + Sync,
>;

/// Execution context passed to all tool calls.
#[derive(Clone)]
pub struct ToolContext {
    /// Current working directory
    pub working_dir: std::path::PathBuf,
    /// Permission level (ask, allow, deny)
    pub permission: PermissionLevel,
    /// Timeout in milliseconds
    pub timeout_ms: u64,
    /// Session ID for tracking
    pub session_id: Option<String>,
    /// Optional executor for delegating to other tools (used by batch, task).
    pub executor: Option<ToolExecutor>,
}

impl std::fmt::Debug for ToolContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ToolContext")
            .field("working_dir", &self.working_dir)
            .field("permission", &self.permission)
            .field("timeout_ms", &self.timeout_ms)
            .field("session_id", &self.session_id)
            .field("has_executor", &self.executor.is_some())
            .finish()
    }
}

impl Default for ToolContext {
    fn default() -> Self {
        Self {
            working_dir: std::env::current_dir().unwrap_or_else(|_| ".".into()),
            permission: PermissionLevel::Ask,
            timeout_ms: 30000, // 30s default
            session_id: None,
            executor: None,
        }
    }
}

impl ToolContext {
    pub fn new(working_dir: std::path::PathBuf) -> Self {
        Self {
            working_dir,
            ..Default::default()
        }
    }

    pub fn with_permission(mut self, permission: PermissionLevel) -> Self {
        self.permission = permission;
        self
    }

    pub fn with_timeout(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = timeout_ms;
        self
    }

    pub fn with_session(mut self, session_id: String) -> Self {
        self.session_id = Some(session_id);
        self
    }

    pub fn with_executor(mut self, executor: ToolExecutor) -> Self {
        self.executor = Some(executor);
        self
    }

    /// Execute a tool call via the attached executor (used by batch/task tools).
    pub async fn execute_tool(&self, call: ToolCall) -> ToolResult<ToolOutput> {
        match &self.executor {
            Some(executor) => (executor)(call, self.clone()).await,
            None => Err(ToolError::Internal("No tool executor available in this context".into())),
        }
    }
}

/// Permission level for tool execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PermissionLevel {
    /// Ask user before executinging
    Ask,
    /// Allow without asking
    Allow,
    /// Deny execution
    Deny,
}

impl Default for PermissionLevel {
    fn default() -> Self {
        Self::Ask
    }
}
