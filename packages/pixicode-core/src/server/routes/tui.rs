//! TUI-specific route handlers.
use axum::{extract::State, Json};
use std::sync::Arc;
use crate::server::error::ApiResult;
use crate::server::state::AppState;

pub async fn get_theme(State(s): State<Arc<AppState>>) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(serde_json::json!({ "theme": s.config.theme })))
}
pub async fn set_theme(State(_s): State<Arc<AppState>>, Json(_b): Json<serde_json::Value>) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(serde_json::json!({ "ok": true })))
}
pub async fn get_keybinds(State(s): State<Arc<AppState>>) -> ApiResult<Json<serde_json::Value>> {
    Ok(Json(serde_json::to_value(&s.config.keybinds).unwrap()))
}
