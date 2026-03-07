//! Apply Patch Tool — unified diff patch application

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::tools::trait_def::{Tool, ToolContext, ToolError, ToolOutput, ToolResult, ToolSchema, ToolParameter};

/// Parameters for the apply_patch tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplyPatchParams {
    /// Unified diff patch content
    pub patch: String,
    /// Base directory for applying patch (default: working dir)
    #[serde(default)]
    pub cwd: Option<String>,
    /// Reverse the patch (default: false)
    #[serde(default)]
    pub reverse: bool,
    /// Create backup files (default: false)
    #[serde(default)]
    pub backup: bool,
    /// Dry run - don't actually apply (default: false)
    #[serde(default)]
    pub dry_run: bool,
}

/// Result from applying a patch to a file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatchResult {
    /// File path
    pub file: String,
    /// Success flag
    pub success: bool,
    /// Lines added
    pub additions: u32,
    /// Lines removed
    pub deletions: u32,
    /// Error message (if failed)
    pub error: Option<String>,
    /// Rejected hunks (if any)
    pub rejected: Vec<String>,
}

/// Tool for applying unified diff patches.
pub struct ApplyPatchTool;

impl ApplyPatchTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Tool for ApplyPatchTool {
    fn name(&self) -> &'static str {
        "apply_patch"
    }

    fn description(&self) -> &'static str {
        "Apply a unified diff patch to files"
    }

    fn schema(&self) -> ToolSchema {
        let mut schema = ToolSchema::default();
        schema.required = vec!["patch".to_string()];
        schema.properties.insert("patch".to_string(), ToolParameter::string("Unified diff patch content"));
        schema.properties.insert("cwd".to_string(), ToolParameter::string("Base directory for applying patch"));
        schema.properties.insert("reverse".to_string(), ToolParameter::boolean("Reverse the patch"));
        schema.properties.insert("backup".to_string(), ToolParameter::boolean("Create backup files"));
        schema.properties.insert("dry_run".to_string(), ToolParameter::boolean("Dry run - don't actually apply"));
        schema
    }

    async fn execute(&self, params: serde_json::Value, context: &ToolContext) -> ToolResult<ToolOutput> {
        let params: ApplyPatchParams = serde_json::from_value(params)
            .map_err(|e| ToolError::InvalidParams(e.to_string()))?;

        let base_dir = match &params.cwd {
            Some(cwd) => resolve_path(cwd, &context.working_dir)?,
            None => context.working_dir.clone(),
        };

        // Parse the patch
        let patches = parse_patch(&params.patch)?;
        
        let mut results = Vec::new();
        let mut total_additions = 0;
        let mut total_deletions = 0;
        let mut successful = 0;

        for patch in &patches {
            let file_path = base_dir.join(&patch.file);
            
            // Read original file
            let original_content = match tokio::fs::read_to_string(&file_path).await {
                Ok(content) => content,
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                    if patch.is_new_file {
                        String::new()
                    } else {
                        results.push(PatchResult {
                            file: patch.file.clone(),
                            success: false,
                            additions: 0,
                            deletions: 0,
                            error: Some(format!("File not found: {}", patch.file)),
                            rejected: vec![],
                        });
                        continue;
                    }
                }
                Err(e) => {
                    results.push(PatchResult {
                        file: patch.file.clone(),
                        success: false,
                        additions: 0,
                        deletions: 0,
                        error: Some(format!("IO error: {}", e)),
                        rejected: vec![],
                    });
                    continue;
                }
            };

            // Apply patch
            let (new_content, additions, deletions, rejected) = apply_patch_to_content(
                &original_content,
                &patch.hunks,
                params.reverse
            );

            if rejected.is_empty() {
                // Apply the changes
                if !params.dry_run {
                    // Create backup if requested
                    if params.backup && file_path.exists() {
                        let backup_path = file_path.with_extension(format!(
                            "{}.bak",
                            file_path.extension().map(|e| e.to_string_lossy()).unwrap_or_default()
                        ));
                        if let Err(e) = tokio::fs::copy(&file_path, &backup_path).await {
                            tracing::warn!("Failed to create backup: {}", e);
                        }
                    }

                    // Write new content
                    if let Err(e) = tokio::fs::write(&file_path, &new_content).await {
                        results.push(PatchResult {
                            file: patch.file.clone(),
                            success: false,
                            additions,
                            deletions,
                            error: Some(format!("Failed to write file: {}", e)),
                            rejected: vec![],
                        });
                        continue;
                    }
                }

                successful += 1;
                total_additions += additions;
                total_deletions += deletions;

                results.push(PatchResult {
                    file: patch.file.clone(),
                    success: true,
                    additions,
                    deletions,
                    error: None,
                    rejected: vec![],
                });
            } else {
                results.push(PatchResult {
                    file: patch.file.clone(),
                    success: false,
                    additions: 0,
                    deletions: 0,
                    error: Some(format!("{} hunk(s) failed to apply", rejected.len())),
                    rejected,
                });
            }
        }

        // Format output
        let mut output = String::new();
        
        if params.dry_run {
            output.push_str("DRY RUN - No changes applied\n\n");
        }
        
        output.push_str(&format!("Applied {} patch(es)\n", patches.len()));
        output.push_str(&format!("Successful: {}\n", successful));
        output.push_str(&format!("Failed: {}\n", patches.len() - successful));
        output.push_str(&format!("Total additions: {}\n", total_additions));
        output.push_str(&format!("Total deletions: {}\n", total_deletions));
        
        output.push_str("\nResults:\n");
        for result in &results {
            let status = if result.success { "✓" } else { "✗" };
            output.push_str(&format!("  {} {} (+{}, -{})\n", 
                status, result.file, result.additions, result.deletions));
            if let Some(error) = &result.error {
                output.push_str(&format!("    Error: {}\n", error));
            }
        }

        let result_data: Vec<serde_json::Value> = results.iter().map(|r| {
            serde_json::json!({
                "file": r.file,
                "success": r.success,
                "additions": r.additions,
                "deletions": r.deletions,
                "error": r.error,
                "rejected": r.rejected,
            })
        }).collect();

        Ok(ToolOutput::success(output).with_data(serde_json::json!({
            "patches": patches.len(),
            "successful": successful,
            "failed": patches.len() - successful,
            "additions": total_additions,
            "deletions": total_deletions,
            "dry_run": params.dry_run,
            "results": result_data,
        })))
    }
}

