//! Session Manager — High-level session management

use std::sync::Arc;
use tokio::sync::RwLock;

use crate::session::types::{Session, Message, ContextConfig, CompactionStrategy};
use crate::session::store::SessionStore;
use crate::session::context::ContextManager;

/// Session manager for handling all session operations.
pub struct SessionManager {
    store: Arc<RwLock<SessionStore>>,
    context_manager: ContextManager,
    auto_compact: bool,
    auto_archive_threshold: Option<u32>,
}

impl SessionManager {
    /// Create a new session manager.
    pub fn new(store: SessionStore) -> Self {
        Self {
            store: Arc::new(RwLock::new(store)),
            context_manager: ContextManager::default(),
            auto_compact: true,
            auto_archive_threshold: None,
        }
    }

    /// Set context configuration.
    pub fn with_context_config(mut self, config: ContextConfig) -> Self {
        self.context_manager = ContextManager::new(config);
        self
    }

    /// Set compaction strategy.
    pub fn with_compaction_strategy(mut self, strategy: CompactionStrategy) -> Self {
        self.context_manager = self.context_manager.clone().with_strategy(strategy);
        self
    }

    /// Enable/disable auto compaction.
    pub fn with_auto_compact(mut self, enabled: bool) -> Self {
        self.auto_compact = enabled;
        self
    }

    /// Set auto-archive threshold (messages count).
    pub fn with_auto_archive(mut self, threshold: u32) -> Self {
        self.auto_archive_threshold = Some(threshold);
        self
    }

    /// Create a new session.
    pub async fn create_session(&self, model: String) -> Result<Session, SessionError> {
        let id = ulid::Ulid::new().to_string();
        let session = Session::new(id.clone(), model);
        
        let store = self.store.read().await;
        store.create(&session)?;
        
        Ok(session)
    }

    /// Create a session with custom ID.
    pub async fn create_session_with_id(&self, id: String, model: String) -> Result<Session, SessionError> {
        let session = Session::new(id.clone(), model);
        
        let store = self.store.read().await;
        store.create(&session)?;
        
        Ok(session)
    }

    /// Get a session by ID.
    pub async fn get_session(&self, id: &str) -> Result<Option<Session>, SessionError> {
        let store = self.store.read().await;
        store.get(id).map_err(SessionError::from)
    }

    /// Update a session.
    pub async fn update_session(&self, session: &Session) -> Result<(), SessionError> {
        let store = self.store.read().await;
        store.update(session)?;
        Ok(())
    }

    /// Delete a session.
    pub async fn delete_session(&self, id: &str) -> Result<(), SessionError> {
        let store = self.store.read().await;
        store.delete(id)?;
        Ok(())
    }

    /// List sessions.
    pub async fn list_sessions(&self, limit: Option<u32>, offset: Option<u32>) -> Result<Vec<Session>, SessionError> {
        let store = self.store.read().await;
        store.list(limit, offset).map_err(SessionError::from)
    }

    /// Add a message to a session.
    pub async fn add_message(&self, session_id: &str, message: Message) -> Result<(), SessionError> {
        let mut store = self.store.write().await;
        
        // Get session to check compaction
        if let Some(mut session) = store.get(session_id)? {
            session.add_message(message.clone());
            
            // Auto-compact if needed
            if self.auto_compact && self.context_manager.needs_compaction(&session) {
                self.context_manager.compact(&mut session);
            }
            
            // Update session
            store.update(&session)?;
            store.add_message(session_id, &message)?;
            
            // Auto-archive if threshold reached
            if let Some(threshold) = self.auto_archive_threshold {
                if session.message_count() >= threshold as usize {
                    store.archive(session_id)?;
                }
            }
        } else {
            return Err(SessionError::NotFound(session_id.to_string()));
        }
        
        Ok(())
    }

    /// Add a user message.
    pub async fn add_user_message(&self, session_id: &str, content: String) -> Result<Message, SessionError> {
        let message = Message::user(content);
        self.add_message(session_id, message.clone()).await?;
        Ok(message)
    }

