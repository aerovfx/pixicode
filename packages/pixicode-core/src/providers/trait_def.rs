//! Provider trait — Core interface for all AI providers

use async_trait::async_trait;
use futures::stream::BoxStream;
use thiserror::Error;

use crate::providers::types::{
    ChatRequest, ChatResponse, ChatChunk, ModelInfo,
};

/// Provider error types.
#[derive(Debug, Error)]
pub enum ProviderError {
    #[error("Provider not found: {0}")]
    NotFound(String),

    #[error("Authentication failed: {0}")]
    AuthError(String),

    #[error("Rate limit exceeded: {0}")]
    RateLimit(String),

    #[error("Invalid request: {0}")]
    InvalidRequest(String),

    #[error("Model not found: {0}")]
    ModelNotFound(String),

    #[error("Context length exceeded: {0}")]
    ContextLengthExceeded(String),

    #[error("Content filter triggered: {0}")]
    ContentFilter(String),

    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Timeout after {0}ms")]
    Timeout(u64),

    #[error("Stream error: {0}")]
    StreamError(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

pub type ProviderResult<T> = Result<T, ProviderError>;

/// Core trait that all AI providers must implement.
#[async_trait]
pub trait Provider: Send + Sync {
    /// Returns the provider's unique name (e.g., "openai", "anthropic", "ollama").
    fn name(&self) -> &'static str;

    /// Returns a human-readable display name.
    fn display_name(&self) -> &'static str {
        self.name()
    }

    /// Lists available models from this provider.
    async fn models(&self) -> ProviderResult<Vec<ModelInfo>>;

    /// Gets info about a specific model.
    async fn model(&self, model_id: &str) -> ProviderResult<Option<ModelInfo>> {
        let models = self.models().await?;
        Ok(models.into_iter().find(|m| m.id == model_id))
    }

    /// Sends a chat completion request and returns the response.
    async fn chat(&self, request: ChatRequest) -> ProviderResult<ChatResponse>;

    /// Sends a chat completion request and returns a stream of chunks.
    async fn chat_stream(&self, request: ChatRequest) -> ProviderResult<BoxStream<'static, ProviderResult<ChatChunk>>>;

    /// Validates that the provider is properly configured.
    async fn validate(&self) -> ProviderResult<()> {
        // Default implementation just tries to list models
        self.models().await?;
        Ok(())
    }

    /// Calculates the cost of a request based on token usage.
    fn calculate_cost(&self, _model: &str, _usage: &crate::providers::types::Usage) -> Option<f64> {
        // Default implementation returns None (no cost tracking)
        None
    }
}

/// Provider configuration.
#[derive(Debug, Clone)]
pub struct ProviderConfig {
    /// Provider type
    pub provider_type: String,
    /// API key or credential
    pub api_key: Option<String>,
    /// Base URL for the API
    pub base_url: Option<String>,
    /// Custom headers to send with every request (e.g. x-api-key, Authorization override)
    pub headers: Option<std::collections::HashMap<String, String>>,
    /// Additional configuration options
    pub options: serde_json::Value,
}

impl ProviderConfig {
    pub fn new(provider_type: impl Into<String>) -> Self {
        Self {
            provider_type: provider_type.into(),
            api_key: None,
            base_url: None,
            headers: None,
            options: serde_json::Value::Object(serde_json::Map::new()),
        }
    }

    /// Add custom headers for this provider (e.g. from config).
    pub fn with_headers(mut self, headers: std::collections::HashMap<String, String>) -> Self {
        self.headers = Some(headers);
        self
    }

    pub fn with_api_key(mut self, api_key: impl Into<String>) -> Self {
        self.api_key = Some(api_key.into());
        self
    }

    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = Some(base_url.into());
        self
    }

    pub fn with_option(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        if let serde_json::Value::Object(ref mut map) = self.options {
            map.insert(key.into(), value);
        }
        self
    }

    pub fn get_option(&self, key: &str) -> Option<&serde_json::Value> {
        self.options.get(key)
    }

    pub fn get_option_string(&self, key: &str) -> Option<String> {
        self.options.get(key).and_then(|v| v.as_str().map(|s| s.to_string()))
    }
}
