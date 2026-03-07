//! SSE (Server-Sent Events) streaming endpoint for the event bus.

use axum::{
    extract::State,
    response::{IntoResponse, Response},
};
use futures::StreamExt;
use std::sync::Arc;
use std::convert::Infallible;
use axum::http::header;
use axum::body::Body;
use tokio_stream::wrappers::BroadcastStream;

use crate::bus::BusEvent;
use crate::server::state::AppState;

/// `GET /events`
///
/// Opens an SSE stream. Every internal event published to the EventBus is
/// forwarded to the client as `data: <json>\n\n`.
pub async fn sse_handler(
    State(state): State<Arc<AppState>>,
) -> Response {
    let rx = state.bus.subscribe();
    let stream = BroadcastStream::new(rx)
        .filter_map(|result| async move {
            match result {
                Ok(event) => {
                    let json = serde_json::to_string(&event).unwrap_or_default();
                    Some(Ok::<_, Infallible>(format!("data: {}\n\n", json)))
                }
                Err(_) => None,
            }
        });

    let body = Body::from_stream(stream);

    Response::builder()
        .status(200)
        .header(header::CONTENT_TYPE, "text/event-stream")
        .header(header::CACHE_CONTROL, "no-cache")
        .header(header::CONNECTION, "keep-alive")
        .header("X-Accel-Buffering", "no")
        .body(body)
        .unwrap()
}
