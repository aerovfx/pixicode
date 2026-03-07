//! Google Provider — Gemini API

use async_stream::stream;
use async_trait::async_trait;
use futures::stream::{BoxStream, StreamExt};
use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::providers::trait_def::{Provider, ProviderError, ProviderResult};
use crate::providers::types::{
    ChatRequest, ChatResponse, ChatChunk, Message, MessageRole,
    ModelInfo, ModelCapabilities, Usage, FinishReason,
};

/// Google provider.
pub struct GoogleProvider {
    client: Client,
    api_key: String,
    base_url: String,
}

impl GoogleProvider {
    pub fn new(api_key: &str) -> Self {
        Self {
            client: Client::new(),
            api_key: api_key.to_string(),
            base_url: "https://generativelanguage.googleapis.com/v1beta".to_string(),
        }
    }

    pub fn with_base_url(mut self, base_url: &str) -> Self {
        self.base_url = base_url.to_string();
        self
    }
}

#[derive(Debug, Serialize)]
struct GoogleChatRequest {
    contents: Vec<GoogleContent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    system_instruction: Option<GoogleSystemInstruction>,
    #[serde(skip_serializing_if = "Option::is_none")]
    generation_config: Option<GoogleGenerationConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<GoogleTool>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct GoogleContent {
    role: String,
    parts: Vec<GooglePart>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct GooglePart {
    text: String,
}

#[derive(Debug, Serialize)]
struct GoogleSystemInstruction {
    parts: Vec<GooglePart>,
}

#[derive(Debug, Serialize)]
struct GoogleGenerationConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_output_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stop_sequences: Option<Vec<String>>,
}

#[derive(Debug, Serialize)]
struct GoogleTool {
    function_declarations: Vec<GoogleFunctionDeclaration>,
}

#[derive(Debug, Serialize)]
struct GoogleFunctionDeclaration {
    name: String,
    description: String,
    parameters: serde_json::Value,
}

#[derive(Debug, Deserialize)]
struct GoogleChatResponse {
    candidates: Option<Vec<GoogleCandidate>>,
    usage_metadata: Option<GoogleUsageMetadata>,
    model_version: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GoogleCandidate {
    content: GoogleContent,
    finish_reason: Option<String>,
    index: Option<u32>,
}

#[derive(Debug, Deserialize, Clone)]
struct GoogleUsageMetadata {
    prompt_token_count: Option<u32>,
    candidates_token_count: Option<u32>,
    total_token_count: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct GoogleModel {
    name: String,
    display_name: Option<String>,
    description: Option<String>,
    input_token_limit: Option<u32>,
    output_token_limit: Option<u32>,
    supported_generation_methods: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
struct GoogleModelsResponse {
    models: Vec<GoogleModel>,
}

#[async_trait]
impl Provider for GoogleProvider {
    fn name(&self) -> &'static str {
        "google"
    }

    fn display_name(&self) -> &'static str {
        "Google AI"
    }

    async fn models(&self) -> ProviderResult<Vec<ModelInfo>> {
        let url = format!("{}/models?key={}", self.base_url, self.api_key);
        
        let response = self.client.get(&url).send().await?;
        
        if !response.status().is_success() {
            let error = response.text().await.unwrap_or_default();
            return Err(ProviderError::InvalidRequest(format!("Google API error: {}", error)));
        }

        let models_response: GoogleModelsResponse = response.json().await?;
        
        let models = models_response.models.into_iter().filter_map(|m| {
            // Extract model ID from full name (e.g., "models/gemini-pro" -> "gemini-pro")
            let model_id = m.name.strip_prefix("models/").unwrap_or(&m.name).to_string();
            
            // Only include generative models
            let methods = m.supported_generation_methods.as_ref()?;
            if !methods.iter().any(|m| m == "generateContent") {
                return None;
            }

            let capabilities = ModelCapabilities {
                functions: model_id.contains("gemini-1.5") || model_id.contains("gemini-pro"),
                vision: !model_id.contains("text"),
                streaming: true,
                json_mode: true,
            };

            Some(ModelInfo {
                id: model_id.clone(),
                name: m.display_name,
                description: m.description,
                context_window: m.input_token_limit,
                max_output_tokens: m.output_token_limit,
                capabilities,
                pricing: crate::providers::cost::get_model_pricing(&model_id),
            })
        }).collect();

        Ok(models)
    }