    /// Add an assistant message.
    pub async fn add_assistant_message(&self, session_id: &str, content: String) -> Result<Message, SessionError> {
        let message = Message::assistant(content);
        self.add_message(session_id, message.clone()).await?;
        Ok(message)
    }

    /// Add a system message.
    pub async fn add_system_message(&self, session_id: &str, content: String) -> Result<Message, SessionError> {
        let message = Message::system(content);
        self.add_message(session_id, message.clone()).await?;
        Ok(message)
    }

    /// Set system prompt for a session.
    pub async fn set_system_prompt(&self, session_id: &str, prompt: String) -> Result<(), SessionError> {
        let mut store = self.store.write().await;
        
        if let Some(mut session) = store.get(session_id)? {
            session.system_prompt = Some(prompt);
            session.updated_at = chrono::Utc::now();
            store.update(&session)?;
        } else {
            return Err(SessionError::NotFound(session_id.to_string()));
        }
        
        Ok(())
    }

    /// Get context for a session (messages that fit in context window).
    pub async fn get_context(&self, session_id: &str) -> Result<Vec<Message>, SessionError> {
        let store = self.store.read().await;
        
        if let Some(session) = store.get(session_id)? {
            Ok(self.context_manager.build_context(&session))
        } else {
            Err(SessionError::NotFound(session_id.to_string()))
        }
    }

    /// Get token budget for a session.
    pub async fn get_budget(&self, session_id: &str) -> Result<crate::session::context::TokenBudget, SessionError> {
        let store = self.store.read().await;
        
        if let Some(session) = store.get(session_id)? {
            Ok(self.context_manager.budget_usage(&session))
        } else {
            Err(SessionError::NotFound(session_id.to_string()))
        }
    }

    /// Compact a session manually.
    pub async fn compact_session(&self, session_id: &str) -> Result<u32, SessionError> {
        let mut store = self.store.write().await;
        
        if let Some(mut session) = store.get(session_id)? {
            let removed = self.context_manager.compact(&mut session);
            store.update(&session)?;
            Ok(removed)
        } else {
            Err(SessionError::NotFound(session_id.to_string()))
        }
    }

    /// Archive a session.
    pub async fn archive_session(&self, session_id: &str) -> Result<(), SessionError> {
        let store = self.store.read().await;
        store.archive(session_id)?;
        Ok(())
    }

    /// Get session count.
    pub async fn count(&self) -> Result<u32, SessionError> {
        let store = self.store.read().await;
        Ok(store.count()?)
    }

    /// Search sessions by title.
    pub async fn search(&self, query: &str) -> Result<Vec<Session>, SessionError> {
        let sessions = self.list_sessions(None, None).await?;
        let query_lower = query.to_lowercase();
        
        Ok(sessions.into_iter().filter(|s| {
            s.title.as_ref().map(|t| t.to_lowercase().contains(&query_lower)).unwrap_or(false)
        }).collect())
    }
}

/// Session management errors.
#[derive(Debug, thiserror::Error)]
pub enum SessionError {
    #[error("Session not found: {0}")]
    NotFound(String),
    
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),
    
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

impl Clone for ContextManager {
    fn clone(&self) -> Self {
        Self::new(self.config.clone()).with_strategy(self.strategy)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::store::SessionStore;

    #[tokio::test]
    async fn test_session_manager() {
        let store = SessionStore::memory().unwrap();
        let manager = SessionManager::new(store);

        let session = manager.create_session("gpt-4".to_string()).await.unwrap();
        assert_eq!(session.model, "gpt-4");

        manager.add_user_message(&session.id, "Hello".to_string()).await.unwrap();
        manager.add_assistant_message(&session.id, "Hi!".to_string()).await.unwrap();

        let loaded = manager.get_session(&session.id).await.unwrap().unwrap();
        assert_eq!(loaded.message_count(), 2);

        let budget = manager.get_budget(&session.id).await.unwrap();
        assert!(budget.usage_percent < 1.0);
    }
}
