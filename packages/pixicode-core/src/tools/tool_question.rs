//! Question Tool — ask user questions

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::tools::trait_def::{Tool, ToolContext, ToolError, ToolOutput, ToolResult, ToolSchema, ToolParameter};

/// Parameters for the question tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestionParams {
    /// Question to ask the user
    pub question: String,
    /// Question type (text, confirm, select, multiselect)
    #[serde(default)]
    pub question_type: Option<String>,
    /// Options for select/multiselect
    #[serde(default)]
    pub options: Option<Vec<String>>,
    /// Default value
    #[serde(default)]
    pub default: Option<String>,
    /// Whether the question is required (default: true)
    #[serde(default = "default_true")]
    pub required: bool,
}

fn default_true() -> bool { true }

/// Tool for asking user questions.
pub struct QuestionTool;

impl QuestionTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Tool for QuestionTool {
    fn name(&self) -> &'static str {
        "question"
    }

    fn description(&self) -> &'static str {
        "Ask the user a question and wait for their response"
    }

    fn schema(&self) -> ToolSchema {
        let mut schema = ToolSchema::default();
        schema.required = vec!["question".to_string()];
        schema.properties.insert("question".to_string(), ToolParameter::string("Question to ask the user"));
        schema.properties.insert("question_type".to_string(), ToolParameter::string("Question type: text, confirm, select, multiselect"));
        schema.properties.insert("options".to_string(), ToolParameter {
            param_type: "array".to_string(),
            description: "Options for select/multiselect questions".to_string(),
            default: None,
            enum_values: None,
            minimum: None,
            maximum: None,
            min_length: None,
            max_length: None,
            items: Some(Box::new(ToolParameter::string("Option"))),
        });
        schema.properties.insert("default".to_string(), ToolParameter::string("Default value"));
        schema.properties.insert("required".to_string(), ToolParameter::boolean("Whether the question is required"));
        schema
    }

    async fn execute(&self, params: serde_json::Value, context: &ToolContext) -> ToolResult<ToolOutput> {
        let params: QuestionParams = serde_json::from_value(params)
            .map_err(|e| ToolError::InvalidParams(e.to_string()))?;

        let question_type = params.question_type.as_deref().unwrap_or("text");

        // Format the question prompt
        let mut prompt = String::new();
        prompt.push_str(&format!("❓ {}\n", params.question));

        let response = match question_type {
            "confirm" | "boolean" => {
                prompt.push_str(&format!(" [Y/n]{}", 
                    params.default.as_ref().map(|d| format!(" (default: {})", d)).unwrap_or_default()));
                prompt.push_str("\n\nNote: In non-interactive mode, using default value");
                
                let default = params.default.as_ref().map(|d| d.to_lowercase() == "y" || d.to_lowercase() == "yes" || d == "true");
                default.unwrap_or(true).to_string()
            }
            
            "select" | "single" => {
                if let Some(options) = &params.options {
                    prompt.push_str("\nOptions:\n");
                    for (i, option) in options.iter().enumerate() {
                        prompt.push_str(&format!("  {}. {}\n", i + 1, option));
                    }
                    prompt.push_str(&format!("\nEnter a number (1-{})", options.len()));
                    if let Some(default) = &params.default {
                        prompt.push_str(&format!(" [default: {}]", default));
                    }
                } else {
                    return Err(ToolError::InvalidParams("Select question requires options".to_string()));
                }
                
                prompt.push_str("\n\nNote: In non-interactive mode, using default value");
                params.default.unwrap_or_else(|| "1".to_string())
            }
            
            "multiselect" | "multiple" => {
                if let Some(options) = &params.options {
                    prompt.push_str("\nOptions:\n");
                    for (i, option) in options.iter().enumerate() {
                        prompt.push_str(&format!("  {}. {}\n", i + 1, option));
                    }
                    prompt.push_str(&format!("\nEnter comma-separated numbers (e.g., 1,3,5)"));
                } else {
                    return Err(ToolError::InvalidParams("Multiselect question requires options".to_string()));
                }
                
                prompt.push_str("\n\nNote: In non-interactive mode, returning empty selection");
                params.default.unwrap_or_default()
            }
            
            "text" | _ => {
                if let Some(default) = &params.default {
                    prompt.push_str(&format!(" [default: {}]", default));
                }
                prompt.push_str("\n\nNote: In non-interactive mode, using default value");
                params.default.unwrap_or_default()
            }
        };

        prompt.push_str(&format!("\n\nResponse: {}", response));

        Ok(ToolOutput::success(prompt).with_data(serde_json::json!({
            "question": params.question,
            "type": question_type,
            "response": response,
            "options": params.options,
        })))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_question_text() {
        let tool = QuestionTool::new();
        let params = serde_json::json!({
            "question": "What is your name?",
            "default": "Anonymous"
        });
        let context = ToolContext::default();

        let result = tool.execute(params, &context).await;
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.output.contains("What is your name?"));
        assert!(output.output.contains("Anonymous"));
    }

    #[tokio::test]
    async fn test_question_confirm() {
        let tool = QuestionTool::new();
        let params = serde_json::json!({
            "question": "Continue?",
            "question_type": "confirm"
        });
        let context = ToolContext::default();

        let result = tool.execute(params, &context).await;
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.output.contains("[Y/n]"));
    }

    #[tokio::test]
    async fn test_question_select() {
        let tool = QuestionTool::new();
        let params = serde_json::json!({
            "question": "Choose an option",
            "question_type": "select",
            "options": ["Option A", "Option B", "Option C"],
            "default": "2"
        });
        let context = ToolContext::default();

        let result = tool.execute(params, &context).await;
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.output.contains("Option A"));
        assert!(output.output.contains("Option B"));
    }
}
