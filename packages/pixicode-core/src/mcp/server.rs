//! MCP Server — expose tools via MCP (stdio)

use crate::mcp::transport::StdioServerTransport;
use crate::mcp::types::{
    JsonRpcRequest, JsonRpcError, JsonRpcResponse, ListToolsResult, McpTool,
};
use std::sync::Arc;

/// Handler for tools/call: (tool_name, arguments) -> result content or error.
pub type ToolCallHandler = Arc<dyn Fn(String, serde_json::Value) -> Result<serde_json::Value, String> + Send + Sync>;

/// MCP Server for exposing pixicode tools via MCP (stdio).
pub struct McpServer {
    tools: Vec<McpTool>,
    tool_handler: Option<ToolCallHandler>,
}

impl McpServer {
    pub fn new(tools: Vec<McpTool>) -> Self {
        Self {
            tools,
            tool_handler: None,
        }
    }

    pub fn with_tool_handler(mut self, handler: ToolCallHandler) -> Self {
        self.tool_handler = Some(handler);
        self
    }

    /// Run the server loop on stdin/stdout (blocking). Handles initialize, tools/list, tools/call, resources/list, prompts/list.
    pub fn run_stdio(&self) -> Result<(), String> {
        let transport = StdioServerTransport::new();
        loop {
            let Some(req) = transport.read_request()? else {
                break;
            };
            let id = req.id.clone();
            let method = req.method.clone();
            let params = req.params.clone().unwrap_or(serde_json::json!({}));

            let result = match method.as_str() {
                "initialize" => Ok(serde_json::json!({
                    "protocolVersion": "2024-11-05",
                    "capabilities": {
                        "tools": {},
                        "resources": {},
                        "prompts": {}
                    },
                    "serverInfo": { "name": "pixicode", "version": "0.1.0" }
                })),
                "tools/list" => {
                    let list = ListToolsResult {
                        tools: self.tools.clone(),
                    };
                    Ok(serde_json::to_value(list).map_err(|e| e.to_string())?)
                }
                "tools/call" => {
                    let name = params.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let args = params.get("arguments").cloned().unwrap_or(serde_json::json!({}));
                    match &self.tool_handler {
                        Some(handler) => handler(name, args).map_err(|e| e.to_string()),
                        None => Err("tools/call not configured".to_string()),
                    }
                }
                "resources/list" => Ok(serde_json::json!({ "resources": serde_json::Value::Array(vec![]) })),
                "prompts/list" => Ok(serde_json::json!({ "prompts": serde_json::Value::Array(vec![]) })),
                _ => Err(format!("unknown method: {}", method)),
            };

            let response = match result {
                Ok(res) => JsonRpcResponse {
                    jsonrpc: Some("2.0".into()),
                    id,
                    result: Some(res),
                    error: None,
                },
                Err(msg) => JsonRpcResponse {
                    jsonrpc: Some("2.0".into()),
                    id,
                    result: None,
                    error: Some(JsonRpcError {
                        code: -32603,
                        message: msg,
                        data: None,
                    }),
                },
            };
            transport.write_response(&response)?;
        }
        Ok(())
    }

    /// Start the server (alias for run_stdio for compatibility).
    pub async fn start(&self) -> Result<(), String> {
        self.run_stdio()
    }
}

impl Default for McpServer {
    fn default() -> Self {
        Self::new(Vec::new())
    }
}
