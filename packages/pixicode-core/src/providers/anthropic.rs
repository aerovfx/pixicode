//! Anthropic Provider — Claude API

use async_stream::stream;
use async_trait::async_trait;
use futures::stream::{BoxStream, StreamExt};
use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::providers::trait_def::{Provider, ProviderError, ProviderResult};
use crate::providers::types::{
    ChatRequest, ChatResponse, ChatChunk, Message, MessageRole,
    ModelInfo, ModelCapabilities, Usage, FinishReason,
    ToolChoice,
};

/// Anthropic provider.
pub struct AnthropicProvider {
    client: Client,
    api_key: String,
    version: String,
}

impl AnthropicProvider {
    pub fn new(api_key: &str) -> Self {
        Self {
            client: Client::new(),
            api_key: api_key.to_string(),
            version: "2023-06-01".to_string(),
        }
    }

    pub fn with_version(mut self, version: &str) -> Self {
        self.version = version.to_string();
        self
    }
}

#[derive(Debug, Serialize)]
struct AnthropicChatRequest {
    model: String,
    messages: Vec<AnthropicMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<AnthropicTool>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_choice: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stop_sequences: Option<Vec<String>>,
    #[serde(default)]
    stream: bool,
}

#[derive(Debug, Serialize)]
struct AnthropicMessage {
    role: String,
    content: String,
}

#[derive(Debug, Serialize)]
struct AnthropicTool {
    name: String,
    description: String,
    input_schema: serde_json::Value,
}

#[derive(Debug, Deserialize)]
struct AnthropicChatResponse {
    id: String,
    #[serde(rename = "type")]
    response_type: String,
    role: String,
    content: Vec<AnthropicContent>,
    model: String,
    stop_reason: Option<String>,
    stop_sequence: Option<String>,
    usage: Option<AnthropicUsage>,
}

#[derive(Debug, Deserialize)]
struct AnthropicContent {
    #[serde(rename = "type")]
    content_type: String,
    text: Option<String>,
    #[serde(default)]
    tool_use: Option<AnthropicToolUse>,
}

#[derive(Debug, Deserialize)]
struct AnthropicToolUse {
    id: String,
    name: String,
    input: serde_json::Value,
}

#[derive(Debug, Deserialize)]
struct AnthropicUsage {
    input_tokens: u32,
    output_tokens: u32,
}

#[derive(Debug, Deserialize)]
struct AnthropicStreamEvent {
    #[serde(rename = "type")]
    event_type: String,
    index: Option<u32>,
    delta: Option<AnthropicDelta>,
    message: Option<AnthropicMessageData>,
    usage: Option<AnthropicUsage>,
}

#[derive(Debug, Deserialize)]
struct AnthropicDelta {
    #[serde(rename = "type")]
    delta_type: Option<String>,
    text: Option<String>,
    partial_json: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AnthropicMessageData {
    id: Option<String>,
    content: Option<Vec<AnthropicContent>>,
    stop_reason: Option<String>,
    usage: Option<AnthropicUsage>,
}

#[derive(Debug, Deserialize)]
struct AnthropicModel {
    id: String,
    display_name: Option<String>,
    context_window: Option<u32>,
    max_completion_tokens: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct AnthropicModelsResponse {
    data: Vec<AnthropicModel>,
}

#[async_trait]
impl Provider for AnthropicProvider {
    fn name(&self) -> &'static str {
        "anthropic"
    }

    fn display_name(&self) -> &'static str {
        "Anthropic"
    }

