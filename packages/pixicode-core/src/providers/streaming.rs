//! Streaming support — SSE parser and stream utilities

use bytes::Bytes;
use futures::stream::Stream;
use serde::Deserialize;
use std::pin::Pin;
use std::task::{Context, Poll};
use pin_project_lite::pin_project;

use crate::providers::types::{ChatChunk, MessageDelta, MessageRole};
use crate::providers::trait_def::{ProviderError, ProviderResult};

/// Wraps a stream so that when the cancellation token is triggered, the stream ends.
/// Returns a boxed stream (Unpin) so it works with async_stream producers that are !Unpin.
pub fn stream_with_cancel<S>(
    inner: S,
    token: std::sync::Arc<tokio_util::sync::CancellationToken>,
) -> futures::stream::BoxStream<'static, S::Item>
where
    S: Stream + Send + 'static,
    S::Item: Send,
{
    let token = token.clone();
    let stream = async_stream::stream! {
        let mut inner = std::pin::pin!(inner);
        loop {
            tokio::select! {
                _ = token.cancelled() => break,
                next = futures::stream::StreamExt::next(&mut inner) => {
                    match next {
                        Some(item) => yield item,
                        None => break,
                    }
                }
            }
        }
    };
    Box::pin(stream)
}

pin_project! {
    /// SSE stream parser for chat completions.
    pub struct SseStream<S> {
        #[pin]
        inner: S,
        buffer: String,
        done: bool,
    }
}

impl<S> SseStream<S>
where
    S: futures::stream::Stream<Item = Result<Bytes, ProviderError>>,
{
    pub fn new(inner: S) -> Self {
        Self {
            inner,
            buffer: String::new(),
            done: false,
        }
    }
}

impl<S> futures::stream::Stream for SseStream<S>
where
    S: futures::stream::Stream<Item = Result<Bytes, ProviderError>>,
{
    type Item = ProviderResult<String>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut this = self.project();

        loop {
            if *this.done {
                return Poll::Ready(None);
            }

            // Poll inner stream
            match this.inner.as_mut().poll_next(cx) {
                Poll::Ready(Some(Ok(chunk))) => {
                    let chunk_str = String::from_utf8_lossy(&chunk);
                    *this.buffer += &chunk_str;

                    // Try to extract SSE events
                    if let Some(pos) = this.buffer.find("\n\n") {
                        let event = this.buffer[..pos].to_string();
                        *this.buffer = this.buffer[pos + 2..].to_string();

                        // Parse SSE event
                        if let Some(data) = parse_sse_event(&event) {
                            if data == "[DONE]" {
                                *this.done = true;
                                return Poll::Ready(None);
                            }
                            return Poll::Ready(Some(Ok(data)));
                        }
                    }
                }
                Poll::Ready(Some(Err(e))) => {
                    return Poll::Ready(Some(Err(e)));
                }
                Poll::Ready(None) => {
                    *this.done = true;
                    // Process any remaining buffer
                    if !this.buffer.is_empty() {
                        if let Some(data) = parse_sse_event(&this.buffer) {
                            if data != "[DONE]" {
                                return Poll::Ready(Some(Ok(data)));
                            }
                        }
                    }
                    return Poll::Ready(None);
                }
                Poll::Pending => {
                    return Poll::Pending;
                }
            }
        }
    }
}

/// Parse an SSE event and extract the data field.
fn parse_sse_event(event: &str) -> Option<String> {
    for line in event.lines() {
        let line = line.trim();
        if line.starts_with("data:") {
            return Some(line[5..].trim().to_string());
        }
        // Handle case where "data:" is on one line and content on next
        if line == "data:" {
            // Look for content on next line (already in buffer)
            continue;
        }
    }
    None
}

/// Parse a JSON chunk from SSE data.
pub fn parse_chunk(data: &str) -> ProviderResult<Option<ChatChunk>> {
    if data.trim().is_empty() || data == "[DONE]" {
        return Ok(None);
    }

    // Try to parse as generic JSON first
    let value: serde_json::Value = serde_json::from_str(data)
        .map_err(|e| ProviderError::Json(e))?;

    // Try different provider formats
    Ok(parse_openai_chunk(&value)
        .or_else(|| parse_anthropic_chunk(&value))
        .or_else(|| parse_generic_chunk(&value)))
}

