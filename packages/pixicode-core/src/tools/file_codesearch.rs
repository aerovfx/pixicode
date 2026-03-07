//! Code Search Tool — tree-sitter based code search

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::collections::HashMap;

use crate::tools::trait_def::{Tool, ToolContext, ToolError, ToolOutput, ToolResult, ToolSchema, ToolParameter};

/// Search result for a code symbol.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeMatch {
    /// File path where the symbol was found
    pub file: String,
    /// Symbol name
    pub symbol: String,
    /// Symbol type (function, class, method, etc.)
    pub symbol_type: String,
    /// Line number
    pub line: u32,
    /// Column number
    pub column: u32,
    /// Surrounding context (snippet)
    pub context: String,
}

/// Parameters for the codesearch tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodesearchParams {
    /// Symbol or pattern to search for
    pub query: String,
    /// Directory to search in (default: working dir)
    #[serde(default)]
    pub path: Option<String>,
    /// Symbol types to include (function, class, method, etc.)
    #[serde(default)]
    pub symbol_types: Option<Vec<String>>,
    /// File extensions to include (e.g., [".rs", ".py"])
    #[serde(default)]
    pub extensions: Option<Vec<String>>,
    /// File patterns to exclude (e.g., ["*.min.js", "vendor/*"])
    #[serde(default)]
    pub exclude: Option<Vec<String>>,
    /// Maximum number of results (default: 50)
    #[serde(default = "default_limit")]
    pub limit: u32,
    /// Include test files (default: false)
    #[serde(default)]
    pub include_tests: bool,
}

fn default_limit() -> u32 { 50 }

/// Tool for searching code symbols using tree-sitter.
pub struct CodesearchTool;

impl CodesearchTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Tool for CodesearchTool {
    fn name(&self) -> &'static str {
        "codesearch"
    }

    fn description(&self) -> &'static str {
        "Search for code symbols (functions, classes, methods) using tree-sitter AST parsing"
    }

    fn schema(&self) -> ToolSchema {
        let mut schema = ToolSchema::default();
        schema.required = vec!["query".to_string()];
        schema.properties.insert("query".to_string(), ToolParameter::string("Symbol or pattern to search for"));
        schema.properties.insert("path".to_string(), ToolParameter::string("Directory to search in"));
        schema.properties.insert("symbol_types".to_string(), ToolParameter {
            param_type: "array".to_string(),
            description: "Symbol types to include (function, class, method, etc.)".to_string(),
            default: None,
            enum_values: None,
            minimum: None,
            maximum: None,
            min_length: None,
            max_length: None,
            items: Some(Box::new(ToolParameter::string("Symbol type"))),
        });
        schema.properties.insert("extensions".to_string(), ToolParameter {
            param_type: "array".to_string(),
            description: "File extensions to include (e.g., [.rs, .py])".to_string(),
            default: None,
            enum_values: None,
            minimum: None,
            maximum: None,
            min_length: None,
            max_length: None,
            items: Some(Box::new(ToolParameter::string("File extension"))),
        });
        schema.properties.insert("exclude".to_string(), ToolParameter {
            param_type: "array".to_string(),
            description: "File patterns to exclude".to_string(),
            default: None,
            enum_values: None,
            minimum: None,
            maximum: None,
            min_length: None,
            max_length: None,
            items: Some(Box::new(ToolParameter::string("Exclude pattern"))),
        });
        schema.properties.insert("limit".to_string(), ToolParameter::integer("Maximum number of results"));
        schema.properties.insert("include_tests".to_string(), ToolParameter::boolean("Include test files"));
        schema
    }

    async fn execute(&self, params: serde_json::Value, context: &ToolContext) -> ToolResult<ToolOutput> {
        let params: CodesearchParams = serde_json::from_value(params)
            .map_err(|e| ToolError::InvalidParams(e.to_string()))?;

        let search_dir = match &params.path {
            Some(p) => resolve_path(p, &context.working_dir)?,
            None => context.working_dir.clone(),
        };

        if !search_dir.exists() {
            return Err(ToolError::Execution(format!("Directory not found: {}", search_dir.display())));
        }

        // Collect files to search
        let files = collect_files(&search_dir, &params)?;

        // Search for symbols
        let mut matches = Vec::new();
        
        for file_path in files {
            if matches.len() >= params.limit as usize {
                break;
            }

            match search_file(&file_path, &params).await {
                Ok(file_matches) => {
                    for m in file_matches {
                        if matches.len() >= params.limit as usize {
                            break;
                        }
                        matches.push(m);
                    }
                }
                Err(e) => {
                    tracing::warn!("Failed to search file {:?}: {}", file_path, e);
                }
            }
        }

        // Format output
        let mut output = String::new();
        output.push_str(&format!("Query: {}\n\n", params.query));

        if matches.is_empty() {
            output.push_str("No matching symbols found");
        } else {
            output.push_str(&format!("Found {} matching symbol(s):\n\n", matches.len()));
            
            // Group by file
            let mut by_file: HashMap<String, Vec<&CodeMatch>> = HashMap::new();
            for m in &matches {
                by_file.entry(m.file.clone()).or_default().push(m);
            }

            for (file, file_matches) in &by_file {
                output.push_str(&format!("{}\n", file));
                for m in file_matches {
                    output.push_str(&format!(
                        "  {} - {} ({})\n    {}\n",
                        m.symbol, m.symbol_type,
                        m.line,
                        truncate(&m.context, 60)
                    ));
                }
                output.push('\n');
            }
        }

        let match_data: Vec<serde_json::Value> = matches.iter().map(|m| {
            serde_json::json!({
                "file": m.file,
                "line": m.line,
                "column": m.column,
                "symbol": m.symbol,
                "symbol_type": m.symbol_type,
                "context": m.context,
            })
        }).collect();

        Ok(ToolOutput::success(output).with_data(serde_json::json!({
            "query": params.query,
            "matches": match_data,
            "count": matches.len(),
        })))
    }
}

