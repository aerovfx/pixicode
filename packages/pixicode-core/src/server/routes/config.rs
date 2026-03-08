//! Config route handlers.

use axum::{extract::{Extension, State}, Json};
use std::sync::Arc;

use crate::bus::BusEvent;
use crate::config::Config;
use crate::server::error::{ApiError, ApiResult};
use crate::server::middleware::WorkspaceCtx;
use crate::server::state::AppState;

pub async fn get(State(s): State<Arc<AppState>>) -> ApiResult<Json<serde_json::Value>> {
    let cfg = serde_json::to_value(&s.config).map_err(|e| ApiError::internal(e.to_string()))?;
    Ok(Json(cfg))
}

/// PATCH /config — merge partial JSON into the project-level config file.
pub async fn update(
    State(s): State<Arc<AppState>>,
    Extension(ctx): Extension<WorkspaceCtx>,
    Json(body): Json<serde_json::Value>,
) -> ApiResult<Json<serde_json::Value>> {
    Config::patch_project_config(&ctx.directory, body)
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?;

    s.bus.publish(BusEvent::ConfigChanged);
    Ok(Json(serde_json::json!({ "ok": true })))
}

/// GET /config/providers — list provider IDs from config (parity with TS config.providers).
pub async fn get_providers(State(s): State<Arc<AppState>>) -> ApiResult<Json<Vec<String>>> {
    let ids: Vec<String> = s.config.providers.keys().cloned().collect();
    Ok(Json(ids))
}
