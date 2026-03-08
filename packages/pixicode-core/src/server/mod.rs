//! HTTP server — Axum + Tower middleware
//!
//! Hosts all 13 route groups plus /doc (OpenAPI) and an SSE bus endpoint.

pub mod error;
pub mod middleware;
pub mod routes;
pub mod state;
pub mod sse;

pub use state::AppState;

use anyhow::Result;
use axum::Router;
use std::net::SocketAddr;
use std::sync::Arc;
use tower::ServiceBuilder;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tracing::info;

use axum::middleware as axum_middleware;
use crate::server::middleware::{auth_middleware, request_logger, workspace_ctx_middleware};
use crate::server::routes::build_router;

/// Run the Axum HTTP server until the shutdown signal fires.
pub async fn run(
    state: Arc<AppState>,
    addr: String,
    mut shutdown_rx: tokio::sync::broadcast::Receiver<()>,
) -> Result<()> {
    let app = build_app(state);

    let sock_addr: SocketAddr = addr.parse()?;
    info!(addr = %sock_addr, "HTTP server listening");

    let listener = tokio::net::TcpListener::bind(sock_addr).await?;
    axum::serve(listener, app)
        .with_graceful_shutdown(async move {
            let _ = shutdown_rx.recv().await;
            info!("HTTP server shutting down");
        })
        .await?;

    Ok(())
}

/// Assemble the full Axum application.
pub fn build_app(state: Arc<AppState>) -> Router {
    // CORS — allow all origins in dev; tighten for production via PIXICODE_CORS_ORIGIN
    let cors_origins = std::env::var("PIXICODE_CORS_ORIGIN")
        .map(|_o| {
            tower_http::cors::Any // TODO: parse allowed origins
        })
        .unwrap_or(tower_http::cors::Any);

    let cors = CorsLayer::new()
        .allow_origin(cors_origins)
        .allow_methods(tower_http::cors::Any)
        .allow_headers(tower_http::cors::Any);

    let middleware_stack = ServiceBuilder::new()
        .layer(TraceLayer::new_for_http())
        .layer(cors);

    build_router(state)
        .layer(axum_middleware::from_fn(workspace_ctx_middleware))
        .layer(middleware_stack)
}
