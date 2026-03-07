//! Typed row models for every database table.
//!
//! These structs derive `serde::{Serialize, Deserialize}` so they play nicely
//! with the Axum JSON responses, and provide named-field access to DB rows.

use serde::{Deserialize, Serialize};

// ─────────────────────────────────────────────────────────────────────────────
//  project
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: String,
    pub worktree: String,
    pub vcs: Option<String>,
    pub name: Option<String>,
    pub icon_url: Option<String>,
    pub icon_color: Option<String>,
    pub commands: Option<String>,   // JSON blob
    pub time_created: i64,
    pub time_updated: i64,
    pub time_initialized: Option<i64>,
    pub sandboxes: String,          // JSON array blob
}

// ─────────────────────────────────────────────────────────────────────────────
//  workspace
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workspace {
    pub id: String,
    pub branch: Option<String>,
    pub project_id: String,
    pub r#type: String,             // "local" | "remote" | etc.
    pub name: Option<String>,
    pub directory: Option<String>,
    pub extra: Option<String>,      // JSON blob
}

// ─────────────────────────────────────────────────────────────────────────────
//  session
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub project_id: String,
    pub parent_id: Option<String>,
    pub workspace_id: Option<String>,
    pub slug: String,
    pub directory: String,
    pub title: String,
    pub version: String,
    pub share_url: Option<String>,
    pub summary_additions: Option<i64>,
    pub summary_deletions: Option<i64>,
    pub summary_files: Option<i64>,
    pub summary_diffs: Option<String>,  // JSON blob
    pub revert: Option<String>,
    pub permission: Option<String>,     // JSON blob
    pub time_created: i64,
    pub time_updated: i64,
    pub time_compacting: Option<i64>,
    pub time_archived: Option<i64>,
}

// ─────────────────────────────────────────────────────────────────────────────
//  message
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: String,
    pub session_id: String,
    pub time_created: i64,
    pub time_updated: i64,
    pub data: String,               // JSON blob
}

// ─────────────────────────────────────────────────────────────────────────────
//  part
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Part {
    pub id: String,
    pub message_id: String,
    pub session_id: String,
    pub time_created: i64,
    pub time_updated: i64,
    pub data: String,               // JSON blob
}

// ─────────────────────────────────────────────────────────────────────────────
//  permission
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Permission {
    pub project_id: String,
    pub time_created: i64,
    pub time_updated: i64,
    pub data: String,               // JSON blob
}

// ─────────────────────────────────────────────────────────────────────────────
//  todo
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Todo {
    pub session_id: String,
    pub content: String,
    pub status: String,
    pub priority: String,
    pub position: i64,
    pub time_created: i64,
    pub time_updated: i64,
}

// ─────────────────────────────────────────────────────────────────────────────
//  session_share
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionShare {
    pub session_id: String,
    pub id: String,
    pub secret: String,
    pub url: String,
    pub time_created: i64,
    pub time_updated: i64,
}

// ─────────────────────────────────────────────────────────────────────────────
//  control_account
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControlAccount {
    pub email: String,
    pub url: String,
    pub access_token: String,
    pub refresh_token: String,
    pub token_expiry: Option<i64>,
    pub active: i64,                // SQLite bool (0/1)
    pub time_created: i64,
    pub time_updated: i64,
}
