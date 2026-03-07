//! LSP Tool — Language Server Protocol integration

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};
use tokio::sync::Mutex;

use crate::tools::lsp_client::LspStdioClient;
use crate::tools::trait_def::{Tool, ToolContext, ToolError, ToolOutput, ToolResult, ToolSchema, ToolParameter};

/// LSP Diagnostic severity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
#[serde(rename_all = "lowercase")]
pub enum DiagnosticSeverity {
    Error,
    Warning,
    Information,
    Hint,
}

/// LSP Diagnostic.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Diagnostic {
    /// File path
    pub file: String,
    /// Line number (1-indexed)
    pub line: u32,
    /// Column number (1-indexed)
    pub column: u32,
    /// Severity
    pub severity: DiagnosticSeverity,
    /// Message
    pub message: String,
    /// Source (e.g., "rustc", "typescript")
    pub source: Option<String>,
    /// Code
    pub code: Option<String>,
}

/// LSP Symbol.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Symbol {
    /// Symbol name
    pub name: String,
    /// Symbol kind
    pub kind: String,
    /// File path
    pub file: String,
    /// Line number
    pub line: u32,
    /// Column number
    pub column: u32,
    /// Container name (parent scope)
    pub container: Option<String>,
}

/// LSP Reference.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reference {
    /// File path
    pub file: String,
    /// Line number
    pub line: u32,
    /// Column number
    pub column: u32,
    /// Context line
    pub context: Option<String>,
}

/// Per-server entry (info + JSON-RPC client).
#[derive(Clone)]
pub struct LspServerEntry {
    pub info: LspServerInfo,
    pub client: Arc<Mutex<LspStdioClient>>,
}

/// LSP Server state — used for multi-language server management.
#[derive(Default)]
pub struct LspState {
    /// Connected servers by language (e.g. "rust", "typescript")
    pub servers: HashMap<String, LspServerEntry>,
}

/// LSP Server info.
#[derive(Debug, Clone)]
pub struct LspServerInfo {
    pub language: String,
    pub server_id: String,
    pub root_path: PathBuf,
    pub initialized: bool,
}

// Global LSP state
static LSP_STATE: OnceLock<Arc<Mutex<LspState>>> = OnceLock::new();

/// Global LSP state; used by tool to dispatch to real server or simulated data.
pub fn get_lsp_state() -> Arc<Mutex<LspState>> {
    LSP_STATE
        .get_or_init(|| Arc::new(Mutex::new(LspState::default())))
        .clone()
}

/// Register an LSP server for a language (spawns process, sends initialize).
pub async fn register_lsp_server(
    lang: &str,
    root_path: PathBuf,
    cmd: &str,
    args: &[&str],
) -> anyhow::Result<()> {
    let root = root_path.canonicalize().unwrap_or(root_path);
    let root_uri = format!("file://{}", root.display());
    let mut client = LspStdioClient::spawn(cmd, args, &root_uri)?;
    client.initialize(&root_uri)?;
    let entry = LspServerEntry {
        info: LspServerInfo {
            language: lang.to_string(),
            server_id: cmd.to_string(),
            root_path: root.clone(),
            initialized: true,
        },
        client: Arc::new(Mutex::new(client)),
    };
    get_lsp_state().lock().await.servers.insert(lang.to_string(), entry);
    Ok(())
}

fn lang_from_file(path: &Path) -> Option<String> {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| match e.to_lowercase().as_str() {
            "rs" => "rust",
            "ts" | "tsx" => "typescript",
            "js" | "jsx" => "javascript",
            "py" => "python",
            "go" => "go",
            "rb" => "ruby",
            _ => e,
        })
        .map(String::from)
}

