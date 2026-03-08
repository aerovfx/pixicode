//! Prompt Loop Engine — streams LLM, detects tool calls, executes tools, loops.
//!
//! This is the core engine that powers `POST /session/:id/prompt_async`.
//!
//! Flow:
//!   1. Build system prompt + conversation history + tool definitions
//!   2. Call `provider.chat_stream()` with the request
//!   3. Accumulate streaming chunks; detect tool calls
//!   4. When finish_reason == ToolCalls → execute tools → loop back to step 2
//!   5. When finish_reason == Stop → persist assistant message → done
//!   6. Publish bus events throughout

use std::sync::Arc;
use futures::StreamExt;
use rusqlite::params;

use crate::bus::{BusEvent, EventBus};
use crate::db::Database;
use crate::providers::registry::ProviderRegistry;
use crate::providers::types::{
    ChatRequest, FinishReason, Message as ProviderMessage, MessageRole as ProviderRole,
    ToolCall as ProviderToolCall, ToolDefinition,
};
use crate::server::state::PendingPermissionReplies;
use crate::tools::registry::ToolRegistry;
use crate::tools::trait_def::{ToolCall as InternalToolCall, ToolContext, PermissionLevel};

use super::permission_gate::{GateResult, PermissionGate};
use super::system::build_system_prompt;

/// Maximum number of tool-call → re-prompt iterations to prevent infinite loops.
const MAX_TOOL_ITERATIONS: usize = 25;

/// Configuration for a prompt run.
#[derive(Debug, Clone)]
pub struct PromptConfig {
    pub session_id: String,
    pub provider_id: String,
    pub model_id: String,
    pub system_override: Option<String>,
    pub working_dir: String,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
}

