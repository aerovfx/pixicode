//! MCP transport — stdio (newline-delimited JSON-RPC)

use crate::mcp::types::{JsonRpcRequest, JsonRpcResponse};
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, ChildStdout, Stdio};
use std::sync::Mutex;

/// Stdio transport for MCP client: spawns a process and communicates via stdin/stdout.
pub struct StdioClientTransport {
    child: Option<Child>,
    stdin: Mutex<Option<ChildStdin>>,
    stdout: Mutex<Option<BufReader<ChildStdout>>>,
}

impl StdioClientTransport {
    /// Spawn server process. `command` is the executable, `args` are arguments.
    pub fn spawn(command: &str, args: &[String]) -> Result<Self, String> {
        let mut child = std::process::Command::new(command)
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .map_err(|e| e.to_string())?;
        let stdin = child.stdin.take().ok_or("failed to take stdin")?;
        let stdout = child.stdout.take().ok_or("failed to take stdout")?;
        Ok(Self {
            child: Some(child),
            stdin: Mutex::new(Some(stdin)),
            stdout: Mutex::new(Some(BufReader::new(stdout))),
        })
    }

    /// Send a JSON-RPC request and read the response (blocking).
    pub fn request(&self, req: &JsonRpcRequest) -> Result<JsonRpcResponse, String> {
        let line = serde_json::to_string(req).map_err(|e| e.to_string())?;
        let mut stdin = self.stdin.lock().map_err(|e| e.to_string())?;
        let stdin = stdin.as_mut().ok_or("transport closed")?;
        stdin.write_all(line.as_bytes()).map_err(|e| e.to_string())?;
        stdin.write_all(b"\n").map_err(|e| e.to_string())?;
        stdin.flush().map_err(|e| e.to_string())?;

        let mut stdout = self.stdout.lock().map_err(|e| e.to_string())?;
        let stdout = stdout.as_mut().ok_or("transport closed")?;
        let mut buf = String::new();
        stdout.read_line(&mut buf).map_err(|e| e.to_string())?;
        let buf = buf.trim_end_matches('\n').trim_end_matches('\r');
        serde_json::from_str(buf).map_err(|e| e.to_string())
    }
}

/// Stdio transport for MCP server: read from stdin, write to stdout (current process).
pub struct StdioServerTransport;

impl StdioServerTransport {
    pub fn new() -> Self {
        Self
    }

    /// Read one JSON-RPC message from stdin (blocking).
    pub fn read_request(&self) -> Result<Option<JsonRpcRequest>, String> {
        let stdin = std::io::stdin();
        let mut reader = BufReader::new(stdin);
        let mut line = String::new();
        let n = reader.read_line(&mut line).map_err(|e| e.to_string())?;
        if n == 0 {
            return Ok(None);
        }
        let line = line.trim_end_matches('\n').trim_end_matches('\r');
        let req: JsonRpcRequest = serde_json::from_str(line).map_err(|e| e.to_string())?;
        Ok(Some(req))
    }

    /// Write one JSON-RPC response to stdout.
    pub fn write_response(&self, res: &JsonRpcResponse) -> Result<(), String> {
        let line = serde_json::to_string(res).map_err(|e| e.to_string())?;
        let mut stdout = std::io::stdout();
        stdout.write_all(line.as_bytes()).map_err(|e| e.to_string())?;
        stdout.write_all(b"\n").map_err(|e| e.to_string())?;
        stdout.flush().map_err(|e| e.to_string())?;
        Ok(())
    }
}

impl Default for StdioServerTransport {
    fn default() -> Self {
        Self::new()
    }
}
