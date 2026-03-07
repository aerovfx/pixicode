//! MCP (Model Context Protocol) — Protocol for AI model context exchange
//!
//! Provides MCP client and server implementations for connecting to
//! external MCP servers and exposing pixicode tools via MCP.
//! Transport: stdio (newline-delimited JSON-RPC).

pub mod client;
pub mod server;
pub mod types;
pub mod transport;

pub use types::*;
pub use client::McpClient;
pub use server::{McpServer, ToolCallHandler};
pub use transport::{StdioClientTransport, StdioServerTransport};

/// Build `Vec<McpTool>` from the tool registry for use with `McpServer::new()`.
pub fn tools_from_registry(registry: &crate::tools::ToolRegistry) -> Vec<McpTool> {
    let schemas = registry.get_schemas();
    schemas
        .into_iter()
        .map(|(name, schema)| McpTool {
            name: name.to_string(),
            description: None,
            input_schema: Some(schema.to_json_value()),
        })
        .collect()
}
