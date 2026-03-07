//! Google Vertex AI Provider — OAuth2 + regional endpoints
//!
//! Endpoints: https://{region}-aiplatform.googleapis.com (e.g. us-central1-aiplatform.googleapis.com).
//! Auth: OAuth2 (application default credentials or service account).
//! TODO: implement OAuth2 token flow and wire to Generative Language API format for Vertex.

use async_trait::async_trait;
use futures::stream::BoxStream;

use crate::providers::trait_def::{Provider, ProviderError, ProviderResult};
use crate::providers::types::{ChatRequest, ChatResponse, ChatChunk, ModelInfo};

/// Vertex AI provider (skeleton).
pub struct VertexProvider {
    region: String,
    project_id: String,
}

impl VertexProvider {
    pub fn new(region: &str, project_id: &str) -> Self {
        Self {
            region: region.to_string(),
            project_id: project_id.to_string(),
        }
    }

    fn base_url(&self) -> String {
        format!("https://{}-aiplatform.googleapis.com/v1", self.region)
    }
}

#[async_trait]
impl Provider for VertexProvider {
    fn name(&self) -> &'static str {
        "vertex"
    }

    fn display_name(&self) -> &'static str {
        "Google Vertex AI"
    }

    async fn models(&self) -> ProviderResult<Vec<ModelInfo>> {
        let _ = self.base_url();
        Err(ProviderError::Internal(
            "Vertex AI provider not yet implemented; use Google (Gemini) provider".to_string(),
        ))
    }

    async fn chat(&self, _request: ChatRequest) -> ProviderResult<ChatResponse> {
        Err(ProviderError::Internal(
            "Vertex AI provider not yet implemented".to_string(),
        ))
    }

    async fn chat_stream(&self, _request: ChatRequest) -> ProviderResult<BoxStream<'static, ProviderResult<ChatChunk>>> {
        Err(ProviderError::Internal(
            "Vertex AI provider not yet implemented".to_string(),
        ))
    }
}
