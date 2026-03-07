//! File Write Tool — create new files

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::tools::trait_def::{Tool, ToolContext, ToolError, ToolOutput, ToolResult, ToolSchema, ToolParameter};

/// Parameters for the write tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WriteParams {
    /// Path to the file to write
    pub path: String,
    /// Content to write
    pub content: String,
    /// Create parent directories if they don't exist (default: false)
    #[serde(default)]
    pub create_dirs: bool,
    /// Fail if file already exists (default: false)
    #[serde(default)]
    pub exclusive: bool,
}

/// Tool for writing/creating files.
pub struct WriteTool;

impl WriteTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Tool for WriteTool {
    fn name(&self) -> &'static str {
        "write"
    }

    fn description(&self) -> &'static str {
        "Create a new file with the specified content"
    }

    fn schema(&self) -> ToolSchema {
        let mut schema = ToolSchema::default();
        schema.required = vec!["path".to_string(), "content".to_string()];
        schema.properties.insert("path".to_string(), ToolParameter::string("Path to the file to write"));
        schema.properties.insert("content".to_string(), ToolParameter::string("Content to write to the file"));
        schema.properties.insert("create_dirs".to_string(), ToolParameter::boolean("Create parent directories if needed"));
        schema.properties.insert("exclusive".to_string(), ToolParameter::boolean("Fail if file already exists"));
        schema
    }

    async fn execute(&self, params: serde_json::Value, context: &ToolContext) -> ToolResult<ToolOutput> {
        let params: WriteParams = serde_json::from_value(params)
            .map_err(|e| ToolError::InvalidParams(e.to_string()))?;

        // Resolve path relative to working directory
        let file_path = resolve_path(&params.path, &context.working_dir)?;

        // Check if file already exists
        if params.exclusive && file_path.exists() {
            return Err(ToolError::Execution(format!(
                "File already exists: {}",
                file_path.display()
            )));
        }

        // Create parent directories if requested
        if params.create_dirs {
            if let Some(parent) = file_path.parent() {
                tokio::fs::create_dir_all(parent)
                    .await
                    .map_err(|e| ToolError::Io(e))?;
            }
        }

        // Write file contents
        tokio::fs::write(&file_path, &params.content)
            .await
            .map_err(|e| ToolError::Io(e))?;

        let byte_count = params.content.len();
        let line_count = params.content.lines().count();

        Ok(ToolOutput::success(format!(
            "Created file: {} ({} bytes, {} lines)",
            file_path.display(),
            byte_count,
            line_count
        )).with_data(serde_json::json!({
            "path": file_path.to_string_lossy(),
            "bytes": byte_count,
            "lines": line_count,
        })))
    }
}

/// Resolves a path relative to the working directory.
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
    async fn test_write_file() {
        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("test.txt");

        let tool = WriteTool::new();
        let params = serde_json::json!({
            "path": file_path.to_string_lossy(),
            "content": "Hello, World!\n"
        });
        let context = ToolContext {
            working_dir: temp_dir.path().to_path_buf(),
            ..Default::default()
        };

        let result = tool.execute(params, &context).await;
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.success);

        // Verify file was created
        assert!(file_path.exists());
        let content = tokio::fs::read_to_string(&file_path).await.unwrap();
        assert_eq!(content, "Hello, World!\n");
    }

    #[tokio::test]
    async fn test_write_exclusive_fails() {
        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        tokio::fs::write(&file_path, "existing").await.unwrap();

        let tool = WriteTool::new();
        let params = serde_json::json!({
            "path": file_path.to_string_lossy(),
            "content": "new content",
            "exclusive": true
        });
        let context = ToolContext {
            working_dir: temp_dir.path().to_path_buf(),
            ..Default::default()
        };

        let result = tool.execute(params, &context).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_write_creates_dirs() {
        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("nested/deep/test.txt");

        let tool = WriteTool::new();
        let params = serde_json::json!({
            "path": "nested/deep/test.txt",
            "content": "content",
            "create_dirs": true
        });
        let context = ToolContext {
            working_dir: temp_dir.path().to_path_buf(),
            ..Default::default()
        };

        let result = tool.execute(params, &context).await;
        assert!(result.is_ok());
        assert!(file_path.exists());
    }
}