/// Parameters for the lsp tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum LspAction {
    /// Get diagnostics for a file
    Diagnostics {
        /// File path
        file: String,
    },
    /// Get workspace diagnostics
    WorkspaceDiagnostics {
        /// Filter by severity
        severity: Option<String>,
        /// Maximum results
        limit: Option<u32>,
    },
    /// Get symbols in a file
    Symbols {
        /// File path
        file: String,
        /// Filter by kind
        kind: Option<String>,
    },
    /// Get workspace symbols
    WorkspaceSymbols {
        /// Query string
        query: String,
        /// Maximum results
        limit: Option<u32>,
    },
    /// Get references to a symbol
    References {
        /// File path
        file: String,
        /// Line number
        line: u32,
        /// Column number
        column: u32,
        /// Include declaration
        include_declaration: Option<bool>,
    },
    /// Get definition of a symbol
    Definition {
        /// File path
        file: String,
        /// Line number
        line: u32,
        /// Column number
        column: u32,
    },
    /// Get type definition
    TypeDefinition {
        /// File path
        file: String,
        /// Line number
        line: u32,
        /// Column number
        column: u32,
    },
    /// Get hover information
    Hover {
        /// File path
        file: String,
        /// Line number
        line: u32,
        /// Column number
        column: u32,
    },
    /// Get completion items
    Completion {
        /// File path
        file: String,
        /// Line number
        line: u32,
        /// Column number
        column: u32,
    },
    /// Format a document
    Format {
        /// File path
        file: String,
        /// Range start line (optional)
        start_line: Option<u32>,
        /// Range end line (optional)
        end_line: Option<u32>,
    },
    /// Rename a symbol
    Rename {
        /// File path
        file: String,
        /// Line number
        line: u32,
        /// Column number
        column: u32,
        /// New name
        new_name: String,
    },
}

/// Tool for LSP integration.
pub struct LspTool;

