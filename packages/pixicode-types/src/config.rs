//! Config types for shared usage

use serde::{Deserialize, Serialize};
use schemars::JsonSchema;
use std::collections::HashMap;

/// Config summary for UI.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ConfigSummary {
    /// Config file path
    pub config_path: String,
    /// Data directory
    pub data_dir: String,
    /// Default model
    pub default_model: Option<String>,
    /// Configured providers
    pub providers: Vec<String>,
    /// Theme name
    pub theme: Option<String>,
}

/// Provider config summary.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ProviderSummary {
    /// Provider name
    pub name: String,
    /// Whether provider is configured
    pub configured: bool,
    /// Available models
    pub models: Vec<String>,
}

/// Config update request.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ConfigUpdate {
    /// Key to update
    pub key: String,
    /// New value
    pub value: serde_json::Value,
}

/// Key binding config.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct KeyBindingConfig {
    /// Key bindings by context
    pub bindings: HashMap<String, Vec<KeyBindingEntry>>,
}

/// Key binding entry.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct KeyBindingEntry {
    /// Key combination
    pub key: String,
    /// Command to execute
    pub command: String,
}

/// Theme config.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ThemeConfig {
    /// Theme name
    pub name: String,
    /// Theme colors
    pub colors: HashMap<String, String>,
}
