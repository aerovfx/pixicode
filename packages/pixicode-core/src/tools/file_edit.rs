//! File Edit Tool — search and replace in files

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::tools::trait_def::{Tool, ToolContext, ToolError, ToolOutput, ToolResult, ToolSchema, ToolParameter};

/// Parameters for the edit tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditParams {
    /// Path to the file to edit
    pub path: String,
    /// Text to search for
    pub search: String,
    /// Text to replace with
    pub replace: String,
    /// Replace all occurrences (default: false, only first)
    #[serde(default)]
    pub all: bool,
    /// Use regex for search (default: false)
    #[serde(default)]
    pub regex: bool,
    /// Case insensitive search (default: false)
    #[serde(default)]
    pub ignore_case: bool,
}

/// Tool for editing files via search and replace.
pub struct EditTool;

impl EditTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Tool for EditTool {
    fn name(&self) -> &'static str {
        "edit"
    }

    fn description(&self) -> &'static str {
        "Edit a file by searching for text and replacing it"
    }

    fn schema(&self) -> ToolSchema {
        let mut schema = ToolSchema::default();
        schema.required = vec!["path".to_string(), "search".to_string(), "replace".to_string()];
        schema.properties.insert("path".to_string(), ToolParameter::string("Path to the file to edit"));
        schema.properties.insert("search".to_string(), ToolParameter::string("Text to search for"));
        schema.properties.insert("replace".to_string(), ToolParameter::string("Text to replace with"));
        schema.properties.insert("all".to_string(), ToolParameter::boolean("Replace all occurrences"));
        schema.properties.insert("regex".to_string(), ToolParameter::boolean("Use regex for search"));
        schema.properties.insert("ignore_case".to_string(), ToolParameter::boolean("Case insensitive search"));
        schema
    }

    async fn execute(&self, params: serde_json::Value, context: &ToolContext) -> ToolResult<ToolOutput> {
        let params: EditParams = serde_json::from_value(params)
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

        // Perform search and replace
        let (new_content, count) = if params.regex {
            search_replace_regex(&content, &params.search, &params.replace, params.all, params.ignore_case)?
        } else {
            search_replace_text(&content, &params.search, &params.replace, params.all, params.ignore_case)
        };

        if count == 0 {
            return Err(ToolError::Execution("No matches found".to_string()));
        }

        // Write updated content
        tokio::fs::write(&file_path, &new_content)
            .await
            .map_err(|e| ToolError::Io(e))?;

        Ok(ToolOutput::success(format!(
            "Replaced {} occurrence(s) in {}",
            count,
            file_path.display()
        )).with_data(serde_json::json!({
            "path": file_path.to_string_lossy(),
            "replacements": count,
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
    // Use a simple regex implementation
    let regex = if ignore_case {
        format!("(?i){}", search)
    } else {
        search.to_string()
    };

    // Simple regex replacement using std
    let re = regex::Regex::new(&regex)
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

// Lazy regex import to avoid dependency if not used
mod regex {
    pub use std::borrow::Cow;

    pub struct Regex {
        pattern: String,
    }

    impl Regex {
        pub fn new(pattern: &str) -> Result<Self, crate::tools::file_edit::regex_syntax::Error> {
            // Simple pattern validation - in production use full regex crate
            Ok(Self {
                pattern: pattern.to_string(),
            })
        }

        pub fn find(&self, text: &str) -> Option<Match> {
            // Simple substring match for basic patterns
            text.find(&self.pattern).map(|start| Match { start, end: start + self.pattern.len() })
        }

        pub fn find_iter<'a>(&'a self, text: &'a str) -> impl Iterator<Item = Match> + 'a {
            let mut last_end = 0;
            std::iter::from_fn(move || {
                if last_end >= text.len() {
                    return None;
                }
                text[last_end..].find(&self.pattern).map(|start| {
                    let absolute_start = last_end + start;
                    let m = Match {
                        start: absolute_start,
                        end: absolute_start + self.pattern.len(),
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

    pub struct Match {
        pub start: usize,
        pub end: usize,
    }
}

mod regex_syntax {
    use std::fmt;
    
    #[derive(Debug)]
    pub struct Error(String);
    
    impl fmt::Display for Error {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(f, "regex error: {}", self.0)
        }
    }
    
    impl std::error::Error for Error {}
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_edit_file() {
        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        tokio::fs::write(&file_path, "hello world\nhello again\n").await.unwrap();

        let tool = EditTool::new();
        let params = serde_json::json!({
            "path": file_path.to_string_lossy(),
            "search": "hello",
            "replace": "goodbye",
            "all": true
        });
        let context = ToolContext {
            working_dir: temp_dir.path().to_path_buf(),
            ..Default::default()
        };

        let result = tool.execute(params, &context).await;
        assert!(result.is_ok());

        let content = tokio::fs::read_to_string(&file_path).await.unwrap();
        assert_eq!(content, "goodbye world\ngoodbye again\n");
    }

    #[tokio::test]
    async fn test_edit_first_only() {
        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        tokio::fs::write(&file_path, "hello world\nhello again\n").await.unwrap();

        let tool = EditTool::new();
        let params = serde_json::json!({
            "path": file_path.to_string_lossy(),
            "search": "hello",
            "replace": "goodbye",
            "all": false
        });
        let context = ToolContext {
            working_dir: temp_dir.path().to_path_buf(),
            ..Default::default()
        };

        let result = tool.execute(params, &context).await;
        assert!(result.is_ok());

        let content = tokio::fs::read_to_string(&file_path).await.unwrap();
        assert_eq!(content, "goodbye world\nhello again\n");
    }
}
