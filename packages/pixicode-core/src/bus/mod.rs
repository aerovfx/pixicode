//! Internal event bus — tokio::broadcast channels.
//!
//! All subsystems publish typed events here.  The SSE endpoint subscribes and
//! forwards them to connected clients.

use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

// ─────────────────────────────────────────────────────────────────────────────
//  Event types
// ─────────────────────────────────────────────────────────────────────────────

/// Discriminated union of every event type the bus can carry.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum BusEvent {
    // Session lifecycle
    SessionCreated { session_id: String, title: String },
    SessionUpdated { session_id: String },
    SessionDeleted { session_id: String },

    // Message / part streaming
    MessageCreated { session_id: String, message_id: String },
    MessageUpdated { session_id: String, message_id: String },
    PartCreated    { session_id: String, message_id: String, part_id: String },
    PartUpdated    { session_id: String, message_id: String, part_id: String },

    // Tool calls
    ToolCallStarted  { session_id: String, tool: String, call_id: String },
    ToolCallFinished { session_id: String, tool: String, call_id: String, ok: bool },

    // Config
    ConfigChanged,

    // Instance
    InstanceDisposed { directory: String },
}

// ─────────────────────────────────────────────────────────────────────────────
//  EventBus
// ─────────────────────────────────────────────────────────────────────────────

/// Thin wrapper around a tokio broadcast sender.
///
/// Clone is cheap — all clones share the same underlying channel.
#[derive(Clone)]
pub struct EventBus {
    tx: broadcast::Sender<BusEvent>,
}

impl EventBus {
    /// Create a new bus with capacity for `capacity` buffered events.
    pub fn with_capacity(capacity: usize) -> Self {
        let (tx, _) = broadcast::channel(capacity);
        Self { tx }
    }

    /// Default capacity of 1024 events.
    pub fn new() -> Self {
        Self::with_capacity(1024)
    }

    /// Publish an event.  Returns the number of active receivers that
    /// received the event (0 if no subscribers).
    pub fn publish(&self, event: BusEvent) -> usize {
        tracing::debug!(event_type = event.type_name(), "bus publish");
        self.tx.send(event).unwrap_or(0)
    }

    /// Subscribe to the bus.  Returns a receiver that will receive all
    /// events published after the subscribe call.
    pub fn subscribe(&self) -> broadcast::Receiver<BusEvent> {
        self.tx.subscribe()
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}

// ─────────────────────────────────────────────────────────────────────────────
//  Helpers
// ─────────────────────────────────────────────────────────────────────────────

impl BusEvent {
    /// Return the string discriminant tag of this event variant.
    pub fn type_name(&self) -> &'static str {
        match self {
            BusEvent::SessionCreated { .. }    => "session_created",
            BusEvent::SessionUpdated { .. }    => "session_updated",
            BusEvent::SessionDeleted { .. }    => "session_deleted",
            BusEvent::MessageCreated { .. }    => "message_created",
            BusEvent::MessageUpdated { .. }    => "message_updated",
            BusEvent::PartCreated { .. }       => "part_created",
            BusEvent::PartUpdated { .. }       => "part_updated",
            BusEvent::ToolCallStarted { .. }   => "tool_call_started",
            BusEvent::ToolCallFinished { .. }  => "tool_call_finished",
            BusEvent::ConfigChanged            => "config_changed",
            BusEvent::InstanceDisposed { .. }  => "instance_disposed",
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
//  Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_publish_subscribe() {
        let bus = EventBus::new();
        let mut rx = bus.subscribe();

        bus.publish(BusEvent::ConfigChanged);

        let ev = rx.recv().await.unwrap();
        assert_eq!(ev.type_name(), "config_changed");
    }

    #[tokio::test]
    async fn test_session_events_serialise() {
        let ev = BusEvent::SessionCreated {
            session_id: "sess_01".into(),
            title: "Hello".into(),
        };
        let json = serde_json::to_string(&ev).unwrap();
        assert!(json.contains("session_created"));
    }
}
