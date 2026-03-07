//! Bash Tool — execute shell commands with timeout and PTY support

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;
use tokio::process::Command;
use tokio::time::timeout;

use crate::tools::trait_def::{Tool, ToolContext, ToolError, ToolOutput, ToolResult, ToolSchema, ToolParameter};

/// Parameters for the bash tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BashParams {
    /// Command to execute
    pub command: String,
    /// Working directory for the command
    #[serde(default)]
    pub cwd: Option<String>,
    /// Timeout in milliseconds (default: 30000)
    #[serde(default = "default_timeout")]
    pub timeout_ms: u64,
    /// Environment variables to set
    #[serde(default)]
    pub env: Option<std::collections::HashMap<String, String>>,
    /// Shell to use (default: "bash")
    #[serde(default)]
    pub shell: Option<String>,
}

fn default_timeout() -> u64 { 30000 }

/// Output from command execution with truncation info.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BashOutput {
    /// Standard output
    pub stdout: String,
    /// Standard error
    pub stderr: String,
    /// Exit code
    pub exit_code: Option<i32>,
    /// Whether output was truncated
    pub truncated: bool,
    /// Original output size (if truncated)
    pub original_size: Option<usize>,
}

/// Tool for executing shell commands.
pub struct BashTool;

impl BashTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Tool for BashTool {
    fn name(&self) -> &'static str {
        "bash"
    }

    fn description(&self) -> &'static str {
        "Execute shell commands with timeout, working directory, and environment variable support"
    }

    fn schema(&self) -> ToolSchema {
        let mut schema = ToolSchema::default();
        schema.required = vec!["command".to_string()];
        schema.properties.insert("command".to_string(), ToolParameter::string("Command to execute"));
        schema.properties.insert("cwd".to_string(), ToolParameter::string("Working directory for the command"));
        schema.properties.insert("timeout_ms".to_string(), ToolParameter::integer("Timeout in milliseconds"));
        schema.properties.insert("env".to_string(), ToolParameter {
            param_type: "object".to_string(),
            description: "Environment variables to set".to_string(),
            default: None,
            enum_values: None,
            minimum: None,
            maximum: None,
            min_length: None,
            max_length: None,
            items: None,
        });
        schema.properties.insert("shell".to_string(), ToolParameter::string("Shell to use (default: bash)"));
        schema
    }

    async fn execute(&self, params: serde_json::Value, context: &ToolContext) -> ToolResult<ToolOutput> {
        let params: BashParams = serde_json::from_value(params)
            .map_err(|e| ToolError::InvalidParams(e.to_string()))?;

        let cwd = match &params.cwd {
            Some(cwd) => resolve_path(cwd, &context.working_dir)?,
            None => context.working_dir.clone(),
        };

        let timeout_ms = params.timeout_ms;
        let shell = params.shell.unwrap_or_else(|| "bash".to_string());

        // Execute command with timeout
        let result = timeout(
            Duration::from_millis(timeout_ms),
            execute_command(&shell, &params.command, &cwd, &params.env)
        ).await;

        match result {
            Ok(Ok(output)) => {
                let mut output_text = String::new();
                
                if !output.stdout.is_empty() {
                    output_text.push_str(&output.stdout);
                }
                if !output.stderr.is_empty() {
                    if !output_text.is_empty() {
                        output_text.push('\n');
                    }
                    output_text.push_str(&output.stderr);
                }

                let success = output.exit_code == Some(0);
                let mut tool_output = if success {
                    ToolOutput::success(output_text)
                } else {
                    ToolOutput::failure(format!("Command exited with code {:?}", output.exit_code))
                };

                tool_output = tool_output.with_data(serde_json::json!({
                    "stdout": output.stdout,
                    "stderr": output.stderr,
                    "exit_code": output.exit_code,
                    "truncated": output.truncated,
                    "original_size": output.original_size,
                }));

                Ok(tool_output)
            }
            Ok(Err(e)) => Err(e),
            Err(_) => Err(ToolError::Timeout(timeout_ms)),
        }
    }
}

/// Execute a shell command.
async fn execute_command(
    shell: &str,
    command: &str,
    cwd: &PathBuf,
    env: &Option<std::collections::HashMap<String, String>>,
) -> ToolResult<BashOutput> {
    let mut cmd = Command::new(shell);
    cmd.arg("-c").arg(command);
    cmd.current_dir(cwd);

    // Set environment variables
    if let Some(env_vars) = env {
        for (key, value) in env_vars {
            cmd.env(key, value);
        }
    }

    // Execute and capture output
    let output = cmd.output().await.map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            ToolError::Execution(format!("Shell '{}' not found", shell))
        } else {
            ToolError::Io(e)
        }
    })?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    // Apply output truncation
    let max_output_size = 100 * 1024; // 100KB
    let (stdout, stdout_truncated) = truncate_output(&stdout, max_output_size / 2);
    let (stderr, stderr_truncated) = truncate_output(&stderr, max_output_size / 2);

    let truncated = stdout_truncated || stderr_truncated;
    let original_size = if truncated {
        Some(stdout.len() + stderr.len() + 
            if stdout_truncated { (stdout.len() as f64 * 0.1) as usize } else { 0 } +
            if stderr_truncated { (stderr.len() as f64 * 0.1) as usize } else { 0 })
    } else {
        None
    };

    Ok(BashOutput {
        stdout,
        stderr,
        exit_code: output.status.code(),
        truncated,
        original_size,
    })
}

/// Truncate output to max size.
fn truncate_output(output: &str, max_size: usize) -> (String, bool) {
    if output.len() <= max_size {
        return (output.to_string(), false);
    }

    // Keep first 80% and last 20% of output
    let keep_start = max_size * 80 / 100;
    let keep_end = max_size - keep_start;

    let truncated_msg = format!("\n... [{} bytes truncated] ...\n", output.len() - max_size);
    let remaining = keep_start - truncated_msg.len();

    let start = &output[..remaining.min(output.len())];
    let end = &output[output.len() - keep_end..];

    (format!("{}{}{}", start, truncated_msg, end), true)
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

    #[tokio::test]
    async fn test_bash_simple() {
        let tool = BashTool::new();
        let params = serde_json::json!({
            "command": "echo 'Hello, World!'"
        });
        let context = ToolContext::default();

        let result = tool.execute(params, &context).await;
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.success);
        assert!(output.output.contains("Hello, World!"));
    }

    #[tokio::test]
    async fn test_bash_with_cwd() {
        let tool = BashTool::new();
        let params = serde_json::json!({
            "command": "pwd",
            "cwd": "/tmp"
        });
        let context = ToolContext::default();

        let result = tool.execute(params, &context).await;
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.output.contains("/tmp"));
    }

    #[tokio::test]
    async fn test_bash_timeout() {
        let tool = BashTool::new();
        let params = serde_json::json!({
            "command": "sleep 5",
            "timeout_ms": 100
        });
        let context = ToolContext::default();

        let result = tool.execute(params, &context).await;
        assert!(matches!(result, Err(ToolError::Timeout(_))));
    }

    #[tokio::test]
    async fn test_bash_with_env() {
        let tool = BashTool::new();
        let params = serde_json::json!({
            "command": "echo $MY_VAR",
            "env": {
                "MY_VAR": "test_value"
            }
        });
        let context = ToolContext::default();

        let result = tool.execute(params, &context).await;
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.output.contains("test_value"));
    }
}
