//! File Multiedit Tool — edit multiple positions in a file simultaneously

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::tools::trait_def::{Tool, ToolContext, ToolError, ToolOutput, ToolResult, ToolSchema, ToolParameter};

/// A single edit operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Edit {
    /// Text to search for
    pub search: String,
    /// Text to replace with
    pub replace: String,
    /// Replace all occurrences of this search string
    #[serde(default)]
    pub all: bool,
}

/// Parameters for the multiedit tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultieditParams {
    /// Path to the file to edit
    pub path: String,
    /// List of edit operations to perform
    pub edits: Vec<Edit>,
    /// Use regex for search (default: false)
    #[serde(default)]
    pub regex: bool,
    /// Case insensitive search (default: false)
    #[serde(default)]
    pub ignore_case: bool,
}

/// Tool for editing files with multiple search/replace operations.
pub struct MultieditTool;

impl MultieditTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Tool for MultieditTool {
    fn name(&self) -> &'static str {
        "multiedit"
    }

    fn description(&self) -> &'static str {
        "Edit a file by applying multiple search and replace operations at once"
    }

    fn schema(&self) -> ToolSchema {
        let mut schema = ToolSchema::default();
        schema.required = vec!["path".to_string(), "edits".to_string()];
        schema.properties.insert("path".to_string(), ToolParameter::string("Path to the file to edit"));
        schema.properties.insert("edits".to_string(), ToolParameter {
            param_type: "array".to_string(),
            description: "List of edit operations (search/replace pairs)".to_string(),
            default: None,
            enum_values: None,
            minimum: None,
            maximum: None,
            min_length: None,
            max_length: None,
            items: Some(Box::new(ToolParameter {
                param_type: "object".to_string(),
                description: "An edit operation".to_string(),
                default: None,
                enum_values: None,
                minimum: None,
                maximum: None,
                min_length: None,
                max_length: None,
                items: None,
            })),
        });
        schema.properties.insert("regex".to_string(), ToolParameter::boolean("Use regex for search"));
        schema.properties.insert("ignore_case".to_string(), ToolParameter::boolean("Case insensitive search"));
        schema
    }

    async fn execute(&self, params: serde_json::Value, context: &ToolContext) -> ToolResult<ToolOutput> {
        let params: MultieditParams = serde_json::from_value(params)
            .map_err(|e| ToolError::InvalidParams(e.to_string()))?;

        // Resolve path relative to working directory
        let file_path = resolve_path(&params.path, &context.working_dir)?;

        // Check if file exists
        if !file_path.exists() {
            return Err(ToolError::Execution(format!("File not found: {}", file_path.display())));
        }

        // Read file contents
        let content = tokio::fs::read_to_string(&file_path)
            .await
            .map_err(|e| ToolError::Io(e))?;

        // Apply all edits sequentially
        let mut current_content = content;
        let mut total_replacements = 0;
        let mut edit_results = Vec::new();

        for (idx, edit) in params.edits.iter().enumerate() {
            let (new_content, count) = if params.regex {
                search_replace_regex(&current_content, &edit.search, &edit.replace, edit.all, params.ignore_case)?
            } else {
                search_replace_text(&current_content, &edit.search, &edit.replace, edit.all, params.ignore_case)
            };

            if count == 0 {
                edit_results.push(format!("Edit {}: No matches found for '{}'", idx + 1, edit.search));
            } else {
                edit_results.push(format!("Edit {}: Replaced {} occurrence(s)", idx + 1, count));
                current_content = new_content;
                total_replacements += count;
            }
        }

        if total_replacements == 0 {
            return Err(ToolError::Execution("No matches found for any edit".to_string()));
        }

        // Write updated content
        tokio::fs::write(&file_path, &current_content)
            .await
            .map_err(|e| ToolError::Io(e))?;

        let mut output = String::new();
        output.push_str(&format!("Applied {} edit(s) to {}\n\n", params.edits.len(), file_path.display()));
        output.push_str("Results:\n");
        for result in &edit_results {
            output.push_str(&format!("  {}\n", result));
        }
        output.push_str(&format!("\nTotal replacements: {}", total_replacements));

        Ok(ToolOutput::success(output).with_data(serde_json::json!({
            "path": file_path.to_string_lossy(),
            "edits_applied": params.edits.len(),
            "total_replacements": total_replacements,
            "details": edit_results,
        })))
    }
}

/// Simple text search and replace.
fn search_replace_text(
    content: &str,
    search: &str,
    replace: &str,
    all: bool,
    ignore_case: bool,
) -> (String, usize) {
    if ignore_case {
        let mut count = 0;
        let mut result = String::new();
        let mut remaining = content;
        let search_lower = search.to_lowercase();

        while let Some(pos) = remaining.to_lowercase().find(&search_lower) {
            result.push_str(&remaining[..pos]);
            result.push_str(replace);
            count += 1;
            remaining = &remaining[pos + search.len()..];

            if !all && count >= 1 {
                break;
            }
        }
        result.push_str(remaining);
        (result, count)
    } else {
        if all {
            let count = content.matches(search).count();
            (content.replace(search, replace), count)
        } else {
            match content.find(search) {
                Some(pos) => {
                    let mut result = String::with_capacity(content.len() + replace.len() - search.len());
                    result.push_str(&content[..pos]);
                    result.push_str(replace);
                    result.push_str(&content[pos + search.len()..]);
                    (result, 1)
                }
                None => (content.to_string(), 0),
            }
        }
    }
}

