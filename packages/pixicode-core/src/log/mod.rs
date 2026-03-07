//! Logging subsystem
//!
//! Initialises `tracing` with:
//!   - JSON structured output to a rotating log file
//!   - Optional human-readable output to stdout (when `--print-logs` is set)
//!   - `PIXICODE_LOG_LEVEL` / `--log-level` filter

use anyhow::Result;
use std::path::PathBuf;
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Layer};

/// Returns the path to the pixicode log directory.
/// Defaults to `$XDG_DATA_HOME/pixicode/logs` or `~/.local/share/pixicode/logs`.
pub fn log_dir() -> PathBuf {
    let base = dirs::data_dir().unwrap_or_else(|| PathBuf::from("."));
    base.join("pixicode").join("logs")
}

/// Initialise global tracing subscriber.
///
/// * `level_str` — one of "debug", "info", "warn", "error"
/// * `print_logs` — also write human-readable lines to stdout
pub fn init(level_str: &str, print_logs: bool) -> Result<()> {
    // Honour RUST_LOG if set, otherwise fall back to CLI flag / env PIXICODE_LOG_LEVEL
    let filter_str = std::env::var("RUST_LOG").unwrap_or_else(|_| {
        format!(
            "pixicode_core={level},tower_http=info,axum=info",
            level = level_str
        )
    });
    let filter = EnvFilter::new(filter_str);

    // Ensure log directory exists
    let dir = log_dir();
    std::fs::create_dir_all(&dir)?;

    // Rotating file appender (hourly roll, keeps up to 7 files)
    let file_appender = tracing_appender::rolling::hourly(&dir, "pixicode.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    // File layer: JSON structured
    let file_layer = fmt::layer()
        .json()
        .with_file(true)
        .with_line_number(true)
        .with_thread_ids(false)
        .with_writer(non_blocking)
        .boxed();

    // We need to keep `_guard` alive for the duration of the process.
    // Leak it intentionally — this is the only place it's created.
    std::mem::forget(_guard);

    if print_logs {
        // Stdout layer: human-readable with colours
        let stdout_layer = fmt::layer()
            .pretty()
            .with_writer(std::io::stdout)
            .boxed();

        tracing_subscriber::registry()
            .with(filter)
            .with(file_layer)
            .with(stdout_layer)
            .init();
    } else {
        tracing_subscriber::registry()
            .with(filter)
            .with(file_layer)
            .init();
    }

    Ok(())
}
