//! Session route handlers — CRUD for sessions and messages.

use axum::{extract::{Path, State}, Json};
use rusqlite::params;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::bus::BusEvent;
use crate::server::error::{ApiError, ApiResult};
use crate::server::state::AppState;

#[derive(Serialize)]
pub struct SessionResponse {
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
    pub time_created: i64,
    pub time_updated: i64,
    pub time_archived: Option<i64>,
}

#[derive(Deserialize)]
pub struct CreateSessionRequest {
    pub title: Option<String>,
    pub directory: String,
    pub project_id: Option<String>,
}

#[derive(Deserialize)]
pub struct CreateMessageRequest {
    pub content: String,
    pub role: Option<String>,
}

#[derive(Serialize)]
pub struct MessageResponse {
    pub id: String,
    pub session_id: String,
    pub time_created: i64,
    pub time_updated: i64,
    pub data: serde_json::Value,
}

fn gen_id(prefix: &str) -> String {
    format!("{}_{}", prefix, ulid::Ulid::new().to_string().to_lowercase())
}

fn now_ts() -> i64 {
    chrono::Utc::now().timestamp()
}

fn slugify(s: &str) -> String {
    s.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .trim_matches('-')
        .to_string()
}

fn row_to_session(row: &rusqlite::Row<'_>) -> rusqlite::Result<SessionResponse> {
    Ok(SessionResponse {
        id: row.get(0)?,
        project_id: row.get(1)?,
        parent_id: row.get(2)?,
        workspace_id: row.get(3)?,
        slug: row.get(4)?,
        directory: row.get(5)?,
        title: row.get(6)?,
        version: row.get(7)?,
        share_url: row.get(8)?,
        summary_additions: row.get(9)?,
        summary_deletions: row.get(10)?,
        summary_files: row.get(11)?,
        time_created: row.get(12)?,
        time_updated: row.get(13)?,
        time_archived: row.get(14)?,
    })
}

const SESSION_COLS: &str =
    "id, project_id, parent_id, workspace_id, slug, directory, title, \
     version, share_url, summary_additions, summary_deletions, \
     summary_files, time_created, time_updated, time_archived";

pub async fn list(State(s): State<Arc<AppState>>) -> ApiResult<Json<Vec<SessionResponse>>> {
    let sessions = s.db.with(|conn| {
        let mut stmt = conn.prepare(&format!(
            "SELECT {} FROM session ORDER BY time_updated DESC", SESSION_COLS
        ))?;
        let rows = stmt.query_map([], |row| row_to_session(row))?;
        rows.collect::<rusqlite::Result<Vec<_>>>().map_err(Into::into)
    }).map_err(|e| ApiError::internal(e.to_string()))?;

    Ok(Json(sessions))
}

pub async fn create(
    State(s): State<Arc<AppState>>,
    Json(body): Json<CreateSessionRequest>,
) -> ApiResult<Json<SessionResponse>> {
    let now = now_ts();
    let id = gen_id("sess");
    let title = body.title.unwrap_or_else(|| "New Session".to_string());
    let project_id = body.project_id.unwrap_or_else(|| "default".to_string());
    let slug = slugify(&title);
    let version = env!("CARGO_PKG_VERSION").to_string();

    let resp = SessionResponse {
        id: id.clone(),
        project_id: project_id.clone(),
        parent_id: None,
        workspace_id: None,
        slug: slug.clone(),
        directory: body.directory.clone(),
        title: title.clone(),
        version: version.clone(),
        share_url: None,
        summary_additions: None,
        summary_deletions: None,
        summary_files: None,
        time_created: now,
        time_updated: now,
        time_archived: None,
    };

    s.db.transaction(|conn| {
        conn.execute(
            "INSERT OR IGNORE INTO project (id, worktree, time_created, time_updated, sandboxes)
             VALUES (?1, ?2, ?3, ?4, '[]')",
            params![project_id, body.directory, now, now],
        )?;
        conn.execute(
            "INSERT INTO session (id, project_id, slug, directory, title, version, time_created, time_updated)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![id, project_id, slug, body.directory, title, version, now, now],
        )?;
        Ok(())
    }).map_err(|e| ApiError::internal(e.to_string()))?;

    s.bus.publish(BusEvent::SessionCreated {
        session_id: resp.id.clone(),
        title: resp.title.clone(),
    });

    Ok(Json(resp))
}

pub async fn get(
    State(s): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> ApiResult<Json<SessionResponse>> {
    s.db.with(|conn| {
        conn.query_row(
            &format!("SELECT {} FROM session WHERE id = ?1", SESSION_COLS),
            params![id],
            |row| row_to_session(row),
        ).map_err(Into::into)
    }).map(Json).map_err(|_| ApiError::not_found("session not found"))
}

pub async fn delete_session(
    State(s): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    let deleted = s.db.with(|conn| {
        conn.execute("DELETE FROM session WHERE id = ?1", params![id]).map_err(Into::into)
    }).map_err(|e: anyhow::Error| ApiError::internal(e.to_string()))?;

    if deleted == 0 {
        return Err(ApiError::not_found("session not found"));
    }

    s.bus.publish(BusEvent::SessionDeleted { session_id: id });
    Ok(Json(serde_json::json!({ "ok": true })))
}

pub async fn list_messages(
    State(s): State<Arc<AppState>>,
    Path(session_id): Path<String>,
) -> ApiResult<Json<Vec<MessageResponse>>> {
    let messages = s.db.with(|conn| {
        let mut stmt = conn.prepare(
            "SELECT id, session_id, time_created, time_updated, data
             FROM message WHERE session_id = ?1 ORDER BY time_created ASC"
        )?;
        let rows = stmt.query_map(params![session_id], |row| {
            let data_str: String = row.get(4)?;
            let data: serde_json::Value = serde_json::from_str(&data_str).unwrap_or_default();
            Ok(MessageResponse {
                id: row.get(0)?,
                session_id: row.get(1)?,
                time_created: row.get(2)?,
                time_updated: row.get(3)?,
                data,
            })
        })?;
        rows.collect::<rusqlite::Result<Vec<_>>>().map_err(Into::into)
    }).map_err(|e| ApiError::internal(e.to_string()))?;

    Ok(Json(messages))
}

pub async fn create_message(
    State(s): State<Arc<AppState>>,
    Path(session_id): Path<String>,
    Json(body): Json<CreateMessageRequest>,
) -> ApiResult<Json<MessageResponse>> {
    let now = now_ts();
    let id = gen_id("msg");
    let role = body.role.unwrap_or_else(|| "user".to_string());

    let data = serde_json::json!({
        "role": role,
        "content": body.content,
    });
    let data_str = serde_json::to_string(&data).unwrap();

    s.db.transaction(|conn| {
        conn.execute(
            "INSERT INTO message (id, session_id, time_created, time_updated, data)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![id, session_id, now, now, data_str],
        )?;
        conn.execute(
            "UPDATE session SET time_updated = ?1 WHERE id = ?2",
            params![now, session_id],
        )?;
        Ok(())
    }).map_err(|e| ApiError::internal(e.to_string()))?;

    s.bus.publish(BusEvent::MessageCreated {
        session_id: session_id.clone(),
        message_id: id.clone(),
    });

    Ok(Json(MessageResponse {
        id,
        session_id,
        time_created: now,
        time_updated: now,
        data,
    }))
}

pub async fn abort(
    State(s): State<Arc<AppState>>,
    Path(session_id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    s.bus.publish(BusEvent::SessionUpdated { session_id });
    Ok(Json(serde_json::json!({ "ok": true })))
}
