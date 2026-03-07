//! XDG base-directory helpers for config and data paths.

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

/// Resolved directories used by pixicode.
#[derive(Debug, Clone)]
pub struct ConfigPaths {
    /// `$XDG_CONFIG_HOME/pixicode`  (default: `~/.config/pixicode`)
    config_dir: PathBuf,
    /// `$XDG_DATA_HOME/pixicode`    (default: `~/.local/share/pixicode`)
    data_dir: PathBuf,
    /// `$XDG_CACHE_HOME/pixicode`   (default: `~/.cache/pixicode`)
    cache_dir: PathBuf,
}

impl ConfigPaths {
    /// Build paths from XDG env-vars (or platform defaults).
    pub fn new() -> Result<Self> {
        let config_dir = Self::resolve_config_dir()?;
        let data_dir = Self::resolve_data_dir()?;
        let cache_dir = Self::resolve_cache_dir()?;

        std::fs::create_dir_all(&config_dir)
            .with_context(|| format!("create config dir: {}", config_dir.display()))?;
        std::fs::create_dir_all(&data_dir)
            .with_context(|| format!("create data dir: {}", data_dir.display()))?;
        std::fs::create_dir_all(&cache_dir)
            .with_context(|| format!("create cache dir: {}", cache_dir.display()))?;

        Ok(Self { config_dir, data_dir, cache_dir })
    }

    fn resolve_config_dir() -> Result<PathBuf> {
        // PIXICODE_CONFIG_HOME wins over XDG_CONFIG_HOME
        if let Ok(v) = std::env::var("PIXICODE_CONFIG_HOME") {
            return Ok(PathBuf::from(v));
        }
        let base = std::env::var("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                dirs::config_dir().unwrap_or_else(|| {
                    dirs::home_dir()
                        .map(|h| h.join(".config"))
                        .unwrap_or_else(|| PathBuf::from(".config"))
                })
            });
        Ok(base.join("pixicode"))
    }

    fn resolve_data_dir() -> Result<PathBuf> {
        if let Ok(v) = std::env::var("PIXICODE_DATA_HOME") {
            return Ok(PathBuf::from(v));
        }
        let base = std::env::var("XDG_DATA_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                dirs::data_dir().unwrap_or_else(|| {
                    dirs::home_dir()
                        .map(|h| h.join(".local").join("share"))
                        .unwrap_or_else(|| PathBuf::from(".local/share"))
                })
            });
        Ok(base.join("pixicode"))
    }

    fn resolve_cache_dir() -> Result<PathBuf> {
        let base = std::env::var("XDG_CACHE_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                dirs::cache_dir().unwrap_or_else(|| {
                    dirs::home_dir()
                        .map(|h| h.join(".cache"))
                        .unwrap_or_else(|| PathBuf::from(".cache"))
                })
            });
        Ok(base.join("pixicode"))
    }

    pub fn config_dir(&self) -> &Path {
        &self.config_dir
    }

    pub fn data_dir(&self) -> &Path {
        &self.data_dir
    }

    pub fn cache_dir(&self) -> &Path {
        &self.cache_dir
    }

    /// Path to the main SQLite database file.
    pub fn db_path(&self) -> PathBuf {
        self.data_dir.join("pixicode.db")
    }

    /// Path to the legacy `opencode.db` file (for migration detection).
    pub fn legacy_db_path(&self) -> PathBuf {
        self.data_dir.join("opencode.db")
    }

    /// Path to the global config file.
    pub fn global_config(&self) -> PathBuf {
        self.config_dir.join("pixicode.jsonc")
    }
}
