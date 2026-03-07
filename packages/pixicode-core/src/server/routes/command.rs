//! Command listing route handler.
use axum::extract::State;
use axum::Json;
use std::sync::Arc;
use crate::server::error::ApiResult;
use crate::server::state::AppState;

pub async fn list(State(_s): State<Arc<AppState>>) -> ApiResult<Json<Vec<serde_json::Value>>> {
    Ok(Json(vec![]))
}