/// Parse OpenAI-style chunk.
fn parse_openai_chunk(value: &serde_json::Value) -> Option<ChatChunk> {
    let choices = value.get("choices")?.as_array()?;
    let first_choice = choices.first()?;

    let delta = first_choice.get("delta")?;
    let content = delta.get("content").and_then(|c| c.as_str()).map(String::from);
    let role = delta.get("role").and_then(|r| r.as_str()).and_then(|r| match r {
        "system" => Some(MessageRole::System),
        "user" => Some(MessageRole::User),
        "assistant" => Some(MessageRole::Assistant),
        _ => None,
    });

    let finish_reason = first_choice.get("finish_reason").and_then(|fr| fr.as_str()).and_then(|fr| match fr {
        "stop" => Some(crate::providers::types::FinishReason::Stop),
        "length" => Some(crate::providers::types::FinishReason::Length),
        "tool_calls" | "function_call" => Some(crate::providers::types::FinishReason::ToolCalls),
        "content_filter" => Some(crate::providers::types::FinishReason::ContentFilter),
        _ => Some(crate::providers::types::FinishReason::Other),
    });

    let usage = value.get("usage").and_then(|u| {
        Some(crate::providers::types::Usage {
            input_tokens: u.get("prompt_tokens")?.as_u64()? as u32,
            output_tokens: u.get("completion_tokens")?.as_u64()? as u32,
            total_tokens: u.get("total_tokens")?.as_u64()? as u32,
            input_token_details: None,
        })
    });

    let tool_calls = delta.get("tool_calls").and_then(|arr| arr.as_array()).map(|arr| {
        arr.iter().filter_map(|t| {
            let index = t.get("index")?.as_u64()? as u32;
            let id = t.get("id").and_then(|v| v.as_str()).map(String::from);
            let name = t.get("function").and_then(|f| f.get("name")).and_then(|v| v.as_str()).map(String::from);
            let arguments = t.get("function").and_then(|f| f.get("arguments")).and_then(|v| v.as_str()).map(String::from);
            Some(crate::providers::types::ToolCallDelta {
                index,
                id,
                name,
                arguments,
            })
        }).collect::<Vec<_>>()
    }).filter(|v| !v.is_empty());

    Some(ChatChunk {
        model: value.get("model").and_then(|m| m.as_str()).unwrap_or("unknown").to_string(),
        delta: MessageDelta {
            role,
            content,
            tool_calls,
        },
        finish_reason,
        usage,
        index: first_choice.get("index").and_then(|i| i.as_u64()).map(|i| i as u32),
    })
}

/// Parse Anthropic-style chunk.
fn parse_anthropic_chunk(value: &serde_json::Value) -> Option<ChatChunk> {
    let event_type = value.get("type")?.as_str()?;

    match event_type {
        "content_block_delta" => {
            let delta = value.get("delta")?;
            let content = delta.get("text").and_then(|t| t.as_str()).map(String::from);

            Some(ChatChunk {
                model: "claude".to_string(),
                delta: MessageDelta {
                    role: Some(MessageRole::Assistant),
                    content,
                    tool_calls: None,
                },
                finish_reason: None,
                usage: None,
                index: Some(0),
            })
        }
        "message_delta" => {
            let delta = value.get("delta")?;
            let stop_reason = delta.get("stop_reason").and_then(|s| s.as_str());
            let finish_reason = stop_reason.and_then(|s| match s {
                "end_turn" | "stop_sequence" => Some(crate::providers::types::FinishReason::Stop),
                "max_tokens" => Some(crate::providers::types::FinishReason::Length),
                "tool_use" => Some(crate::providers::types::FinishReason::ToolCalls),
                _ => Some(crate::providers::types::FinishReason::Other),
            });

            let usage = value.get("usage").and_then(|u| {
                Some(crate::providers::types::Usage {
                    input_tokens: u.get("input_tokens")?.as_u64()? as u32,
                    output_tokens: u.get("output_tokens")?.as_u64()? as u32,
                    total_tokens: (u.get("input_tokens")?.as_u64()? + u.get("output_tokens")?.as_u64()?) as u32,
                    input_token_details: None,
                })
            });

            Some(ChatChunk {
                model: "claude".to_string(),
                delta: MessageDelta::default(),
                finish_reason,
                usage,
                index: None,
            })
        }
        _ => None,
    }
}

