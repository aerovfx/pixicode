//! Workspace route handlers — CRUD for workspaces.

use axum::{extract::{Path, State}, Json};
use rusqlite::params;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::server::error::{ApiError, ApiResult};
use crate::server::state::AppState;

#[derive(Serialize)]
pub struct WorkspaceResponse {
    pub id: String,
    pub branch: Option<String>,
    pub project_id: String,
    pub r#type: String,
    pub name: Option<String>,
    pub directory: Option<String>,
}

#[derive(Deserialize)]
pub struct CreateWorkspaceRequest {
    pub project_id: String,
    pub name: Option<String>,
    pub directory: Option<String>,
    pub branch: Option<String>,
    pub r#type: Option<String>,
}

pub async fn list(State(s): State<Arc<AppState>>) -> ApiResult<Json<Vec<WorkspaceResponse>>> {
    let workspaces = s.db.with(|conn| {
        let mut stmt = conn.prepare(
            "SELECT id, branch, project_id, type, name, directory FROM workspace ORDER BY id"
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(WorkspaceResponse {
                id: row.get(0)?,
                branch: row.get(1)?,
                project_id: row.get(2)?,
                r#type: row.get(3)?,
                name: row.get(4)?,
                directory: row.get(5)?,
            })
        })?;
        rows.collect::<rusqlite::Result<Vec<_>>>().map_err(Into::into)
    }).map_err(|e| ApiError::internal(e.to_string()))?;

    Ok(Json(workspaces))
}

pub async fn create(
    State(s): State<Arc<AppState>>,
    Json(body): Json<CreateWorkspaceRequest>,
) -> ApiResult<Json<WorkspaceResponse>> {
    let id = format!("ws_{}", ulid::Ulid::new().to_string().to_lowercase());
    let ws_type = body.r#type.unwrap_or_else(|| "local".to_string());

    s.db.with(|conn| {
        conn.execute(
            "INSERT INTO workspace (id, branch, project_id, type, name, directory)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![id, body.branch, body.project_id, ws_type, body.name, body.directory],
        ).map_err(Into::into)
    }).map_err(|e: anyhow::Error| ApiError::internal(e.to_string()))?;

    Ok(Json(WorkspaceResponse {
        id,
        branch: body.branch,
        project_id: body.project_id,
        r#type: ws_type,
        name: body.name,
        directory: body.directory,
    }))
}

pub async fn get(
    State(s): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> ApiResult<Json<WorkspaceResponse>> {
    s.db.with(|conn| {
        conn.query_row(
            "SELECT id, branch, project_id, type, name, directory FROM workspace WHERE id = ?1",
            params![id],
            |row| {
                Ok(WorkspaceResponse {
                    id: row.get(0)?,
                    branch: row.get(1)?,
                    project_id: row.get(2)?,
                    r#type: row.get(3)?,
                    name: row.get(4)?,
                    directory: row.get(5)?,
                })
            },
        ).map_err(Into::into)
    }).map(Json).map_err(|_: anyhow::Error| ApiError::not_found("workspace not found"))
}

pub async fn delete_workspace(
    State(s): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    let deleted = s.db.with(|conn| {
        conn.execute("DELETE FROM workspace WHERE id = ?1", params![id]).map_err(Into::into)
    }).map_err(|e: anyhow::Error| ApiError::internal(e.to_string()))?;

    if deleted == 0 {
        return Err(ApiError::not_found("workspace not found"));
    }
    Ok(Json(serde_json::json!({ "ok": true })))
}
