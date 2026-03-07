//! Azure OpenAI Provider — Azure OpenAI Service with AD auth

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
use crate::providers::openai::{OpenAIProvider, OpenAIChatRequest, OpenAIMessage};

/// Azure OpenAI provider.
pub struct AzureOpenAIProvider {
    client: Client,
    endpoint: String,
    deployment_id: String,
    api_version: String,
    auth: AzureAuth,
}

/// Azure authentication method.
#[derive(Clone)]
enum AzureAuth {
    ApiKey(String),
    Token(String),
}

impl AzureOpenAIProvider {
    /// Create with API key authentication.
    pub fn with_api_key(endpoint: &str, deployment_id: &str, api_key: &str) -> Self {
        Self {
            client: Client::new(),
            endpoint: endpoint.trim_end_matches('/').to_string(),
            deployment_id: deployment_id.to_string(),
            api_version: "2024-02-15-preview".to_string(),
            auth: AzureAuth::ApiKey(api_key.to_string()),
        }
    }

    /// Create with Azure AD token authentication.
    pub fn with_token(endpoint: &str, deployment_id: &str, token: &str) -> Self {
        Self {
            client: Client::new(),
            endpoint: endpoint.trim_end_matches('/').to_string(),
            deployment_id: deployment_id.to_string(),
            api_version: "2024-02-15-preview".to_string(),
            auth: AzureAuth::Token(token.to_string()),
        }
    }

    /// Create from environment variables.
    pub fn from_env(deployment_id: &str) -> Option<Self> {
        let endpoint = std::env::var("AZURE_OPENAI_ENDPOINT").ok()?;
        let api_key = std::env::var("AZURE_OPENAI_API_KEY").ok();
        let api_version = std::env::var("AZURE_OPENAI_API_VERSION")
            .unwrap_or_else(|_| "2024-02-15-preview".to_string());
        
        if let Some(key) = api_key {
            Some(Self {
                client: Client::new(),
                endpoint: endpoint.trim_end_matches('/').to_string(),
                deployment_id: deployment_id.to_string(),
                api_version,
                auth: AzureAuth::ApiKey(key),
            })
        } else {
            None
        }
    }

    /// Set API version.
    pub fn with_api_version(mut self, version: &str) -> Self {
        self.api_version = version.to_string();
        self
    }

    /// Get the chat completion URL.
    fn chat_url(&self) -> String {
        format!(
            "{}/openai/deployments/{}/chat/completions?api-version={}",
            self.endpoint, self.deployment_id, self.api_version
        )
    }

    /// Get authorization header.
    fn auth_header(&self) -> (String, String) {
        match &self.auth {
            AzureAuth::ApiKey(key) => ("api-key".to_string(), key.clone()),
            AzureAuth::Token(token) => ("Authorization".to_string(), format!("Bearer {}", token)),
        }
    }
}

#[derive(Debug, Serialize)]
struct AzureChatRequest {
    model: Option<String>,
    messages: Vec<OpenAIMessage>,
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
    #[serde(default)]
    stream: bool,
}

#[derive(Debug, Deserialize)]
struct AzureChatResponse {
    id: String,
    object: String,
    created: u64,
    model: String,
    choices: Vec<AzureChoice>,
    usage: Option<AzureUsage>,
}

#[derive(Debug, Deserialize)]
struct AzureChoice {
    index: u32,
    message: AzureResponseMessage,
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AzureResponseMessage {
    role: String,
    content: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AzureUsage {
    prompt_tokens: u32,
    completion_tokens: u32,
    total_tokens: u32,
}

#[async_trait]
impl Provider for AzureOpenAIProvider {
    fn name(&self) -> &'static str {
        "azure_openai"
    }

    fn display_name(&self) -> &'static str {
        "Azure OpenAI"
    }

    async fn models(&self) -> ProviderResult<Vec<ModelInfo>> {
        // Azure doesn't have a models endpoint, return deployment info
        let model_id = self.deployment_id.to_lowercase();
        
        let model_info = ModelInfo {
            id: self.deployment_id.clone(),
            name: Some(format!("Azure Deployment: {}", self.deployment_id)),
            description: None,
            context_window: get_azure_context_window(&model_id),
            max_output_tokens: Some(4096),
            capabilities: ModelCapabilities {
                functions: true,
                vision: model_id.contains("gpt-4o") || model_id.contains("vision"),
                streaming: true,
                json_mode: true,
            },
            pricing: None, // Azure pricing varies by region
        };

        Ok(vec![model_info])
    }

