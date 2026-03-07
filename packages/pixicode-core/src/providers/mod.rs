//! AI Provider Layer — Multi-provider AI chat completion framework
//!
//! Provides a unified interface for multiple AI providers:
//!  - Provider trait with chat and streaming support
//!  - Multiple provider implementations (OpenAI, Anthropic, Google, Ollama, etc.)
//!  - Token counting and cost calculation
//!  - Rate limiting and retry logic
//!  - Authentication management

pub mod trait_def;
pub mod types;
pub mod registry;

// Provider implementations
pub mod ollama;
pub mod openai;
pub mod anthropic;
pub mod google;
pub mod vertex;
pub mod bedrock;
pub mod azure_openai;
pub mod compatible;

// Utilities
pub mod streaming;
pub mod auth;
pub mod oauth_flows;
pub mod cost;
pub mod retry;

pub use trait_def::{Provider, ProviderError, ProviderResult};
pub use types::{
    Message, MessageRole, Model, ModelInfo,
    ChatRequest, ChatResponse,
    ToolDefinition, ToolCall, ToolResult,
    Usage, FinishReason,
};
pub use registry::ProviderRegistry;

// Re-export compatible providers
pub use compatible::{
    GroqProvider, MistralProvider, XAIProvider,
    CerebrasProvider, CohereProvider,
    DeepInfraProvider, TogetherProvider, PerplexityProvider,
    OpenRouterProvider, FireworksProvider, AnyscaleProvider,
};
