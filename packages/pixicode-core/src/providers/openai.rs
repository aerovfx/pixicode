//! OpenAI Provider — OpenAI API and compatible providers

use async_stream::stream;
use async_trait::async_trait;
use futures::stream::{BoxStream, StreamExt};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::providers::retry;
use crate::providers::streaming::stream_with_cancel;
use crate::providers::trait_def::{Provider, ProviderError, ProviderResult};
use crate::providers::types::{
    ChatRequest, ChatResponse, ChatChunk, Message, MessageRole,
    ModelInfo, ModelCapabilities, Usage, FinishReason, ToolChoice,
};

/// OpenAI provider.
pub struct OpenAIProvider {
    client: Client,
    api_key: String,
    base_url: String,
    organization: Option<String>,
    /// Custom headers from config (e.g. x-api-key, custom auth).
    extra_headers: Option<HashMap<String, String>>,
}

impl OpenAIProvider {
    pub fn new(api_key: &str) -> Self {
        Self {
            client: Client::new(),
            api_key: api_key.to_string(),
            base_url: "https://api.openai.com/v1".to_string(),
            organization: None,
            extra_headers: None,
        }
    }

    pub fn with_base_url(mut self, base_url: &str) -> Self {
        self.base_url = base_url.to_string();
        self
    }

    pub fn with_organization(mut self, org: &str) -> Self {
        self.organization = Some(org.to_string());
        self
    }

    pub fn with_headers(mut self, headers: HashMap<String, String>) -> Self {
        self.extra_headers = Some(headers);
        self
    }

    fn apply_headers(&self, mut req: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        if let Some(ref h) = self.extra_headers {
            for (k, v) in h {
                req = req.header(k.as_str(), v.as_str());
            }
        }
        req
    }

    /// Create a provider compatible with OpenAI's API (e.g., Groq, Together, etc.)
    pub fn compatible(api_key: &str, base_url: &str) -> Self {
        Self::new(api_key).with_base_url(base_url)
    }
}

#[derive(Debug, Serialize, Clone)]
pub struct OpenAIChatRequest {
    model: String,
    messages: Vec<OpenAIMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<OpenAITool>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_choice: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    frequency_penalty: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    presence_penalty: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stop: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    response_format: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    user: Option<String>,
    #[serde(default)]
    stream: bool,
}

