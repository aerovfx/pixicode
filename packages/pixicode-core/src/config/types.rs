//! Config types
//!
//! Mirrors the TypeScript config schema so the Rust layer can read and write
//! the same `.pixicode/pixicode.jsonc` files used by the TS/Bun side.

use std::collections::HashMap;
use std::path::PathBuf;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use tracing::{debug, warn};

use crate::config::paths::ConfigPaths;

// ─────────────────────────────────────────────────────────────────────────────
//  Top-level Config struct
// ─────────────────────────────────────────────────────────────────────────────

/// Full pixicode configuration (mirrors TS `Config.Info`).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Config {
    pub providers: HashMap<String, ProviderConfig>,
    pub model: Option<String>,
    pub models: HashMap<String, ModelConfig>,
    pub theme: Option<String>,
    pub keybinds: KeybindsConfig,
    pub agents: HashMap<String, AgentConfig>,
    pub mcp: HashMap<String, McpConfig>,
    pub permission: PermissionConfig,
    pub share: Option<String>,
    pub username: Option<String>,
    pub autoshare: Option<bool>,
    pub instructions: Vec<String>,
    pub plugin: Vec<String>,

    // Internal — not persisted to disk
    #[serde(skip)]
    pub _data_dir: PathBuf,
    #[serde(skip)]
    pub _config_dir: PathBuf,
}