/// Parse generic chunk (fallback).
fn parse_generic_chunk(value: &serde_json::Value) -> Option<ChatChunk> {
    let content = value.get("content")
        .or_else(|| value.get("text"))
        .and_then(|c| c.as_str())
        .map(String::from);

    let finish_reason = value.get("finish_reason").and_then(|fr| fr.as_str()).and_then(|fr| match fr {
        "stop" => Some(crate::providers::types::FinishReason::Stop),
        "length" => Some(crate::providers::types::FinishReason::Length),
        _ => Some(crate::providers::types::FinishReason::Other),
    });

    Some(ChatChunk {
        model: "unknown".to_string(),
        delta: MessageDelta {
            role: Some(MessageRole::Assistant),
            content,
            tool_calls: None,
        },
        finish_reason,
        usage: None,
        index: None,
    })
}

/// Accumulator for building complete response from chunks.
#[derive(Debug, Default)]
pub struct ChunkAccumulator {
    content: String,
    role: Option<MessageRole>,
    finish_reason: Option<crate::providers::types::FinishReason>,
    usage: Option<crate::providers::types::Usage>,
    model: Option<String>,
}

impl ChunkAccumulator {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn accumulate(&mut self, chunk: ChatChunk) {
        if let Some(role) = chunk.delta.role {
            self.role = Some(role);
        }

        if let Some(content) = chunk.delta.content {
            self.content.push_str(&content);
        }

        if chunk.finish_reason.is_some() {
            self.finish_reason = chunk.finish_reason;
        }

        if chunk.usage.is_some() {
            self.usage = chunk.usage;
        }

        if chunk.model != "unknown" {
            self.model = Some(chunk.model);
        }
    }

    pub fn finish(self) -> (String, Option<MessageRole>, Option<crate::providers::types::FinishReason>, Option<crate::providers::types::Usage>) {
        (self.content, self.role, self.finish_reason, self.usage)
    }
}

/// Partial JSON parser for incremental tool call parsing.
pub struct PartialJsonParser {
    buffer: String,
}

impl PartialJsonParser {
    pub fn new() -> Self {
        Self { buffer: String::new() }
    }

    pub fn push(&mut self, chunk: &str) {
        self.buffer.push_str(chunk);
    }

    pub fn try_parse<T: for<'de> Deserialize<'de>>(&self) -> Option<T> {
        serde_json::from_str(&self.buffer).ok()
    }

    pub fn clear(&mut self) {
        self.buffer.clear();
    }
}

impl Default for PartialJsonParser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_sse_event() {
        let event = "data: {\"choices\": [{\"delta\": {\"content\": \"Hello\"}}]}\n";
        let data = parse_sse_event(event);
        assert!(data.is_some());
        assert!(data.unwrap().contains("Hello"));
    }

    #[test]
    fn test_parse_openai_chunk() {
        let json = r#"{
            "model": "gpt-4",
            "choices": [{
                "index": 0,
                "delta": {"role": "assistant", "content": "Hello"},
                "finish_reason": null
            }]
        }"#;
        let value: serde_json::Value = serde_json::from_str(json).unwrap();
        let chunk = parse_openai_chunk(&value);
        assert!(chunk.is_some());
        let chunk = chunk.unwrap();
        assert_eq!(chunk.model, "gpt-4");
        assert_eq!(chunk.delta.content, Some("Hello".to_string()));
        assert_eq!(chunk.delta.role, Some(MessageRole::Assistant));
    }

    #[test]
    fn test_accumulator() {
        let mut acc = ChunkAccumulator::new();
        
        acc.accumulate(ChatChunk {
            model: "test".to_string(),
            delta: MessageDelta {
                role: Some(MessageRole::Assistant),
                content: Some("Hello ".to_string()),
                tool_calls: None,
            },
            finish_reason: None,
            usage: None,
            index: None,
        });

        acc.accumulate(ChatChunk {
            model: "test".to_string(),
            delta: MessageDelta {
                role: None,
                content: Some("World".to_string()),
                tool_calls: None,
            },
            finish_reason: Some(crate::providers::types::FinishReason::Stop),
            usage: None,
            index: None,
        });

        let (content, role, finish_reason, _) = acc.finish();
        assert_eq!(content, "Hello World");
        assert_eq!(role, Some(MessageRole::Assistant));
        assert_eq!(finish_reason, Some(crate::providers::types::FinishReason::Stop));
    }
}
