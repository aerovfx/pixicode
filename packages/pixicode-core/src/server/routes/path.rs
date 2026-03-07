//! Path info route handler.
use axum::extract::State;
use axum::Json;
use std::sync::Arc;
use crate::server::error::ApiResult;
use crate::server::state::AppState;

pub async fn info(State(_s): State<Arc<AppState>>) -> ApiResult<Json<serde_json::Value>> {
    let cwd = std::env::current_dir().unwrap_or_default();
    Ok(Json(serde_json::json!({
        "cwd": cwd.display().to_string(),
        "home": dirs::home_dir().map(|p| p.display().to_string()),
    })))
}
