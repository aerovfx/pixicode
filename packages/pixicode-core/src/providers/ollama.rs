//! Ollama Provider — Local LLM provider

use async_stream::stream;
use async_trait::async_trait;
use futures::stream::{BoxStream, StreamExt};
use reqwest::{Client, Url};
use serde::{Deserialize, Serialize};

use crate::providers::trait_def::{Provider, ProviderError, ProviderResult};
use crate::providers::types::{
    ChatRequest, ChatResponse, ChatChunk, Message, MessageDelta, MessageRole,
    ModelInfo, ModelCapabilities, Usage, FinishReason,
};

/// Ollama provider.
pub struct OllamaProvider {
    client: Client,
    base_url: Url,
}

impl OllamaProvider {
    pub fn new() -> Self {
        Self::with_url("http://localhost:11434")
    }

    pub fn with_url(url: &str) -> Self {
        Self {
            client: Client::new(),
            base_url: Url::parse(url).unwrap_or_else(|_| Url::parse("http://localhost:11434").unwrap()),
        }
    }

    pub fn with_base_url(base_url: impl Into<String>) -> Self {
        let url = base_url.into();
        Self::with_url(&url)
    }
}

impl Default for OllamaProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Serialize)]
struct OllamaChatRequest {
    model: String,
    messages: Vec<OllamaMessage>,
    stream: bool,
    options: Option<OllamaOptions>,
}

#[derive(Debug, Serialize, Deserialize)]
struct OllamaMessage {
    role: String,
    content: String,
}

#[derive(Debug, Serialize, Default)]
struct OllamaOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    num_predict: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_p: Option<f32>,
}

#[derive(Debug, Deserialize)]
struct OllamaChatResponse {
    model: String,
    message: OllamaMessage,
    done: bool,
    total_duration: Option<u64>,
    prompt_eval_count: Option<u32>,
    eval_count: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct OllamaModel {
    name: String,
    model: String,
    size: Option<u64>,
    digest: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OllamaModelsResponse {
    models: Vec<OllamaModel>,
}

#[async_trait]
impl Provider for OllamaProvider {
    fn name(&self) -> &'static str {
        "ollama"
    }

    fn display_name(&self) -> &'static str {
        "Ollama"
    }

    async fn models(&self) -> ProviderResult<Vec<ModelInfo>> {
        let url = self.base_url.join("/api/tags").unwrap();
        let response = self.client.get(url).send().await?;
        
        if !response.status().is_success() {
            return Err(ProviderError::InvalidRequest(
                format!("Failed to list models: {}", response.status())
            ));
        }

        let models_response: OllamaModelsResponse = response.json().await?;
        
        let models = models_response.models.into_iter().map(|m| {
            let name = m.name.clone();
            ModelInfo {
                id: name.clone(),
                name: Some(name.clone()),
                description: None,
                context_window: Some(4096), // Default, Ollama doesn't provide this
                max_output_tokens: None,
                capabilities: ModelCapabilities {
                    functions: false, // Ollama doesn't support function calling yet
                    vision: name.contains("llava") || name.contains("vision"),
                    streaming: true,
                    json_mode: false,
                },
                pricing: None, // Local models are free
            }
        }).collect();

        Ok(models)
    }

    async fn chat(&self, request: ChatRequest) -> ProviderResult<ChatResponse> {
        let url = self.base_url.join("/api/chat").unwrap();
        
        let ollama_request = OllamaChatRequest {
            model: request.model,
            messages: request.messages.into_iter().map(|m| OllamaMessage {
                role: format!("{:?}", m.role).to_lowercase(),
                content: m.content,
            }).collect(),
            stream: false,
            options: Some(OllamaOptions {
                temperature: request.temperature,
                num_predict: request.max_tokens,
                top_p: request.top_p,
            }),
        };

        let response = self.client.post(url).json(&ollama_request).send().await?;
        
        if !response.status().is_success() {
            let error = response.text().await.unwrap_or_default();
            return Err(ProviderError::InvalidRequest(format!("Ollama API error: {}", error)));
        }

        let ollama_response: OllamaChatResponse = response.json().await?;

        Ok(ChatResponse {
            model: ollama_response.model,
            message: Message {
                role: MessageRole::Assistant,
                content: ollama_response.message.content,
                name: None,
                tool_call_id: None,
                tool_calls: None,
            },
            finish_reason: FinishReason::Stop,
            usage: Some(Usage {
                input_tokens: ollama_response.prompt_eval_count.unwrap_or(0),
                output_tokens: ollama_response.eval_count.unwrap_or(0),
                total_tokens: ollama_response.prompt_eval_count.unwrap_or(0) + ollama_response.eval_count.unwrap_or(0),
                input_token_details: None,
            }),
            id: None,
            created_at: None,
        })
    }

    async fn chat_stream(&self, request: ChatRequest) -> ProviderResult<BoxStream<'static, ProviderResult<ChatChunk>>> {
        let url = self.base_url.join("/api/chat").unwrap();
        
        let ollama_request = OllamaChatRequest {
            model: request.model.clone(),
            messages: request.messages.into_iter().map(|m| OllamaMessage {
                role: format!("{:?}", m.role).to_lowercase(),
                content: m.content,
            }).collect(),
            stream: true,
            options: Some(OllamaOptions {
                temperature: request.temperature,
                num_predict: request.max_tokens,
                top_p: request.top_p,
            }),
        };

        let response = self.client.post(url).json(&ollama_request).send().await?;
        
        if !response.status().is_success() {
            let error = response.text().await.unwrap_or_default();
            return Err(ProviderError::InvalidRequest(format!("Ollama API error: {}", error)));
        }

        let model = request.model;
        
        // Use response.chunk() in a loop instead of bytes_stream()
        let stream = stream! {
            let mut response = response;
            loop {
                match response.chunk().await {
                    Ok(Some(chunk)) => {
                        let text = String::from_utf8_lossy(&chunk);
                        // Ollama streams newline-delimited JSON
                        let mut chunk_result = ChatChunk {
                            model: model.clone(),
                            delta: MessageDelta::default(),
                            finish_reason: None,
                            usage: None,
                            index: None,
                        };

                        // Try to parse as JSON
                        if let Ok(ollama_response) = serde_json::from_str::<OllamaChatResponse>(&text) {
                            chunk_result.delta.content = Some(ollama_response.message.content);
                            if ollama_response.done {
                                chunk_result.finish_reason = Some(FinishReason::Stop);
                                chunk_result.usage = Some(Usage {
                                    input_tokens: ollama_response.prompt_eval_count.unwrap_or(0),
                                    output_tokens: ollama_response.eval_count.unwrap_or(0),
                                    total_tokens: ollama_response.prompt_eval_count.unwrap_or(0) + ollama_response.eval_count.unwrap_or(0),
                                    input_token_details: None,
                                });
                            }
                            yield Ok(chunk_result);
                        }
                    }
                    Ok(None) => break,
                    Err(e) => {
                        yield Err(ProviderError::Network(e));
                        break;
                    }
                }
            }
        };

        Ok(stream.boxed())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_name() {
        let provider = OllamaProvider::new();
        assert_eq!(provider.name(), "ollama");
        assert_eq!(provider.display_name(), "Ollama");
    }

    #[tokio::test]
    async fn test_models_connection() {
        // This test requires Ollama to be running locally
        let provider = OllamaProvider::new();
        let result = provider.models().await;
        
        // Test will fail if Ollama is not running, which is expected
        // Just verify the error type is appropriate
        if let Err(e) = result {
            assert!(matches!(e, ProviderError::Network(_) | ProviderError::InvalidRequest(_)));
        }
    }
}