    async fn chat(&self, request: ChatRequest) -> ProviderResult<ChatResponse> {
        let url = self.chat_url();
        let (auth_header, auth_value) = self.auth_header();

        let azure_request = AzureChatRequest {
            model: None, // Azure uses deployment ID from URL
            messages: request.messages.into_iter().map(|m| OpenAIMessage {
                role: format!("{:?}", m.role).to_lowercase(),
                content: m.content,
                name: m.name,
                tool_calls: None,
                tool_call_id: m.tool_call_id,
            }).collect(),
            temperature: request.temperature,
            max_tokens: request.max_tokens,
            top_p: request.top_p,
            frequency_penalty: request.frequency_penalty,
            presence_penalty: request.presence_penalty,
            stop: request.stop,
            stream: false,
        };

        let response = self.client.post(&url)
            .json(&azure_request)
            .header(&auth_header, &auth_value)
            .send()
            .await?;
        
        if !response.status().is_success() {
            let error = response.text().await.unwrap_or_default();
            return Err(ProviderError::InvalidRequest(format!("Azure OpenAI error: {}", error)));
        }

        let azure_response: AzureChatResponse = response.json().await?;
        let choice = azure_response.choices.first().ok_or_else(|| {
            ProviderError::Internal("No choices in response".to_string())
        })?;

        let finish_reason = match choice.finish_reason.as_deref() {
            Some("stop") => FinishReason::Stop,
            Some("length") => FinishReason::Length,
            Some("tool_calls") | Some("function_call") => FinishReason::ToolCalls,
            Some("content_filter") => FinishReason::ContentFilter,
            _ => FinishReason::Other,
        };

        Ok(ChatResponse {
            model: azure_response.model,
            message: Message {
                role: MessageRole::Assistant,
                content: choice.message.content.clone().unwrap_or_default(),
                name: None,
                tool_call_id: None,
                tool_calls: None,
            },
            finish_reason,
            usage: azure_response.usage.map(|u| Usage {
                input_tokens: u.prompt_tokens,
                output_tokens: u.completion_tokens,
                total_tokens: u.total_tokens,
                input_token_details: None,
            }),
            id: Some(azure_response.id),
            created_at: Some(azure_response.created),
        })
    }

    async fn chat_stream(&self, request: ChatRequest) -> ProviderResult<BoxStream<'static, ProviderResult<ChatChunk>>> {
        let url = self.chat_url();
        let (auth_header, auth_value) = self.auth_header();

        let azure_request = AzureChatRequest {
            model: None,
            messages: request.messages.into_iter().map(|m| OpenAIMessage {
                role: format!("{:?}", m.role).to_lowercase(),
                content: m.content,
                name: m.name,
                tool_calls: None,
                tool_call_id: m.tool_call_id,
            }).collect(),
            temperature: request.temperature,
            max_tokens: request.max_tokens,
            top_p: request.top_p,
            frequency_penalty: request.frequency_penalty,
            presence_penalty: request.presence_penalty,
            stop: request.stop,
            stream: true,
        };

        let response = self.client.post(&url)
            .json(&azure_request)
            .header(&auth_header, &auth_value)
            .send()
            .await?;
        
        if !response.status().is_success() {
            let error = response.text().await.unwrap_or_default();
            return Err(ProviderError::InvalidRequest(format!("Azure OpenAI error: {}", error)));
        }