/// Run the prompt loop to completion (blocking async — run inside `tokio::spawn`).
///
/// 1. Loads existing messages from DB
/// 2. Builds tool definitions from ToolRegistry
/// 3. Streams LLM response, handles tool calls iteratively
/// 4. Persists the final assistant message to DB
/// 5. Publishes bus events
pub async fn run_prompt(
    db: Database,
    bus: EventBus,
    registry: Arc<ProviderRegistry>,
    tool_registry: Arc<ToolRegistry>,
    permission_replies: PendingPermissionReplies,
    config: PromptConfig,
) {
    let session_id = &config.session_id;

    // ── 0. Create permission gate ────────────────────────────────────────────
    let permission_gate = PermissionGate::new(
        db.clone(),
        bus.clone(),
        permission_replies,
    );

    // ── 1. Load conversation history from DB ────────────────────────────────
    let history = match load_messages(&db, session_id) {
        Ok(m) => m,
        Err(e) => {
            tracing::error!(%session_id, error = %e, "prompt: load messages failed");
            return;
        }
    };

    // ── 2. Build system prompt ──────────────────────────────────────────────
    let tool_names: Vec<&str> = tool_registry
        .list_tools()
        .into_iter()
        .collect();
    let system_text = build_system_prompt(
        config.system_override.as_deref(),
        &config.working_dir,
        &tool_names,
    );

    // ── 3. Build tool definitions for the LLM ───────────────────────────────
    let tool_defs = build_tool_definitions(&tool_registry);
    let tools_for_request = if tool_defs.is_empty() { None } else { Some(tool_defs) };

    // ── 4. Resolve provider ─────────────────────────────────────────────────
    let provider = match registry.get(&config.provider_id).await {
        Some(p) => p,
        None => {
            tracing::error!(provider = %config.provider_id, "prompt: provider not found");
            return;
        }
    };

    // ── 5. Assemble initial messages ────────────────────────────────────────
    let mut messages: Vec<ProviderMessage> = Vec::new();
    messages.push(ProviderMessage::system(system_text));
    messages.extend(history);

    // ── 6. Prompt loop (may iterate when LLM calls tools) ───────────────────
    let tool_ctx = ToolContext::new(std::path::PathBuf::from(&config.working_dir))
        .with_permission(PermissionLevel::Allow)
        .with_timeout(120_000)
        .with_session(config.session_id.clone());

    for iteration in 0..MAX_TOOL_ITERATIONS {
        tracing::debug!(%session_id, iteration, msgs = messages.len(), "prompt: iteration start");

        let request = ChatRequest {
            cancel: None,
            model: config.model_id.clone(),
            messages: messages.clone(),
            tools: tools_for_request.clone(),
            tool_choice: None,
            temperature: config.temperature,
            max_tokens: config.max_tokens,
            top_p: None,
            frequency_penalty: None,
            presence_penalty: None,
            stop: None,
            response_format: None,
            stream: true,
            user: None,
        };

        // ── Stream response ─────────────────────────────────────────────
        let stream = match provider.chat_stream(request).await {
            Ok(s) => s,
            Err(e) => {
                tracing::error!(%session_id, error = %e, "prompt: chat_stream failed");
                persist_error_message(&db, &bus, session_id, &format!("LLM error: {e}"));
                return;
            }
        };

        let (content, tool_calls, finish_reason) = accumulate_stream(stream).await;

        tracing::debug!(
            %session_id, iteration,
            content_len = content.len(),
            tool_calls = tool_calls.len(),
            ?finish_reason,
            "prompt: stream complete"
        );

        // ── Handle tool calls ───────────────────────────────────────────
        if finish_reason == Some(FinishReason::ToolCalls) && !tool_calls.is_empty() {
            // Append the assistant message with tool calls to history
            messages.push(ProviderMessage {
                role: ProviderRole::Assistant,
                content: content.clone(),
                name: None,
                tool_call_id: None,
                tool_calls: Some(tool_calls.clone()),
            });

            // Resolve project_id for permission checking
            let project_id = resolve_project_id(&db, session_id);

            // Execute each tool call and append results
            for tc in &tool_calls {
                // ── Permission gate check ──────────────────────────────────
                let gate_result = permission_gate
                    .check(&tc.name, session_id, &project_id)
                    .await;

                match gate_result {
                    GateResult::Denied { reason } => {
                        tracing::info!(
                            tool = %tc.name,
                            %session_id,
                            "permission denied: {reason}"
                        );
                        messages.push(ProviderMessage::tool_response(
                            &tc.id,
                            &format!("Permission denied: {reason}"),
                        ));
                        continue;
                    }
                    GateResult::Allowed => {}
                }

                bus.publish(BusEvent::ToolCallStarted {
                    session_id: session_id.to_string(),
                    tool: tc.name.clone(),
                    call_id: tc.id.clone(),
                });

                let internal_call = InternalToolCall {
                    name: tc.name.clone(),
                    arguments: tc.arguments.clone(),
                    call_id: tc.id.clone(),
                };

                let result = tool_registry.execute(&internal_call, &tool_ctx).await;
                let (output_text, ok) = match result {
                    Ok(output) => {
                        let text = if output.success {
                            output.output
                        } else {
                            output.error.unwrap_or_else(|| "Tool failed".to_string())
                        };
                        (text, output.success)
                    }
                    Err(e) => (format!("Tool error: {e}"), false),
                };

                bus.publish(BusEvent::ToolCallFinished {
                    session_id: session_id.to_string(),
                    tool: tc.name.clone(),
                    call_id: tc.id.clone(),
                    ok,
                });

                messages.push(ProviderMessage::tool_response(&tc.id, &output_text));
            }

            // Continue to next iteration (re-prompt with tool results)
            continue;
        }

        // ── No tool calls → persist final assistant message ─────────────
        persist_assistant_message(&db, &bus, session_id, &content, &tool_calls);
        return;
    }

    // If we exhausted iterations, persist whatever we have
    tracing::warn!(%session_id, "prompt: hit max tool iterations ({MAX_TOOL_ITERATIONS})");
    persist_assistant_message(&db, &bus, session_id, "I've reached the maximum number of tool iterations. Here's what I have so far.", &[]);
}

// ─── Helpers ────────────────────────────────────────────────────────────────

