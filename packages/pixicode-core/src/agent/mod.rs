//! Agent System — AI agent definitions and permissions
//!
//! Provides agent definitions with different access levels,
//! permission system, and custom agent configurations.

pub mod types;
pub mod registry;
pub mod permissions;

pub use types::{Agent, AgentConfig, AgentType, AgentPermission};
pub use registry::AgentRegistry;
pub use permissions::PermissionChecker;