/// A parsed patch for a single file.
#[derive(Debug, Clone)]
struct FilePatch {
    file: String,
    hunks: Vec<Hunk>,
    is_new_file: bool,
}

/// A hunk in a patch.
#[derive(Debug, Clone)]
struct Hunk {
    old_start: u32,
    old_lines: u32,
    new_start: u32,
    new_lines: u32,
    lines: Vec<HunkLine>,
}

/// A single line in a hunk.
#[derive(Debug, Clone)]
enum HunkLine {
    Context(String),
    Add(String),
    Remove(String),
}

/// Parse a unified diff patch.
fn parse_patch(patch: &str) -> ToolResult<Vec<FilePatch>> {
    let mut patches = Vec::new();
    let mut lines: Vec<&str> = patch.lines().collect();
    let mut idx = 0;

    while idx < lines.len() {
        let line = lines[idx];
        
        // Look for file header
        if line.starts_with("diff --git") || line.starts_with("--- ") {
            let (file, hunk_start) = parse_file_header(&lines[idx..])?;
            idx += hunk_start;
            
            // Parse hunks
            let mut hunks = Vec::new();
            let mut is_new_file = line.starts_with("diff --git") && 
                lines[idx..].iter().any(|l| l.starts_with("new file"));
            
            while idx < lines.len() && (lines[idx].starts_with("@@") || 
                   lines[idx].starts_with("+") || lines[idx].starts_with("-") ||
                   lines[idx].starts_with(" ") || lines[idx].starts_with("\\") ||
                   lines[idx].is_empty()) {
                if lines[idx].starts_with("@@") {
                    let (hunk, hunk_lines) = parse_hunk(&lines[idx..])?;
                    hunks.push(hunk);
                    idx += hunk_lines;
                } else {
                    idx += 1;
                }
            }
            
            if !hunks.is_empty() || is_new_file {
                patches.push(FilePatch {
                    file,
                    hunks,
                    is_new_file,
                });
            }
        } else {
            idx += 1;
        }
    }

    Ok(patches)
}