/// Load conversation messages from DB and convert to provider Message format.
fn load_messages(db: &Database, session_id: &str) -> Result<Vec<ProviderMessage>, anyhow::Error> {
    db.with(|conn| {
        let mut stmt = conn.prepare(
            "SELECT data FROM message WHERE session_id = ?1 ORDER BY time_created ASC",
        )?;
        let rows = stmt.query_map(params![session_id], |row| row.get::<_, String>(0))?;

        let mut out = Vec::new();
        for row in rows {
            let data_str = row?;
            let data: serde_json::Value = serde_json::from_str(&data_str).unwrap_or_default();
            let role = data.get("role").and_then(|r| r.as_str()).unwrap_or("user");
            let content = data
                .get("content")
                .and_then(|c| c.as_str())
                .unwrap_or("")
                .to_string();
            let role_enum = match role {
                "system" => ProviderRole::System,
                "assistant" => ProviderRole::Assistant,
                "tool" => ProviderRole::Tool,
                _ => ProviderRole::User,
            };
            let tool_call_id = data
                .get("tool_call_id")
                .and_then(|v| v.as_str())
                .map(String::from);
            out.push(ProviderMessage {
                role: role_enum,
                content,
                name: None,
                tool_call_id,
                tool_calls: None,
            });
        }
        Ok(out)
    })
}

/// Build `Vec<ToolDefinition>` from the ToolRegistry for the ChatRequest.
fn build_tool_definitions(registry: &ToolRegistry) -> Vec<ToolDefinition> {
    let schemas = registry.get_schemas();
    schemas
        .into_iter()
        .map(|(name, schema)| {
            let tool = registry.get(name);
            let description = tool
                .map(|t| t.description().to_string())
                .unwrap_or_default();
            ToolDefinition {
                name: name.to_string(),
                description,
                parameters: schema.to_json_value(),
            }
        })
        .collect()
}

/// Accumulate a streaming response into content + tool calls.
async fn accumulate_stream(
    stream: futures::stream::BoxStream<
        'static,
        crate::providers::trait_def::ProviderResult<crate::providers::types::ChatChunk>,
    >,
) -> (String, Vec<ProviderToolCall>, Option<FinishReason>) {
    let mut content = String::new();
    let mut finish_reason: Option<FinishReason> = None;

    // Tool call accumulation: index → (id, name, arguments_json)
    let mut tool_acc: std::collections::HashMap<u32, (String, String, String)> =
        std::collections::HashMap::new();

    let mut stream = std::pin::pin!(stream);
    while let Some(chunk) = stream.next().await {
        match chunk {
            Ok(c) => {
                if let Some(text) = &c.delta.content {
                    content.push_str(text);
                }
                if let Some(tcs) = &c.delta.tool_calls {
                    for tc_delta in tcs {
                        let entry = tool_acc.entry(tc_delta.index).or_insert_with(|| {
                            (String::new(), String::new(), String::new())
                        });
                        if let Some(id) = &tc_delta.id {
                            entry.0 = id.clone();
                        }
                        if let Some(name) = &tc_delta.name {
                            entry.1 = name.clone();
                        }
                        if let Some(args) = &tc_delta.arguments {
                            entry.2.push_str(args);
                        }
                    }
                }
                if let Some(fr) = c.finish_reason {
                    finish_reason = Some(fr);
                }
            }
            Err(e) => {
                tracing::warn!(error = %e, "prompt: stream chunk error");
            }
        }
    }

    // Convert accumulated tool calls
    let mut tool_calls: Vec<ProviderToolCall> = tool_acc
        .into_iter()
        .map(|(_idx, (id, name, args_str))| {
            let arguments = serde_json::from_str(&args_str).unwrap_or(serde_json::Value::Object(
                serde_json::Map::new(),
            ));
            ProviderToolCall {
                id,
                name,
                arguments,
            }
        })
        .collect();
    tool_calls.sort_by_key(|tc| tc.id.clone());

    (content, tool_calls, finish_reason)
}

