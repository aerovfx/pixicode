//! Glob Tool — glob pattern matching for files

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::tools::trait_def::{Tool, ToolContext, ToolError, ToolOutput, ToolResult, ToolSchema, ToolParameter};

/// Parameters for the glob tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobParams {
    /// Glob pattern (e.g., "**/*.rs")
    pub pattern: String,
    /// Base directory to search from (default: working dir)
    #[serde(default)]
    pub cwd: Option<String>,
    /// Include hidden files (default: false)
    #[serde(default)]
    pub include_hidden: bool,
    /// Maximum number of results (default: 100)
    #[serde(default = "default_limit")]
    pub limit: u32,
}

fn default_limit() -> u32 { 100 }

/// Tool for glob pattern matching.
pub struct GlobTool;

impl GlobTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Tool for GlobTool {
    fn name(&self) -> &'static str {
        "glob"
    }

    fn description(&self) -> &'static str {
        "Find files matching a glob pattern"
    }

    fn schema(&self) -> ToolSchema {
        let mut schema = ToolSchema::default();
        schema.required = vec!["pattern".to_string()];
        schema.properties.insert("pattern".to_string(), ToolParameter::string("Glob pattern (e.g., **/*.rs)"));
        schema.properties.insert("cwd".to_string(), ToolParameter::string("Base directory to search from"));
        schema.properties.insert("include_hidden".to_string(), ToolParameter::boolean("Include hidden files"));
        schema.properties.insert("limit".to_string(), ToolParameter::integer("Maximum number of results"));
        schema
    }

    async fn execute(&self, params: serde_json::Value, context: &ToolContext) -> ToolResult<ToolOutput> {
        let params: GlobParams = serde_json::from_value(params)
            .map_err(|e| ToolError::InvalidParams(e.to_string()))?;

        let base_dir = match &params.cwd {
            Some(cwd) => resolve_path(cwd, &context.working_dir)?,
            None => context.working_dir.clone(),
        };

        if !base_dir.exists() {
            return Err(ToolError::Execution(format!("Directory not found: {}", base_dir.display())));
        }

        // Use the glob crate
        let pattern = base_dir.join(&params.pattern).to_string_lossy().to_string();
        
        let mut matches = Vec::new();
        for entry in glob::glob(&pattern).map_err(|e| ToolError::Execution(e.to_string()))? {
            if matches.len() >= params.limit as usize {
                break;
            }

            match entry {
                Ok(path) => {
                    // Skip hidden files unless requested
                    if !params.include_hidden {
                        if let Some(name) = path.file_name() {
                            if name.to_string_lossy().starts_with('.') {
                                continue;
                            }
                        }
                    }

                    // Make path relative to base_dir for cleaner output
                    let relative = path.strip_prefix(&base_dir)
                        .map(|p| p.to_string_lossy().to_string())
                        .unwrap_or_else(|_| path.to_string_lossy().to_string());
                    matches.push(relative);
                }
                Err(e) => {
                    tracing::warn!("Glob entry error: {}", e);
                }
            }
        }

        matches.sort();

        let mut output = String::new();
        output.push_str(&format!("Pattern: {}\n\n", params.pattern));
        
        if matches.is_empty() {
            output.push_str("No matches found");
        } else {
            output.push_str(&format!("Found {} match(es):\n", matches.len()));
            for m in &matches {
                output.push_str(&format!("  {}\n", m));
            }
        }

        Ok(ToolOutput::success(output).with_data(serde_json::json!({
            "pattern": params.pattern,
            "matches": matches,
            "count": matches.len(),
        })))
    }
}

fn resolve_path(path: &str, working_dir: &PathBuf) -> Result<PathBuf, ToolError> {
    let path = PathBuf::from(path);
    if path.is_absolute() {
        Ok(path)
    } else {
        Ok(working_dir.join(path))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_glob() {
        let temp_dir = tempdir().unwrap();
        tokio::fs::write(temp_dir.path().join("file1.txt"), "content").await.unwrap();
        tokio::fs::write(temp_dir.path().join("file2.txt"), "content").await.unwrap();
        tokio::fs::write(temp_dir.path().join("file3.rs"), "content").await.unwrap();

        let tool = GlobTool::new();
        let params = serde_json::json!({
            "pattern": "*.txt",
            "cwd": temp_dir.path().to_string_lossy()
        });
        let context = ToolContext::default();

        let result = tool.execute(params, &context).await;
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.output.contains("file1.txt"));
        assert!(output.output.contains("file2.txt"));
        assert!(!output.output.contains("file3.rs"));
    }
}
