//! Grep Tool — search file contents using ripgrep

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};

use crate::tools::trait_def::{Tool, ToolContext, ToolError, ToolOutput, ToolResult, ToolSchema, ToolParameter};

/// Parameters for the grep tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrepParams {
    /// Pattern to search for
    pub pattern: String,
    /// Directory to search in (default: working dir)
    #[serde(default)]
    pub path: Option<String>,
    /// Case insensitive (default: false)
    #[serde(default)]
    pub ignore_case: bool,
    /// Use regex (default: true)
    #[serde(default = "default_true")]
    pub regex: bool,
    /// Include hidden files (default: false)
    #[serde(default)]
    pub hidden: bool,
    /// Maximum number of results (default: 50)
    #[serde(default = "default_limit")]
    pub limit: u32,
    /// File pattern to include (e.g., "*.rs")
    #[serde(default)]
    pub include: Option<String>,
    /// File pattern to exclude (e.g., "*.min.js")
    #[serde(default)]
    pub exclude: Option<String>,
}

fn default_true() -> bool { true }
fn default_limit() -> u32 { 50 }

/// Match result with context.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrepMatch {
    pub file: String,
    pub line_number: u32,
    pub line: String,
    pub matches: Vec<String>,
}

/// Tool for searching file contents.
pub struct GrepTool;

impl GrepTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Tool for GrepTool {
    fn name(&self) -> &'static str {
        "grep"
    }

    fn description(&self) -> &'static str {
        "Search for text patterns in files using ripgrep"
    }

    fn schema(&self) -> ToolSchema {
        let mut schema = ToolSchema::default();
        schema.required = vec!["pattern".to_string()];
        schema.properties.insert("pattern".to_string(), ToolParameter::string("Pattern to search for"));
        schema.properties.insert("path".to_string(), ToolParameter::string("Directory to search in"));
        schema.properties.insert("ignore_case".to_string(), ToolParameter::boolean("Case insensitive search"));
        schema.properties.insert("regex".to_string(), ToolParameter::boolean("Use regex (default: true)"));
        schema.properties.insert("hidden".to_string(), ToolParameter::boolean("Include hidden files"));
        schema.properties.insert("limit".to_string(), ToolParameter::integer("Maximum number of results"));
        schema.properties.insert("include".to_string(), ToolParameter::string("File pattern to include"));
        schema.properties.insert("exclude".to_string(), ToolParameter::string("File pattern to exclude"));
        schema
    }

    async fn execute(&self, params: serde_json::Value, context: &ToolContext) -> ToolResult<ToolOutput> {
        let params: GrepParams = serde_json::from_value(params)
            .map_err(|e| ToolError::InvalidParams(e.to_string()))?;

        let search_dir = match &params.path {
            Some(p) => resolve_path(p, &context.working_dir)?,
            None => context.working_dir.clone(),
        };

        if !search_dir.exists() {
            return Err(ToolError::Execution(format!("Directory not found: {}", search_dir.display())));
        }

        // Build ripgrep command
        let mut cmd = tokio::process::Command::new("rg");
        cmd.arg("--json")
            .arg("--line-number")
            .arg("--color=never")
            .arg("--max-count")
            .arg(params.limit.to_string())
            .arg(&params.pattern)
            .arg(&search_dir);

        // Add flags
        if params.ignore_case {
            cmd.arg("--ignore-case");
        }
        if params.hidden {
            cmd.arg("--hidden");
        }
        if !params.regex {
            cmd.arg("--fixed-strings");
        }

        // Add file filters
        if let Some(include) = &params.include {
            cmd.arg("--glob").arg(include);
        }
        if let Some(exclude) = &params.exclude {
            cmd.arg("--glob").arg(format!("!{}", exclude));
        }

        // Execute ripgrep
        let child = cmd
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::NotFound {
                    ToolError::Execution("ripgrep (rg) not found. Please install it.".to_string())
                } else {
                    ToolError::Io(e)
                }
            })?;

        let output = child.wait_with_output().await.map_err(|e| ToolError::Io(e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if stderr.contains("no matches found") {
                return Ok(ToolOutput::success("No matches found").with_data(serde_json::json!({
                    "pattern": params.pattern,
                    "matches": [],
                    "count": 0,
                })));
            }
            return Err(ToolError::Execution(format!("ripgrep error: {}", stderr)));
        }

        // Parse JSON output
        let mut matches = Vec::new();
        let reader = BufReader::new(&output.stderr[..]);
        let mut lines = reader.lines();

        while let Ok(Some(line)) = lines.next_line().await {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&line) {
                if let Some(data) = json.get("data") {
                    let file = data.get("path").and_then(|p| p.get("text"))
                        .map(|s| s.to_string())
                        .unwrap_or_default();
                    
                    let line_num = data.get("line_number").and_then(|n| n.as_u64())
                        .unwrap_or(0) as u32;
                    
                    let line_text = data.get("lines").and_then(|l| l.as_str())
                        .unwrap_or("")
                        .trim()
                        .to_string();

                    // Make file path relative
                    let relative_file = file.strip_prefix(&format!("{}/", search_dir.display()))
                        .unwrap_or(&file)
                        .to_string();

                    matches.push(GrepMatch {
                        file: relative_file,
                        line_number: line_num,
                        line: line_text,
                        matches: vec![],
                    });
                }
            }
        }

        // Format output
        let mut output_text = String::new();
        output_text.push_str(&format!("Pattern: {}\n\n", params.pattern));

        if matches.is_empty() {
            output_text.push_str("No matches found");
        } else {
            output_text.push_str(&format!("Found {} match(es):\n\n", matches.len()));
            let mut current_file = String::new();
            
            for m in &matches {
                if m.file != current_file {
                    current_file = m.file.clone();
                    output_text.push_str(&format!("{}\n", current_file));
                }
                output_text.push_str(&format!("  {}: {}\n", m.line_number, m.line));
            }
        }

        let match_data: Vec<serde_json::Value> = matches.iter().map(|m| {
            serde_json::json!({
                "file": m.file,
                "line": m.line_number,
                "text": m.line,
            })
        }).collect();

        Ok(ToolOutput::success(output_text).with_data(serde_json::json!({
            "pattern": params.pattern,
            "matches": match_data,
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
    async fn test_grep() {
        let temp_dir = tempdir().unwrap();
        tokio::fs::write(temp_dir.path().join("test.txt"), "hello world\nfoo bar\nhello again\n").await.unwrap();

        let tool = GrepTool::new();
        let params = serde_json::json!({
            "pattern": "hello",
            "path": temp_dir.path().to_string_lossy()
        });
        let context = ToolContext::default();

        // Skip if rg is not installed
        if std::process::Command::new("rg").output().is_err() {
            println!("Skipping test - ripgrep not installed");
            return;
        }

        let result = tool.execute(params, &context).await;
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.output.contains("hello"));
    }
}
