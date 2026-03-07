//! LSP JSON-RPC client over stdio (Content-Length delimited messages).

use anyhow::{Context, Result};
use serde_json::Value;
use std::io::{BufRead, BufReader, Read, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};

/// JSON-RPC client talking to a language server over stdio.
pub struct LspStdioClient {
    _child: Child,
    stdin: ChildStdin,
    reader: BufReader<ChildStdout>,
    next_id: AtomicU64,
}

impl LspStdioClient {
    /// Spawn the server process and return a client. Caller should send initialize next.
    pub fn spawn(cmd: &str, args: &[&str], _root_uri: &str) -> Result<Self> {
        let mut child = Command::new(cmd)
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .with_context(|| format!("spawn LSP: {}", cmd))?;
        let stdin = child.stdin.take().context("no stdin")?;
        let stdout = child.stdout.take().context("no stdout")?;
        Ok(Self {
            _child: child,
            stdin,
            reader: BufReader::new(stdout),
            next_id: AtomicU64::new(1),
        })
    }

    /// Send initialize then initialized notification. Call after spawn before other requests.
    pub fn initialize(&mut self, root_uri: &str) -> Result<Value> {
        let id = self.next_id();
        let params = serde_json::json!({
            "processId": std::process::id(),
            "rootUri": root_uri,
            "capabilities": {},
            "clientInfo": { "name": "pixicode", "version": "0.1.0" }
        });
        self.request(id, "initialize", params)?;
        // Send initialized notification (no id)
        self.notify("initialized", serde_json::json!({}))?;
        Ok(serde_json::Value::Null)
    }

    fn next_id(&self) -> u64 {
        self.next_id.fetch_add(1, Ordering::Relaxed)
    }

    fn write_message(&mut self, body: &[u8]) -> Result<()> {
        let header = format!("Content-Length: {}\r\n\r\n", body.len());
        self.stdin.write_all(header.as_bytes())?;
        self.stdin.write_all(body)?;
        self.stdin.flush()?;
        Ok(())
    }

    fn read_message(&mut self) -> Result<Vec<u8>> {
        let mut line = String::new();
        self.reader.read_line(&mut line)?;
        let header = line.trim_end();
        if !header.starts_with("Content-Length:") {
            return Err(anyhow::anyhow!("expected Content-Length, got {}", header));
        }
        let n: usize = header
            .strip_prefix("Content-Length:")
            .and_then(|s| s.trim().parse().ok())
            .context("parse Content-Length")?;
        let mut blank = String::new();
        self.reader.read_line(&mut blank)?;
        let mut buf = vec![0u8; n];
        self.reader.read_exact(&mut buf)?;
        Ok(buf)
    }

    /// Send a request and return the result. Skips notifications until response with matching id.
    pub fn request(&mut self, id: u64, method: &str, params: Value) -> Result<Value> {
        let msg = serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params
        });
        let body = serde_json::to_vec(&msg)?;
        self.write_message(&body)?;
        loop {
            let buf = self.read_message()?;
            let resp: Value = serde_json::from_slice(&buf)?;
            if resp.get("id").and_then(|v| v.as_u64()) == Some(id) {
                if let Some(err) = resp.get("error") {
                    let code = err.get("code").and_then(|c| c.as_i64()).unwrap_or(0);
                    let message = err.get("message").and_then(|m| m.as_str()).unwrap_or("unknown");
                    return Err(anyhow::anyhow!("LSP error {}: {}", code, message));
                }
                return Ok(resp.get("result").cloned().unwrap_or(serde_json::Value::Null));
            }
            // Notification or response for other id; skip (could buffer for future)
        }
    }

    fn notify(&mut self, method: &str, params: Value) -> Result<()> {
        let msg = serde_json::json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params
        });
        let body = serde_json::to_vec(&msg)?;
        self.write_message(&body)?;
        Ok(())
    }

    /// Convenience: request with auto-generated id.
    pub fn call(&mut self, method: &str, params: Value) -> Result<Value> {
        let id = self.next_id();
        self.request(id, method, params)
    }
}