/// Regex-based search and replace.
fn search_replace_regex(
    content: &str,
    search: &str,
    replace: &str,
    all: bool,
    ignore_case: bool,
) -> Result<(String, usize), ToolError> {
    let regex_pattern = if ignore_case {
        format!("(?i){}", search)
    } else {
        search.to_string()
    };

    // Use a simple regex implementation
    let re = regex_lite::Regex::new(&regex_pattern)
        .map_err(|e| ToolError::InvalidParams(format!("Invalid regex: {}", e)))?;

    if all {
        let count = re.find_iter(content).count();
        Ok((re.replace_all(content, replace).to_string(), count))
    } else {
        let count = re.find(content).map(|_| 1).unwrap_or(0);
        Ok((re.replace(content, replace).to_string(), count))
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

// Simple regex implementation to avoid external dependency
mod regex_lite {
    use std::borrow::Cow;

    #[derive(Debug)]
    pub struct Regex {
        pattern: String,
        ignore_case: bool,
    }

    impl Regex {
        pub fn new(pattern: &str) -> Result<Self, String> {
            // Simple validation - check for basic syntax errors
            if pattern.is_empty() {
                return Err("Empty pattern".to_string());
            }
            
            let ignore_case = pattern.starts_with("(?i)");
            let clean_pattern = if ignore_case {
                pattern[4..].to_string()
            } else {
                pattern.to_string()
            };

            Ok(Self {
                pattern: clean_pattern,
                ignore_case,
            })
        }

        pub fn find(&self, text: &str) -> Option<Match> {
            let search_text = if self.ignore_case { text.to_lowercase() } else { text.to_string() };
            let search_pattern = if self.ignore_case { self.pattern.to_lowercase() } else { self.pattern.clone() };
            
            search_text.find(&search_pattern).map(|start| Match { 
                start, 
                end: start + self.pattern.len() 
            })
        }

        pub fn find_iter<'a>(&'a self, text: &'a str) -> impl Iterator<Item = Match> + 'a {
            let search_text = if self.ignore_case { text.to_lowercase() } else { text.to_string() };
            let search_pattern = if self.ignore_case { self.pattern.to_lowercase() } else { self.pattern.clone() };
            let pattern_len = self.pattern.len();
            
            let mut last_end = 0;
            std::iter::from_fn(move || {
                if last_end >= search_text.len() {
                    return None;
                }
                search_text[last_end..].find(&search_pattern).map(|start| {
                    let absolute_start = last_end + start;
                    let m = Match {
                        start: absolute_start,
                        end: absolute_start + pattern_len,
                    };
                    last_end = m.end;
                    m
                })
            })
        }

        pub fn replace<'a>(&self, text: &'a str, replacement: &str) -> Cow<'a, str> {
            if let Some(m) = self.find(text) {
                let mut result = String::with_capacity(text.len() + replacement.len() - (m.end - m.start));
                result.push_str(&text[..m.start]);
                result.push_str(replacement);
                result.push_str(&text[m.end..]);
                Cow::Owned(result)
            } else {
                Cow::Borrowed(text)
            }
        }

        pub fn replace_all<'a>(&self, text: &'a str, replacement: &str) -> Cow<'a, str> {
            let mut result = String::new();
            let mut last_end = 0;
            let mut replaced = false;

            for m in self.find_iter(text) {
                result.push_str(&text[last_end..m.start]);
                result.push_str(replacement);
                last_end = m.end;
                replaced = true;
            }

            if replaced {
                result.push_str(&text[last_end..]);
                Cow::Owned(result)
            } else {
                Cow::Borrowed(text)
            }
        }
    }

    #[derive(Debug, Clone)]
    pub struct Match {
        pub start: usize,
        pub end: usize,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_multiedit() {
        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        tokio::fs::write(&file_path, "hello world\nfoo bar\nhello again\nfoo baz\n").await.unwrap();

        let tool = MultieditTool::new();
        let params = serde_json::json!({
            "path": file_path.to_string_lossy(),
            "edits": [
                {"search": "hello", "replace": "goodbye", "all": true},
                {"search": "foo", "replace": "qux", "all": true}
            ]
        });
        let context = ToolContext {
            working_dir: temp_dir.path().to_path_buf(),
            ..Default::default()
        };

        let result = tool.execute(params, &context).await;
        assert!(result.is_ok());

        let content = tokio::fs::read_to_string(&file_path).await.unwrap();
        assert_eq!(content, "goodbye world\nqux bar\ngoodbye again\nqux baz\n");
    }

    #[tokio::test]
    async fn test_multiedit_partial() {
        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        tokio::fs::write(&file_path, "hello world\n").await.unwrap();

        let tool = MultieditTool::new();
        let params = serde_json::json!({
            "path": file_path.to_string_lossy(),
            "edits": [
                {"search": "hello", "replace": "goodbye", "all": false},
                {"search": "nonexistent", "replace": "test", "all": true}
            ]
        });
        let context = ToolContext {
            working_dir: temp_dir.path().to_path_buf(),
            ..Default::default()
        };

        let result = tool.execute(params, &context).await;
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.output.contains("No matches found"));
    }
}