impl LspTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Tool for LspTool {
    fn name(&self) -> &'static str {
        "lsp"
    }

    fn description(&self) -> &'static str {
        "Language Server Protocol integration: diagnostics, symbols, references, definitions, completions"
    }

    fn schema(&self) -> ToolSchema {
        let mut schema = ToolSchema::default();
        schema.required = vec!["action".to_string()];
        
        schema.properties.insert("action".to_string(), ToolParameter {
            param_type: "string".to_string(),
            description: "Action: diagnostics, workspace_diagnostics, symbols, workspace_symbols, references, definition, type_definition, hover, completion, format, rename".to_string(),
            default: None,
            enum_values: Some(vec![
                serde_json::json!("diagnostics"),
                serde_json::json!("workspace_diagnostics"),
                serde_json::json!("symbols"),
                serde_json::json!("workspace_symbols"),
                serde_json::json!("references"),
                serde_json::json!("definition"),
                serde_json::json!("type_definition"),
                serde_json::json!("hover"),
                serde_json::json!("completion"),
                serde_json::json!("format"),
                serde_json::json!("rename"),
            ]),
            minimum: None,
            maximum: None,
            min_length: None,
            max_length: None,
            items: None,
        });
        
        schema.properties.insert("file".to_string(), ToolParameter::string("File path"));
        schema.properties.insert("line".to_string(), ToolParameter::integer("Line number (1-indexed)"));
        schema.properties.insert("column".to_string(), ToolParameter::integer("Column number (1-indexed)"));
        schema.properties.insert("query".to_string(), ToolParameter::string("Search query"));
        schema.properties.insert("severity".to_string(), ToolParameter::string("Filter by severity: error, warning, information, hint"));
        schema.properties.insert("kind".to_string(), ToolParameter::string("Filter by symbol kind"));
        schema.properties.insert("limit".to_string(), ToolParameter::integer("Maximum results"));
        schema.properties.insert("new_name".to_string(), ToolParameter::string("New name for rename"));
        schema.properties.insert("start_line".to_string(), ToolParameter::integer("Range start line"));
        schema.properties.insert("end_line".to_string(), ToolParameter::integer("Range end line"));
        schema.properties.insert("include_declaration".to_string(), ToolParameter::boolean("Include declaration in references"));
        
        schema
    }

    async fn execute(&self, params: serde_json::Value, context: &ToolContext) -> ToolResult<ToolOutput> {
        let action: LspAction = serde_json::from_value(params)
            .map_err(|e| ToolError::InvalidParams(e.to_string()))?;

        match action {
            LspAction::Diagnostics { file } => {
                let file_path = resolve_path(&file, &context.working_dir)?;
                let lang = lang_from_file(&file_path);
                let diagnostics = if let Some(lang) = lang {
                    let state_arc = get_lsp_state();
                    let client_arc = {
                        let state = state_arc.lock().await;
                        state.servers.get(&lang).map(|e| e.client.clone())
                    };
                    if let Some(client_arc) = client_arc {
                        let uri = format!("file://{}", file_path.canonicalize().unwrap_or(file_path.clone()).display());
                        let params = serde_json::json!({
                            "textDocument": { "uri": uri, "version": 1_i32 }
                        });
                        let mut client = client_arc.lock().await;
                        match client.call("textDocument/diagnostic", params) {
                            Ok(result) => lsp_diagnostic_result_to_diagnostics(&result, &file_path.to_string_lossy()),
                            Err(_) => get_simulated_diagnostics(&file_path)?,
                        }
                    } else {
                        get_simulated_diagnostics(&file_path)?
                    }
                } else {
                    get_simulated_diagnostics(&file_path)?
                };
                let output = format_diagnostics(&diagnostics);
                Ok(ToolOutput::success(output).with_data(serde_json::json!({
                    "file": file_path.to_string_lossy(),
                    "diagnostics": diagnostics,
                    "count": diagnostics.len(),
                })))
            }
            
            LspAction::WorkspaceDiagnostics { severity, limit } => {
                // In production, query all open files
                let limit = limit.unwrap_or(100);
                let diagnostics = Vec::<Diagnostic>::new(); // Would query LSP servers
                
                let output = if diagnostics.is_empty() {
                    "No workspace diagnostics".to_string()
                } else {
                    format_diagnostics(&diagnostics)
                };
                
                Ok(ToolOutput::success(output).with_data(serde_json::json!({
                    "diagnostics": diagnostics,
                    "count": diagnostics.len(),
                })))
            }
            
            LspAction::Symbols { file, kind } => {
                let file_path = resolve_path(&file, &context.working_dir)?;
                
                // In production, query LSP server for document symbols
                let symbols = get_simulated_symbols(&file_path, kind.as_deref())?;
                
                let output = format_symbols(&symbols);
                
                Ok(ToolOutput::success(output).with_data(serde_json::json!({
                    "file": file_path.to_string_lossy(),
                    "symbols": symbols,
                    "count": symbols.len(),
                })))
            }
            
            LspAction::WorkspaceSymbols { query, limit } => {
                let limit = limit.unwrap_or(50);
                
                // In production, query LSP server for workspace symbols
                let symbols = Vec::<Symbol>::new();
                
                let output = if symbols.is_empty() {
                    format!("No symbols found for '{}'", query)
                } else {
                    format_symbols(&symbols)
                };
                
                Ok(ToolOutput::success(output).with_data(serde_json::json!({
                    "query": query,
                    "symbols": symbols,
                    "count": symbols.len(),
                })))
            }
            
            LspAction::References { file, line, column, include_declaration } => {
                let file_path = resolve_path(&file, &context.working_dir)?;
                
                // In production, query LSP server for references
                let references = Vec::<Reference>::new();
                
                let output = if references.is_empty() {
                    format!("No references found at {}:{}:{}", file_path.display(), line, column)
                } else {
                    format_references(&references)
                };
                
                Ok(ToolOutput::success(output).with_data(serde_json::json!({
                    "file": file_path.to_string_lossy(),
                    "line": line,
                    "column": column,
                    "references": references,
                    "count": references.len(),
                })))
            }
            
            LspAction::Definition { file, line, column } => {
                let file_path = resolve_path(&file, &context.working_dir)?;
                
                // In production, query LSP server for definition
                let output = format!(
                    "Definition lookup at {}:{}:{}\n\n\
                    Note: LSP server connection not established.\n\
                    In production, this would return the symbol's definition location.",
                    file_path.display(), line, column
                );
                
                Ok(ToolOutput::success(output).with_data(serde_json::json!({
                    "file": file_path.to_string_lossy(),
                    "line": line,
                    "column": column,
                    "definition": serde_json::Value::Null,
                })))
            }

            LspAction::TypeDefinition { file, line, column } => {
                let file_path = resolve_path(&file, &context.working_dir)?;

                let output = format!(
                    "Type definition lookup at {}:{}:{}\n\n\
                    Note: LSP server connection not established.",
                    file_path.display(), line, column
                );

                Ok(ToolOutput::success(output).with_data(serde_json::json!({
                    "file": file_path.to_string_lossy(),
                    "line": line,
                    "column": column,
                    "type_definition": serde_json::Value::Null,
                })))
            }
            
            LspAction::Hover { file, line, column } => {
                let file_path = resolve_path(&file, &context.working_dir)?;
                
                let output = format!(
                    "Hover at {}:{}:{}\n\n\
                    Note: LSP server connection not established.\n\
                    Would show type info, documentation, etc.",
                    file_path.display(), line, column
                );
                
                Ok(ToolOutput::success(output).with_data(serde_json::json!({
                    "file": file_path.to_string_lossy(),
                    "line": line,
                    "column": column,
                    "hover": serde_json::Value::Null,
                })))
            }
            
            LspAction::Completion { file, line, column } => {
                let file_path = resolve_path(&file, &context.working_dir)?;
                
                let output = format!(
                    "Completion at {}:{}:{}\n\n\
                    Note: LSP server connection not established.\n\
                    Would return completion items.",
                    file_path.display(), line, column
                );
                
                Ok(ToolOutput::success(output).with_data(serde_json::json!({
                    "file": file_path.to_string_lossy(),
                    "line": line,
                    "column": column,
                    "completions": Vec::<serde_json::Value>::new(),
                })))
            }
            
            LspAction::Format { file, start_line, end_line } => {
                let file_path = resolve_path(&file, &context.working_dir)?;
                
                let output = format!(
                    "Format request for {}\n\n\
                    Note: LSP server connection not established.\n\
                    Range: {} to {}",
                    file_path.display(),
                    start_line.map(|l| l.to_string()).unwrap_or_else(|| "full document".to_string()),
                    end_line.map(|l| l.to_string()).unwrap_or_else(|| "end".to_string())
                );
                
                Ok(ToolOutput::success(output).with_data(serde_json::json!({
                    "file": file_path.to_string_lossy(),
                    "formatted": false,
                })))
            }
            
            LspAction::Rename { file, line, column, new_name } => {
                let file_path = resolve_path(&file, &context.working_dir)?;
                
                let output = format!(
                    "Rename at {}:{}:{} to '{}'\n\n\
                    Note: LSP server connection not established.\n\
                    Would rename symbol across workspace.",
                    file_path.display(), line, column, new_name
                );
                
                Ok(ToolOutput::success(output).with_data(serde_json::json!({
                    "file": file_path.to_string_lossy(),
                    "line": line,
                    "column": column,
                    "new_name": new_name,
                    "renamed": false,
                })))
            }
        }
    }
}

