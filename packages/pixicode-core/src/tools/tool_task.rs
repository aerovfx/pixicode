//! Task Tool — sub-agent task delegation

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::tools::trait_def::{Tool, ToolContext, ToolError, ToolOutput, ToolResult, ToolSchema, ToolParameter};

/// Parameters for the task tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskParams {
    /// Task description
    pub description: String,
    /// Agent type to use (general, researcher, coder, etc.)
    #[serde(default)]
    pub agent: Option<String>,
    /// Context to pass to sub-agent
    #[serde(default)]
    pub context: Option<String>,
    /// Expected output format
    #[serde(default)]
    pub output_format: Option<String>,
    /// Timeout in milliseconds (default: 300000 = 5 min)
    #[serde(default = "default_timeout")]
    pub timeout_ms: u64,
    /// Whether to stream progress
    #[serde(default)]
    pub stream: bool,
}

fn default_timeout() -> u64 { 300000 }

/// Result from sub-agent task.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResult {
    /// Task ID
    pub task_id: String,
    /// Agent used
    pub agent: String,
    /// Success flag
    pub success: bool,
    /// Output from sub-agent
    pub output: String,
    /// Duration in milliseconds
    pub duration_ms: u64,
    /// Token usage (if available)
    pub tokens: Option<TaskTokens>,
}

/// Token usage info.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskTokens {
    pub input: u32,
    pub output: u32,
    pub total: u32,
}

/// Tool for delegating tasks to sub-agents.
pub struct TaskTool;

impl TaskTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Tool for TaskTool {
    fn name(&self) -> &'static str {
        "task"
    }

    fn description(&self) -> &'static str {
        "Delegate a complex task to a sub-agent for execution"
    }

    fn schema(&self) -> ToolSchema {
        let mut schema = ToolSchema::default();
        schema.required = vec!["description".to_string()];
        schema.properties.insert("description".to_string(), ToolParameter::string("Task description"));
        schema.properties.insert("agent".to_string(), ToolParameter::string("Agent type: general, researcher, coder, reviewer"));
        schema.properties.insert("context".to_string(), ToolParameter::string("Additional context for the sub-agent"));
        schema.properties.insert("output_format".to_string(), ToolParameter::string("Expected output format"));
        schema.properties.insert("timeout_ms".to_string(), ToolParameter::integer("Timeout in milliseconds"));
        schema.properties.insert("stream".to_string(), ToolParameter::boolean("Stream progress updates"));
        schema
    }

    async fn execute(&self, params: serde_json::Value, context: &ToolContext) -> ToolResult<ToolOutput> {
        let params: TaskParams = serde_json::from_value(params)
            .map_err(|e| ToolError::InvalidParams(e.to_string()))?;

        let task_id = ulid::Ulid::new().to_string();
        let agent = params.agent.as_deref().unwrap_or("general");
        let start_time = std::time::Instant::now();

        // In a real implementation, this would:
        // 1. Spawn a sub-agent with the given description
        // 2. Pass context and configuration
        // 3. Monitor progress and stream updates
        // 4. Collect and return results
        
        // For now, we'll simulate a task delegation response
        let output = format!(
            "Task delegated to '{}' agent\n\n\
            Task ID: {}\n\
            Description: {}\n\
            \n\
            Note: Sub-agent execution is not fully implemented in this version.\n\
            In production, this would spawn a separate agent process to handle the task.\n\
            \n\
            Suggested approach for '{}':\n\
            1. Analyze the task requirements\n\
            2. Break down into sub-tasks if needed\n\
            3. Use appropriate tools (read, search, bash, etc.)\n\
            4. Synthesize findings into a coherent response\n\
            \n\
            Context provided: {}\n\
            ",
            agent,
            task_id,
            params.description,
            agent,
            params.context.as_deref().unwrap_or("None"),
        );

        let duration_ms = start_time.elapsed().as_millis() as u64;

        let result = TaskResult {
            task_id: task_id.clone(),
            agent: agent.to_string(),
            success: true,
            output: output.clone(),
            duration_ms,
            tokens: Some(TaskTokens {
                input: params.description.len() as u32 / 4,  // Rough estimate
                output: output.len() as u32 / 4,
                total: (params.description.len() + output.len()) as u32 / 4,
            }),
        };

        Ok(ToolOutput::success(output).with_data(serde_json::json!({
            "task_id": task_id,
            "agent": agent,
            "success": result.success,
            "duration_ms": duration_ms,
            "tokens": result.tokens,
        })))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_task_delegation() {
        let tool = TaskTool::new();
        let params = serde_json::json!({
            "description": "Research best practices for Rust error handling",
            "agent": "researcher",
            "context": "Focus on thiserror vs anyhow tradeoffs"
        });
        let context = ToolContext::default();

        let result = tool.execute(params, &context).await;
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.output.contains("Task delegated"));
        assert!(output.output.contains("researcher"));
    }

    #[tokio::test]
    async fn test_task_default_agent() {
        let tool = TaskTool::new();
        let params = serde_json::json!({
            "description": "Do something"
        });
        let context = ToolContext::default();

        let result = tool.execute(params, &context).await;
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.output.contains("general"));
    }
}
