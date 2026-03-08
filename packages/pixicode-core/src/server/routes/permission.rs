//! Permission route handlers — read/write permission data per project.

use axum::{extract::{Path, State}, Json};
use rusqlite::params;
use serde::Deserialize;
use std::sync::Arc;

use crate::server::error::{ApiError, ApiResult};
use crate::server::state::{AppState, PermissionReply};

#[derive(Deserialize)]
pub struct GrantRequest {
    pub project_id: String,
    pub tool: String,
    pub action: String,
}

#[derive(Deserialize)]
pub struct ReplyBody {
    pub reply: String,
    pub message: Option<String>,
}

pub async fn get(State(s): State<Arc<AppState>>) -> ApiResult<Json<serde_json::Value>> {
    let perms = s.db.with(|conn| {
        let mut stmt = conn.prepare(
            "SELECT project_id, data FROM permission"
        )?;
        let rows = stmt.query_map([], |row| {
            let project_id: String = row.get(0)?;
            let data_str: String = row.get(1)?;
            let data: serde_json::Value = serde_json::from_str(&data_str).unwrap_or_default();
            Ok((project_id, data))
        })?;
        let mut map = serde_json::Map::new();
        for row in rows {
            let (pid, data) = row?;
            map.insert(pid, data);
        }
        Ok(serde_json::Value::Object(map))
    }).map_err(|e: anyhow::Error| ApiError::internal(e.to_string()))?;

    Ok(Json(perms))
}

pub async fn grant(
    State(s): State<Arc<AppState>>,
    Json(body): Json<GrantRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    let now = chrono::Utc::now().timestamp();

    s.db.transaction(|conn| {
        // Load existing data or start fresh
        let existing: Option<String> = conn.query_row(
            "SELECT data FROM permission WHERE project_id = ?1",
            params![body.project_id],
            |row| row.get(0),
        ).ok();

        let mut data: serde_json::Value = existing
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_else(|| serde_json::json!({}));

        data[&body.tool] = serde_json::json!(body.action);
        let data_str = serde_json::to_string(&data).unwrap();

        conn.execute(
            "INSERT INTO permission (project_id, time_created, time_updated, data)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(project_id) DO UPDATE SET data = ?4, time_updated = ?3",
            params![body.project_id, now, now, data_str],
        )?;
        Ok(())
    }).map_err(|e| ApiError::internal(e.to_string()))?;

    Ok(Json(serde_json::json!({ "ok": true })))
}

pub async fn revoke(
    State(s): State<Arc<AppState>>,
    Path(tool): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    let now = chrono::Utc::now().timestamp();

    // Revoke from all projects
    s.db.transaction(|conn| {
        let mut stmt = conn.prepare("SELECT project_id, data FROM permission")?;
        let rows: Vec<(String, String)> = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?.collect::<rusqlite::Result<Vec<_>>>()?;

        for (pid, data_str) in rows {
            if let Ok(mut data) = serde_json::from_str::<serde_json::Value>(&data_str) {
                if let Some(obj) = data.as_object_mut() {
                    if obj.remove(&tool).is_some() {
                        let updated = serde_json::to_string(&data).unwrap();
                        conn.execute(
                            "UPDATE permission SET data = ?1, time_updated = ?2 WHERE project_id = ?3",
                            params![updated, now, pid],
                        )?;
                    }
                }
            }
        }
        Ok(())
    }).map_err(|e| ApiError::internal(e.to_string()))?;

    Ok(Json(serde_json::json!({ "ok": true })))
}

/// POST /permission/:request_id/reply — store reply for a permission request (Phase 1 parity with TS).
pub async fn reply(
    State(s): State<Arc<AppState>>,
    Path(request_id): Path<String>,
    Json(body): Json<ReplyBody>,
) -> ApiResult<Json<serde_json::Value>> {
    let reply = PermissionReply {
        reply: body.reply,
        message: body.message,
    };
    s.permission_replies.write().await.insert(request_id, reply);
    Ok(Json(serde_json::json!({ "ok": true })))
}