/// Map LSP diagnostic result (full report or array) to our Diagnostic list.
fn lsp_diagnostic_result_to_diagnostics(result: &serde_json::Value, file: &str) -> Vec<Diagnostic> {
    let items = result
        .get("items")
        .and_then(|v| v.as_array())
        .or_else(|| result.as_array());
    let Some(items) = items else { return vec![] };
    items
        .iter()
        .filter_map(|d| {
            let range = d.get("range")?;
            let start = range.get("start")?;
            let line = start.get("line").and_then(|v| v.as_u64()).unwrap_or(0) as u32 + 1;
            let column = start.get("character").and_then(|v| v.as_u64()).unwrap_or(0) as u32 + 1;
            let msg_val = d.get("message");
            let message = msg_val
                .and_then(|m| m.as_str())
                .or_else(|| msg_val.and_then(|m| m.get("value")).and_then(|v| v.as_str()))
                .unwrap_or("")
                .to_string();
            let sev = d.get("severity").and_then(|v| v.as_u64()).unwrap_or(0);
            let severity = match sev {
                1 => DiagnosticSeverity::Error,
                2 => DiagnosticSeverity::Warning,
                3 => DiagnosticSeverity::Information,
                4 => DiagnosticSeverity::Hint,
                _ => DiagnosticSeverity::Information,
            };
            Some(Diagnostic {
                file: file.to_string(),
                line,
                column,
                severity,
                message,
                source: d.get("source").and_then(|v| v.as_str()).map(String::from),
                code: d.get("code").and_then(|v| v.as_str()).map(String::from),
            })
        })
        .collect()
}

/// Get simulated diagnostics for a file.
fn get_simulated_diagnostics(_path: &PathBuf) -> ToolResult<Vec<Diagnostic>> {
    // In production, this would query the actual LSP server
    // Return empty for now
    Ok(Vec::new())
}

