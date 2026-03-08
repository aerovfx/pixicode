//! Project route handlers — parity with TS ProjectRoutes (list, current, patch).

use axum::{
    extract::{Extension, Path, State},
    Json,
};
use rusqlite::params;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::server::error::{ApiError, ApiResult};
use crate::server::middleware::WorkspaceCtx;
use crate::server::state::AppState;

#[derive(Serialize)]
pub struct ProjectInfo {
    pub id: String,
    pub worktree: String,
    pub vcs: Option<String>,
    pub name: Option<String>,
    pub icon_url: Option<String>,
    pub icon_color: Option<String>,
    pub commands: Option<String>,
    pub time_created: i64,
    pub time_updated: i64,
    pub time_initialized: Option<i64>,
    pub sandboxes: Option<String>,
}

#[derive(Deserialize)]
pub struct ProjectUpdate {
    pub name: Option<String>,
    pub icon_url: Option<String>,
    pub icon_color: Option<String>,
    pub commands: Option<String>,
}

fn row_to_project(row: &rusqlite::Row<'_>) -> rusqlite::Result<ProjectInfo> {
    Ok(ProjectInfo {
        id: row.get(0)?,
        worktree: row.get(1)?,
        vcs: row.get(2)?,
        name: row.get(3)?,
        icon_url: row.get(4)?,
        icon_color: row.get(5)?,
        commands: row.get(6)?,
        time_created: row.get(7)?,
        time_updated: row.get(8)?,
        time_initialized: row.get(9)?,
        sandboxes: row.get(10)?,
    })
}

pub async fn list(State(s): State<Arc<AppState>>) -> ApiResult<Json<Vec<ProjectInfo>>> {
    let list = s.db.with(|conn| {
        let mut stmt = conn.prepare(
            "SELECT id, worktree, vcs, name, icon_url, icon_color, commands,
                    time_created, time_updated, time_initialized, sandboxes
             FROM project ORDER BY time_updated DESC",
        )?;
        let rows = stmt.query_map([], row_to_project)?;
        rows.collect::<rusqlite::Result<Vec<_>>>().map_err(Into::into)
    }).map_err(|e| ApiError::internal(e.to_string()))?;
    Ok(Json(list))
}

pub async fn current(
    State(s): State<Arc<AppState>>,
    Extension(ctx): Extension<WorkspaceCtx>,
) -> ApiResult<Json<ProjectInfo>> {
    let dir = ctx.directory;
    let project = s.db.with(|conn| {
        conn.query_row(
            "SELECT id, worktree, vcs, name, icon_url, icon_color, commands,
                    time_created, time_updated, time_initialized, sandboxes
             FROM project WHERE worktree = ?1
             ORDER BY time_updated DESC LIMIT 1",
            params![dir],
            row_to_project,
        )
        .or_else(|_| {
            conn.query_row(
                "SELECT id, worktree, vcs, name, icon_url, icon_color, commands,
                        time_created, time_updated, time_initialized, sandboxes
                 FROM project ORDER BY time_updated DESC LIMIT 1",
                [],
                row_to_project,
            )
        })
        .map_err(Into::into)
    }).map_err(|e: anyhow::Error| ApiError::not_found(e.to_string()))?;
    Ok(Json(project))
}

pub async fn patch(
    State(s): State<Arc<AppState>>,
    Path(project_id): Path<String>,
    Json(body): Json<ProjectUpdate>,
) -> ApiResult<Json<ProjectInfo>> {
    let now = chrono::Utc::now().timestamp();
    s.db.with(|conn| {
        if let Some(ref v) = body.name {
            conn.execute("UPDATE project SET name = ?1, time_updated = ?2 WHERE id = ?3", params![v, now, project_id])?;
        }
        if let Some(ref v) = body.icon_url {
            conn.execute("UPDATE project SET icon_url = ?1, time_updated = ?2 WHERE id = ?3", params![v, now, project_id])?;
        }
        if let Some(ref v) = body.icon_color {
            conn.execute("UPDATE project SET icon_color = ?1, time_updated = ?2 WHERE id = ?3", params![v, now, project_id])?;
        }
        if let Some(ref v) = body.commands {
            conn.execute("UPDATE project SET commands = ?1, time_updated = ?2 WHERE id = ?3", params![v, now, project_id])?;
        }
        Ok(())
    }).map_err(|e| ApiError::internal(e.to_string()))?;
    get_one(s, &project_id).await
}

async fn get_one(s: Arc<AppState>, project_id: &str) -> ApiResult<Json<ProjectInfo>> {
    let project = s.db.with(|conn| {
        conn.query_row(
            "SELECT id, worktree, vcs, name, icon_url, icon_color, commands,
                    time_created, time_updated, time_initialized, sandboxes
             FROM project WHERE id = ?1",
            params![project_id],
            row_to_project,
        )
        .map_err(Into::into)
    }).map_err(|_| ApiError::not_found("project not found"))?;
    Ok(Json(project))
}
