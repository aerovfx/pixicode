//! Provider route handlers — list providers and their models.

use axum::{extract::{Path, State}, Json};
use serde::Serialize;
use std::sync::Arc;

use crate::server::error::{ApiError, ApiResult};
use crate::server::state::AppState;

#[derive(Serialize)]
pub struct ProviderInfo {
    pub id: String,
    pub name: String,
    pub enabled: bool,
    pub base_url: Option<String>,
    pub model_count: usize,
}

#[derive(Serialize)]
pub struct ModelInfo {
    pub id: String,
    pub temperature: Option<f64>,
    pub max_tokens: Option<u32>,
}

pub async fn list(State(s): State<Arc<AppState>>) -> ApiResult<Json<Vec<ProviderInfo>>> {
    let providers = s.config.providers
        .iter()
        .map(|(id, cfg)| ProviderInfo {
            id: id.clone(),
            name: id.clone(),
            enabled: !cfg.disabled,
            base_url: cfg.base_url.clone(),
            model_count: cfg.models.len(),
        })
        .collect();
    Ok(Json(providers))
}

pub async fn list_models(
    State(s): State<Arc<AppState>>,
    Path(provider_id): Path<String>,
) -> ApiResult<Json<Vec<ModelInfo>>> {
    let provider = s.config.providers.get(&provider_id)
        .ok_or_else(|| ApiError::not_found("provider not found"))?;

    let models: Vec<ModelInfo> = provider.models.iter().map(|model_name| {
        let full_id = format!("{}/{}", provider_id, model_name);
        let model_cfg = s.config.models.get(&full_id);
        ModelInfo {
            id: model_name.clone(),
            temperature: model_cfg.and_then(|m| m.temperature),
            max_tokens: model_cfg.and_then(|m| m.max_tokens),
        }
    }).collect();

    Ok(Json(models))
}
