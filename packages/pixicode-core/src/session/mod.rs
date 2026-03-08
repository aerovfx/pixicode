//! Session Management — AI conversation session handling
//!
//! Provides session CRUD, message threading, context window management,
//! token budget calculation, and system prompt assembly.

pub mod types;
pub mod store;
pub mod context;
pub mod manager;
pub mod permission_gate;
pub mod prompt;
pub mod status;
pub mod system;

pub use types::{Session, Message, MessagePart, SessionMetadata};
pub use store::SessionStore;
pub use context::ContextManager;
pub use manager::SessionManager;
pub use prompt::{run_prompt, PromptConfig};
