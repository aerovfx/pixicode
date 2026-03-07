//! Batch Tool — parallel tool execution

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::tools::trait_def::{Tool, ToolContext, ToolError, ToolOutput, ToolResult, ToolSchema, ToolParameter};

/// A single tool call in the batch.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchCall {
    /// Tool name to invoke
    pub name: String,
    /// Tool arguments
    pub arguments: serde_json::Value,
}

/// Parameters for the batch tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchParams {
    /// List of tool calls to execute
    pub calls: Vec<BatchCall>,
    /// Execute in parallel (default: true)
    #[serde(default = "default_true")]
    pub parallel: bool,
    /// Stop on first error (default: false)
    #[serde(default)]
    pub stop_on_error: bool,
}

fn default_true() -> bool { true }

/// Result from a single batch call.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchResult {
    /// Tool name
    pub name: String,
    /// Success flag
    pub success: bool,
    /// Output text
    pub output: String,
    /// Error message (if failed)
    pub error: Option<String>,
    /// Data payload
    pub data: Option<serde_json::Value>,
}

/// Tool for executing multiple tools in batch.
pub struct BatchTool;

impl BatchTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Tool for BatchTool {
    fn name(&self) -> &'static str {
        "batch"
    }

    fn description(&self) -> &'static str {
        "Execute multiple tool calls in parallel or sequence"
    }

    fn schema(&self) -> ToolSchema {
        let mut schema = ToolSchema::default();
        schema.required = vec!["calls".to_string()];
        schema.properties.insert("calls".to_string(), ToolParameter {
            param_type: "array".to_string(),
            description: "List of tool calls to execute".to_string(),
            default: None,
            enum_values: None,
            minimum: None,
            maximum: None,
            min_length: None,
            max_length: None,
            items: Some(Box::new(ToolParameter {
                param_type: "object".to_string(),
                description: "A tool call with name and arguments".to_string(),
                default: None,
                enum_values: None,
                minimum: None,
                maximum: None,
                min_length: None,
                max_length: None,
                items: None,
            })),
        });
        schema.properties.insert("parallel".to_string(), ToolParameter::boolean("Execute in parallel"));
        schema.properties.insert("stop_on_error".to_string(), ToolParameter::boolean("Stop on first error"));
        schema
    }

    async fn execute(&self, params: serde_json::Value, context: &ToolContext) -> ToolResult<ToolOutput> {
        let params: BatchParams = serde_json::from_value(params)
            .map_err(|e| ToolError::InvalidParams(e.to_string()))?;

        if params.calls.is_empty() {
            return Err(ToolError::InvalidParams("No tool calls provided".to_string()));
        }

        // Note: In a real implementation, we'd need access to the ToolRegistry here
        // For now, we'll return a placeholder response
        let mut results = Vec::new();
        let mut all_success = true;

        if params.parallel {
            // Parallel execution would go here
            // This requires registry access which we don't have in this context
            for call in &params.calls {
                results.push(BatchResult {
                    name: call.name.clone(),
                    success: false,
                    output: String::new(),
                    error: Some("Batch execution requires registry access - not implemented in this context".to_string()),
                    data: None,
                });
                all_success = false;
                
                if params.stop_on_error {
                    break;
                }
            }
        } else {
            // Sequential execution
            for call in &params.calls {
                results.push(BatchResult {
                    name: call.name.clone(),
                    success: false,
                    output: String::new(),
                    error: Some("Batch execution requires registry access - not implemented in this context".to_string()),
                    data: None,
                });
                all_success = false;
                
                if params.stop_on_error {
                    break;
                }
            }
        }

        // Format output
        let mut output = String::new();
        output.push_str(&format!("Executed {} tool call(s)\n\n", results.len()));
        
        for (i, result) in results.iter().enumerate() {
            let status = if result.success { "✓" } else { "✗" };
            output.push_str(&format!("{}. {} {}\n", i + 1, status, result.name));
            if let Some(error) = &result.error {
                output.push_str(&format!("   Error: {}\n", error));
            }
            if !result.output.is_empty() {
                output.push_str(&format!("   Output: {}\n", truncate(&result.output, 100)));
            }
        }

        let result_data: Vec<serde_json::Value> = results.iter().map(|r| {
            serde_json::json!({
                "name": r.name,
                "success": r.success,
                "output": r.output,
                "error": r.error,
                "data": r.data,
            })
        }).collect();

        Ok(ToolOutput::success(output).with_data(serde_json::json!({
            "total": results.len(),
            "successful": results.iter().filter(|r| r.success).count(),
            "failed": results.iter().filter(|r| !r.success).count(),
            "results": result_data,
        })))
    }
}

/// Truncate string to max length.
fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_batch_empty() {
        let tool = BatchTool::new();
        let params = serde_json::json!({
            "calls": []
        });
        let context = ToolContext::default();

        let result = tool.execute(params, &context).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_batch_placeholder() {
        let tool = BatchTool::new();
        let params = serde_json::json!({
            "calls": [
                {"name": "read", "arguments": {"path": "test.txt"}}
            ],
            "parallel": true
        });
        let context = ToolContext::default();

        let result = tool.execute(params, &context).await;
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.output.contains("Executed 1 tool call"));
    }
}
