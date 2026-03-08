//! Permission Gate — checks tool permissions before execution in the prompt loop.
//!
//! Flow:
//!   1. Check if tool is "safe" (read-only) → auto-allow
//!   2. Check DB for project-level grant → auto-allow if granted
//!   3. Otherwise, publish a permission request event and wait for user reply
//!   4. Return Allow/Deny based on reply

use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

use crate::bus::{BusEvent, EventBus};
use crate::db::Database;
use crate::server::state::PendingPermissionReplies;
use crate::tools::trait_def::PermissionLevel;

/// Tools that are always safe to execute without permission.
/// Read-only tools that don't modify the filesystem or execute commands.
const SAFE_TOOLS: &[&str] = &[
    "read", "ls", "glob", "grep", "codesearch",
    "plan", "todo", "question", "skill",
];

/// Tools that require write permission.
const WRITE_TOOLS: &[&str] = &[
    "write", "edit", "multiedit", "apply_patch",
];

/// Tools that require execute permission (shell).
const EXECUTE_TOOLS: &[&str] = &["bash"];

/// Tools that require web access.
const WEB_TOOLS: &[&str] = &["webfetch", "websearch"];

/// Permission arity — how long the grant lasts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PermissionArity {
    /// Grant for this single call only
    Once,
    /// Grant for the remainder of this session
    Session,
    /// Grant for all sessions in this project
    Project,
    /// Grant forever (all projects)
    Always,
}

impl PermissionArity {
    /// Parse from a string (from user reply).
    pub fn from_str_loose(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "once" => Self::Once,
            "session" => Self::Session,
            "project" => Self::Project,
            "always" => Self::Always,
            // "allow" without qualifier defaults to session
            "allow" => Self::Session,
            _ => Self::Once,
        }
    }
}

/// Result of a permission check.
#[derive(Debug, Clone)]
pub enum GateResult {
    /// Tool is allowed to execute.
    Allowed,
    /// Tool execution was denied.
    Denied { reason: String },
}

/// The Permission Gate checks whether a tool call should proceed.
pub struct PermissionGate {
    db: Database,
    bus: EventBus,
    permission_replies: PendingPermissionReplies,
    /// In-memory session-scoped grants: tool_name → true
    session_grants: Arc<RwLock<std::collections::HashSet<String>>>,
}

impl PermissionGate {
    pub fn new(
        db: Database,
        bus: EventBus,
        permission_replies: PendingPermissionReplies,
    ) -> Self {
        Self {
            db,
            bus,
            permission_replies,
            session_grants: Arc::new(RwLock::new(std::collections::HashSet::new())),
        }
    }

    /// Check permission for a tool call. Returns `Allowed` or `Denied`.
    ///
    /// Steps:
    /// 1. Safe tools → always allowed
    /// 2. Session-scoped grants → allowed
    /// 3. Project-scoped grants (from DB) → allowed
    /// 4. Request permission from user → wait for reply
    pub async fn check(
        &self,
        tool_name: &str,
        session_id: &str,
        project_id: &str,
    ) -> GateResult {
        // 1. Safe tools are always allowed
        if is_safe_tool(tool_name) {
            return GateResult::Allowed;
        }

        // 2. Check session-scoped grants (in-memory)
        {
            let grants = self.session_grants.read().await;
            if grants.contains(tool_name) {
                return GateResult::Allowed;
            }
        }

        // 3. Check project-scoped grants (from DB)
        if self.is_granted_in_db(tool_name, project_id) {
            return GateResult::Allowed;
        }

        // 4. Request permission from user
        self.request_permission(tool_name, session_id).await
    }

    /// Check if a tool has been granted in the DB for a given project.
    fn is_granted_in_db(&self, tool_name: &str, project_id: &str) -> bool {
        self.db
            .with(|conn| {
                let data_str: Option<String> = conn
                    .query_row(
                        "SELECT data FROM permission WHERE project_id = ?1",
                        rusqlite::params![project_id],
                        |row| row.get(0),
                    )
                    .ok();

                if let Some(data_str) = data_str {
                    if let Ok(data) = serde_json::from_str::<serde_json::Value>(&data_str) {
                        // Check if tool is in the granted permissions
                        if let Some(action) = data.get(tool_name).and_then(|v| v.as_str()) {
                            return Ok(action == "allow" || action == "grant" || action == "always");
                        }
                    }
                }
                Ok(false)
            })
            .unwrap_or(false)
    }

