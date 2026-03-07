//! Tower / Axum middleware: basic auth + request logger.

use axum::{
    extract::Request,
    http::{header, StatusCode},
    middleware::Next,
    response::Response,
};
use base64::{engine::general_purpose::STANDARD, Engine};

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
