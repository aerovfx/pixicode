//! Directory Listing Tool — list directory contents

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::tools::trait_def::{Tool, ToolContext, ToolError, ToolOutput, ToolResult, ToolSchema, ToolParameter};

/// Parameters for the ls tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LsParams {
    /// Path to the directory (default: current dir)
    #[serde(default)]
    pub path: Option<String>,
    /// Show hidden files (default: false)
    #[serde(default)]
    pub all: bool,
    /// Show detailed info (default: false)
    #[serde(default)]
    pub long: bool,
}

/// Tool for listing directory contents.
pub struct LsTool;

impl LsTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Tool for LsTool {
    fn name(&self) -> &'static str {
        "ls"
    }

    fn description(&self) -> &'static str {
        "List contents of a directory"
    }

    fn schema(&self) -> ToolSchema {
        let mut schema = ToolSchema::default();
        schema.properties.insert("path".to_string(), ToolParameter::string("Path to the directory"));
        schema.properties.insert("all".to_string(), ToolParameter::boolean("Show hidden files"));
        schema.properties.insert("long".to_string(), ToolParameter::boolean("Show detailed info"));
        schema
    }

    async fn execute(&self, params: serde_json::Value, context: &ToolContext) -> ToolResult<ToolOutput> {
        let params: LsParams = serde_json::from_value(params)
            .map_err(|e| ToolError::InvalidParams(e.to_string()))?;

        let dir_path = match &params.path {
            Some(p) => resolve_path(p, &context.working_dir)?,
            None => context.working_dir.clone(),
        };

        if !dir_path.exists() {
            return Err(ToolError::Execution(format!("Directory not found: {}", dir_path.display())));
        }

        if !dir_path.is_dir() {
            return Err(ToolError::Execution(format!("Not a directory: {}", dir_path.display())));
        }

        let mut entries = tokio::fs::read_dir(&dir_path)
            .await
            .map_err(|e| ToolError::Io(e))?;

        let mut files = Vec::new();
        let mut dirs = Vec::new();

        while let Some(entry) = entries.next_entry().await.map_err(|e| ToolError::Io(e))? {
            let name = entry.file_name().to_string_lossy().to_string();

            // Skip hidden files unless --all
            if !params.all && name.starts_with('.') {
                continue;
            }

            let path = entry.path();
            let is_dir = entry.file_type().await.map_err(|e| ToolError::Io(e))?.is_dir();

            if is_dir {
                dirs.push(name);
            } else {
                files.push(name);
            }
        }

        dirs.sort();
        files.sort();

        let mut output = String::new();
        output.push_str(&format!("Directory: {}\n\n", dir_path.display()));

        if !dirs.is_empty() {
            output.push_str("Directories:\n");
            for dir in &dirs {
                output.push_str(&format!("  {}/\n", dir));
            }
            output.push('\n');
        }

        if !files.is_empty() {
            output.push_str("Files:\n");
            for file in &files {
                output.push_str(&format!("  {}\n", file));
            }
        }

        let total = dirs.len() + files.len();
        output.push_str(&format!("\nTotal: {} item(s)", total));

        Ok(ToolOutput::success(output).with_data(serde_json::json!({
            "path": dir_path.to_string_lossy(),
            "directories": dirs,
            "files": files,
            "total": total,
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
    async fn test_ls() {
        let temp_dir = tempdir().unwrap();
        tokio::fs::write(temp_dir.path().join("file1.txt"), "content").await.unwrap();
        tokio::fs::write(temp_dir.path().join("file2.txt"), "content").await.unwrap();
        tokio::fs::create_dir(temp_dir.path().join("subdir")).await.unwrap();

        let tool = LsTool::new();
        let params = serde_json::json!({
            "path": temp_dir.path().to_string_lossy()
        });
        let context = ToolContext::default();

        let result = tool.execute(params, &context).await;
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.output.contains("file1.txt"));
        assert!(output.output.contains("subdir/"));
    }
}
