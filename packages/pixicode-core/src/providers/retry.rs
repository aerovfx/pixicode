//! Retry logic with exponential backoff for provider HTTP requests.
//!
//! Retries on retryable errors: network, 429 (rate limit), 5xx.

use std::future::Future;
use std::time::Duration;
use crate::providers::trait_def::{ProviderError, ProviderResult};

const MAX_RETRIES: u32 = 3;
const INITIAL_DELAY_MS: u64 = 500;
const MAX_DELAY_MS: u64 = 10_000;

/// Returns true if the error is retryable (network, rate limit, server error).
pub fn is_retryable(err: &ProviderError) -> bool {
    use crate::providers::trait_def::ProviderError::*;
    match err {
        Network(_) => true,
        RateLimit(_) => true,
        InvalidRequest(s) if s.contains("429") || s.contains("rate") => true,
        Internal(s) if s.contains("5") || s.contains("timeout") => true,
        Timeout(_) => true,
        _ => false,
    }
}

/// Execute an async operation with exponential backoff retries.
pub async fn execute_async<F, Fut, T>(mut f: F) -> ProviderResult<T>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = ProviderResult<T>>,
{
    let mut attempt = 0u32;
    loop {
        match f().await {
            Ok(t) => return Ok(t),
            Err(e) if attempt < MAX_RETRIES && is_retryable(&e) => {
                attempt += 1;
                let delay_ms = (INITIAL_DELAY_MS * 2u64.pow(attempt - 1)).min(MAX_DELAY_MS);
                tracing::warn!(attempt, delay_ms, error = %e, "retrying after error");
                tokio::time::sleep(Duration::from_millis(delay_ms)).await;
            }
            Err(e) => return Err(e),
        }
    }
}
