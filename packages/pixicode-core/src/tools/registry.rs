//! Tool Registry — manages tool registration and lookup with lazy initialization

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::tools::trait_def::{Tool, ToolCall, ToolContext, ToolError, ToolResult};
use crate::tools::types::{ToolOutput, ToolSchema};

/// Thread-safe tool registry with lazy initialization.
pub struct ToolRegistry {
    tools: RwLock<HashMap<&'static str, Arc<dyn Tool>>>,
}

impl ToolRegistry {
    /// Creates a new empty registry.
    pub fn new() -> Self {
        Self {
            tools: RwLock::new(HashMap::new()),
        }
    }

    /// Creates a registry with all built-in tools registered.
    pub fn with_builtins() -> Self {
        let registry = Self::new();
        registry.register_all_builtins();
        registry
    }

    /// Registers a tool with the registry.
    pub fn register<T: Tool + 'static>(&self, tool: T) {
        let name = tool.name();
        tracing::debug!(name, "registering tool");
        futures::executor::block_on(async {
            self.tools.write().await.insert(name, Arc::new(tool));
        });
    }

    /// Registers multiple tools at once.
    pub fn register_all(&self, tools: Vec<Box<dyn Tool>>) {
        for tool in tools {
            self.register_boxed(tool);
        }
    }

    /// Registers a boxed tool.
    pub fn register_boxed(&self, tool: Box<dyn Tool>) {
        let name = tool.name();
        tracing::debug!(name, "registering boxed tool");
        futures::executor::block_on(async {
            self.tools.write().await.insert(name, Arc::from(tool));
        });
    }

    /// Looks up a tool by name.
    pub fn get(&self, name: &str) -> Option<Arc<dyn Tool>> {
        futures::executor::block_on(async {
            self.tools.read().await.get(name).cloned()
        })
    }

    /// Lists all registered tool names.
    pub fn list_tools(&self) -> Vec<&'static str> {
        futures::executor::block_on(async {
            self.tools.read().await.keys().copied().collect()
        })
    }

    /// Returns the number of registered tools.
    pub fn len(&self) -> usize {
        futures::executor::block_on(async {
            self.tools.read().await.len()
        })
    }

    /// Checks if the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Executes a tool call.
    pub async fn execute(&self, call: &ToolCall, context: &ToolContext) -> ToolResult<ToolOutput> {
        let tool = self
            .get(&call.name)
            .ok_or_else(|| ToolError::NotFound(call.name.clone()))?;

        tracing::info!(
            tool = call.name,
            call_id = call.call_id,
            "executing tool"
        );

        // Validate parameters against schema
        if let Err(e) = self.validate_params(&tool, &call.arguments) {
            return Err(e);
        }

        // Execute the tool
        match tool.execute(call.arguments.clone(), context).await {
            Ok(output) => {
                if output.success {
                    tracing::info!(tool = call.name, call_id = call.call_id, "tool executed successfully");
                } else {
                    tracing::warn!(tool = call.name, call_id = call.call_id, error = ?output.error, "tool execution failed");
                }
                Ok(output)
            }
            Err(e) => {
                tracing::error!(tool = call.name, call_id = call.call_id, error = ?e, "tool execution error");
                Err(e)
            }
        }
    }

    /// Validates parameters against the tool's schema.
    fn validate_params(&self, tool: &Arc<dyn Tool>, params: &serde_json::Value) -> ToolResult<()> {
        let schema = tool.schema();

        // Check required fields
        if let Some(obj) = params.as_object() {
            for required in &schema.required {
                if !obj.contains_key(required) {
                    return Err(ToolError::InvalidParams(format!(
                        "Missing required parameter: {}",
                        required
                    )));
                }
            }
        } else {
            return Err(ToolError::InvalidParams(
                "Parameters must be an object".to_string()
            ));
        }

        Ok(())
    }

    /// Registers all built-in tools.
    fn register_all_builtins(&self) {
        use crate::tools::*;

        // File tools (8 tools)
        self.register(file_read::ReadTool::new());
        self.register(file_write::WriteTool::new());
        self.register(file_edit::EditTool::new());
        self.register(file_multiedit::MultieditTool::new());
        self.register(file_ls::LsTool::new());
        self.register(file_glob::GlobTool::new());
        self.register(file_grep::GrepTool::new());
        self.register(file_codesearch::CodesearchTool::new());

        // Shell tools (1 tool)
        self.register(shell_bash::BashTool::new());

        // Web tools (2 tools)
        self.register(web_fetch::WebFetchTool::new());
        self.register(web_search::WebSearchTool::new());

        // Advanced tools (8 tools)
        self.register(tool_batch::BatchTool::new());
        self.register(tool_apply_patch::ApplyPatchTool::new());
        self.register(tool_task::TaskTool::new());
        self.register(tool_plan::PlanTool::new());
        self.register(tool_question::QuestionTool::new());
        self.register(tool_todo::TodoTool::new());
        self.register(tool_skill::SkillTool::new());
        self.register(tool_lsp::LspTool::new());

        tracing::info!("Registered {} built-in tools", self.len());
    }

    /// Returns JSON Schema definitions for all tools.
    pub fn get_schemas(&self) -> HashMap<&'static str, ToolSchema> {
        futures::executor::block_on(async {
            let tools = self.tools.read().await;
            tools
                .iter()
                .map(|(name, tool)| (*name, tool.schema()))
                .collect()
        })
    }

    /// Returns JSON Schema for each tool's parameters as OpenAPI-compatible values.
    pub fn get_schemas_json(&self) -> std::collections::HashMap<String, serde_json::Value> {
        self.get_schemas()
            .into_iter()
            .map(|(name, schema)| (name.to_string(), schema.to_json_value()))
            .collect()
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;

    struct TestTool;

    #[async_trait]
    impl Tool for TestTool {
        fn name(&self) -> &'static str {
            "test_tool"
        }

        fn description(&self) -> &'static str {
            "A test tool"
        }

        fn schema(&self) -> crate::tools::types::ToolSchema {
            crate::tools::types::ToolSchema::default()
        }

        async fn execute(&self, _params: serde_json::Value, _context: &ToolContext) -> ToolResult<ToolOutput> {
            Ok(ToolOutput::success("Test executed"))
        }
    }

    #[tokio::test]
    async fn test_registry() {
        let registry = ToolRegistry::new();
        registry.register(TestTool);

        assert_eq!(registry.len(), 1);
        assert!(registry.get("test_tool").is_some());
        assert!(registry.get("nonexistent").is_none());

        let tools = registry.list_tools();
        assert!(tools.contains(&"test_tool"));
    }

    #[tokio::test]
    async fn test_execute() {
        let registry = ToolRegistry::new();
        registry.register(TestTool);

        let call = ToolCall::new("test_tool", serde_json::json!({}));
        let context = ToolContext::default();

        let result = registry.execute(&call, &context).await;
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.success);
    }
}
