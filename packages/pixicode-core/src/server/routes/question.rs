//! Question route handlers — pending user questions and answers.

use axum::{extract::{Path, State}, Json};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::server::error::{ApiError, ApiResult};
use crate::server::state::AppState;

#[derive(Serialize)]
pub struct QuestionResponse {
    pub id: String,
    pub prompt: String,
}

#[derive(Deserialize)]
pub struct AnswerRequest {
    pub answer: String,
}

pub async fn list(State(s): State<Arc<AppState>>) -> ApiResult<Json<Vec<QuestionResponse>>> {
    let questions = s.questions.read().await;
    let result: Vec<QuestionResponse> = questions
        .iter()
        .map(|(id, state)| QuestionResponse {
            id: id.clone(),
            prompt: state.prompt.clone(),
        })
        .collect();
    Ok(Json(result))
}

pub async fn answer(
    State(s): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(body): Json<AnswerRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    let tx = {
        let mut questions = s.questions.write().await;
        questions.remove(&id)
    };

    match tx {
        Some(state) => {
            // Try to send answer; if receiver dropped, that's fine
            let _ = Arc::try_unwrap(state.answer_tx)
                .map(|tx| tx.send(body.answer));
            Ok(Json(serde_json::json!({ "ok": true })))
        }
        None => Err(ApiError::not_found("question not found")),
    }
}

/// POST /question/:id/reject — reject a pending question.
pub async fn reject(
    State(s): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> ApiResult<Json<serde_json::Value>> {
    let tx = {
        let mut questions = s.questions.write().await;
        questions.remove(&id)
    };

    match tx {
        Some(state) => {
            // Send a rejection marker
            let _ = Arc::try_unwrap(state.answer_tx)
                .map(|tx| tx.send("__rejected__".to_string()));
            Ok(Json(serde_json::json!({ "ok": true, "rejected": true })))
        }
        None => Err(ApiError::not_found("question not found")),
    }
}