/// Parse file header from patch.
fn parse_file_header(lines: &[&str]) -> ToolResult<(String, usize)> {
    let mut file = String::new();
    let mut idx = 0;

    for (i, line) in lines.iter().enumerate() {
        idx = i;
        
        if line.starts_with("diff --git") {
            // Extract filename from git diff
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 3 {
                let path = parts[2].trim_start_matches("a/").trim_start_matches("b/");
                file = path.to_string();
            }
        } else if line.starts_with("--- ") {
            // Old file path
            let path = line[4..].split('\t').next().unwrap_or("");
            let path = path.trim_start_matches("a/").trim_start_matches("b/");
            if path.starts_with("/dev/null") {
                // New file
                if i + 1 < lines.len() && lines[i + 1].starts_with("+++ ") {
                    let new_path = lines[i + 1][4..].split('\t').next().unwrap_or("");
                    file = new_path.trim_start_matches("a/").trim_start_matches("b/").to_string();
                }
            } else {
                file = path.to_string();
            }
            idx = i + 1;
            break;
        }
    }

    if file.is_empty() {
        return Err(ToolError::InvalidParams("No file found in patch".to_string()));
    }

    Ok((file, idx + 1))
}

/// Parse a hunk from patch.
fn parse_hunk(lines: &[&str]) -> ToolResult<(Hunk, usize)> {
    if lines.is_empty() || !lines[0].starts_with("@@") {
        return Err(ToolError::InvalidParams("Invalid hunk header".to_string()));
    }

    // Parse hunk header: @@ -old_start,old_lines +new_start,new_lines @@
    let header = lines[0];
    let mut old_start = 1;
    let mut old_lines = 1;
    let mut new_start = 1;
    let mut new_lines = 1;

    if let Some(old_part) = header.split('-').nth(1).and_then(|s| s.split(' ').next()) {
        let parts: Vec<&str> = old_part.split(',').collect();
        old_start = parts[0].parse().unwrap_or(1);
        if parts.len() > 1 {
            old_lines = parts[1].parse().unwrap_or(1);
        }
    }

    if let Some(new_part) = header.split('+').nth(1).and_then(|s| s.split(' ').next()) {
        let parts: Vec<&str> = new_part.split(',').collect();
        new_start = parts[0].parse().unwrap_or(1);
        if parts.len() > 1 {
            new_lines = parts[1].parse().unwrap_or(1);
        }
    }

    let mut hunk_lines = Vec::new();
    let mut idx = 1;

    while idx < lines.len() {
        let line = lines[idx];
        
        if line.starts_with("@@") || line.starts_with("diff --git") {
            break;
        }
        
        if line.starts_with('+') {
            hunk_lines.push(HunkLine::Add(line[1..].to_string()));
        } else if line.starts_with('-') {
            hunk_lines.push(HunkLine::Remove(line[1..].to_string()));
        } else if line.starts_with(' ') || line.is_empty() {
            hunk_lines.push(HunkLine::Context(line.get(1..).unwrap_or("").to_string()));
        } else if line.starts_with('\\') {
            // "\ No newline at end of file" - skip
        } else {
            break;
        }
        
        idx += 1;
    }

    Ok((Hunk {
        old_start,
        old_lines,
        new_start,
        new_lines,
        lines: hunk_lines,
    }, idx))
}

/// Apply patch hunks to content.
fn apply_patch_to_content(
    content: &str,
    hunks: &[Hunk],
    reverse: bool,
) -> (String, u32, u32, Vec<String>) {
    let mut lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();
    let mut additions = 0;
    let mut deletions = 0;
    let mut rejected = Vec::new();
    let mut offset: i32 = 0;

    for hunk in hunks {
        let old_start = if reverse { hunk.new_start } else { hunk.old_start };
        let start_idx = (old_start as i32 - 1 + offset) as usize;

        if start_idx > lines.len() {
            rejected.push(format!("Hunk at line {} out of range", old_start));
            continue;
        }

        // Try to find matching context
        let mut match_found = false;
        let mut search_range = start_idx.min(lines.len());
        
        for try_start in start_idx..=search_range.min(lines.len()) {
            if hunk_matches(&lines[try_start..], hunk, reverse) {
                // Apply hunk
                let (new_lines, adds, dels) = apply_hunk(hunk, reverse);
                
                let old_end = try_start + count_old_lines(hunk, reverse);
                lines.splice(try_start..old_end.min(lines.len()), new_lines);
                
                additions += adds;
                deletions += dels;
                offset += (adds as i32) - (dels as i32);
                match_found = true;
                break;
            }
        }

        if !match_found {
            rejected.push(format!("Hunk at line {} failed to apply", old_start));
        }
    }

    let out = lines.join("\n");
    let out = if content.ends_with('\n') { format!("{}\n", out) } else { out };
    (out, additions, deletions, rejected)
}