/// Persist the final assistant message to the database.
fn persist_assistant_message(
    db: &Database,
    bus: &EventBus,
    session_id: &str,
    content: &str,
    tool_calls: &[ProviderToolCall],
) {
    let now = chrono::Utc::now().timestamp();
    let msg_id = format!("msg_{}", ulid::Ulid::new().to_string().to_lowercase());

    let mut data = serde_json::json!({
        "role": "assistant",
        "content": content,
    });
    if !tool_calls.is_empty() {
        data["tool_calls"] = serde_json::to_value(tool_calls).unwrap_or_default();
    }

    if let Err(e) = db.transaction(|conn| {
        conn.execute(
            "INSERT INTO message (id, session_id, time_created, time_updated, data) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![msg_id, session_id, now, now, data.to_string()],
        )?;
        conn.execute(
            "UPDATE session SET time_updated = ?1 WHERE id = ?2",
            params![now, session_id],
        )?;
        Ok(())
    }) {
        tracing::error!(%session_id, error = %e, "prompt: persist assistant message failed");
        return;
    }

    bus.publish(BusEvent::MessageCreated {
        session_id: session_id.to_string(),
        message_id: msg_id,
    });
}

/// Resolve the project_id for a given session (used by permission gate).
fn resolve_project_id(db: &Database, session_id: &str) -> String {
    db.with(|conn| {
        conn.query_row(
            "SELECT project_id FROM session WHERE id = ?1",
            params![session_id],
            |row| row.get::<_, String>(0),
        )
        .map_err(Into::into)
    })
    .unwrap_or_else(|_: anyhow::Error| "default".to_string())
}

