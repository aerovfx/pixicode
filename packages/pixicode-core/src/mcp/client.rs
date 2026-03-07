//! MCP Client — connect to MCP servers, list tools/resources/prompts

use crate::mcp::transport::StdioClientTransport;
use crate::mcp::types::{
    JsonRpcRequest, JsonRpcResponse, ListPromptsResult, ListResourcesResult, ListToolsResult,
    McpPrompt, McpResource, McpTool,
};

/// MCP Client for connecting to external MCP servers (stdio transport).
pub struct McpClient {
    transport: Option<StdioClientTransport>,
    next_id: std::sync::atomic::AtomicU64,
}

impl McpClient {
    pub fn new() -> Self {
        Self {
            transport: None,
            next_id: std::sync::atomic::AtomicU64::new(1),
        }
    }

    /// Connect via stdio by spawning the server process. `command` = executable, `args` = arguments.
    pub fn connect_stdio(&mut self, command: &str, args: &[String]) -> Result<(), String> {
        let transport = StdioClientTransport::spawn(command, args)?;
        self.transport = Some(transport);
        let req = JsonRpcRequest {
            jsonrpc: Some("2.0".into()),
            id: Some(serde_json::json!(1)),
            method: "initialize".into(),
            params: Some(serde_json::json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": { "name": "pixicode", "version": "0.1.0" }
            })),
        };
        let t = self.transport.as_ref().ok_or("no transport")?;
        let res = t.request(&req)?;
        if res.error.is_some() {
            return Err(res.error.unwrap().message);
        }
        Ok(())
    }

    fn next_id(&self) -> u64 {
        self.next_id.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    }

    fn request(&self, method: &str, params: Option<serde_json::Value>) -> Result<JsonRpcResponse, String> {
        let t = self.transport.as_ref().ok_or("not connected: call connect_stdio first")?;
        let req = JsonRpcRequest {
            jsonrpc: Some("2.0".into()),
            id: Some(serde_json::json!(self.next_id())),
            method: method.to_string(),
            params,
        };
        let res = t.request(&req)?;
        if let Some(ref err) = res.error {
            return Err(format!("{}: {}", err.code, err.message));
        }
        Ok(res)
    }

    pub async fn list_tools(&self) -> Result<Vec<McpTool>, String> {
        let res = self.request("tools/list", Some(serde_json::json!({})))?;
        let result = res.result.ok_or("no result")?;
        let list: ListToolsResult = serde_json::from_value(result).map_err(|e| e.to_string())?;
        Ok(list.tools)
    }

    pub async fn list_resources(&self) -> Result<Vec<McpResource>, String> {
        let res = self.request("resources/list", Some(serde_json::json!({})))?;
        let result = res.result.ok_or("no result")?;
        let list: ListResourcesResult = serde_json::from_value(result).map_err(|e| e.to_string())?;
        Ok(list.resources)
    }

    pub async fn list_prompts(&self) -> Result<Vec<McpPrompt>, String> {
        let res = self.request("prompts/list", Some(serde_json::json!({})))?;
        let result = res.result.ok_or("no result")?;
        let list: ListPromptsResult = serde_json::from_value(result).map_err(|e| e.to_string())?;
        Ok(list.prompts)
    }

    /// Check if client is connected.
    pub fn is_connected(&self) -> bool {
        self.transport.is_some()
    }
}

impl Default for McpClient {
    fn default() -> Self {
        Self::new()
    }
}
