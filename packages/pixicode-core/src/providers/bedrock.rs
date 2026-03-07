//! AWS Bedrock Provider — AWS Bedrock Converse API with SigV4 auth

use async_trait::async_trait;
use futures::stream::BoxStream;
use hmac::{Hmac, Mac};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use time::{OffsetDateTime, Date};

use crate::providers::trait_def::{Provider, ProviderError, ProviderResult};
use crate::providers::types::{
    ChatRequest, ChatResponse, ChatChunk, Message, MessageRole,
    ModelInfo, ModelCapabilities, Usage, FinishReason,
};

/// AWS Bedrock provider.
pub struct BedrockProvider {
    client: Client,
    region: String,
    access_key_id: String,
    secret_access_key: String,
    session_token: Option<String>,
}

impl BedrockProvider {
    pub fn new(region: &str, access_key_id: &str, secret_access_key: &str) -> Self {
        Self {
            client: Client::new(),
            region: region.to_string(),
            access_key_id: access_key_id.to_string(),
            secret_access_key: secret_access_key.to_string(),
            session_token: None,
        }
    }

    pub fn with_session_token(mut self, token: &str) -> Self {
        self.session_token = Some(token.to_string());
        self
    }

    /// Create from environment variables
    pub fn from_env(region: &str) -> Option<Self> {
        let access_key_id = std::env::var("AWS_ACCESS_KEY_ID").ok()?;
        let secret_access_key = std::env::var("AWS_SECRET_ACCESS_KEY").ok()?;
        let session_token = std::env::var("AWS_SESSION_TOKEN").ok();
        
        let mut provider = Self::new(region, &access_key_id, &secret_access_key);
        if let Some(token) = session_token {
            provider = provider.with_session_token(&token);
        }
        Some(provider)
    }

    /// Get Bedrock endpoint URL
    fn endpoint(&self, model_id: &str) -> String {
        format!(
            "https://bedrock-runtime.{}.amazonaws.com/model/{}/converse",
            self.region, model_id
        )
    }

    /// Sign request with AWS SigV4
    fn sign_request(
        &self,
        method: &str,
        url: &str,
        body: &str,
        content_type: &str,
    ) -> Result<std::collections::HashMap<String, String>, ProviderError> {
        let now = OffsetDateTime::now_utc();
        let date = Date::from_ordinal_date(now.year(), now.ordinal()).unwrap();

        let amz_date = now.format(&time::format_description::well_known::Rfc3339)
            .map_err(|e| ProviderError::AuthError(format!("Date format error: {}", e)))?;
        let date_stamp = date.format(&time::format_description::well_known::Iso8601::DATE)
            .map_err(|e| ProviderError::AuthError(format!("Date format error: {}", e)))?;
        
        let service = "bedrock";
        let algorithm = "AWS4-HMAC-SHA256";
        
        // Parse URL
        let parsed_url = url::Url::parse(url)
            .map_err(|e| ProviderError::Internal(format!("URL parse error: {}", e)))?;
        let host = parsed_url.host_str().unwrap_or("");
        let canonical_uri = parsed_url.path();
        
        // Create canonical request
        let payload_hash = sha256_hex(body);
        let canonical_headers = format!(
            "content-type:{}\nhost:{}\nx-amz-date:{}\n",
            content_type, host, amz_date.replace("-", "").replace(":", "").replace(".", "")
        );
        let signed_headers = "content-type;host;x-amz-date";
        
        let canonical_request = format!(
            "{}\n{}\n{}\n{}\n{}\n{}",
            method,
            canonical_uri,
            "", // query string
            canonical_headers,
            signed_headers,
            payload_hash
        );
        
        let credential_scope = format!("{}/{}/{}/aws4_request", date_stamp, self.region, service);
        let string_to_sign = format!(
            "{}\n{}\n{}\n{}",
            algorithm,
            amz_date.replace("-", "").replace(":", "").replace(".", ""),
            credential_scope,
            sha256_hex(&canonical_request)
        );
        
        // Calculate signature
        let k_date = hmac_sha256(date_stamp.as_bytes(), format!("AWS4{}", self.secret_access_key).as_bytes());
        let k_region = hmac_sha256(self.region.as_bytes(), &k_date);
        let k_service = hmac_sha256(service.as_bytes(), &k_region);
        let k_signing = hmac_sha256(b"aws4_request", &k_service);
        let signature = hmac_sha256(string_to_sign.as_bytes(), &k_signing);
        
        let signature_hex = hex::encode(signature);
        let authorization = format!(
            "{} Credential={}/{}, SignedHeaders={}, Signature={}",
            algorithm, self.access_key_id, credential_scope, signed_headers, signature_hex
        );
        
        let mut headers = std::collections::HashMap::new();
        headers.insert("Authorization".to_string(), authorization);
        headers.insert("X-Amz-Date".to_string(), amz_date.replace("-", "").replace(":", "").replace(".", ""));
        headers.insert("Content-Type".to_string(), content_type.to_string());
        
        if let Some(ref token) = self.session_token {
            headers.insert("X-Amz-Security-Token".to_string(), token.clone());
        }
        
        Ok(headers)
    }
}