    async fn models(&self) -> ProviderResult<Vec<ModelInfo>> {
        // Anthropic doesn't have a public models endpoint, so we return known models
        let models = vec![
            ModelInfo {
                id: "claude-3-5-sonnet-20241022".to_string(),
                name: Some("Claude 3.5 Sonnet".to_string()),
                description: Some("Most intelligent model".to_string()),
                context_window: Some(200000),
                max_output_tokens: Some(8192),
                capabilities: ModelCapabilities {
                    functions: true,
                    vision: true,
                    streaming: true,
                    json_mode: false,
                },
                pricing: crate::providers::cost::get_model_pricing("claude-3-5-sonnet"),
            },
            ModelInfo {
                id: "claude-3-opus-20240229".to_string(),
                name: Some("Claude 3 Opus".to_string()),
                description: Some("Most powerful model".to_string()),
                context_window: Some(200000),
                max_output_tokens: Some(4096),
                capabilities: ModelCapabilities {
                    functions: true,
                    vision: true,
                    streaming: true,
                    json_mode: false,
                },
                pricing: crate::providers::cost::get_model_pricing("claude-3-opus"),
            },
            ModelInfo {
                id: "claude-3-sonnet-20240229".to_string(),
                name: Some("Claude 3 Sonnet".to_string()),
                description: Some("Balanced model".to_string()),
                context_window: Some(200000),
                max_output_tokens: Some(4096),
                capabilities: ModelCapabilities {
                    functions: true,
                    vision: true,
                    streaming: true,
                    json_mode: false,
                },
                pricing: crate::providers::cost::get_model_pricing("claude-3-sonnet"),
            },
            ModelInfo {
                id: "claude-3-haiku-20240307".to_string(),
                name: Some("Claude 3 Haiku".to_string()),
                description: Some("Fastest model".to_string()),
                context_window: Some(200000),
                max_output_tokens: Some(4096),
                capabilities: ModelCapabilities {
                    functions: true,
                    vision: true,
                    streaming: true,
                    json_mode: false,
                },
                pricing: crate::providers::cost::get_model_pricing("claude-3-haiku"),
            },
        ];

        Ok(models)
    }

    async fn chat(&self, request: ChatRequest) -> ProviderResult<ChatResponse> {
        let url = "https://api.anthropic.com/v1/messages";
        
        // Extract system message
        let system_message = request.messages.iter()
            .find(|m| m.role == MessageRole::System)
            .map(|m| m.content.clone());

        // Filter non-system messages
        let messages: Vec<_> = request.messages.into_iter()
            .filter(|m| m.role != MessageRole::System)
            .map(|m| AnthropicMessage {
                role: match m.role {
                    MessageRole::User => "user".to_string(),
                    MessageRole::Assistant => "assistant".to_string(),
                    _ => "user".to_string(),
                },
                content: m.content,
            }).collect();

        let anthropic_request = AnthropicChatRequest {
            model: request.model,
            messages,
            system: system_message,
            tools: request.tools.map(|tools| tools.into_iter().map(|t| AnthropicTool {
                name: t.name,
                description: t.description,
                input_schema: t.parameters,
            }).collect()),
            tool_choice: request.tool_choice.map(|tc| match tc {
                crate::providers::types::ToolChoice::None => serde_json::json!("none"),
                crate::providers::types::ToolChoice::Auto => serde_json::json!("auto"),
                crate::providers::types::ToolChoice::Required => serde_json::json!("any"),
                crate::providers::types::ToolChoice::Specific { function, .. } => serde_json::json!({
                    "type": "tool",
                    "name": function.name
                }),
            }),
            max_tokens: request.max_tokens.or(Some(4096)),
            temperature: request.temperature,
            top_p: request.top_p,
            stop_sequences: request.stop,
            stream: false,
        };

        let mut request_builder = self.client.post(url)
            .json(&anthropic_request)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", &self.version)
            .header("Content-Type", "application/json");

        let response = request_builder.send().await?;
        
        if !response.status().is_success() {
            let error = response.text().await.unwrap_or_default();
            return Err(ProviderError::InvalidRequest(format!("Anthropic API error: {}", error)));
        }

        let anthropic_response: AnthropicChatResponse = response.json().await?;

        // Extract text content and tool calls
        let mut content = String::new();
        let mut tool_calls = Vec::new();

        for c in &anthropic_response.content {
            match c.content_type.as_str() {
                "text" => {
                    if let Some(text) = &c.text {
                        content.push_str(text);
                    }
                }
                "tool_use" => {
                    if let Some(tool_use) = &c.tool_use {
                        tool_calls.push(crate::providers::types::ToolCall {
                            id: tool_use.id.clone(),
                            name: tool_use.name.clone(),
                            arguments: tool_use.input.clone(),
                        });
                    }
                }
                _ => {}
            }
        }

        let finish_reason = match anthropic_response.stop_reason.as_deref() {
            Some("end_turn") | Some("stop_sequence") => FinishReason::Stop,
            Some("max_tokens") => FinishReason::Length,
            Some("tool_use") => FinishReason::ToolCalls,
            _ => FinishReason::Other,
        };

        Ok(ChatResponse {
            model: anthropic_response.model,
            message: Message {
                role: MessageRole::Assistant,
                content,
                name: None,
                tool_call_id: None,
                tool_calls: if tool_calls.is_empty() { None } else { Some(tool_calls) },
            },
            finish_reason,
            usage: anthropic_response.usage.map(|u| Usage {
                input_tokens: u.input_tokens,
                output_tokens: u.output_tokens,
                total_tokens: u.input_tokens + u.output_tokens,
                input_token_details: None,
            }),
            id: Some(anthropic_response.id),
            created_at: None,
        })
    }

