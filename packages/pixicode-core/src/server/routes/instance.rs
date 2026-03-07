//! Instance lifecycle route handlers.

use axum::extract::State;
use axum::Json;
use std::sync::Arc;

use crate::bus::BusEvent;
use crate::server::error::ApiResult;
use crate::server::state::AppState;

pub async fn dispose(State(s): State<Arc<AppState>>) -> ApiResult<Json<serde_json::Value>> {
    let cwd = std::env::current_dir()
        .unwrap_or_default()
        .display()
        .to_string();

    tracing::info!(directory = %cwd, "instance dispose requested");

    s.bus.publish(BusEvent::InstanceDisposed { directory: cwd });

    Ok(Json(serde_json::json!({ "ok": true })))
}