#[derive(Debug, Serialize)]
struct BedrockChatRequest {
    model_id: Option<String>,
    messages: Vec<BedrockMessage>,
    system: Option<Vec<BedrockSystemContent>>,
    inference_config: BedrockInferenceConfig,
}

#[derive(Debug, Serialize)]
struct BedrockMessage {
    role: String,
    content: Vec<BedrockContent>,
}

#[derive(Debug, Serialize)]
struct BedrockSystemContent {
    text: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct BedrockContent {
    text: String,
}

#[derive(Debug, Serialize)]
struct BedrockInferenceConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stop_sequences: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
struct BedrockChatResponse {
    output: BedrockOutput,
    stop_reason: Option<String>,
    usage: Option<BedrockUsage>,
    metrics: Option<BedrockMetrics>,
}

#[derive(Debug, Deserialize)]
struct BedrockOutput {
    message: BedrockResponseMessage,
}

#[derive(Debug, Deserialize)]
struct BedrockResponseMessage {
    role: String,
    content: Vec<BedrockContent>,
}

#[derive(Debug, Deserialize)]
struct BedrockUsage {
    input_tokens: u32,
    output_tokens: u32,
    total_tokens: u32,
}

#[derive(Debug, Deserialize)]
struct BedrockMetrics {
    latency_ms: Option<u64>,
}

#[async_trait]
impl Provider for BedrockProvider {
    fn name(&self) -> &'static str {
        "bedrock"
    }