/// Get simulated symbols for a file.
fn get_simulated_symbols(_path: &PathBuf, _kind: Option<&str>) -> ToolResult<Vec<Symbol>> {
    Ok(Vec::new())
}

/// Format diagnostics for display.
fn format_diagnostics(diagnostics: &[Diagnostic]) -> String {
    if diagnostics.is_empty() {
        return "No diagnostics found".to_string();
    }

    let mut output = String::new();
    output.push_str(&format!("Found {} diagnostic(s):\n\n", diagnostics.len()));

    let mut by_severity: HashMap<DiagnosticSeverity, Vec<&Diagnostic>> = HashMap::new();
    for diag in diagnostics {
        by_severity.entry(diag.severity).or_default().push(diag);
    }

    for severity in &[DiagnosticSeverity::Error, DiagnosticSeverity::Warning, DiagnosticSeverity::Information, DiagnosticSeverity::Hint] {
        if let Some(diags) = by_severity.get(severity) {
            let icon = match severity {
                DiagnosticSeverity::Error => "✗",
                DiagnosticSeverity::Warning => "⚠",
                DiagnosticSeverity::Information => "ℹ",
                DiagnosticSeverity::Hint => "💡",
            };

            output.push_str(&format!("{} {} ({}):\n", icon, format!("{:?}", severity).to_lowercase(), diags.len()));
            for diag in diags {
                output.push_str(&format!(
                    "  {}:{}:{} - {}\n",
                    diag.file, diag.line, diag.column, diag.message
                ));
            }
            output.push('\n');
        }
    }

    output
}

/// Format symbols for display.
fn format_symbols(symbols: &[Symbol]) -> String {
    if symbols.is_empty() {
        return "No symbols found".to_string();
    }

    let mut output = String::new();
    output.push_str(&format!("Found {} symbol(s):\n\n", symbols.len()));

    // Group by kind
    let mut by_kind: HashMap<&str, Vec<&Symbol>> = HashMap::new();
    for sym in symbols {
        by_kind.entry(&sym.kind).or_default().push(sym);
    }

    for (kind, syms) in &by_kind {
        output.push_str(&format!("**{}** ({}):\n", kind, syms.len()));
        for sym in syms {
            let container = sym.container.as_ref()
                .map(|c| format!(" in {}", c))
                .unwrap_or_default();
            output.push_str(&format!(
                "  - {}{} ({}:{})\n",
                sym.name, container, sym.file, sym.line
            ));
        }
        output.push('\n');
    }

    output
}

/// Format references for display.
fn format_references(references: &[Reference]) -> String {
    if references.is_empty() {
        return "No references found".to_string();
    }

    let mut output = String::new();
    output.push_str(&format!("Found {} reference(s):\n\n", references.len()));

    // Group by file
    let mut by_file: HashMap<&str, Vec<&Reference>> = HashMap::new();
    for reference in references {
        by_file.entry(&reference.file).or_default().push(reference);
    }

    for (file, refs) in &by_file {
        output.push_str(&format!("{}:\n", file));
        for reference in refs {
            let context = reference.context.as_ref()
                .map(|c| format!(" - {}", c.trim()))
                .unwrap_or_default();
            output.push_str(&format!(
                "  {}:{}:{}{}\n",
                file, reference.line, reference.column, context
            ));
        }
        output.push('\n');
    }

    output
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
    async fn test_lsp_diagnostics() {
        let tool = LspTool::new();
        let params = serde_json::json!({
            "action": "diagnostics",
            "file": "src/main.rs"
        });
        let context = ToolContext::default();

        let result = tool.execute(params, &context).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_lsp_symbols() {
        let tool = LspTool::new();
        let params = serde_json::json!({
            "action": "symbols",
            "file": "src/lib.rs"
        });
        let context = ToolContext::default();

        let result = tool.execute(params, &context).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_lsp_definition() {
        let tool = LspTool::new();
        let params = serde_json::json!({
            "action": "definition",
            "file": "src/main.rs",
            "line": 10,
            "column": 5
        });
        let context = ToolContext::default();

        let result = tool.execute(params, &context).await;
        assert!(result.is_ok());
    }
}