// ─────────────────────────────────────────────────────────────────────────────
//  Provider / Model
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ProviderConfig {
    pub api_key: Option<String>,
    pub base_url: Option<String>,
    pub disabled: bool,
    pub models: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ModelConfig {
    pub temperature: Option<f64>,
    pub top_p: Option<f64>,
    pub max_tokens: Option<u32>,
}

// ─────────────────────────────────────────────────────────────────────────────
//  Keybinds (optional overrides — all default to sensible values)
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct KeybindsConfig {
    pub leader: String,
    pub app_exit: String,
    pub session_new: String,
    pub session_list: String,
    pub session_interrupt: String,
    pub messages_page_up: String,
    pub messages_page_down: String,
}

impl Default for KeybindsConfig {
    fn default() -> Self {
        Self {
            leader: "ctrl+x".into(),
            app_exit: "ctrl+c,ctrl+d,<leader>q".into(),
            session_new: "<leader>n".into(),
            session_list: "<leader>l".into(),
            session_interrupt: "escape".into(),
            messages_page_up: "pageup,ctrl+alt+b".into(),
            messages_page_down: "pagedown,ctrl+alt+f".into(),
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
//  Agent
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AgentConfig {
    pub model: Option<String>,
    pub temperature: Option<f64>,
    pub prompt: Option<String>,
    pub disable: bool,
    pub mode: Option<String>,
    pub steps: Option<u32>,
    pub permission: Option<PermissionConfig>,
}

// ─────────────────────────────────────────────────────────────────────────────
//  MCP
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum McpConfig {
    Local {
        command: Vec<String>,
        #[serde(default)]
        environment: HashMap<String, String>,
        #[serde(default = "default_true")]
        enabled: bool,
        #[serde(default = "default_timeout")]
        timeout: u64,
    },
    Remote {
        url: String,
        #[serde(default)]
        headers: HashMap<String, String>,
        #[serde(default = "default_true")]
        enabled: bool,
        #[serde(default = "default_timeout")]
        timeout: u64,
    },
}

fn default_true() -> bool {
    true
}
fn default_timeout() -> u64 {
    5000
}

// ─────────────────────────────────────────────────────────────────────────────
//  Permissions
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct PermissionConfig {
    pub read: Option<PermissionRule>,
    pub edit: Option<PermissionRule>,
    pub bash: Option<PermissionRule>,
    pub webfetch: Option<PermissionAction>,
    pub websearch: Option<PermissionAction>,
    pub glob: Option<PermissionRule>,
    pub grep: Option<PermissionRule>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PermissionRule {
    Action(PermissionAction),
    PathMap(HashMap<String, PermissionAction>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PermissionAction {
    Ask,
    Allow,
    Deny,
}

// ─────────────────────────────────────────────────────────────────────────────
//  JSONC parser utility
// ─────────────────────────────────────────────────────────────────────────────

/// Strip JSONC comments (`// …` and `/* … */`) and parse as JSON.
pub fn parse_jsonc<T: for<'de> Deserialize<'de>>(input: &str) -> Result<T> {
    let stripped = strip_jsonc_comments(input);
    serde_json::from_str(&stripped).context("Failed to parse config JSON")
}

/// Minimal JSONC comment stripper that handles both `//` (line) and `/* */`
/// (block) comments while respecting string literals.
fn strip_jsonc_comments(src: &str) -> String {
    let mut out = String::with_capacity(src.len());
    let mut chars = src.chars().peekable();
    let mut in_string = false;
    let mut escaped = false;

    while let Some(c) = chars.next() {
        if escaped {
            out.push(c);
            escaped = false;
            continue;
        }
        if c == '\\' && in_string {
            out.push(c);
            escaped = true;
            continue;
        }
        if c == '"' {
            in_string = !in_string;
            out.push(c);
            continue;
        }
        if in_string {
            out.push(c);
            continue;
        }
        // Outside a string — check for comment starts
        if c == '/' {
            match chars.peek() {
                Some('/') => {
                    // Line comment — consume until newline
                    for ch in chars.by_ref() {
                        if ch == '\n' {
                            out.push('\n');
                            break;
                        }
                    }
                }
                Some('*') => {
                    // Block comment — consume until `*/`
                    chars.next(); // consume '*'
                    loop {
                        match chars.next() {
                            Some('*') if chars.peek() == Some(&'/') => {
                                chars.next(); // consume '/'
                                break;
                            }
                            Some('\n') => out.push('\n'),
                            None => break,
                            _ => {}
                        }
                    }
                }
                _ => out.push(c),
            }
        } else {
            out.push(c);
        }
    }
    out
}

// ─────────────────────────────────────────────────────────────────────────────
//  Loading
// ─────────────────────────────────────────────────────────────────────────────

impl Config {
    /// Load configuration with full precedence chain:
    ///
    /// 1. Global config (`~/.config/pixicode/pixicode.jsonc`)
    /// 2. `$PIXICODE_CONFIG` override file
    /// 3. Project `.pixicode/pixicode.jsonc`
    /// 4. Inline `$PIXICODE_CONFIG_CONTENT`
    /// 5. `PIXICODE_*` env-var overrides
    pub async fn load() -> Result<Config> {
        let paths = ConfigPaths::new()?;
        let mut cfg = Config {
            _data_dir: paths.data_dir().to_path_buf(),
            _config_dir: paths.config_dir().to_path_buf(),
            ..Default::default()
        };

        // 1. Global config
        for candidate in ["pixicode.jsonc", "pixicode.json"] {
            let p = paths.config_dir().join(candidate);
            if p.exists() {
                debug!(path = %p.display(), "Loading global config");
                let raw = tokio::fs::read_to_string(&p).await?;
                let partial: serde_json::Value = parse_jsonc(&raw).unwrap_or_default();
                merge_into(&mut cfg, partial)?;
                break;
            }
        }

        // 2. PIXICODE_CONFIG override
        if let Ok(custom_path) = std::env::var("PIXICODE_CONFIG") {
            debug!(path = %custom_path, "Loading PIXICODE_CONFIG");
            let raw = tokio::fs::read_to_string(&custom_path).await?;
            let partial: serde_json::Value = parse_jsonc(&raw)?;
            merge_into(&mut cfg, partial)?;
        }

        // 3. Project .pixicode/pixicode.jsonc (walk up CWD)
        if let Ok(cwd) = std::env::current_dir() {
            let mut dir = cwd.as_path();
            loop {
                for candidate in ["pixicode.jsonc", "pixicode.json"] {
                    let p = dir.join(".pixicode").join(candidate);
                    if p.exists() {
                        debug!(path = %p.display(), "Loading project config");
                        let raw = tokio::fs::read_to_string(&p).await?;
                        let partial: serde_json::Value = parse_jsonc(&raw).unwrap_or_default();
                        merge_into(&mut cfg, partial)?;
                    }
                }
                match dir.parent() {
                    Some(p) => dir = p,
                    None => break,
                }
            }
        }

        // 4. Inline env content
        if let Ok(content) = std::env::var("PIXICODE_CONFIG_CONTENT") {
            debug!("Loading PIXICODE_CONFIG_CONTENT");
            let partial: serde_json::Value = parse_jsonc(&content)?;
            merge_into(&mut cfg, partial)?;
        }

        // 5. Individual PIXICODE_* env overrides
        apply_env_overrides(&mut cfg);

        Ok(cfg)
    }

    /// Returns the user-facing data directory (where `pixicode.db` lives).
    pub fn data_dir(&self) -> &std::path::Path {
        &self._data_dir
    }

    /// Returns the config directory.
    pub fn config_dir(&self) -> &std::path::Path {
        &self._config_dir
    }
}

/// Naive deep-merge: deserialise `partial` and overlay on `base`.
fn merge_into(base: &mut Config, partial: serde_json::Value) -> Result<()> {
    if partial.is_null() || (partial.is_object() && partial.as_object().map(|o| o.is_empty()).unwrap_or(true)) {
        return Ok(());
    }
    // Re-serialise base, merge at JSON level, deserialise back
    let mut base_val = serde_json::to_value(&*base)?;
    json_merge(&mut base_val, partial);
    let merged: Config = serde_json::from_value(base_val)?;
    // Preserve internal (skip) fields
    let data_dir = std::mem::take(&mut base._data_dir);
    let config_dir = std::mem::take(&mut base._config_dir);
    *base = merged;
    base._data_dir = data_dir;
    base._config_dir = config_dir;
    Ok(())
}

/// Recursive JSON merge — object keys are merged, arrays/scalars replaced.
fn json_merge(base: &mut serde_json::Value, overlay: serde_json::Value) {
    if let (Some(base_obj), Some(overlay_obj)) = (base.as_object_mut(), overlay.as_object()) {
        for (key, val) in overlay_obj {
            let entry = base_obj.entry(key).or_insert(serde_json::Value::Null);
            json_merge(entry, val.clone());
        }
    } else {
        *base = overlay;
    }
}

/// Apply `PIXICODE_MODEL`, `PIXICODE_THEME`, etc. env overrides.
fn apply_env_overrides(cfg: &mut Config) {
    if let Ok(model) = std::env::var("PIXICODE_MODEL") {
        cfg.model = Some(model);
    }
    if let Ok(theme) = std::env::var("PIXICODE_THEME") {
        cfg.theme = Some(theme);
    }
    if let Ok(username) = std::env::var("PIXICODE_USERNAME") {
        cfg.username = Some(username);
    }
}

// ─────────────────────────────────────────────────────────────────────────────
//  Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_jsonc_line_comment() {
        let src = r#"{ "a": 1 // comment
        }"#;
        let stripped = strip_jsonc_comments(src);
        let v: serde_json::Value = serde_json::from_str(&stripped).unwrap();
        assert_eq!(v["a"], 1);
    }

    #[test]
    fn test_strip_jsonc_block_comment() {
        let src = r#"{ /* block */ "b": 2 }"#;
        let stripped = strip_jsonc_comments(src);
        let v: serde_json::Value = serde_json::from_str(&stripped).unwrap();
        assert_eq!(v["b"], 2);
    }

    #[test]
    fn test_strip_jsonc_url_in_string() {
        let src = r#"{ "url": "https://example.com/path" }"#;
        let stripped = strip_jsonc_comments(src);
        let v: serde_json::Value = serde_json::from_str(&stripped).unwrap();
        assert_eq!(v["url"], "https://example.com/path");
    }

    #[test]
    fn test_parse_full_config() {
        let src = r#"{
            // my config
            "model": "anthropic/claude-opus-4",
            "theme": "dark",
            /* providers section */
            "providers": {
                "anthropic": { "apiKey": "sk-xxx" }
            }
        }"#;
        let cfg: Config = parse_jsonc(src).unwrap();
        assert_eq!(cfg.model.as_deref(), Some("anthropic/claude-opus-4"));
        assert_eq!(cfg.theme.as_deref(), Some("dark"));
    }
}
