//! MCP management route handlers.
use axum::{extract::{Path, State}, Json};
use serde::Deserialize;
use std::sync::Arc;
use crate::server::error::ApiResult;
use crate::server::state::AppState;

pub async fn list(State(s): State<Arc<AppState>>) -> ApiResult<Json<serde_json::Value>> {
    let keys: Vec<_> = s.config.mcp.keys().collect();
    Ok(Json(serde_json::to_value(keys).unwrap()))
}
pub async fn add(State(_s): State<Arc<AppState>>, Json(_b): Json<serde_json::Value>) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(serde_json::json!({ "ok": true })))
}
pub async fn remove(State(_s): State<Arc<AppState>>, Path(_name): Path<String>) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(serde_json::json!({ "ok": true })))
}
pub async fn enable(State(_s): State<Arc<AppState>>, Path(_name): Path<String>) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(serde_json::json!({ "ok": true })))
}
pub async fn disable(State(_s): State<Arc<AppState>>, Path(_name): Path<String>) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(serde_json::json!({ "ok": true })))
}