/// Persist an error message when the prompt loop fails.
fn persist_error_message(db: &Database, bus: &EventBus, session_id: &str, error: &str) {
    let now = chrono::Utc::now().timestamp();
    let msg_id = format!("msg_{}", ulid::Ulid::new().to_string().to_lowercase());
    let data = serde_json::json!({
        "role": "assistant",
        "content": format!("⚠️ {error}"),
    });

    if let Err(e) = db.transaction(|conn| {
        conn.execute(
            "INSERT INTO message (id, session_id, time_created, time_updated, data) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![msg_id, session_id, now, now, data.to_string()],
        )?;
        Ok(())
    }) {
        tracing::error!(%session_id, error = %e, "prompt: persist error message failed");
    }

    bus.publish(BusEvent::MessageCreated {
        session_id: session_id.to_string(),
        message_id: msg_id,
    });
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;

    fn test_db() -> Database {
        Database::open_in_memory().expect("in-memory db")
    }

    fn seed_session(db: &Database, session_id: &str, project_id: &str) {
        let now = chrono::Utc::now().timestamp();
        db.transaction(|conn| {
            conn.execute(
                "INSERT OR IGNORE INTO project (id, worktree, time_created, time_updated, sandboxes)
                 VALUES (?1, '/tmp', ?2, ?3, '[]')",
                params![project_id, now, now],
            )?;
            conn.execute(
                "INSERT INTO session (id, project_id, slug, directory, title, version, time_created, time_updated)
                 VALUES (?1, ?2, 'test', '/tmp', 'Test Session', '0.1.0', ?3, ?4)",
                params![session_id, project_id, now, now],
            )?;
            Ok(())
        }).unwrap();
    }

    fn seed_messages(db: &Database, session_id: &str, messages: &[(&str, &str)]) {
        let now = chrono::Utc::now().timestamp();
        for (i, (role, content)) in messages.iter().enumerate() {
            let msg_id = format!("msg_test_{i}");
            let data = serde_json::json!({ "role": role, "content": content });
            db.transaction(|conn| {
                conn.execute(
                    "INSERT INTO message (id, session_id, time_created, time_updated, data) VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![msg_id, session_id, now + i as i64, now + i as i64, data.to_string()],
                )?;
                Ok(())
            }).unwrap();
        }
    }

    #[test]
    fn test_load_messages_empty() {
        let db = test_db();
        seed_session(&db, "sess_1", "proj_1");
        let msgs = load_messages(&db, "sess_1").unwrap();
        assert!(msgs.is_empty());
    }

    #[test]
    fn test_load_messages_with_history() {
        let db = test_db();
        seed_session(&db, "sess_1", "proj_1");
        seed_messages(&db, "sess_1", &[
            ("user", "Hello"),
            ("assistant", "Hi there!"),
            ("user", "How are you?"),
        ]);

        let msgs = load_messages(&db, "sess_1").unwrap();
        assert_eq!(msgs.len(), 3);
        assert_eq!(msgs[0].role, ProviderRole::User);
        assert_eq!(msgs[0].content, "Hello");
        assert_eq!(msgs[1].role, ProviderRole::Assistant);
        assert_eq!(msgs[1].content, "Hi there!");
        assert_eq!(msgs[2].role, ProviderRole::User);
        assert_eq!(msgs[2].content, "How are you?");
    }

    #[test]
    fn test_resolve_project_id_found() {
        let db = test_db();
        seed_session(&db, "sess_1", "my_project");
        assert_eq!(resolve_project_id(&db, "sess_1"), "my_project");
    }

    #[test]
    fn test_resolve_project_id_not_found() {
        let db = test_db();
        assert_eq!(resolve_project_id(&db, "nonexistent"), "default");
    }

    #[test]
    fn test_persist_assistant_message() {
        let db = test_db();
        let bus = EventBus::new();
        seed_session(&db, "sess_1", "proj_1");

        persist_assistant_message(&db, &bus, "sess_1", "Hello from AI", &[]);

        let msgs = load_messages(&db, "sess_1").unwrap();
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].role, ProviderRole::Assistant);
        assert_eq!(msgs[0].content, "Hello from AI");
    }

    #[test]
    fn test_persist_error_message() {
        let db = test_db();
        let bus = EventBus::new();
        seed_session(&db, "sess_1", "proj_1");

        persist_error_message(&db, &bus, "sess_1", "Something went wrong");

        let msgs = load_messages(&db, "sess_1").unwrap();
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].role, ProviderRole::Assistant);
        assert!(msgs[0].content.contains("Something went wrong"));
    }

    #[test]
    fn test_build_tool_definitions() {
        let registry = ToolRegistry::with_builtins();
        let defs = build_tool_definitions(&registry);
        assert!(!defs.is_empty());
        let names: Vec<&str> = defs.iter().map(|d| d.name.as_str()).collect();
        assert!(names.contains(&"read"));
        assert!(names.contains(&"write"));
        assert!(names.contains(&"bash"));
        assert!(names.contains(&"glob"));
    }

    #[tokio::test]
    async fn test_accumulate_stream_empty() {
        let stream = futures::stream::empty();
        let boxed: futures::stream::BoxStream<
            'static,
            crate::providers::trait_def::ProviderResult<crate::providers::types::ChatChunk>,
        > = Box::pin(stream);

        let (content, tool_calls, finish_reason) = accumulate_stream(boxed).await;
        assert!(content.is_empty());
        assert!(tool_calls.is_empty());
        assert!(finish_reason.is_none());
    }

    #[tokio::test]
    async fn test_accumulate_stream_text_chunks() {
        use crate::providers::types::{ChatChunk, MessageDelta};

        let chunks = vec![
            Ok(ChatChunk {
                model: "test".to_string(),
                delta: MessageDelta {
                    content: Some("Hello ".to_string()),
                    tool_calls: None,
                    role: None,
                },
                finish_reason: None,
                usage: None,
                index: None,
            }),
            Ok(ChatChunk {
                model: "test".to_string(),
                delta: MessageDelta {
                    content: Some("World!".to_string()),
                    tool_calls: None,
                    role: None,
                },
                finish_reason: Some(FinishReason::Stop),
                usage: None,
                index: None,
            }),
        ];

        let stream = futures::stream::iter(chunks);
        let boxed: futures::stream::BoxStream<
            'static,
            crate::providers::trait_def::ProviderResult<crate::providers::types::ChatChunk>,
        > = Box::pin(stream);

        let (content, tool_calls, finish_reason) = accumulate_stream(boxed).await;
        assert_eq!(content, "Hello World!");
        assert!(tool_calls.is_empty());
        assert_eq!(finish_reason, Some(FinishReason::Stop));
    }
}