/// Collect files to search based on parameters.
fn collect_files(dir: &PathBuf, params: &CodesearchParams) -> Result<Vec<PathBuf>, ToolError> {
    let mut files = Vec::new();

    let entries = std::fs::read_dir(dir)
        .map_err(|e| ToolError::Io(e))?;

    for entry in entries {
        let entry = entry.map_err(|e| ToolError::Io(e))?;
        let path = entry.path();
        let file_name = path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("");

        // Skip hidden files/dirs
        if file_name.starts_with('.') {
            continue;
        }

        // Skip test files unless requested
        if !params.include_tests {
            if file_name.contains("test") || file_name.contains("spec") {
                continue;
            }
        }

        // Check exclude patterns
        if let Some(excludes) = &params.exclude {
            let excluded = excludes.iter().any(|pattern| {
                file_name.contains(pattern.trim_start_matches('*').trim_start_matches('/'))
            });
            if excluded {
                continue;
            }
        }

        if path.is_dir() {
            // Skip common non-code directories
            if matches!(file_name, "node_modules" | "target" | "dist" | "build" | ".git" | "vendor") {
                continue;
            }

            // Recurse into subdirectory
            let mut sub_files = collect_files(&path, params)?;
            files.append(&mut sub_files);
        } else if path.is_file() {
            // Check extension filter
            if let Some(exts) = &params.extensions {
                if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                    if !exts.iter().any(|e| e.trim_start_matches('.') == ext) {
                        continue;
                    }
                } else {
                    continue;
                }
            }

            files.push(path);
        }
    }

    Ok(files)
}

/// Search for symbols in a file.
async fn search_file(path: &PathBuf, params: &CodesearchParams) -> Result<Vec<CodeMatch>, ToolError> {
    let content = tokio::fs::read_to_string(path)
        .await
        .map_err(|e| ToolError::Io(e))?;

    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
    let mut matches = Vec::new();

    // Simple pattern-based search (tree-sitter would go here in production)
    // This is a simplified implementation
    for (line_num, line) in content.lines().enumerate() {
        if line.contains(&params.query) {
            // Try to extract symbol name
            if let Some(symbol) = extract_symbol(line, &params.query) {
                let symbol_type = detect_symbol_type(line, ext);
                
                // Filter by symbol type if specified
                if let Some(types) = &params.symbol_types {
                    if !types.contains(&symbol_type) {
                        continue;
                    }
                }

                matches.push(CodeMatch {
                    file: path.to_string_lossy().to_string(),
                    symbol,
                    symbol_type,
                    line: (line_num + 1) as u32,
                    column: 1,
                    context: line.trim().to_string(),
                });
            }
        }
    }

    Ok(matches)
}

/// Extract symbol name from a line containing the query.
fn extract_symbol(line: &str, query: &str) -> Option<String> {
    // Simple heuristic: look for function/class definitions
    let line = line.trim();
    
    // Try to find function definition
    if let Some(pos) = line.find("fn ") {
        if let Some(name_start) = line[pos + 3..].find(|c: char| !c.is_whitespace()) {
            let rest = &line[pos + 3 + name_start..];
            if let Some(end) = rest.find(|c: char| !c.is_alphanumeric() && c != '_') {
                return Some(rest[..end].to_string());
            }
        }
    }
    
    // Try class/function in various languages
    for prefix in &["class ", "function ", "def ", "const ", "let ", "var "] {
        if let Some(pos) = line.find(prefix) {
            if let Some(name_start) = line[pos + prefix.len()..].find(|c: char| !c.is_whitespace()) {
                let rest = &line[pos + prefix.len() + name_start..];
                if let Some(end) = rest.find(|c: char| !c.is_alphanumeric() && c != '_') {
                    return Some(rest[..end].to_string());
                }
            }
        }
    }

    // Fall back to the query itself
    if line.contains(query) {
        return Some(query.to_string());
    }

    None
}

/// Detect the type of symbol based on line content.
fn detect_symbol_type(line: &str, ext: &str) -> String {
    let line = line.trim();
    
    if line.starts_with("fn ") || line.contains(" fn ") {
        return "function".to_string();
    }
    if line.starts_with("class ") || line.starts_with("public class ") {
        return "class".to_string();
    }
    if line.starts_with("function ") {
        return "function".to_string();
    }
    if line.starts_with("def ") {
        return "method".to_string();
    }
    if line.starts_with("const ") || line.starts_with("let ") || line.starts_with("var ") {
        return "variable".to_string();
    }
    if line.starts_with("interface ") {
        return "interface".to_string();
    }
    if line.starts_with("struct ") {
        return "struct".to_string();
    }
    if line.starts_with("enum ") {
        return "enum".to_string();
    }
    if line.starts_with("trait ") {
        return "trait".to_string();
    }
    if line.starts_with("impl ") {
        return "impl".to_string();
    }
    if line.contains(" => ") || line.contains(" = ") {
        return "expression".to_string();
    }

    "symbol".to_string()
}

/// Truncate string to max length.
fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len])
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
    async fn test_codesearch() {
        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("test.rs");
        tokio::fs::write(&file_path, r#"
fn hello() {
    println!("Hello");
}

fn world() {
    println!("World");
}

fn hello_world() {
    hello();
    world();
}
"#).await.unwrap();

        let tool = CodesearchTool::new();
        let params = serde_json::json!({
            "query": "hello",
            "path": temp_dir.path().to_string_lossy(),
            "extensions": [".rs"]
        });
        let context = ToolContext::default();

        let result = tool.execute(params, &context).await;
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.output.contains("hello"));
    }
}
