//! Plugin system — runtime load/run plugins from config.

use std::path::Path;
use std::process::Command;

/// Manages plugin paths from config; can run a plugin by path or name.
#[derive(Debug, Clone, Default)]
pub struct PluginManager {
    plugins: Vec<String>,
}

impl PluginManager {
    pub fn new(plugins: Vec<String>) -> Self {
        Self { plugins }
    }

    /// Plugin paths/names from config.
    pub fn list(&self) -> &[String] {
        &self.plugins
    }

    /// Run a plugin: `name_or_path` is either an index into list, a path, or a basename match.
    /// Spawns the process and returns stdout/stderr. Does not load .so/.dylib; runs as subprocess.
    pub fn run(&self, name_or_path: &str, args: &[String]) -> Result<std::process::Output, String> {
        let exe = self.resolve(name_or_path)?;
        let out = Command::new(&exe)
            .args(args)
            .output()
            .map_err(|e| e.to_string())?;
        Ok(out)
    }

    fn resolve(&self, name_or_path: &str) -> Result<String, String> {
        if Path::new(name_or_path).exists() {
            return Ok(name_or_path.to_string());
        }
        if let Ok(i) = name_or_path.parse::<usize>() {
            if let Some(p) = self.plugins.get(i) {
                return Ok(p.clone());
            }
        }
        for p in &self.plugins {
            if p.ends_with(name_or_path) {
                return Ok(p.clone());
            }
            if Path::new(p).file_stem().and_then(|s| s.to_str()) == Some(name_or_path) {
                return Ok(p.clone());
            }
        }
        Err(format!("plugin not found: {}", name_or_path))
    }
}