        // Parse SSE stream using chunk() in a loop
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
                                    if data == "[DONE]" {
                                        return;
                                    }
                                    // Parse chunk
                                    if let Ok(chunk) = serde_json::from_str::<serde_json::Value>(data) {
                                        if let Some(choices) = chunk.get("choices").and_then(|c| c.as_array()) {
                                            if let Some(choice) = choices.first() {
                                                if let Some(delta) = choice.get("delta") {
                                                    let content = delta.get("content")
                                                        .and_then(|c| c.as_str())
                                                        .map(String::from);

                                                    yield Ok(ChatChunk {
                                                        model: chunk.get("model")
                                                            .and_then(|m| m.as_str())
                                                            .unwrap_or("azure")
                                                            .to_string(),
                                                        delta: crate::providers::types::MessageDelta {
                                                            role: None,
                                                            content,
                                                            tool_calls: None,
                                                        },
                                                        finish_reason: None,
                                                        usage: None,
                                                        index: None,
                                                    });
                                                }
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

/// Get context window for known Azure deployments.
fn get_azure_context_window(deployment_id: &str) -> Option<u32> {
    let id = deployment_id.to_lowercase();
    if id.contains("gpt-4o") {
        Some(128000)
    } else if id.contains("gpt-4-turbo") || id.contains("gpt-4-32k") {
        Some(128000)
    } else if id.contains("gpt-4") {
        Some(8192)
    } else if id.contains("gpt-35-turbo-16k") || id.contains("gpt-3.5-turbo-16k") {
        Some(16384)
    } else if id.contains("gpt-35") || id.contains("gpt-3.5") {
        Some(4096)
    } else {
        None
    }
}

/// OAuth2 token provider for Azure AD.
pub struct AzureTokenProvider {
    tenant_id: String,
    client_id: String,
    client_secret: String,
    token: Option<String>,
    expires_at: Option<std::time::SystemTime>,
}

impl AzureTokenProvider {
    pub fn new(tenant_id: &str, client_id: &str, client_secret: &str) -> Self {
        Self {
            tenant_id: tenant_id.to_string(),
            client_id: client_id.to_string(),
            client_secret: client_secret.to_string(),
            token: None,
            expires_at: None,
        }
    }

    pub fn from_env() -> Option<Self> {
        let tenant_id = std::env::var("AZURE_TENANT_ID").ok()?;
        let client_id = std::env::var("AZURE_CLIENT_ID").ok()?;
        let client_secret = std::env::var("AZURE_CLIENT_SECRET").ok()?;
        Some(Self::new(&tenant_id, &client_id, &client_secret))
    }

    /// Get access token, refreshing if necessary.
    pub async fn get_token(&mut self) -> ProviderResult<String> {
        // Check if we have a valid token
        if let Some(ref token) = self.token {
            if let Some(expires_at) = self.expires_at {
                if expires_at > std::time::SystemTime::now() {
                    return Ok(token.clone());
                }
            }
        }

        // Request new token
        let token_url = format!(
            "https://login.microsoftonline.com/{}/oauth2/v2.0/token",
            self.tenant_id
        );

        let params = [
            ("grant_type", "client_credentials"),
            ("client_id", &self.client_id),
            ("client_secret", &self.client_secret),
            ("scope", "https://cognitiveservices.azure.com/.default"),
        ];

        let response = reqwest::Client::new()
            .post(&token_url)
            .form(&params)
            .send()
            .await?;

        if !response.status().is_success() {
            let error = response.text().await.unwrap_or_default();
            return Err(ProviderError::AuthError(format!("Azure AD token error: {}", error)));
        }

        let token_response: serde_json::Value = response.json().await?;
        let access_token = token_response.get("access_token")
            .and_then(|t| t.as_str())
            .ok_or_else(|| ProviderError::AuthError("No access token in response".to_string()))?
            .to_string();

        let expires_in = token_response.get("expires_in")
            .and_then(|e| e.as_u64())
            .unwrap_or(3600);

        self.token = Some(access_token.clone());
        self.expires_at = Some(
            std::time::SystemTime::now() + std::time::Duration::from_secs(expires_in - 60)
        );

        Ok(access_token)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_name() {
        let provider = AzureOpenAIProvider::with_api_key(
            "https://test.openai.azure.com",
            "gpt-4",
            "test-key"
        );
        assert_eq!(provider.name(), "azure_openai");
        assert_eq!(provider.display_name(), "Azure OpenAI");
    }

    #[test]
    fn test_chat_url() {
        let provider = AzureOpenAIProvider::with_api_key(
            "https://test.openai.azure.com",
            "gpt-4",
            "test-key"
        );
        let url = provider.chat_url();
        assert!(url.contains("openai.azure.com"));
        assert!(url.contains("gpt-4"));
        assert!(url.contains("api-version="));
    }

    #[test]
    fn test_context_window() {
        assert_eq!(get_azure_context_window("gpt-4o"), Some(128000));
        assert_eq!(get_azure_context_window("gpt-4"), Some(8192));
        assert_eq!(get_azure_context_window("gpt-35-turbo"), Some(4096));
    }
}
