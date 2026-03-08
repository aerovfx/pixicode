//! Tower / Axum middleware: basic auth + request logger + workspace context.

use axum::{
    extract::Request,
    http::{header, StatusCode},
    middleware::Next,
    response::Response,
};
use base64::{engine::general_purpose::STANDARD, Engine};

// ─────────────────────────────────────────────────────────────────────────────
//  Workspace context (Phase 1: parity with TS WorkspaceContext + Instance)
// ─────────────────────────────────────────────────────────────────────────────

/// Per-request workspace/directory context from query or headers.
#[derive(Debug, Clone)]
pub struct WorkspaceCtx {
    pub directory: String,
    pub workspace_id: Option<String>,
    /// Detected project type based on marker files.
    pub project_type: Option<ProjectType>,
}

/// Detected project type by presence of marker files in `directory`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectType {
    Rust,
    Node,
    Python,
    Go,
    Git,
    Unknown,
}

/// Detect project type from directory markers.
fn detect_project_type(directory: &str) -> Option<ProjectType> {
    use std::path::Path;
    let dir = Path::new(directory);
    if dir.join("Cargo.toml").exists() {
        Some(ProjectType::Rust)
    } else if dir.join("package.json").exists() {
        Some(ProjectType::Node)
    } else if dir.join("pyproject.toml").exists() || dir.join("setup.py").exists() {
        Some(ProjectType::Python)
    } else if dir.join("go.mod").exists() {
        Some(ProjectType::Go)
    } else if dir.join(".git").exists() {
        Some(ProjectType::Git)
    } else {
        None
    }
}

fn query_param(q: Option<&str>, name: &str) -> Option<String> {
    let q = q?;
    for pair in q.split('&') {
        let mut it = pair.splitn(2, '=');
        if it.next()? == name {
            return it.next().map(String::from);
        }
    }
    None
}

/// Reads `directory` and optional `workspace` from query or headers and inserts
/// `Extension(WorkspaceCtx)` so handlers can extract it.
pub async fn workspace_ctx_middleware(req: Request, next: Next) -> Response {
    let directory = query_param(req.uri().query(), "directory")
        .or_else(|| {
            req.headers()
                .get("x-pixicode-directory")
                .and_then(|v| v.to_str().ok())
                .map(String::from)
        })
        .unwrap_or_else(|| std::env::current_dir().map(|p| p.display().to_string()).unwrap_or_else(|_| ".".into()));

    let workspace_id = query_param(req.uri().query(), "workspace").or_else(|| {
        req.headers()
            .get("x-pixicode-workspace")
            .and_then(|v| v.to_str().ok())
            .map(String::from)
    });

    let project_type = detect_project_type(&directory);
    let ctx = WorkspaceCtx {
        directory,
        workspace_id,
        project_type,
    };
    let mut req = req;
    req.extensions_mut().insert(ctx);
    next.run(req).await
}

// ─────────────────────────────────────────────────────────────────────────────
//  Basic auth middleware
// ─────────────────────────────────────────────────────────────────────────────

/// Optional basic-auth gate.
///
/// Only active when `PIXICODE_SERVER_PASSWORD` is set.
/// Expects `Authorization: Basic base64(pixicode:<password>)`.
pub async fn auth_middleware(req: Request, next: Next) -> Result<Response, StatusCode> {
    let password = std::env::var("PIXICODE_SERVER_PASSWORD").ok();

    if let Some(pwd) = password {
        let auth_header = req
            .headers()
            .get(header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");

        if !verify_basic_auth(auth_header, &pwd) {
            return Err(StatusCode::UNAUTHORIZED);
        }
    }

    Ok(next.run(req).await)
}

fn verify_basic_auth(header: &str, expected_password: &str) -> bool {
    let encoded = header.strip_prefix("Basic ").unwrap_or("");
    let decoded = STANDARD.decode(encoded).unwrap_or_default();
    let s = String::from_utf8(decoded).unwrap_or_default();
    // Accept "pixicode:<password>" or just ":<password>" or "<password>"
    s.ends_with(&format!(":{}", expected_password))
        || s == expected_password
}

// ─────────────────────────────────────────────────────────────────────────────
//  Request logger middleware
// ─────────────────────────────────────────────────────────────────────────────

/// Lightweight per-request logger (method + path + status + latency).
/// Tower-Http `TraceLayer` is the primary logger; this adds pixicode-specific
/// structured fields.
pub async fn request_logger(req: Request, next: Next) -> Response {
    let method = req.method().clone();
    let uri = req.uri().clone();
    let start = std::time::Instant::now();

    let response = next.run(req).await;

    tracing::info!(
        method = %method,
        path   = %uri.path(),
        status = response.status().as_u16(),
        latency_ms = start.elapsed().as_millis(),
        "request"
    );

    response
}