    /// Publish a permission request event and wait for user reply.
    async fn request_permission(
        &self,
        tool_name: &str,
        session_id: &str,
    ) -> GateResult {
        let request_id = format!(
            "perm_{}",
            ulid::Ulid::new().to_string().to_lowercase()
        );

        // Publish the permission request event
        self.bus.publish(BusEvent::ToolCallStarted {
            session_id: session_id.to_string(),
            tool: format!("permission_request:{}", tool_name),
            call_id: request_id.clone(),
        });

        tracing::info!(
            tool = tool_name,
            request_id = %request_id,
            "permission gate: waiting for user reply"
        );

        // Poll for reply with timeout (60 seconds)
        let timeout = Duration::from_secs(60);
        let poll_interval = Duration::from_millis(250);
        let start = tokio::time::Instant::now();

        loop {
            // Check if reply has arrived
            {
                let replies = self.permission_replies.read().await;
                if let Some(reply) = replies.get(&request_id) {
                    let allowed = reply.reply.to_lowercase();
                    let is_allowed = matches!(
                        allowed.as_str(),
                        "allow" | "grant" | "yes" | "once" | "session" | "project" | "always"
                    );

                    if is_allowed {
                        // Determine arity and store grant if needed
                        let arity = PermissionArity::from_str_loose(&reply.reply);
                        drop(replies); // Release read lock before acquiring write lock

                        match arity {
                            PermissionArity::Session | PermissionArity::Once => {
                                // Store session-scoped grant
                                if arity == PermissionArity::Session {
                                    self.session_grants
                                        .write()
                                        .await
                                        .insert(tool_name.to_string());
                                }
                            }
                            PermissionArity::Project | PermissionArity::Always => {
                                // Store session grant + DB grant is handled by the permission route
                                self.session_grants
                                    .write()
                                    .await
                                    .insert(tool_name.to_string());
                            }
                        }

                        // Clean up the reply
                        self.permission_replies
                            .write()
                            .await
                            .remove(&request_id);

                        return GateResult::Allowed;
                    } else {
                        drop(replies);
                        self.permission_replies
                            .write()
                            .await
                            .remove(&request_id);
                        return GateResult::Denied {
                            reason: format!("User denied permission for tool '{}'", tool_name),
                        };
                    }
                }
            }

            // Check timeout
            if start.elapsed() >= timeout {
                tracing::warn!(
                    tool = tool_name,
                    request_id = %request_id,
                    "permission gate: timed out waiting for reply"
                );
                return GateResult::Denied {
                    reason: format!(
                        "Permission request for tool '{}' timed out after {}s",
                        tool_name,
                        timeout.as_secs()
                    ),
                };
            }

            tokio::time::sleep(poll_interval).await;
        }
    }
}

/// Check if a tool is considered safe (read-only, no side effects).
pub fn is_safe_tool(name: &str) -> bool {
    SAFE_TOOLS.contains(&name)
}

/// Categorize a tool by its permission requirement.
pub fn tool_category(name: &str) -> &'static str {
    if SAFE_TOOLS.contains(&name) {
        "safe"
    } else if WRITE_TOOLS.contains(&name) {
        "write"
    } else if EXECUTE_TOOLS.contains(&name) {
        "execute"
    } else if WEB_TOOLS.contains(&name) {
        "web"
    } else {
        "unknown"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_safe_tools() {
        assert!(is_safe_tool("read"));
        assert!(is_safe_tool("glob"));
        assert!(is_safe_tool("grep"));
        assert!(!is_safe_tool("write"));
        assert!(!is_safe_tool("bash"));
        assert!(!is_safe_tool("webfetch"));
    }

    #[test]
    fn test_tool_category() {
        assert_eq!(tool_category("read"), "safe");
        assert_eq!(tool_category("write"), "write");
        assert_eq!(tool_category("bash"), "execute");
        assert_eq!(tool_category("webfetch"), "web");
        assert_eq!(tool_category("custom_tool"), "unknown");
    }

    #[test]
    fn test_permission_arity() {
        assert_eq!(PermissionArity::from_str_loose("once"), PermissionArity::Once);
        assert_eq!(PermissionArity::from_str_loose("session"), PermissionArity::Session);
        assert_eq!(PermissionArity::from_str_loose("project"), PermissionArity::Project);
        assert_eq!(PermissionArity::from_str_loose("always"), PermissionArity::Always);
        assert_eq!(PermissionArity::from_str_loose("allow"), PermissionArity::Session);
        assert_eq!(PermissionArity::from_str_loose("unknown"), PermissionArity::Once);
    }
}
