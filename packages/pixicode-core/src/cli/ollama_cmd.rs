//! Ollama CLI commands — Manage Ollama models and configuration

use anyhow::{Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::process::Command;

/// Ollama model info.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OllamaModel {
    pub name: String,
    pub size: u64,
    pub digest: String,
    pub modified_at: Option<String>,
}

/// Ollama list response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OllamaListResponse {
    pub models: Vec<OllamaModel>,
}

/// List available Ollama models.
pub async fn list_models() -> Result<Vec<OllamaModel>> {
    let client = Client::new();
    let response = client
        .get("http://localhost:11434/api/tags")
        .send()
        .await
        .context("Failed to connect to Ollama. Is it running?")?;

    if !response.status().is_success() {
        anyhow::bail!("Ollama API error: {}", response.status());
    }

    let list_response: OllamaListResponse = response
        .json()
        .await
        .context("Failed to parse Ollama response")?;

    Ok(list_response.models)
}

/// Pull a model from Ollama.
pub async fn pull_model(name: &str) -> Result<()> {
    println!("🦙 Pulling model: {}", name);
    
    let status = Command::new("ollama")
        .args(["pull", name])
        .status()
        .context("Failed to run ollama command. Is Ollama installed?")?;

    if !status.success() {
        anyhow::bail!("Failed to pull model: {}", status);
    }

    println!("✅ Model pulled successfully: {}", name);
    Ok(())
}

/// Remove a model from Ollama.
pub async fn remove_model(name: &str) -> Result<()> {
    println!("🗑️  Removing model: {}", name);
    
    let status = Command::new("ollama")
        .args(["rm", name])
        .status()
        .context("Failed to run ollama command")?;

    if !status.success() {
        anyhow::bail!("Failed to remove model: {}", status);
    }

    println!("✅ Model removed: {}", name);
    Ok(())
}

/// Show model info.
pub async fn show_model(name: &str) -> Result<()> {
    let status = Command::new("ollama")
        .args(["show", name])
        .status()
        .context("Failed to run ollama command")?;

    if !status.success() {
        anyhow::bail!("Failed to show model info: {}", status);
    }

    Ok(())
}

/// Run a model interactively.
pub async fn run_model(name: &str, prompt: Option<&str>) -> Result<()> {
    if let Some(p) = prompt {
        // One-shot prompt
        let output = Command::new("ollama")
            .args(["run", name, p])
            .output()
            .context("Failed to run ollama command")?;

        if !output.status.success() {
            anyhow::bail!("Failed to run model: {}", output.status);
        }

        println!("{}", String::from_utf8_lossy(&output.stdout));
    } else {
        // Interactive mode
        let status = Command::new("ollama")
            .args(["run", name])
            .status()
            .context("Failed to run ollama command")?;

        if !status.success() {
            anyhow::bail!("Interactive session failed: {}", status);
        }
    }

    Ok(())
}

/// Check if Ollama is installed and running.
pub async fn check_ollama() -> Result<()> {
    // Check if ollama command exists
    let status = Command::new("ollama").arg("--version").status();

    match status {
        Ok(s) if s.success() => {
            println!("✅ Ollama is installed");

            // Check if Ollama is running
            let client = Client::new();
            match client.get("http://localhost:11434/api/tags").send().await {
                Ok(response) if response.status().is_success() => {
                    println!("✅ Ollama is running");
                    Ok(())
                }
                _ => {
                    anyhow::bail!("Ollama is installed but not running. Start it with: ollama serve");
                }
            }
        }
        _ => {
            anyhow::bail!("Ollama is not installed. Install it from: https://ollama.com");
        }
    }
}

/// Get default Ollama model or prompt user to select.
pub async fn get_default_model() -> Result<String> {
    let models = list_models().await?;
    
    if models.is_empty() {
        anyhow::bail!("No Ollama models found. Pull a model with: ollama pull <model-name>");
    }

    // Return first model if only one exists
    if models.len() == 1 {
        return Ok(models[0].name.clone());
    }

    // For multiple models, return the most recently modified one
    let mut sorted_models = models.clone();
    sorted_models.sort_by(|a, b| {
        b.modified_at.cmp(&a.modified_at)
    });

    Ok(sorted_models[0].name.clone())
}

/// Format size in human-readable format.
pub fn format_size(size: u64) -> String {
    const GB: u64 = 1024 * 1024 * 1024;
    const MB: u64 = 1024 * 1024;
    
    if size >= GB {
        format!("{:.1} GB", size as f64 / GB as f64)
    } else if size >= MB {
        format!("{:.1} MB", size as f64 / MB as f64)
    } else {
        format!("{} KB", size / 1024)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_size() {
        assert_eq!(format_size(1024), "1 KB");
        assert_eq!(format_size(1024 * 1024), "1.0 MB");
        assert_eq!(format_size(1024 * 1024 * 1024), "1.0 GB");
        assert_eq!(format_size(6 * 1024 * 1024 * 1024), "6.0 GB");
    }
}
