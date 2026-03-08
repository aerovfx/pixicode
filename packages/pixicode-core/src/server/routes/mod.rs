//! Route registration — all 13 route groups + /doc + /events SSE

use axum::{
    middleware,
    Router,
    routing::{delete, get, post, put},
};
use std::sync::Arc;

use crate::server::middleware::workspace_ctx_middleware;
use crate::server::state::AppState;
use crate::server::sse::sse_handler;

// ─── Handler sub-modules ──────────────────────────────────────────────────────

mod auth;
mod command;
mod config;
mod file;
mod global;
mod instance;
mod mcp;
mod path;
mod permission;
mod project;
mod provider;
mod question;
mod session;
mod tui;
mod vcs;
mod workspace;

// OpenAPI doc stub
async fn openapi_doc() -> axum::Json<serde_json::Value> {
    axum::Json(serde_json::json!({
        "openapi": "3.1.0",
        "info": {
            "title": "pixicode API",
            "version": env!("CARGO_PKG_VERSION")
        },
        "paths": {}
    }))
}

/// Build the full router tree with all route groups.
pub fn build_router(state: Arc<AppState>) -> Router {
    Router::new()
        // ── Session ────────────────────────────────────────────────
        .route("/session",                   get(session::list).post(session::create))
        .route("/session/:id",               get(session::get).delete(session::delete_session).patch(session::patch_session))
        .route("/session/:id/messages",      get(session::list_messages))
        .route("/session/:id/message",       post(session::create_message))
        .route("/session/:id/prompt_async",  post(session::prompt_async))
        .route("/session/:id/abort",         post(session::abort))
        .route("/session/:id/init",          post(session::init))
        .route("/session/status",            get(session::status))

        // ── Config ─────────────────────────────────────────────────
        .route("/config",                    get(config::get).put(config::update))
        .route("/config/providers",          get(config::get_providers))

        // ── Project (Phase 1 parity with TS ProjectRoutes) ─────────
        .route("/project",                   get(project::list))
        .route("/project/current",           get(project::current))
        .route("/project/:id",               axum::routing::patch(project::patch))

        // ── Provider ───────────────────────────────────────────────
        .route("/provider",                  get(provider::list))
        .route("/provider/:id/models",       get(provider::list_models))

        // ── Permission ─────────────────────────────────────────────
        .route("/permission",                get(permission::get).post(permission::grant))
        .route("/permission/:tool",          delete(permission::revoke))
        .route("/permission/:request_id/reply", post(permission::reply))

        // ── Question ───────────────────────────────────────────────
        .route("/question",                  get(question::list))
        .route("/question/:id/answer",       post(question::answer))
        .route("/question/:id/reject",       post(question::reject))

        // ── Global ─────────────────────────────────────────────────
        .route("/global",                    get(global::get).put(global::update))
        .route("/global/health",             get(global::health))

        // ── MCP ────────────────────────────────────────────────────
        .route("/mcp",                       get(mcp::list).post(mcp::add))
        .route("/mcp/:name",                 delete(mcp::remove))
        .route("/mcp/:name/enable",          post(mcp::enable))
        .route("/mcp/:name/disable",         post(mcp::disable))

        // ── TUI ────────────────────────────────────────────────────
        .route("/tui/theme",                 get(tui::get_theme).put(tui::set_theme))
        .route("/tui/keybinds",              get(tui::get_keybinds))

        // ── Workspace ──────────────────────────────────────────────
        .route("/workspace",                 get(workspace::list).post(workspace::create))
        .route("/workspace/:id",             get(workspace::get).delete(workspace::delete_workspace))

        // ── File ───────────────────────────────────────────────────
        .route("/file",                      get(file::read_file))
        .route("/file/write",                post(file::write_file))
        .route("/file/ls",                   get(file::ls))

        // ── Find (file search) ────────────────────────────────────
        .route("/find",                      get(file::find))
        .route("/find/file",                 get(file::find_file))

        // ── Auth ───────────────────────────────────────────────────
        .route("/auth",                      get(auth::list))
        .route("/auth/:provider",            post(auth::set).delete(auth::remove))

        // ── Instance ───────────────────────────────────────────────
        .route("/instance/dispose",          post(instance::dispose))

        // ── Info routes ────────────────────────────────────────────
        .route("/path",                      get(path::info))
        .route("/vcs",                       get(vcs::info))
        .route("/command",                   get(command::list))

        // ── SSE event bus ──────────────────────────────────────────
        .route("/events",                    get(sse_handler))

        // ── OpenAPI doc ────────────────────────────────────────────
        .route("/doc",                       get(openapi_doc))

        .with_state(state)
}
