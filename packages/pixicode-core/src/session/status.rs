//! Session Status Tracking — real-time status of all active sessions.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use serde::Serialize;

/// Real-time status of a single session.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionStatus {
    pub session_id: String,
    pub state: SessionState,
    /// Name of the tool currently executing, if any.
    pub active_tool: Option<String>,
    /// Number of prompt loop iterations completed.
    pub iterations: u32,
    /// Tokens generated so far.
    pub tokens_generated: u32,
}

/// Session execution state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionState {
    Idle,
    Streaming,
    ToolExecuting,
    WaitingPermission,
    Error,
}

/// Thread-safe registry of active session statuses.
#[derive(Clone, Default)]
pub struct StatusTracker {
    statuses: Arc<RwLock<HashMap<String, SessionStatus>>>,
}

impl StatusTracker {
    pub fn new() -> Self {
        Self::default()
    }

    /// Set a session as actively streaming.
    pub async fn set_streaming(&self, session_id: &str) {
        let mut map = self.statuses.write().await;
        let entry = map.entry(session_id.to_string()).or_insert_with(|| SessionStatus {
            session_id: session_id.to_string(),
            state: SessionState::Idle,
            active_tool: None,
            iterations: 0,
            tokens_generated: 0,
        });
        entry.state = SessionState::Streaming;
        entry.active_tool = None;
    }

    /// Set a session as executing a tool.
    pub async fn set_tool_executing(&self, session_id: &str, tool_name: &str) {
        let mut map = self.statuses.write().await;
        if let Some(status) = map.get_mut(session_id) {
            status.state = SessionState::ToolExecuting;
            status.active_tool = Some(tool_name.to_string());
        }
    }

    /// Increment iteration counter.
    pub async fn inc_iteration(&self, session_id: &str) {
        let mut map = self.statuses.write().await;
        if let Some(status) = map.get_mut(session_id) {
            status.iterations += 1;
        }
    }

    /// Mark a session as idle (done or error).
    pub async fn set_idle(&self, session_id: &str) {
        let mut map = self.statuses.write().await;
        map.remove(session_id);
    }

    /// Get all active session statuses.
    pub async fn all(&self) -> Vec<SessionStatus> {
        let map = self.statuses.read().await;
        map.values().cloned().collect()
    }

    /// Get status for a specific session.
    pub async fn get(&self, session_id: &str) -> Option<SessionStatus> {
        let map = self.statuses.read().await;
        map.get(session_id).cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_status_tracker_lifecycle() {
        let tracker = StatusTracker::new();

        // Initially empty
        assert!(tracker.all().await.is_empty());
        assert!(tracker.get("sess_1").await.is_none());

        // Set streaming creates entry
        tracker.set_streaming("sess_1").await;
        let status = tracker.get("sess_1").await.unwrap();
        assert_eq!(status.state, SessionState::Streaming);
        assert_eq!(status.iterations, 0);
        assert!(status.active_tool.is_none());

        // Set tool executing
        tracker.set_tool_executing("sess_1", "bash").await;
        let status = tracker.get("sess_1").await.unwrap();
        assert_eq!(status.state, SessionState::ToolExecuting);
        assert_eq!(status.active_tool.as_deref(), Some("bash"));

        // Increment iteration
        tracker.inc_iteration("sess_1").await;
        tracker.inc_iteration("sess_1").await;
        let status = tracker.get("sess_1").await.unwrap();
        assert_eq!(status.iterations, 2);

        // Set idle removes the session
        tracker.set_idle("sess_1").await;
        assert!(tracker.get("sess_1").await.is_none());
        assert!(tracker.all().await.is_empty());
    }

    #[tokio::test]
    async fn test_multiple_sessions() {
        let tracker = StatusTracker::new();

        tracker.set_streaming("sess_a").await;
        tracker.set_streaming("sess_b").await;
        tracker.set_tool_executing("sess_a", "read").await;

        let all = tracker.all().await;
        assert_eq!(all.len(), 2);

        let a = tracker.get("sess_a").await.unwrap();
        assert_eq!(a.state, SessionState::ToolExecuting);

        let b = tracker.get("sess_b").await.unwrap();
        assert_eq!(b.state, SessionState::Streaming);

        // Remove one
        tracker.set_idle("sess_a").await;
        assert_eq!(tracker.all().await.len(), 1);
    }

    #[test]
    fn test_session_state_serialize() {
        let status = SessionStatus {
            session_id: "test".to_string(),
            state: SessionState::ToolExecuting,
            active_tool: Some("bash".to_string()),
            iterations: 3,
            tokens_generated: 150,
        };
        let json = serde_json::to_value(&status).unwrap();
        assert_eq!(json["state"], "tool_executing");
        assert_eq!(json["sessionId"], "test");
        assert_eq!(json["activeTool"], "bash");
        assert_eq!(json["iterations"], 3);
    }
}
