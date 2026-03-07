//! Shared application state threaded through Axum handlers.

use std::sync::Arc;
use tokio::sync::broadcast;
use tokio::sync::RwLock;

use crate::bus::EventBus;
use crate::config::Config;
use crate::db::Database;

/// Global questions waiting for user answers (keyed by question ID).
pub type PendingQuestions = Arc<RwLock<std::collections::HashMap<String, QuestionState>>>;

#[derive(Debug, Clone)]
pub struct QuestionState {
    pub prompt: String,
    pub answer_tx: Arc<tokio::sync::oneshot::Sender<String>>,
}

/// Full application state shared across all Axum handlers.
pub struct AppState {
    pub config: Config,
    pub db: Database,
    pub bus: EventBus,
    pub shutdown_rx: broadcast::Receiver<()>,
    pub questions: PendingQuestions,
}

impl AppState {
    pub fn new(config: Config, db: Database, shutdown_rx: broadcast::Receiver<()>) -> Self {
        Self {
            config,
            db,
            bus: EventBus::new(),
            shutdown_rx,
            questions: Arc::new(RwLock::new(std::collections::HashMap::new())),
        }
    }
}
