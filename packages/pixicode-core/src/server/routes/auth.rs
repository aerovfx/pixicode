//! Auth (credential management) route handlers.
use axum::{extract::{Path, State}, Json};
use std::sync::Arc;
use crate::server::error::ApiResult;
use crate::server::state::AppState;

pub async fn list(State(s): State<Arc<AppState>>) -> ApiResult<Json<Vec<String>>> {
    Ok(Json(s.config.providers.keys().cloned().collect()))
}
pub async fn set(
    State(_s): State<Arc<AppState>>,
    Path(_provider): Path<String>,
    Json(_b): Json<serde_json::Value>,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(serde_json::json!({ "ok": true })))
}
pub async fn remove(
    State(_s): State<Arc<AppState>>,
    Path(_provider): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(serde_json::json!({ "ok": true })))
}
