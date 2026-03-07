//! File operation route handlers.
use axum::{extract::{Query, State}, Json};
use serde::Deserialize;
use std::sync::Arc;
use crate::server::error::{ApiError, ApiResult};
use crate::server::state::AppState;

#[derive(Deserialize)]
pub struct FileQuery { pub path: String }

pub async fn read_file(
    State(_s): State<Arc<AppState>>,
    Query(q): Query<FileQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    let content = tokio::fs::read_to_string(&q.path).await
        .map_err(|e| ApiError::not_found(e.to_string()))?;
    Ok(Json(serde_json::json!({ "path": q.path, "content": content })))
}

pub async fn write_file(
    State(_s): State<Arc<AppState>>,
    Json(body): Json<serde_json::Value>,
) -> ApiResult<Json<serde_json::Value>> {
    let path = body["path"].as_str().ok_or_else(|| ApiError::bad_request("missing path"))?;
    let content = body["content"].as_str().ok_or_else(|| ApiError::bad_request("missing content"))?;
    tokio::fs::write(path, content).await.map_err(|e| ApiError::internal(e.to_string()))?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

pub async fn ls(
    State(_s): State<Arc<AppState>>,
    Query(q): Query<FileQuery>,
) -> ApiResult<Json<Vec<String>>> {
    let mut entries = tokio::fs::read_dir(&q.path).await.map_err(|e| ApiError::not_found(e.to_string()))?;
    let mut names = vec![];
    while let Ok(Some(entry)) = entries.next_entry().await {
        names.push(entry.file_name().to_string_lossy().into_owned());
    }
    Ok(Json(names))
}
