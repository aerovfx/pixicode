//! Global state route handlers.
use axum::{extract::State, Json};
use std::sync::Arc;
use crate::server::error::ApiResult;
use crate::server::state::AppState;

pub async fn get(State(_s): State<Arc<AppState>>) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(serde_json::json!({})))
}
pub async fn update(State(_s): State<Arc<AppState>>, Json(_b): Json<serde_json::Value>) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(serde_json::json!({ "ok": true })))
}

/// GET /global/health — health check endpoint for monitoring.
pub async fn health(State(s): State<Arc<AppState>>) -> ApiResult<Json<serde_json::Value>> {
    let tool_count = s.tool_registry.len();
    let uptime = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    Ok(Json(serde_json::json!({
        "status": "ok",
        "version": env!("CARGO_PKG_VERSION"),
        "tools_registered": tool_count,
        "timestamp": uptime,
    })))
}
