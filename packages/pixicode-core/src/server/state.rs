//! Shared application state threaded through Axum handlers.

use std::sync::Arc;
use tokio::sync::broadcast;
use tokio::sync::RwLock;

use crate::bus::EventBus;
use crate::config::Config;
use crate::db::Database;
use crate::providers::registry::ProviderRegistry;
use crate::session::status::StatusTracker;
use crate::tools::registry::ToolRegistry;

/// Global questions waiting for user answers (keyed by question ID).
pub type PendingQuestions = Arc<RwLock<std::collections::HashMap<String, QuestionState>>>;

#[derive(Debug, Clone)]
pub struct QuestionState {
    pub prompt: String,
    pub answer_tx: Arc<tokio::sync::oneshot::Sender<String>>,
}

/// Stored reply for a permission request (Phase 1: parity with TS permission.reply).
#[derive(Debug, Clone)]
pub struct PermissionReply {
    pub reply: String,
    pub message: Option<String>,
}

pub type PendingPermissionReplies = Arc<RwLock<std::collections::HashMap<String, PermissionReply>>>;

/// Full application state shared across all Axum handlers.
pub struct AppState {
    pub config: Config,
    pub db: Database,
    pub bus: EventBus,
    pub shutdown_rx: broadcast::Receiver<()>,
    pub questions: PendingQuestions,
    pub permission_replies: PendingPermissionReplies,
    pub registry: Arc<ProviderRegistry>,
    pub tool_registry: Arc<ToolRegistry>,
    pub status_tracker: StatusTracker,
}

impl AppState {
    pub fn new(
        config: Config,
        db: Database,
        shutdown_rx: broadcast::Receiver<()>,
        registry: Arc<ProviderRegistry>,
    ) -> Self {
        Self {
            config,
            db,
            bus: EventBus::new(),
            shutdown_rx,
            questions: Arc::new(RwLock::new(std::collections::HashMap::new())),
            permission_replies: Arc::new(RwLock::new(std::collections::HashMap::new())),
            registry,
            tool_registry: Arc::new(ToolRegistry::with_builtins()),
            status_tracker: StatusTracker::new(),
        }
    }
}
