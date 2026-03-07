//! File Read Tool — read file contents with line range and size limits

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::tools::trait_def::{Tool, ToolContext, ToolError, ToolOutput, ToolResult, ToolSchema, ToolParameter};

/// Parameters for the read tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadParams {
    /// Path to the file to read
    pub path: String,
    /// Start line (1-indexed, default: 1)
    #[serde(default = "default_start_line")]
    pub start_line: u32,
    /// End line (inclusive, default: all)
    #[serde(default)]
    pub end_line: Option<u32>,
    /// Maximum output size in bytes (default: 100KB)
    #[serde(default = "default_max_size")]
    pub max_size: u32,
}

fn default_start_line() -> u32 { 1 }
fn default_max_size() -> u32 { 100 * 1024 }

/// Tool for reading file contents.
pub struct ReadTool;

impl ReadTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Tool for ReadTool {
    fn name(&self) -> &'static str {
        "read"
    }

    fn description(&self) -> &'static str {
        "Read contents of a file with optional line range and size limits"
    }

    fn schema(&self) -> ToolSchema {
        let mut schema = ToolSchema::default();
        schema.required = vec!["path".to_string()];
        schema.properties.insert("path".to_string(), ToolParameter::string("Path to the file to read"));
        schema.properties.insert("start_line".to_string(), ToolParameter::integer("Start line (1-indexed)"));
        schema.properties.insert("end_line".to_string(), ToolParameter::integer("End line (inclusive)"));
        schema.properties.insert("max_size".to_string(), ToolParameter::integer("Maximum output size in bytes"));
        schema
    }

    async fn execute(&self, params: serde_json::Value, context: &ToolContext) -> ToolResult<ToolOutput> {
        let params: ReadParams = serde_json::from_value(params)
            .map_err(|e| ToolError::InvalidParams(e.to_string()))?;

        // Resolve path relative to working directory
        let file_path = resolve_path(&params.path, &context.working_dir)?;

        // Check if file exists
        if !file_path.exists() {
            return Err(ToolError::Execution(format!("File not found: {}", file_path.display())));
        }

        // Check if it's a file (not a directory)
        if !file_path.is_file() {
            return Err(ToolError::Execution(format!("Not a file: {}", file_path.display())));
        }

        // Read file contents
        let content = tokio::fs::read_to_string(&file_path)
            .await
            .map_err(|e| ToolError::Io(e))?;

        // Check size limit
        if content.len() > params.max_size as usize {
            return Err(ToolError::Execution(format!(
                "File too large ({} bytes, max {} bytes)",
                content.len(),
                params.max_size
            )));
        }

        // Apply line range if specified
        let output = if params.end_line.is_some() || params.start_line > 1 {
            let lines: Vec<&str> = content.lines().collect();
            let start = (params.start_line - 1) as usize;
            let end = params.end_line.map(|e| e as usize).unwrap_or(lines.len());

            if start >= lines.len() {
                return Err(ToolError::Execution(format!(
                    "Start line {} exceeds file length ({} lines)",
                    params.start_line,
                    lines.len()
                )));
            }

            let end = end.min(lines.len());
            lines[start..end].join("\n")
        } else {
            content
        };

        // Add line count and path info
        let line_count = output.lines().count();
        let formatted = format!(
            "File: {}\nLines: {}-{} of {}\n\n{}",
            file_path.display(),
            params.start_line,
            params.end_line.unwrap_or(line_count as u32),
            line_count,
            output
        );

        Ok(ToolOutput::success(formatted).with_data(serde_json::json!({
            "path": file_path.to_string_lossy(),
            "content": output,
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
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[tokio::test]
    async fn test_read_file() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "line 1").unwrap();
        writeln!(temp_file, "line 2").unwrap();
        writeln!(temp_file, "line 3").unwrap();

        let tool = ReadTool::new();
        let params = serde_json::json!({
            "path": temp_file.path().to_string_lossy()
        });
        let context = ToolContext::default();

        let result = tool.execute(params, &context).await;
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.success);
        assert!(output.output.contains("line 1"));
    }

    #[tokio::test]
    async fn test_read_line_range() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "line 1").unwrap();
        writeln!(temp_file, "line 2").unwrap();
        writeln!(temp_file, "line 3").unwrap();

        let tool = ReadTool::new();
        let params = serde_json::json!({
            "path": temp_file.path().to_string_lossy(),
            "start_line": 2,
            "end_line": Some(3)
        });
        let context = ToolContext::default();

        let result = tool.execute(params, &context).await;
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.output.contains("line 2"));
        assert!(output.output.contains("line 3"));
        assert!(!output.output.contains("line 1"));
    }
}