#[derive(Debug, Serialize, Clone)]
pub struct OpenAIMessage {
    pub role: String,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<OpenAIToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

#[derive(Debug, Serialize, Clone)]
struct OpenAITool {
    #[serde(rename = "type")]
    tool_type: String,
    function: OpenAIFunction,
}

#[derive(Debug, Serialize, Clone)]
struct OpenAIFunction {
    name: String,
    description: String,
    parameters: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OpenAIToolCall {
    pub id: String,
    #[serde(rename = "type")]
    pub tool_type: String,
    pub function: OpenAIToolFunction,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct OpenAIToolFunction {
    pub name: String,
    pub arguments: String,
}

#[derive(Debug, Deserialize)]
struct OpenAIChatResponse {
    id: String,
    object: String,
    created: u64,
    model: String,
    choices: Vec<OpenAIChoice>,
    usage: Option<OpenAIUsage>,
}

#[derive(Debug, Deserialize)]
struct OpenAIChoice {
    index: u32,
    message: OpenAIResponseMessage,
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OpenAIResponseMessage {
    role: String,
    content: Option<String>,
    #[serde(default)]
    tool_calls: Vec<OpenAIToolCall>,
}

#[derive(Debug, Deserialize)]
struct OpenAIUsage {
    prompt_tokens: u32,
    completion_tokens: u32,
    total_tokens: u32,
}

#[derive(Debug, Deserialize)]
struct OpenAIModel {
    id: String,
    object: Option<String>,
    owned_by: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OpenAIModelsResponse {
    data: Vec<OpenAIModel>,
}

#[derive(Debug, Deserialize)]
struct OpenAIStreamChunk {
    id: Option<String>,
    object: Option<String>,
    created: Option<u64>,
    model: Option<String>,
    choices: Vec<OpenAIStreamChoice>,
    usage: Option<OpenAIUsage>,
}

#[derive(Debug, Deserialize)]
struct OpenAIStreamChoice {
    index: Option<u32>,
    delta: OpenAIDelta,
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
struct OpenAIDelta {
    role: Option<String>,
    content: Option<String>,
    #[serde(default)]
    tool_calls: Vec<OpenAIToolCallDelta>,
}

#[derive(Debug, Deserialize)]
struct OpenAIToolCallDelta {
    index: Option<u32>,
    id: Option<String>,
    #[serde(rename = "type")]
    tool_type: Option<String>,
    function: Option<OpenAIToolFunctionDelta>,
}

#[derive(Debug, Deserialize)]
struct OpenAIToolFunctionDelta {
    name: Option<String>,
    arguments: Option<String>,
}

#[async_trait]
impl Provider for OpenAIProvider {
    fn name(&self) -> &'static str {
        "openai"
    }

    fn display_name(&self) -> &'static str {
        "OpenAI"
    }

    async fn models(&self) -> ProviderResult<Vec<ModelInfo>> {
        let client = self.client.clone();
        let url = format!("{}/models", self.base_url);
        let api_key = self.api_key.clone();
        let organization = self.organization.clone();
        let extra = self.extra_headers.clone();
        let response = retry::execute_async(|| {
            let client = client.clone();
            let url = url.clone();
            let api_key = api_key.clone();
            let organization = organization.clone();
            let extra = extra.clone();
            async move {
                let mut req = client.get(&url).header("Authorization", format!("Bearer {}", api_key));
                if let Some(ref org) = organization {
                    req = req.header("OpenAI-Organization", org);
                }
                if let Some(ref h) = extra {
                    for (k, v) in h {
                        req = req.header(k.as_str(), v.as_str());
                    }
                }
                req.send().await.map_err(ProviderError::from)
            }
        }).await?;
        
        if !response.status().is_success() {
            let error = response.text().await.unwrap_or_default();
            return Err(ProviderError::InvalidRequest(format!("Failed to list models: {}", error)));
        }

        let models_response: OpenAIModelsResponse = response.json().await?;
        
        let models = models_response.data.into_iter().map(|m| {
            let model_id = m.id.to_lowercase();
            let id = m.id.clone();
            ModelInfo {
                id,
                name: None,
                description: None,
                context_window: get_openai_context_window(&model_id),
                max_output_tokens: get_openai_max_output(&model_id),
                capabilities: ModelCapabilities {
                    functions: true,
                    vision: model_id.contains("vision") || model_id.contains("gpt-4o"),
                    streaming: true,
                    json_mode: true,
                },
                pricing: crate::providers::cost::get_model_pricing(&model_id),
            }
        }).collect();

        Ok(models)
    }

    async fn chat(&self, request: ChatRequest) -> ProviderResult<ChatResponse> {
        let url = format!("{}/chat/completions", self.base_url);
        
        let openai_request = OpenAIChatRequest {
            model: request.model,
            messages: request.messages.into_iter().map(|m| OpenAIMessage {
                role: format!("{:?}", m.role).to_lowercase(),
                content: m.content,
                name: m.name,
                tool_calls: m.tool_calls.map(|calls| calls.into_iter().map(|c| OpenAIToolCall {
                    id: c.id,
                    tool_type: "function".to_string(),
                    function: OpenAIToolFunction {
                        name: c.name,
                        arguments: c.arguments.to_string(),
                    },
                }).collect()),
                tool_call_id: m.tool_call_id,
            }).collect(),
            tools: request.tools.map(|tools| tools.into_iter().map(|t| OpenAITool {
                tool_type: "function".to_string(),
                function: OpenAIFunction {
                    name: t.name,
                    description: t.description,
                    parameters: t.parameters,
                },
            }).collect()),
            tool_choice: request.tool_choice.map(|tc| match tc {
                ToolChoice::None => serde_json::json!("none"),
                ToolChoice::Auto => serde_json::json!("auto"),
                ToolChoice::Required => serde_json::json!("required"),
                ToolChoice::Specific { function, .. } => serde_json::json!({
                    "type": "function",
                    "function": {"name": function.name}
                }),
            }),
            temperature: request.temperature,
            max_tokens: request.max_tokens,
            top_p: request.top_p,
            frequency_penalty: request.frequency_penalty,
            presence_penalty: request.presence_penalty,
            stop: request.stop,
            response_format: request.response_format.map(|rf| serde_json::json!({"type": rf.format_type})),
            user: request.user,
            stream: false,
        };

        let client = self.client.clone();
        let api_key = self.api_key.clone();
        let organization = self.organization.clone();
        let extra = self.extra_headers.clone();
        let response = retry::execute_async(|| {
            let client = client.clone();
            let body = openai_request.clone();
            let url = url.clone();
            let api_key = api_key.clone();
            let organization = organization.clone();
            let extra = extra.clone();
            async move {
                let mut rb = client.post(&url).json(&body).header("Authorization", format!("Bearer {}", api_key));
                if let Some(ref org) = organization {
                    rb = rb.header("OpenAI-Organization", org);
                }
                if let Some(ref h) = extra {
                    for (k, v) in h {
                        rb = rb.header(k.as_str(), v.as_str());
                    }
                }
                let res = rb.send().await?;
                if !res.status().is_success() {
                    let text = res.text().await.unwrap_or_default();
                    return Err(ProviderError::InvalidRequest(format!("OpenAI API error: {}", text)));
                }
                Ok(res)
            }
        }).await?;

        let openai_response: OpenAIChatResponse = response.json().await?;
        let choice = openai_response.choices.first().ok_or_else(|| {
            ProviderError::Internal("No choices in response".to_string())
        })?;

        let finish_reason = match choice.finish_reason.as_deref() {
            Some("stop") => FinishReason::Stop,
            Some("length") => FinishReason::Length,
            Some("tool_calls") | Some("function_call") => FinishReason::ToolCalls,
            Some("content_filter") => FinishReason::ContentFilter,
            _ => FinishReason::Other,
        };

        let tool_calls = if !choice.message.tool_calls.is_empty() {
            Some(choice.message.tool_calls.iter().map(|tc| {
                crate::providers::types::ToolCall {
                    id: tc.id.clone(),
                    name: tc.function.name.clone(),
                    arguments: serde_json::Value::String(tc.function.arguments.clone()),
                }
            }).collect())
        } else {
            None
        };

        Ok(ChatResponse {
            model: openai_response.model,
            message: Message {
                role: MessageRole::Assistant,
                content: choice.message.content.clone().unwrap_or_default(),
                name: None,
                tool_call_id: None,
                tool_calls,
            },
            finish_reason,
            usage: openai_response.usage.map(|u| Usage {
                input_tokens: u.prompt_tokens,
                output_tokens: u.completion_tokens,
                total_tokens: u.total_tokens,
                input_token_details: None,
            }),
            id: Some(openai_response.id),
            created_at: Some(openai_response.created),
        })
    }

    async fn chat_stream(&self, request: ChatRequest) -> ProviderResult<BoxStream<'static, ProviderResult<ChatChunk>>> {
        let url = format!("{}/chat/completions", self.base_url);
        
        let mut openai_request = OpenAIChatRequest {
            model: request.model,
            messages: request.messages.into_iter().map(|m| OpenAIMessage {
                role: format!("{:?}", m.role).to_lowercase(),
                content: m.content,
                name: m.name,
                tool_calls: None,
                tool_call_id: m.tool_call_id,
            }).collect(),
            tools: request.tools.map(|tools| tools.into_iter().map(|t| OpenAITool {
                tool_type: "function".to_string(),
                function: OpenAIFunction {
                    name: t.name,
                    description: t.description,
                    parameters: t.parameters,
                },
            }).collect()),
            tool_choice: request.tool_choice.map(|tc| match tc {
                ToolChoice::None => serde_json::json!("none"),
                ToolChoice::Auto => serde_json::json!("auto"),
                ToolChoice::Required => serde_json::json!("required"),
                ToolChoice::Specific { function, .. } => serde_json::json!({
                    "type": "function",
                    "function": {"name": function.name}
                }),
            }),
            temperature: request.temperature,
            max_tokens: request.max_tokens,
            top_p: request.top_p,
            frequency_penalty: request.frequency_penalty,
            presence_penalty: request.presence_penalty,
            stop: request.stop,
            response_format: request.response_format.map(|rf| serde_json::json!({"type": rf.format_type})),
            user: request.user,
            stream: true,
        };

        // Clear tool_calls from messages for streaming (simplification)
        for msg in &mut openai_request.messages {
            msg.tool_calls = None;
        }

        let response = {
            let client = self.client.clone();
            let body = openai_request.clone();
            let api_key = self.api_key.clone();
            let organization = self.organization.clone();
            let extra = self.extra_headers.clone();
            retry::execute_async(|| {
                let client = client.clone();
                let body = body.clone();
                let api_key = api_key.clone();
                let organization = organization.clone();
                let extra = extra.clone();
                let url = url.clone();
                async move {
                    let mut rb = client.post(&url).json(&body).header("Authorization", format!("Bearer {}", api_key));
                    if let Some(ref org) = organization {
                        rb = rb.header("OpenAI-Organization", org);
                    }
                    if let Some(ref h) = extra {
                        for (k, v) in h {
                            rb = rb.header(k.as_str(), v.as_str());
                        }
                    }
                    let res = rb.send().await?;
                    if !res.status().is_success() {
                        let text = res.text().await.unwrap_or_default();
                        return Err(ProviderError::InvalidRequest(format!("OpenAI API error: {}", text)));
                    }
                    Ok(res)
                }
            }).await?
        };

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
                                let line = line.trim();
                                if line.starts_with("data: ") {
                                    let data = &line[6..];
                                    if data == "[DONE]" {
                                        return;
                                    }
                                    if let Ok(chunk) = serde_json::from_str::<OpenAIStreamChunk>(data) {
                                        if let Some(choice) = chunk.choices.first() {
                                            let finish_reason = choice.finish_reason.as_deref().map(|fr| match fr {
                                                "stop" => FinishReason::Stop,
                                                "length" => FinishReason::Length,
                                                "tool_calls" | "function_call" => FinishReason::ToolCalls,
                                                "content_filter" => FinishReason::ContentFilter,
                                                _ => FinishReason::Other,
                                            });

                                            let role = choice.delta.role.as_ref().and_then(|r| match r.as_str() {
                                                "system" => Some(MessageRole::System),
                                                "user" => Some(MessageRole::User),
                                                "assistant" => Some(MessageRole::Assistant),
                                                _ => None,
                                            });

                                            yield Ok(ChatChunk {
                                                model: chunk.model.unwrap_or_default(),
                                                delta: crate::providers::types::MessageDelta {
                                                    role,
                                                    content: choice.delta.content.clone(),
                                                    tool_calls: None,
                                                },
                                                finish_reason,
                                                usage: chunk.usage.map(|u| Usage {
                                                    input_tokens: u.prompt_tokens,
                                                    output_tokens: u.completion_tokens,
                                                    total_tokens: u.total_tokens,
                                                    input_token_details: None,
                                                }),
                                                index: choice.index,
                                            });
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

        let stream: BoxStream<'static, ProviderResult<ChatChunk>> = match request.cancel {
            Some(t) => stream_with_cancel(stream, t),
            None => stream.boxed(),
        };
        Ok(stream)
    }
}

/// Get context window for known OpenAI models.
fn get_openai_context_window(model_id: &str) -> Option<u32> {
    if model_id.contains("gpt-4o") {
        Some(128000)
    } else if model_id.contains("gpt-4-turbo") {
        Some(128000)
    } else if model_id.contains("gpt-4-32k") {
        Some(32768)
    } else if model_id.contains("gpt-4") {
        Some(8192)
    } else if model_id.contains("gpt-3.5-turbo-16k") {
        Some(16384)
    } else if model_id.contains("gpt-3.5") {
        Some(4096)
    } else {
        None
    }
}

/// Get max output tokens for known OpenAI models.
fn get_openai_max_output(model_id: &str) -> Option<u32> {
    if model_id.contains("gpt-4o") || model_id.contains("gpt-4-turbo") {
        Some(4096)
    } else if model_id.contains("gpt-4") {
        Some(2048)
    } else if model_id.contains("gpt-3.5-turbo-16k") {
        Some(4096)
    } else if model_id.contains("gpt-3.5") {
        Some(2048)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_name() {
        let provider = OpenAIProvider::new("test-key");
        assert_eq!(provider.name(), "openai");
        assert_eq!(provider.display_name(), "OpenAI");
    }

    #[test]
    fn test_compatible_provider() {
        let provider = OpenAIProvider::compatible("test-key", "https://api.groq.com/openai/v1");
        assert_eq!(provider.name(), "openai");
    }

    #[tokio::test]
    async fn test_models_requires_auth() {
        let provider = OpenAIProvider::new("invalid-key");
        let result = provider.models().await;
        assert!(result.is_err());
    }
}
