//! Tool System — AI-powered tool execution framework
//!
//! Provides a trait-based architecture for tools that AI agents can invoke:
//!  - File operations (read, write, edit, multiedit, ls, glob, grep, codesearch)
//!  - Shell execution (bash with PTY support)
//!  - Web operations (fetch, search)
//!  - Advanced tools (batch, patch, task, plan, question, todo, skill, lsp)

pub mod trait_def;
pub mod registry;
pub mod types;

// File tools
pub mod file_read;
pub mod file_write;
pub mod file_edit;
pub mod file_multiedit;
pub mod file_ls;
pub mod file_glob;
pub mod file_grep;
pub mod file_codesearch;

// Shell tools
pub mod shell_bash;

// Web tools
pub mod web_fetch;
pub mod web_search;

// Advanced tools
pub mod tool_batch;
pub mod tool_apply_patch;
pub mod tool_task;
pub mod tool_plan;
pub mod tool_question;
pub mod tool_todo;
pub mod tool_skill;
pub mod lsp_client;
pub mod tool_lsp;

pub use trait_def::{Tool, ToolResult, ToolError};
pub use registry::ToolRegistry;
pub use types::{ToolCall, ToolOutput, ToolParameter, ToolSchema};
