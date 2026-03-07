//! OpenAI-Compatible Providers — Groq, Mistral, xAI, Cerebras, Cohere, etc.
//!
//! These providers use the OpenAI API format and can use the OpenAIProvider
//! with different base URLs. This module provides convenience constructors.

use crate::providers::openai::OpenAIProvider;

/// Groq provider (ultra-fast inference).
pub struct GroqProvider(OpenAIProvider);

impl GroqProvider {
    pub fn new(api_key: &str) -> Self {
        Self(
            OpenAIProvider::compatible(
                api_key,
                "https://api.groq.com/openai/v1"
            )
        )
    }

    pub fn into_inner(self) -> OpenAIProvider {
        self.0
    }
}

impl std::ops::Deref for GroqProvider {
    type Target = OpenAIProvider;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Mistral AI provider.
pub struct MistralProvider(OpenAIProvider);

impl MistralProvider {
    pub fn new(api_key: &str) -> Self {
        Self(
            OpenAIProvider::compatible(
                api_key,
                "https://api.mistral.ai/v1"
            )
        )
    }

    pub fn into_inner(self) -> OpenAIProvider {
        self.0
    }
}

impl std::ops::Deref for MistralProvider {
    type Target = OpenAIProvider;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// xAI (Grok) provider.
pub struct XAIProvider(OpenAIProvider);

impl XAIProvider {
    pub fn new(api_key: &str) -> Self {
        Self(
            OpenAIProvider::compatible(
                api_key,
                "https://api.x.ai/v1"
            )
        )
    }

    pub fn into_inner(self) -> OpenAIProvider {
        self.0
    }
}

impl std::ops::Deref for XAIProvider {
    type Target = OpenAIProvider;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Cerebras provider.
pub struct CerebrasProvider(OpenAIProvider);

impl CerebrasProvider {
    pub fn new(api_key: &str) -> Self {
        Self(
            OpenAIProvider::compatible(
                api_key,
                "https://api.cerebras.ai/v1"
            )
        )
    }

    pub fn into_inner(self) -> OpenAIProvider {
        self.0
    }
}

impl std::ops::Deref for CerebrasProvider {
    type Target = OpenAIProvider;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Cohere provider (Command models).
pub struct CohereProvider(OpenAIProvider);

impl CohereProvider {
    pub fn new(api_key: &str) -> Self {
        Self(
            OpenAIProvider::compatible(
                api_key,
                "https://api.cohere.ai/v1"
            )
        )
    }

    pub fn into_inner(self) -> OpenAIProvider {
        self.0
    }
}

impl std::ops::Deref for CohereProvider {
    type Target = OpenAIProvider;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// DeepInfra provider.
pub struct DeepInfraProvider(OpenAIProvider);

impl DeepInfraProvider {
    pub fn new(api_key: &str) -> Self {
        Self(
            OpenAIProvider::compatible(
                api_key,
                "https://api.deepinfra.com/v1/openai"
            )
        )
    }

    pub fn into_inner(self) -> OpenAIProvider {
        self.0
    }
}

impl std::ops::Deref for DeepInfraProvider {
    type Target = OpenAIProvider;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Together AI provider.
pub struct TogetherProvider(OpenAIProvider);

impl TogetherProvider {
    pub fn new(api_key: &str) -> Self {
        Self(
            OpenAIProvider::compatible(
                api_key,
                "https://api.together.xyz/v1"
            )
        )
    }

    pub fn into_inner(self) -> OpenAIProvider {
        self.0
    }
}

impl std::ops::Deref for TogetherProvider {
    type Target = OpenAIProvider;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Perplexity AI provider.
pub struct PerplexityProvider(OpenAIProvider);

impl PerplexityProvider {
    pub fn new(api_key: &str) -> Self {
        Self(
            OpenAIProvider::compatible(
                api_key,
                "https://api.perplexity.ai"
            )
        )
    }

    pub fn into_inner(self) -> OpenAIProvider {
        self.0
    }
}

impl std::ops::Deref for PerplexityProvider {
    type Target = OpenAIProvider;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// OpenRouter provider (multi-provider routing).
pub struct OpenRouterProvider(OpenAIProvider);

impl OpenRouterProvider {
    pub fn new(api_key: &str) -> Self {
        Self(
            OpenAIProvider::compatible(
                api_key,
                "https://openrouter.ai/api/v1"
            )
        )
    }

    pub fn into_inner(self) -> OpenAIProvider {
        self.0
    }
}

impl std::ops::Deref for OpenRouterProvider {
    type Target = OpenAIProvider;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Fireworks AI provider.
pub struct FireworksProvider(OpenAIProvider);

impl FireworksProvider {
    pub fn new(api_key: &str) -> Self {
        Self(
            OpenAIProvider::compatible(
                api_key,
                "https://api.fireworks.ai/inference/v1"
            )
        )
    }

    pub fn into_inner(self) -> OpenAIProvider {
        self.0
    }
}

impl std::ops::Deref for FireworksProvider {
    type Target = OpenAIProvider;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Anyscale provider.
pub struct AnyscaleProvider(OpenAIProvider);

impl AnyscaleProvider {
    pub fn new(api_key: &str) -> Self {
        Self(
            OpenAIProvider::compatible(
                api_key,
                "https://api.endpoints.anyscale.com/v1"
            )
        )
    }

    pub fn into_inner(self) -> OpenAIProvider {
        self.0
    }
}

impl std::ops::Deref for AnyscaleProvider {
    type Target = OpenAIProvider;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::trait_def::Provider;

    #[test]
    fn test_groq_provider() {
        let provider = GroqProvider::new("test-key");
        assert_eq!(provider.name(), "openai");
    }

    #[test]
    fn test_mistral_provider() {
        let provider = MistralProvider::new("test-key");
        assert_eq!(provider.name(), "openai");
    }

    #[test]
    fn test_xai_provider() {
        let provider = XAIProvider::new("test-key");
        assert_eq!(provider.name(), "openai");
    }

    #[test]
    fn test_openrouter_provider() {
        let provider = OpenRouterProvider::new("test-key");
        assert_eq!(provider.name(), "openai");
    }
}