    fn display_name(&self) -> &'static str {
        "AWS Bedrock"
    }

    async fn models(&self) -> ProviderResult<Vec<ModelInfo>> {
        // Return known Bedrock models
        let models = vec![
            ModelInfo {
                id: "anthropic.claude-3-5-sonnet-20241022-v2:0".to_string(),
                name: Some("Claude 3.5 Sonnet".to_string()),
                description: Some("Anthropic Claude 3.5 Sonnet".to_string()),
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
                id: "anthropic.claude-3-sonnet-20240229-v1:0".to_string(),
                name: Some("Claude 3 Sonnet".to_string()),
                description: Some("Anthropic Claude 3 Sonnet".to_string()),
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
                id: "anthropic.claude-3-haiku-20240307-v1:0".to_string(),
                name: Some("Claude 3 Haiku".to_string()),
                description: Some("Anthropic Claude 3 Haiku".to_string()),
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
            ModelInfo {
                id: "meta.llama3-70b-instruct-v1:0".to_string(),
                name: Some("Llama 3 70B".to_string()),
                description: Some("Meta Llama 3 70B Instruct".to_string()),
                context_window: Some(8192),
                max_output_tokens: Some(2048),
                capabilities: ModelCapabilities {
                    functions: false,
                    vision: false,
                    streaming: true,
                    json_mode: false,
                },
                pricing: crate::providers::cost::get_model_pricing("llama-3-70b"),
            },
            ModelInfo {
                id: "amazon.titan-text-express-v1".to_string(),
                name: Some("Titan Text Express".to_string()),
                description: Some("Amazon Titan Text Express".to_string()),
                context_window: Some(8192),
                max_output_tokens: Some(2048),
                capabilities: ModelCapabilities {
                    functions: false,
                    vision: false,
                    streaming: true,
                    json_mode: false,
                },
                pricing: None,
            },
        ];

        Ok(models)
    }

    async fn chat(&self, request: ChatRequest) -> ProviderResult<ChatResponse> {
        let url = self.endpoint(&request.model);
        
        // Extract system message
        let system_message = request.messages.iter()
            .find(|m| m.role == MessageRole::System)
            .map(|m| vec![BedrockSystemContent { text: m.content.clone() }]);

        // Convert messages
        let messages: Vec<BedrockMessage> = request.messages.into_iter()
            .filter(|m| m.role != MessageRole::System)
            .map(|m| BedrockMessage {
                role: match m.role {
                    MessageRole::User => "user".to_string(),
                    MessageRole::Assistant => "assistant".to_string(),
                    _ => "user".to_string(),
                },
                content: vec![BedrockContent { text: m.content }],
            }).collect();

        let bedrock_request = BedrockChatRequest {
            model_id: Some(request.model.clone()),
            messages,
            system: system_message,
            inference_config: BedrockInferenceConfig {
                max_tokens: request.max_tokens,
                temperature: request.temperature,
                top_p: request.top_p,
                stop_sequences: request.stop,
            },
        };

        let body = serde_json::to_string(&bedrock_request)?;
        let headers = self.sign_request("POST", &url, &body, "application/json")?;

        let mut req_builder = self.client.post(&url).body(body);
        for (key, value) in &headers {
            req_builder = req_builder.header(key, value);
        }

        let response = req_builder.send().await?;
        
        if !response.status().is_success() {
            let error = response.text().await.unwrap_or_default();
            return Err(ProviderError::InvalidRequest(format!("Bedrock API error: {}", error)));
        }

        let bedrock_response: BedrockChatResponse = response.json().await?;

        let content = bedrock_response.output.message.content.iter()
            .map(|c| c.text.clone())
            .collect::<Vec<_>>()
            .join("\n");

        let finish_reason = match bedrock_response.stop_reason.as_deref() {
            Some("end_turn") | Some("stop_sequence") => FinishReason::Stop,
            Some("max_tokens") => FinishReason::Length,
            Some("tool_use") => FinishReason::ToolCalls,
            _ => FinishReason::Other,
        };

        Ok(ChatResponse {
            model: request.model,
            message: Message {
                role: MessageRole::Assistant,
                content,
                name: None,
                tool_call_id: None,
                tool_calls: None,
            },
            finish_reason,
            usage: bedrock_response.usage.map(|u| Usage {
                input_tokens: u.input_tokens,
                output_tokens: u.output_tokens,
                total_tokens: u.total_tokens,
                input_token_details: None,
            }),
            id: None,
            created_at: None,
        })
    }

    async fn chat_stream(&self, _request: ChatRequest) -> ProviderResult<BoxStream<'static, ProviderResult<ChatChunk>>> {
        // Streaming not yet implemented for Bedrock
        Err(ProviderError::Internal("Streaming not implemented for Bedrock".to_string()))
    }
}

// Helper functions for SigV4 signing
fn sha256_hex(data: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data.as_bytes());
    hex::encode(hasher.finalize())
}

fn hmac_sha256(key: &[u8], data: &[u8]) -> Vec<u8> {
    type HmacSha256 = Hmac<Sha256>;
    let mut mac = HmacSha256::new_from_slice(key).unwrap();
    mac.update(data);
    mac.finalize().into_bytes().to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_name() {
        let provider = BedrockProvider::new("us-east-1", "test-key", "test-secret");
        assert_eq!(provider.name(), "bedrock");
        assert_eq!(provider.display_name(), "AWS Bedrock");
    }

    #[test]
    fn test_endpoint() {
        let provider = BedrockProvider::new("us-east-1", "test-key", "test-secret");
        let endpoint = provider.endpoint("anthropic.claude-3-sonnet-20240229-v1:0");
        assert!(endpoint.contains("bedrock-runtime.us-east-1.amazonaws.com"));
    }
}
