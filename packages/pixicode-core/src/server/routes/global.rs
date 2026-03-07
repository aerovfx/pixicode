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
