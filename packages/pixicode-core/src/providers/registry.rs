//! Provider Registry — Manages provider registration and lookup

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::providers::trait_def::{Provider, ProviderConfig, ProviderError, ProviderResult};
use crate::providers::types::ModelInfo;

/// Thread-safe provider registry.
pub struct ProviderRegistry {
    providers: RwLock<HashMap<String, Arc<dyn Provider>>>,
    configs: RwLock<HashMap<String, ProviderConfig>>,
}

impl ProviderRegistry {
    /// Creates a new empty registry.
    pub fn new() -> Self {
        Self {
            providers: RwLock::new(HashMap::new()),
            configs: RwLock::new(HashMap::new()),
        }
    }

    /// Creates a registry with built-in providers registered.
    pub fn with_builtins() -> Self {
        let registry = Self::new();
        registry.register_builtins();
        registry
    }

    /// Registers a provider with the registry.
    pub async fn register<P: Provider + 'static>(&self, provider: P) {
        let name = provider.name().to_string();
        tracing::debug!(name, "registering provider");
        self.providers.write().await.insert(name, Arc::new(provider));
    }

    /// Registers a provider with configuration.
    pub async fn register_with_config<P: Provider + 'static>(
        &self,
        provider: P,
        config: ProviderConfig,
    ) {
        let name = provider.name().to_string();
        tracing::debug!(name, "registering provider with config");
        self.providers.write().await.insert(name.clone(), Arc::new(provider));
        self.configs.write().await.insert(name, config);
    }

    /// Gets a provider by name.
    pub async fn get(&self, name: &str) -> Option<Arc<dyn Provider>> {
        self.providers.read().await.get(name).cloned()
    }

    /// Gets a provider or returns an error.
    pub async fn get_or_error(&self, name: &str) -> ProviderResult<Arc<dyn Provider>> {
        self.get(name).await.ok_or_else(|| ProviderError::NotFound(name.to_string()))
    }

    /// Lists all registered provider names.
    pub async fn list_providers(&self) -> Vec<String> {
        self.providers.read().await.keys().cloned().collect()
    }

    /// Gets configuration for a provider.
    pub async fn get_config(&self, name: &str) -> Option<ProviderConfig> {
        self.configs.read().await.get(name).cloned()
    }

    /// Returns the number of registered providers.
    pub async fn len(&self) -> usize {
        self.providers.read().await.len()
    }

    /// Checks if the registry is empty.
    pub async fn is_empty(&self) -> bool {
        self.len().await == 0
    }

    /// Gets all available models across all providers.
    pub async fn all_models(&self) -> ProviderResult<Vec<(String, ModelInfo)>> {
        let providers = self.providers.read().await;
        let mut all_models = Vec::new();

        for (name, provider) in providers.iter() {
            match provider.models().await {
                Ok(models) => {
                    for model in models {
                        all_models.push((name.to_string(), model));
                    }
                }
                Err(e) => {
                    tracing::warn!(provider = name, error = ?e, "failed to list models");
                }
            }
        }

        Ok(all_models)
    }

    /// Gets a specific model by ID (searches all providers).
    pub async fn get_model(&self, model_id: &str) -> ProviderResult<Option<(String, ModelInfo)>> {
        let providers = self.providers.read().await;

        for (name, provider) in providers.iter() {
            if let Some(model) = provider.model(model_id).await? {
                return Ok(Some((name.to_string(), model)));
            }
        }

        Ok(None)
    }

    /// Registers all built-in providers.
    pub fn register_builtins(&self) {
        use crate::providers::{ollama, openai, anthropic, google};

        // Register with default configurations
        // Actual API keys should be set via config or environment variables

        // Ollama (local, no API key needed)
        let ollama_provider = ollama::OllamaProvider::new();
        futures::executor::block_on(async {
            self.register(ollama_provider).await;
        });

        // OpenAI (requires API key)
        if let Ok(api_key) = std::env::var("OPENAI_API_KEY") {
            let openai_provider = openai::OpenAIProvider::new(&api_key);
            futures::executor::block_on(async {
                self.register(openai_provider).await;
            });
        }

        // Anthropic (requires API key)
        if let Ok(api_key) = std::env::var("ANTHROPIC_API_KEY") {
            let anthropic_provider = anthropic::AnthropicProvider::new(&api_key);
            futures::executor::block_on(async {
                self.register(anthropic_provider).await;
            });
        }

        // Google (requires API key)
        if let Ok(api_key) = std::env::var("GOOGLE_API_KEY") {
            let google_provider = google::GoogleProvider::new(&api_key);
            futures::executor::block_on(async {
                self.register(google_provider).await;
            });
        }

        tracing::info!("Registered {} built-in providers", futures::executor::block_on(self.len()));
    }
}

impl Default for ProviderRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::types::{ChatRequest, ChatResponse, ChatChunk, Message, FinishReason, Usage};
    use futures::stream;
    use futures::stream::BoxStream;
    use futures::StreamExt;

    struct TestProvider;

    #[async_trait::async_trait]
    impl Provider for TestProvider {
        fn name(&self) -> &'static str {
            "test"
        }

        async fn models(&self) -> ProviderResult<Vec<ModelInfo>> {
            Ok(vec![ModelInfo {
                id: "test-model".to_string(),
                name: Some("Test Model".to_string()),
                description: None,
                context_window: Some(4096),
                max_output_tokens: Some(1024),
                capabilities: Default::default(),
                pricing: None,
            }])
        }

        async fn chat(&self, _request: ChatRequest) -> ProviderResult<ChatResponse> {
            Ok(ChatResponse {
                model: "test-model".to_string(),
                message: Message::assistant("Test response"),
                finish_reason: FinishReason::Stop,
                usage: Some(Usage::new(10, 5)),
                id: None,
                created_at: None,
            })
        }

        async fn chat_stream(&self, _request: ChatRequest) -> ProviderResult<BoxStream<'static, ProviderResult<ChatChunk>>> {
            Ok(stream::empty().boxed())
        }
    }

    #[tokio::test]
    async fn test_registry() {
        let registry = ProviderRegistry::new();
        registry.register(TestProvider).await;

        assert_eq!(registry.len().await, 1);
        assert!(registry.get("test").await.is_some());
        assert!(registry.get("nonexistent").await.is_none());

        let providers = registry.list_providers().await;
        assert!(providers.contains(&"test".to_string()));
    }

    #[tokio::test]
    async fn test_chat() {
        let registry = ProviderRegistry::new();
        registry.register(TestProvider).await;

        let provider = registry.get("test").await.unwrap();
        let request = ChatRequest {
            cancel: None,
            model: "test-model".to_string(),
            messages: vec![Message::user("Hello")],
            tools: None,
            tool_choice: None,
            temperature: None,
            max_tokens: None,
            top_p: None,
            frequency_penalty: None,
            presence_penalty: None,
            stop: None,
            response_format: None,
            stream: false,
            user: None,
        };

        let response = provider.chat(request).await.unwrap();
        assert_eq!(response.model, "test-model");
        assert_eq!(response.message.content, "Test response");
    }
}
