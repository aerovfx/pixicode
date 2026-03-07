//! VCS info route handler — query git status.

use axum::extract::State;
use axum::Json;
use std::sync::Arc;

use crate::server::error::ApiResult;
use crate::server::state::AppState;

pub async fn info(State(_s): State<Arc<AppState>>) -> ApiResult<Json<serde_json::Value>> {
    let cwd = std::env::current_dir().unwrap_or_default();

    // Check if we're in a git repo
    let output = tokio::process::Command::new("git")
        .args(["rev-parse", "--is-inside-work-tree"])
        .current_dir(&cwd)
        .output()
        .await;

    let is_git = output.as_ref().map(|o| o.status.success()).unwrap_or(false);

    if !is_git {
        return Ok(Json(serde_json::json!({
            "vcs": null,
            "branch": null,
            "dirty": false,
        })));
    }

    // Get current branch
    let branch = tokio::process::Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .current_dir(&cwd)
        .output()
        .await
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string());

    // Check dirty status
    let dirty = tokio::process::Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(&cwd)
        .output()
        .await
        .ok()
        .map(|o| !o.stdout.is_empty())
        .unwrap_or(false);

    // Get root directory
    let root = tokio::process::Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .current_dir(&cwd)
        .output()
        .await
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string());

    Ok(Json(serde_json::json!({
        "vcs": "git",
        "branch": branch,
        "dirty": dirty,
        "root": root,
    })))
}
