//! Config route handlers.

use axum::{extract::State, Json};
use std::sync::Arc;

use crate::server::error::{ApiError, ApiResult};
use crate::server::state::AppState;

pub async fn get(State(s): State<Arc<AppState>>) -> ApiResult<Json<serde_json::Value>> {
    let cfg = serde_json::to_value(&s.config).map_err(|e| ApiError::internal(e.to_string()))?;
    Ok(Json(cfg))
}

pub async fn update(
    State(_s): State<Arc<AppState>>,
    Json(_body): Json<serde_json::Value>,
) -> ApiResult<Json<serde_json::Value>> {
    Err(ApiError::internal("config writes not yet implemented"))
}