    async fn chat_stream(&self, request: ChatRequest) -> ProviderResult<BoxStream<'static, ProviderResult<ChatChunk>>> {
        let url = "https://api.anthropic.com/v1/messages";
        
        let system_message = request.messages.iter()
            .find(|m| m.role == MessageRole::System)
            .map(|m| m.content.clone());

        let messages: Vec<_> = request.messages.into_iter()
            .filter(|m| m.role != MessageRole::System)
            .map(|m| AnthropicMessage {
                role: match m.role {
                    MessageRole::User => "user".to_string(),
                    MessageRole::Assistant => "assistant".to_string(),
                    _ => "user".to_string(),
                },
                content: m.content,
            }).collect();

        let anthropic_request = AnthropicChatRequest {
            model: request.model,
            messages,
            system: system_message,
            tools: None, // Simplified for streaming
            tool_choice: None,
            max_tokens: request.max_tokens.or(Some(4096)),
            temperature: request.temperature,
            top_p: request.top_p,
            stop_sequences: request.stop,
            stream: true,
        };

        let response = self.client.post(url)
            .json(&anthropic_request)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", &self.version)
            .header("Content-Type", "application/json")
            .send()
            .await?;
        
        if !response.status().is_success() {
            let error = response.text().await.unwrap_or_default();
            return Err(ProviderError::InvalidRequest(format!("Anthropic API error: {}", error)));
        }

        let stream = stream! {
            let mut response = response;
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
                                    if let Ok(event) = serde_json::from_str::<AnthropicStreamEvent>(data) {
                                        match event.event_type.as_str() {
                                            "content_block_delta" => {
                                                if let Some(delta) = event.delta {
                                                    if let Some(text) = delta.text {
                                                        yield Ok(ChatChunk {
                                                            model: "claude".to_string(),
                                                            delta: crate::providers::types::MessageDelta {
                                                                role: Some(MessageRole::Assistant),
                                                                content: Some(text),
                                                                tool_calls: None,
                                                            },
                                                            finish_reason: None,
                                                            usage: None,
                                                            index: event.index,
                                                        });
                                                    }
                                                }
                                            }
                                            "message_delta" => {
                                                let finish_reason = event.message.as_ref()
                                                    .and_then(|m| m.stop_reason.as_deref())
                                                    .map(|fr| match fr {
                                                        "end_turn" | "stop_sequence" => FinishReason::Stop,
                                                        "max_tokens" => FinishReason::Length,
                                                        "tool_use" => FinishReason::ToolCalls,
                                                        _ => FinishReason::Other,
                                                    });

                                                let usage = event.usage.map(|u| Usage {
                                                    input_tokens: u.input_tokens,
                                                    output_tokens: u.output_tokens,
                                                    total_tokens: u.input_tokens + u.output_tokens,
                                                    input_token_details: None,
                                                });

                                                yield Ok(ChatChunk {
                                                    model: "claude".to_string(),
                                                    delta: crate::providers::types::MessageDelta::default(),
                                                    finish_reason,
                                                    usage,
                                                    index: None,
                                                });
                                            }
                                            _ => {}
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
        let provider = AnthropicProvider::new("test-key");
        assert_eq!(provider.name(), "anthropic");
        assert_eq!(provider.display_name(), "Anthropic");
    }

    #[tokio::test]
    async fn test_models() {
        let provider = AnthropicProvider::new("test-key");
        let models = provider.models().await.unwrap();
        assert!(!models.is_empty());
        assert!(models.iter().any(|m| m.id.contains("claude")));
    }
}
