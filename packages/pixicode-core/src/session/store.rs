//! Session Store — Persistent session storage

use rusqlite::{Connection, params};
use std::path::Path;
use chrono::{DateTime, Utc};

use crate::session::types::{Session, SessionMetadata, Message, MessageRole, MessagePart, ToolCallInfo, SessionUsage};

/// Session store for persistent storage.
pub struct SessionStore {
    conn: Connection,
}

impl SessionStore {
    /// Open or create a session store.
    pub fn open(path: &Path) -> Result<Self, rusqlite::Error> {
        let conn = Connection::open(path)?;
        let store = Self { conn };
        store.init()?;
        Ok(store)
    }

    /// Create an in-memory session store.
    pub fn memory() -> Result<Self, rusqlite::Error> {
        let conn = Connection::open_in_memory()?;
        let store = Self { conn };
        store.init()?;
        Ok(store)
    }

    /// Initialize database schema.
    fn init(&self) -> Result<(), rusqlite::Error> {
        self.conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS sessions (
                id TEXT PRIMARY KEY,
                title TEXT,
                system_prompt TEXT,
                model TEXT NOT NULL,
                metadata TEXT NOT NULL DEFAULT '{}',
                input_tokens INTEGER DEFAULT 0,
                output_tokens INTEGER DEFAULT 0,
                total_tokens INTEGER DEFAULT 0,
                total_cost REAL DEFAULT 0.0,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                archived INTEGER DEFAULT 0
            );

            CREATE TABLE IF NOT EXISTS messages (
                id TEXT PRIMARY KEY,
                session_id TEXT NOT NULL,
                role TEXT NOT NULL,
                parts TEXT NOT NULL,
                created_at TEXT NOT NULL,
                token_count INTEGER,
                tool_calls TEXT,
                tool_call_id TEXT,
                parent_id TEXT,
                FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE
            );

            CREATE INDEX IF NOT EXISTS idx_messages_session ON messages(session_id);
            CREATE INDEX IF NOT EXISTS idx_sessions_archived ON sessions(archived);
            CREATE INDEX IF NOT EXISTS idx_sessions_updated ON sessions(updated_at);
            "
        )?;
        Ok(())
    }

    /// Create a new session.
    pub fn create(&self, session: &Session) -> Result<(), rusqlite::Error> {
        let metadata_json = serde_json::to_string(&session.metadata).unwrap_or_default();
        
        self.conn.execute(
            "INSERT INTO sessions (id, title, system_prompt, model, metadata, input_tokens, output_tokens, total_tokens, total_cost, created_at, updated_at, archived)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![
                session.id,
                session.title,
                session.system_prompt,
                session.model,
                metadata_json,
                session.usage.input_tokens,
                session.usage.output_tokens,
                session.usage.total_tokens,
                session.usage.total_cost,
                session.created_at.to_rfc3339(),
                session.updated_at.to_rfc3339(),
                if session.archived { 1 } else { 0 }
            ],
        )?;

        // Insert messages
        for msg in &session.messages {
            self.insert_message(&session.id, msg)?;
        }

        Ok(())
    }

    /// Get a session by ID.
    pub fn get(&self, id: &str) -> Result<Option<Session>, rusqlite::Error> {
        let mut stmt = self.conn.prepare(
            "SELECT id, title, system_prompt, model, metadata, input_tokens, output_tokens, 
                    total_tokens, total_cost, created_at, updated_at, archived
             FROM sessions WHERE id = ?1"
        )?;

        let session = stmt.query_row(params![id], |row| {
            let metadata_json: String = row.get(4)?;
            let metadata: SessionMetadata = serde_json::from_str(&metadata_json).unwrap_or_default();
            
            Ok(Session {
                id: row.get(0)?,
                title: row.get(1)?,
                system_prompt: row.get(2)?,
                model: row.get(3)?,
                metadata,
                usage: crate::session::types::SessionUsage {
                    input_tokens: row.get(5)?,
                    output_tokens: row.get(6)?,
                    total_tokens: row.get(7)?,
                    total_cost: row.get(8)?,
                    message_usage: Vec::new(),
                },
                created_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(9)?)
                    .unwrap_or_else(|_| Utc::now().into())
                    .into(),
                updated_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(10)?)
                    .unwrap_or_else(|_| Utc::now().into())
                    .into(),
                archived: row.get::<_, i32>(11)? != 0,
                messages: Vec::new(),
            })
        })?;

        // Load messages
        let mut session = session;
        session.messages = self.get_messages(&session.id)?;

        Ok(Some(session))
    }

    /// Update a session.
    pub fn update(&self, session: &Session) -> Result<(), rusqlite::Error> {
        let metadata_json = serde_json::to_string(&session.metadata).unwrap_or_default();
        
        self.conn.execute(
            "UPDATE sessions 
             SET title = ?2, system_prompt = ?3, model = ?4, metadata = ?5,
                 input_tokens = ?6, output_tokens = ?7, total_tokens = ?8, 
                 total_cost = ?9, updated_at = ?10, archived = ?11
             WHERE id = ?1",
            params![
                session.id,
                session.title,
                session.system_prompt,
                session.model,
                metadata_json,
                session.usage.input_tokens,
                session.usage.output_tokens,
                session.usage.total_tokens,
                session.usage.total_cost,
                session.updated_at.to_rfc3339(),
                if session.archived { 1 } else { 0 }
            ],
        )?;

        Ok(())
    }

    /// Delete a session.
    pub fn delete(&self, id: &str) -> Result<(), rusqlite::Error> {
        self.conn.execute("DELETE FROM sessions WHERE id = ?1", params![id])?;
        Ok(())
    }

    /// List all sessions.
    pub fn list(&self, limit: Option<u32>, offset: Option<u32>) -> Result<Vec<Session>, rusqlite::Error> {
        let mut stmt = self.conn.prepare(
            "SELECT id, title, model, created_at, updated_at, archived 
             FROM sessions 
             ORDER BY updated_at DESC 
             LIMIT ?1 OFFSET ?2"
        )?;

        let sessions = stmt.query_map(params![limit.unwrap_or(100), offset.unwrap_or(0)], |row| {
            Ok(Session {
                id: row.get(0)?,
                title: row.get(1)?,
                model: row.get(2)?,
                created_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(3)?)
                    .unwrap_or_else(|_| Utc::now().into())
                    .into(),
                updated_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(4)?)
                    .unwrap_or_else(|_| Utc::now().into())
                    .into(),
                archived: row.get::<_, i32>(5)? != 0,
                system_prompt: None,
                metadata: SessionMetadata::default(),
                usage: SessionUsage::default(),
                messages: Vec::new(),
            })
        })?;

        sessions.collect()
    }

    /// Insert a message.
    fn insert_message(&self, session_id: &str, message: &Message) -> Result<(), rusqlite::Error> {
        let parts_json = serde_json::to_string(&message.parts).unwrap_or_default();
        let tool_calls_json = message.tool_calls.as_ref()
            .and_then(|tc| serde_json::to_string(tc).ok());

        self.conn.execute(
            "INSERT INTO messages (id, session_id, role, parts, created_at, token_count, tool_calls, tool_call_id, parent_id)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                message.id,
                session_id,
                format!("{:?}", message.role).to_lowercase(),
                parts_json,
                message.created_at.to_rfc3339(),
                message.token_count,
                tool_calls_json,
                message.tool_call_id,
                message.parent_id
            ],
        )?;

        Ok(())
    }

    /// Get messages for a session.
    fn get_messages(&self, session_id: &str) -> Result<Vec<Message>, rusqlite::Error> {
        let mut stmt = self.conn.prepare(
            "SELECT id, role, parts, created_at, token_count, tool_calls, tool_call_id, parent_id
             FROM messages 
             WHERE session_id = ?1 
             ORDER BY created_at ASC"
        )?;

        let messages = stmt.query_map(params![session_id], |row| {
            let parts_json: String = row.get(2)?;
            let parts: Vec<MessagePart> = serde_json::from_str(&parts_json).unwrap_or_default();
            
            let tool_calls: Option<Vec<ToolCallInfo>> = row.get::<_, Option<String>>(5)?
                .and_then(|json| serde_json::from_str(&json).ok());

            Ok(Message {
                id: row.get(0)?,
                role: MessageRole::from_str(&row.get::<_, String>(1)?),
                parts,
                created_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(3)?)
                    .unwrap_or_else(|_| Utc::now().into())
                    .into(),
                token_count: row.get(4)?,
                tool_calls,
                tool_call_id: row.get(6)?,
                parent_id: row.get(7)?,
            })
        })?;

        messages.collect()
    }

    /// Add a message to a session.
    pub fn add_message(&self, session_id: &str, message: &Message) -> Result<(), rusqlite::Error> {
        self.insert_message(session_id, message)?;
        
        // Update session timestamp
        self.conn.execute(
            "UPDATE sessions SET updated_at = ?1 WHERE id = ?2",
            params![Utc::now().to_rfc3339(), session_id],
        )?;

        Ok(())
    }

    /// Archive a session.
    pub fn archive(&self, id: &str) -> Result<(), rusqlite::Error> {
        self.conn.execute(
            "UPDATE sessions SET archived = 1, updated_at = ?1 WHERE id = ?2",
            params![Utc::now().to_rfc3339(), id],
        )?;
        Ok(())
    }

    /// Get session count.
    pub fn count(&self) -> Result<u32, rusqlite::Error> {
        let count: u32 = self.conn.query_row(
            "SELECT COUNT(*) FROM sessions",
            [],
            |row| row.get(0)
        )?;
        Ok(count)
    }
}

impl MessageRole {
    fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "system" => Self::System,
            "user" => Self::User,
            "assistant" => Self::Assistant,
            "tool" => Self::Tool,
            _ => Self::User,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::types::Message;

    #[test]
    fn test_session_store() {
        let store = SessionStore::memory().unwrap();
        
        let mut session = Session::new("test-1".to_string(), "gpt-4".to_string());
        session.add_message(Message::user("Hello".to_string()));
        session.add_message(Message::assistant("Hi there!".to_string()));
        
        store.create(&session).unwrap();
        
        let loaded = store.get("test-1").unwrap().unwrap();
        assert_eq!(loaded.message_count(), 2);
        
        let sessions = store.list(None, None).unwrap();
        assert_eq!(sessions.len(), 1);
    }
}