    async fn chat(&self, request: ChatRequest) -> ProviderResult<ChatResponse> {
        let model = &request.model;
        let url = format!("{}/models/{}:generateContent?key={}", self.base_url, model, self.api_key);
        
        // Extract system instruction
        let system_instruction = request.messages.iter()
            .find(|m| m.role == MessageRole::System)
            .map(|m| GoogleSystemInstruction {
                parts: vec![GooglePart { text: m.content.clone() }],
            });

        // Convert messages to Google format
        let contents: Vec<GoogleContent> = request.messages.into_iter()
            .filter(|m| m.role != MessageRole::System)
            .map(|m| GoogleContent {
                role: match m.role {
                    MessageRole::User => "user".to_string(),
                    MessageRole::Assistant => "model".to_string(),
                    _ => "user".to_string(),
                },
                parts: vec![GooglePart { text: m.content }],
            }).collect();

        let google_request = GoogleChatRequest {
            contents,
            system_instruction,
            generation_config: Some(GoogleGenerationConfig {
                temperature: request.temperature,
                max_output_tokens: request.max_tokens,
                top_p: request.top_p,
                stop_sequences: request.stop,
            }),
            tools: request.tools.map(|tools| vec![GoogleTool {
                function_declarations: tools.into_iter().map(|t| GoogleFunctionDeclaration {
                    name: t.name,
                    description: t.description,
                    parameters: t.parameters,
                }).collect(),
            }]),
        };

        let response = self.client.post(&url)
            .json(&google_request)
            .send()
            .await?;
        
        if !response.status().is_success() {
            let error = response.text().await.unwrap_or_default();
            return Err(ProviderError::InvalidRequest(format!("Google API error: {}", error)));
        }

        let google_response: GoogleChatResponse = response.json().await?;

        let candidate = google_response.candidates.and_then(|mut c| c.pop()).ok_or_else(|| {
            ProviderError::Internal("No candidates in response".to_string())
        })?;

        let content = candidate.content.parts.iter()
            .map(|p| p.text.clone())
            .collect::<Vec<_>>()
            .join("\n");

        let finish_reason = match candidate.finish_reason.as_deref() {
            Some("STOP") => FinishReason::Stop,
            Some("MAX_TOKENS") => FinishReason::Length,
            Some("SAFETY") => FinishReason::ContentFilter,
            Some("RECITATION") => FinishReason::Other,
            _ => FinishReason::Other,
        };

        Ok(ChatResponse {
            model: google_response.model_version.unwrap_or(model.clone()),
            message: Message {
                role: MessageRole::Assistant,
                content,
                name: None,
                tool_call_id: None,
                tool_calls: None,
            },
            finish_reason,
            usage: google_response.usage_metadata.map(|u| Usage {
                input_tokens: u.prompt_token_count.unwrap_or(0),
                output_tokens: u.candidates_token_count.unwrap_or(0),
                total_tokens: u.total_token_count.unwrap_or(0),
                input_token_details: None,
            }),
            id: None,
            created_at: None,
        })
    }

    async fn chat_stream(&self, request: ChatRequest) -> ProviderResult<BoxStream<'static, ProviderResult<ChatChunk>>> {
        let model = request.model.clone();
        let url = format!("{}/models/{}:streamGenerateContent?alt=sse&key={}", self.base_url, model, self.api_key);
        
        let system_instruction = request.messages.iter()
            .find(|m| m.role == MessageRole::System)
            .map(|m| GoogleSystemInstruction {
                parts: vec![GooglePart { text: m.content.clone() }],
            });

        let contents: Vec<GoogleContent> = request.messages.into_iter()
            .filter(|m| m.role != MessageRole::System)
            .map(|m| GoogleContent {
                role: match m.role {
                    MessageRole::User => "user".to_string(),
                    MessageRole::Assistant => "model".to_string(),
                    _ => "user".to_string(),
                },
                parts: vec![GooglePart { text: m.content }],
            }).collect();

        let google_request = GoogleChatRequest {
            contents,
            system_instruction,
            generation_config: Some(GoogleGenerationConfig {
                temperature: request.temperature,
                max_output_tokens: request.max_tokens,
                top_p: request.top_p,
                stop_sequences: request.stop,
            }),
            tools: None, // Simplified for streaming
        };

        let response = self.client.post(&url)
            .json(&google_request)
            .send()
            .await?;
        
        if !response.status().is_success() {
            let error = response.text().await.unwrap_or_default();
            return Err(ProviderError::InvalidRequest(format!("Google API error: {}", error)));
        }

        // Google returns SSE stream with data events
        let stream = stream! {
            let mut response = response;
            let model = model.clone();
            let mut buffer = String::new();
            loop {
                match response.chunk().await {
                    Ok(Some(chunk)) => {
                        let text = String::from_utf8_lossy(&chunk);
                        buffer.push_str(&text);
                        
                        // Process complete lines
                        while let Some(pos) = buffer.find("\n\n") {
                            let event = buffer[..pos].to_string();
                            buffer = buffer[pos + 2..].to_string();
                            
                            // Parse SSE event
                            for line in event.lines() {
                                if line.starts_with("data: ") {
                                    let data = &line[6..];
                                    if let Ok(response) = serde_json::from_str::<GoogleChatResponse>(data) {
                                        if let Some(candidates) = response.candidates {
                                            if let Some(candidate) = candidates.first() {
                                                let content = candidate.content.parts.iter()
                                                    .map(|p| p.text.clone())
                                                    .collect::<Vec<_>>()
                                                    .join("");

                                                let finish_reason = candidate.finish_reason.as_deref().map(|fr| match fr {
                                                    "STOP" => FinishReason::Stop,
                                                    "MAX_TOKENS" => FinishReason::Length,
                                                    "SAFETY" => FinishReason::ContentFilter,
                                                    _ => FinishReason::Other,
                                                });

                                                yield Ok(ChatChunk {
                                                    model: model.clone(),
                                                    delta: crate::providers::types::MessageDelta {
                                                        role: Some(MessageRole::Assistant),
                                                        content: Some(content),
                                                        tool_calls: None,
                                                    },
                                                    finish_reason,
                                                    usage: response.usage_metadata.clone().map(|u| Usage {
                                                        input_tokens: u.prompt_token_count.unwrap_or(0),
                                                        output_tokens: u.candidates_token_count.unwrap_or(0),
                                                        total_tokens: u.total_token_count.unwrap_or(0),
                                                        input_token_details: None,
                                                    }),
                                                    index: candidate.index,
                                                });
                                            }
                                        }
                                    }
                                }
                            }
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
        let provider = GoogleProvider::new("test-key");
        assert_eq!(provider.name(), "google");
        assert_eq!(provider.display_name(), "Google AI");
    }

    #[tokio::test]
    async fn test_models_requires_auth() {
        let provider = GoogleProvider::new("invalid-key");
        let result = provider.models().await;
        assert!(result.is_err());
    }
}
