//! File operation route handlers — read, write, ls, find (grep), find/file (glob).
use axum::{extract::{Query, State}, Json};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;
use crate::server::error::{ApiError, ApiResult};
use crate::server::state::AppState;

#[derive(Deserialize)]
pub struct FileQuery { pub path: String }

pub async fn read_file(
    State(_s): State<Arc<AppState>>,
    Query(q): Query<FileQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    let content = tokio::fs::read_to_string(&q.path).await
        .map_err(|e| ApiError::not_found(e.to_string()))?;
    Ok(Json(serde_json::json!({ "path": q.path, "content": content })))
}

pub async fn write_file(
    State(_s): State<Arc<AppState>>,
    Json(body): Json<serde_json::Value>,
) -> ApiResult<Json<serde_json::Value>> {
    let path = body["path"].as_str().ok_or_else(|| ApiError::bad_request("missing path"))?;
    let content = body["content"].as_str().ok_or_else(|| ApiError::bad_request("missing content"))?;
    tokio::fs::write(path, content).await.map_err(|e| ApiError::internal(e.to_string()))?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

pub async fn ls(
    State(_s): State<Arc<AppState>>,
    Query(q): Query<FileQuery>,
) -> ApiResult<Json<Vec<String>>> {
    let mut entries = tokio::fs::read_dir(&q.path).await.map_err(|e| ApiError::not_found(e.to_string()))?;
    let mut names = vec![];
    while let Ok(Some(entry)) = entries.next_entry().await {
        names.push(entry.file_name().to_string_lossy().into_owned());
    }
    Ok(Json(names))
}

// ─── Find (ripgrep text search) ──────────────────────────────────────────────

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FindQuery {
    /// Search pattern (regex by default)
    pub pattern: String,
    /// Directory to search in
    #[serde(default)]
    pub path: Option<String>,
    /// Case insensitive
    #[serde(default)]
    pub ignore_case: Option<bool>,
    /// Fixed string (disable regex)
    #[serde(default)]
    pub fixed_strings: Option<bool>,
    /// Include hidden files
    #[serde(default)]
    pub hidden: Option<bool>,
    /// Max results (default 50)
    #[serde(default)]
    pub limit: Option<u32>,
    /// Glob include filter, e.g. "*.rs"
    #[serde(default)]
    pub include: Option<String>,
    /// Glob exclude filter, e.g. "*.min.js"
    #[serde(default)]
    pub exclude: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GrepMatchResponse {
    pub file: String,
    pub line_number: u32,
    pub text: String,
}

/// GET /find — text search using ripgrep
pub async fn find(
    State(_s): State<Arc<AppState>>,
    Query(q): Query<FindQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    let search_dir = q.path.clone().unwrap_or_else(|| ".".to_string());
    let search_path = PathBuf::from(&search_dir);
    if !search_path.exists() {
        return Err(ApiError::not_found(format!("Directory not found: {search_dir}")));
    }

    let limit = q.limit.unwrap_or(50);

    // Build ripgrep command
    let mut cmd = tokio::process::Command::new("rg");
    cmd.arg("--json")
        .arg("--line-number")
        .arg("--color=never")
        .arg("--max-count")
        .arg(limit.to_string())
        .arg(&q.pattern)
        .arg(&search_dir);

    if q.ignore_case.unwrap_or(false) {
        cmd.arg("--ignore-case");
    }
    if q.hidden.unwrap_or(false) {
        cmd.arg("--hidden");
    }
    if q.fixed_strings.unwrap_or(false) {
        cmd.arg("--fixed-strings");
    }
    if let Some(ref inc) = q.include {
        cmd.arg("--glob").arg(inc);
    }
    if let Some(ref exc) = q.exclude {
        cmd.arg("--glob").arg(format!("!{exc}"));
    }

    let child = cmd
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                ApiError::internal("ripgrep (rg) not found. Please install it.")
            } else {
                ApiError::internal(e.to_string())
            }
        })?;

    let output = child.wait_with_output().await.map_err(|e| ApiError::internal(e.to_string()))?;

    // Parse JSON lines from stdout
    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut matches: Vec<GrepMatchResponse> = Vec::new();

    for line in stdout.lines() {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(line) {
            if json.get("type").and_then(|t| t.as_str()) != Some("match") {
                continue;
            }
            if let Some(data) = json.get("data") {
                let file = data
                    .get("path")
                    .and_then(|p| p.get("text"))
                    .and_then(|s| s.as_str())
                    .unwrap_or_default();

                let line_num = data
                    .get("line_number")
                    .and_then(|n| n.as_u64())
                    .unwrap_or(0) as u32;

                let line_text = data
                    .get("lines")
                    .and_then(|l| l.get("text"))
                    .and_then(|t| t.as_str())
                    .unwrap_or("")
                    .trim()
                    .to_string();

                // Make file path relative to search_dir
                let relative = file
                    .strip_prefix(&format!("{}/", search_path.display()))
                    .or_else(|| file.strip_prefix(&format!("{}", search_path.display())))
                    .unwrap_or(file)
                    .to_string();

                matches.push(GrepMatchResponse {
                    file: relative,
                    line_number: line_num,
                    text: line_text,
                });
            }
        }
    }

    Ok(Json(serde_json::json!({
        "pattern": q.pattern,
        "matches": matches,
        "count": matches.len(),
    })))
}

// ─── Find File (glob pattern matching) ───────────────────────────────────────

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FindFileQuery {
    /// Glob pattern, e.g. "**/*.rs"
    pub pattern: String,
    /// Base directory (default: ".")
    #[serde(default)]
    pub cwd: Option<String>,
    /// Include hidden files (default: false)
    #[serde(default)]
    pub include_hidden: Option<bool>,
    /// Max results (default 100)
    #[serde(default)]
    pub limit: Option<u32>,
}

/// GET /find/file — find files by glob pattern
pub async fn find_file(
    State(_s): State<Arc<AppState>>,
    Query(q): Query<FindFileQuery>,
) -> ApiResult<Json<serde_json::Value>> {
    let base_dir = PathBuf::from(q.cwd.as_deref().unwrap_or("."));
    if !base_dir.exists() {
        return Err(ApiError::not_found(format!("Directory not found: {}", base_dir.display())));
    }

    let limit = q.limit.unwrap_or(100) as usize;
    let include_hidden = q.include_hidden.unwrap_or(false);

    let full_pattern = base_dir.join(&q.pattern).to_string_lossy().to_string();

    let mut matches: Vec<String> = Vec::new();
    let entries = glob::glob(&full_pattern)
        .map_err(|e| ApiError::bad_request(format!("Invalid glob pattern: {e}")))?;

    for entry in entries {
        if matches.len() >= limit {
            break;
        }
        match entry {
            Ok(path) => {
                // Skip hidden files unless requested
                if !include_hidden {
                    if let Some(name) = path.file_name() {
                        if name.to_string_lossy().starts_with('.') {
                            continue;
                        }
                    }
                }
                // Make path relative to base_dir
                let relative = path
                    .strip_prefix(&base_dir)
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_else(|_| path.to_string_lossy().to_string());
                matches.push(relative);
            }
            Err(e) => {
                tracing::warn!("Glob entry error: {e}");
            }
        }
    }

    matches.sort();

    Ok(Json(serde_json::json!({
        "pattern": q.pattern,
        "matches": matches,
        "count": matches.len(),
    })))
}