/// Check if hunk matches at position.
fn hunk_matches(lines: &[String], hunk: &Hunk, reverse: bool) -> bool {
    let mut line_idx = 0;
    
    for hunk_line in &hunk.lines {
        match hunk_line {
            HunkLine::Context(expected) => {
                if line_idx >= lines.len() || lines[line_idx] != *expected {
                    return false;
                }
                line_idx += 1;
            }
            HunkLine::Remove(_) if !reverse => {
                line_idx += 1; // consumed one source line
            }
            HunkLine::Add(_) if reverse => {
                // Added line in reverse = removed line - don't check
            }
            _ => {}
        }
    }
    
    true
}

/// Apply a single hunk and return new lines.
fn apply_hunk(hunk: &Hunk, reverse: bool) -> (Vec<String>, u32, u32) {
    let mut new_lines = Vec::new();
    let mut additions = 0;
    let mut deletions = 0;

    for hunk_line in &hunk.lines {
        match hunk_line {
            HunkLine::Context(line) => {
                new_lines.push(line.clone());
            }
            HunkLine::Add(line) if !reverse => {
                new_lines.push(line.clone());
                additions += 1;
            }
            HunkLine::Remove(line) if !reverse => {
                deletions += 1;
            }
            HunkLine::Add(line) if reverse => {
                // In reverse, add becomes remove
                deletions += 1;
            }
            HunkLine::Remove(line) if reverse => {
                // In reverse, remove becomes add
                new_lines.push(line.clone());
                additions += 1;
            }
            _ => {}
        }
    }

    (new_lines, additions, deletions)
}

/// Count old lines in hunk.
fn count_old_lines(hunk: &Hunk, reverse: bool) -> usize {
    let mut count = 0;
    for line in &hunk.lines {
        match line {
            HunkLine::Context(_) => count += 1,
            HunkLine::Remove(_) if !reverse => count += 1,
            HunkLine::Add(_) if reverse => count += 1,
            _ => {}
        }
    }
    count
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
    async fn test_apply_patch_simple() {
        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        tokio::fs::write(&file_path, "line 1\nline 2\nline 3\n").await.unwrap();

        let patch = r#"--- a/test.txt
+++ b/test.txt
@@ -1,3 +1,3 @@
 line 1
-line 2
+modified line 2
 line 3
"#;

        let tool = ApplyPatchTool::new();
        let params = serde_json::json!({
            "patch": patch,
            "cwd": temp_dir.path().to_string_lossy()
        });
        let context = ToolContext::default();

        let result = tool.execute(params, &context).await;
        assert!(result.is_ok());

        let content = tokio::fs::read_to_string(&file_path).await.unwrap();
        assert_eq!(content, "line 1\nmodified line 2\nline 3\n");
    }

    #[tokio::test]
    async fn test_apply_patch_dry_run() {
        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        tokio::fs::write(&file_path, "line 1\nline 2\n").await.unwrap();

        let patch = r#"--- a/test.txt
+++ b/test.txt
@@ -1,2 +1,2 @@
 line 1
-line 2
+new line 2
"#;

        let tool = ApplyPatchTool::new();
        let params = serde_json::json!({
            "patch": patch,
            "cwd": temp_dir.path().to_string_lossy(),
            "dry_run": true
        });
        let context = ToolContext::default();

        let result = tool.execute(params, &context).await;
        assert!(result.is_ok());

        // File should be unchanged
        let content = tokio::fs::read_to_string(&file_path).await.unwrap();
        assert_eq!(content, "line 1\nline 2\n");
    }
}
