// MLX sidecar subprocess management.
// Traces to: FR-INF-001, FR-INF-004

use crate::error::MlxError;
use crate::protocol::*;
use serde_json::json;
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::{mpsc, oneshot, Mutex, RwLock};
use tracing::{error, info, warn};
use uuid::Uuid;

pub struct MlxSidecarConfig {
    /// Path to Python interpreter (or "python" if in PATH).
    pub python: PathBuf,

    /// Optional venv path for uv run.
    pub venv: Option<PathBuf>,

    /// Python module to run (default: "omlx.__main_hwledger__").
    pub omlx_module: String,

    /// Working directory for subprocess.
    pub cwd: Option<PathBuf>,

    /// Extra environment variables.
    pub env: Vec<(String, String)>,
}

impl Default for MlxSidecarConfig {
    fn default() -> Self {
        MlxSidecarConfig {
            python: PathBuf::from("python3"),
            venv: None,
            omlx_module: "omlx.__main_hwledger__".to_string(),
            cwd: None,
            env: vec![],
        }
    }
}

/// Pending request waiting for a response.
#[derive(Debug)]
struct PendingRequest {
    tx: oneshot::Sender<serde_json::Value>,
}

/// Manager for MLX sidecar subprocess.
#[derive(Debug)]
pub struct MlxSidecar {
    child: Arc<Mutex<Option<Child>>>,
    stdin_tx: mpsc::UnboundedSender<String>,
    pending_requests: Arc<RwLock<HashMap<u64, PendingRequest>>>,
    next_id: Arc<Mutex<u64>>,
    token_listeners: Arc<RwLock<HashMap<String, mpsc::UnboundedSender<TokenEvent>>>>,
}

impl MlxSidecar {
    /// Spawn an MLX sidecar process.
    pub async fn spawn(config: MlxSidecarConfig) -> Result<Self, MlxError> {
        // Build command
        let (cmd_name, cmd_args) = if let Some(venv) = config.venv {
            (
                "uv".to_string(),
                vec![
                    "run".to_string(),
                    "--project".to_string(),
                    venv.to_string_lossy().to_string(),
                    "python".to_string(),
                    "-m".to_string(),
                    config.omlx_module.clone(),
                ],
            )
        } else {
            (
                config.python.to_string_lossy().to_string(),
                vec!["-m".to_string(), config.omlx_module.clone()],
            )
        };

        let mut cmd = Command::new(&cmd_name);
        for arg in cmd_args {
            cmd.arg(arg);
        }

        if let Some(cwd) = config.cwd {
            cmd.current_dir(cwd);
        }

        for (k, v) in config.env {
            cmd.env(k, v);
        }

        cmd.stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped());

        let mut child = cmd.spawn().map_err(MlxError::spawn_io)?;

        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| MlxError::Protocol { reason: "Failed to capture stdin".to_string() })?;

        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| MlxError::Protocol { reason: "Failed to capture stdout".to_string() })?;

        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| MlxError::Protocol { reason: "Failed to capture stderr".to_string() })?;

        // Shared state
        let pending_requests = Arc::new(RwLock::new(HashMap::new()));
        let token_listeners = Arc::new(RwLock::new(HashMap::new()));
        let child_arc = Arc::new(Mutex::new(Some(child)));

        // Channel for stdin writes
        let (stdin_tx, mut stdin_rx) = mpsc::unbounded_channel::<String>();

        // Spawn stdin writer task
        let _child_arc_stdin = child_arc.clone();
        tokio::spawn(async move {
            let mut stdin = stdin;
            while let Some(msg) = stdin_rx.recv().await {
                if let Err(e) = stdin.write_all(msg.as_bytes()).await {
                    error!("Error writing to sidecar stdin: {}", e);
                    break;
                }
                if let Err(e) = stdin.flush().await {
                    error!("Error flushing sidecar stdin: {}", e);
                    break;
                }
            }
        });

        // Spawn stdout reader task
        let pending_requests_read = pending_requests.clone();
        let token_listeners_read = token_listeners.clone();
        tokio::spawn(async move {
            let reader = BufReader::new(stdout);
            let mut lines = reader.lines();

            while let Ok(Some(line)) = lines.next_line().await {
                if let Err(e) =
                    Self::dispatch_message(&line, &pending_requests_read, &token_listeners_read)
                        .await
                {
                    warn!("Error dispatching message: {}", e);
                }
            }
        });

        // Spawn stderr logger task
        tokio::spawn(async move {
            let reader = BufReader::new(stderr);
            let mut lines = reader.lines();
            while let Ok(Some(line)) = lines.next_line().await {
                info!("MLX stderr: {}", line);
            }
        });

        Ok(MlxSidecar {
            child: child_arc,
            stdin_tx,
            pending_requests,
            next_id: Arc::new(Mutex::new(1)),
            token_listeners,
        })
    }

    /// Dispatch a JSON-RPC message from the sidecar.
    async fn dispatch_message(
        line: &str,
        pending_requests: &Arc<RwLock<HashMap<u64, PendingRequest>>>,
        token_listeners: &Arc<RwLock<HashMap<String, mpsc::UnboundedSender<TokenEvent>>>>,
    ) -> Result<(), MlxError> {
        let msg: serde_json::Value = serde_json::from_str(line).map_err(MlxError::json_error)?;

        if let Some(method) = msg.get("method").and_then(|m| m.as_str()) {
            // Handle notifications (streaming tokens)
            if method == "token" {
                let params = msg.get("params").ok_or_else(|| MlxError::Protocol {
                    reason: "Missing params in token notification".to_string(),
                })?;
                let request_id =
                    params.get("request_id").and_then(|r| r.as_str()).ok_or_else(|| {
                        MlxError::Protocol {
                            reason: "Missing request_id in token notification".to_string(),
                        }
                    })?;
                let text = params.get("text").and_then(|t| t.as_str()).unwrap_or("");

                let listeners = token_listeners.read().await;
                if let Some(tx) = listeners.get(request_id) {
                    let _ = tx.send(TokenEvent {
                        request_id: request_id.to_string(),
                        text: text.to_string(),
                    });
                }
            }
        } else if let Some(id) = msg.get("id") {
            // Handle responses (results or errors)
            let id_val = if let Some(n) = id.as_u64() {
                n
            } else if let Some(s) = id.as_str() {
                s.parse::<u64>().unwrap_or(0)
            } else {
                return Ok(());
            };

            let mut pending = pending_requests.write().await;
            if let Some(req) = pending.remove(&id_val) {
                let _ = req.tx.send(msg);
            }
        }

        Ok(())
    }

    /// Send a JSON-RPC request and await a response.
    async fn send_request(
        &self,
        method: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, MlxError> {
        let id = {
            let mut next_id = self.next_id.lock().await;
            let id = *next_id;
            *next_id += 1;
            id
        };

        let request = json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params,
            "id": id
        });

        let (tx, rx) = oneshot::channel();
        {
            let mut pending = self.pending_requests.write().await;
            pending.insert(id, PendingRequest { tx });
        }

        self.stdin_tx
            .send(format!("{}\n", request))
            .map_err(|e| MlxError::channel_error(e.to_string()))?;

        let response = tokio::time::timeout(std::time::Duration::from_secs(30), rx)
            .await
            .map_err(|_| MlxError::Timeout)?
            .map_err(|e| MlxError::channel_error(e.to_string()))?;

        // Check for error response
        if let Some(error) = response.get("error") {
            let code = error.get("code").and_then(|c| c.as_i64()).unwrap_or(-32000) as i32;
            let message = error
                .get("message")
                .and_then(|m| m.as_str())
                .unwrap_or("Unknown error")
                .to_string();
            return Err(MlxError::RequestFailed { code, message });
        }

        response
            .get("result")
            .cloned()
            .ok_or_else(|| MlxError::Protocol { reason: "Missing result in response".to_string() })
    }

    /// Generate tokens from a prompt.
    pub async fn generate(
        &self,
        prompt: String,
        model: String,
        max_tokens: u32,
        temperature: f32,
    ) -> Result<TokenStream, MlxError> {
        let request_id = Uuid::new_v4().to_string();

        // Create token listener channel
        let (token_tx, token_rx) = mpsc::unbounded_channel();
        {
            let mut listeners = self.token_listeners.write().await;
            listeners.insert(request_id.clone(), token_tx);
        }

        // Send generate request
        let params = json!({
            "prompt": prompt,
            "model": model,
            "max_tokens": max_tokens,
            "temperature": temperature,
            "stream": true,
            "request_id": request_id.clone(),
        });

        // Send the request (will complete asynchronously)
        let _result = self.send_request("generate", params).await;

        Ok(TokenStream {
            request_id,
            token_rx,
            token_listeners: self.token_listeners.clone(),
            generation_complete: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
        })
    }

    /// Cancel a running generation.
    pub async fn cancel(&self, request_id: String) -> Result<(), MlxError> {
        let params = json!({"request_id": request_id.clone()});
        let _result = self.send_request("cancel", params).await?;

        // Clean up listener
        let mut listeners = self.token_listeners.write().await;
        listeners.remove(&request_id);

        Ok(())
    }

    /// Load a model into the sidecar.
    pub async fn load_model(
        &self,
        model: String,
        max_kv_size: u64,
    ) -> Result<LoadResult, MlxError> {
        let params = json!({
            "model": model,
            "max_kv_size": max_kv_size,
        });

        let result = self.send_request("load_model", params).await?;
        serde_json::from_value(result).map_err(MlxError::json_error)
    }

    /// Unload a model from the sidecar.
    pub async fn unload_model(&self, model: String) -> Result<UnloadResult, MlxError> {
        let params = json!({"model": model});
        let result = self.send_request("unload_model", params).await?;
        serde_json::from_value(result).map_err(MlxError::json_error)
    }

    /// Get memory report from the sidecar.
    pub async fn memory_report(&self) -> Result<MemoryReport, MlxError> {
        let result = self.send_request("memory_report", json!({})).await?;
        serde_json::from_value(result).map_err(MlxError::json_error)
    }

    /// Get health status from the sidecar.
    pub async fn health(&self) -> Result<HealthReport, MlxError> {
        let result = self.send_request("health", json!({})).await?;
        serde_json::from_value(result).map_err(MlxError::json_error)
    }

    /// Gracefully shut down the sidecar.
    pub async fn shutdown(self) -> Result<(), MlxError> {
        // Try SIGTERM first
        if let Some(mut child) = self.child.lock().await.take() {
            let _ = child.kill().await.ok();
            let _ = child.wait().await.ok();
        }
        Ok(())
    }
}

/// Stream of tokens from a generation request.
pub struct TokenStream {
    pub request_id: String,
    token_rx: mpsc::UnboundedReceiver<TokenEvent>,
    token_listeners: Arc<RwLock<HashMap<String, mpsc::UnboundedSender<TokenEvent>>>>,
    generation_complete: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

impl TokenStream {
    /// Get the next token event.
    pub async fn next_token(&mut self) -> Option<Result<String, MlxError>> {
        if self.generation_complete.load(std::sync::atomic::Ordering::Relaxed) {
            return None;
        }

        match tokio::time::timeout(std::time::Duration::from_secs(60), self.token_rx.recv()).await {
            Ok(Some(event)) => Some(Ok(event.text)),
            Ok(None) => None,
            Err(_) => Some(Err(MlxError::Timeout)),
        }
    }

    /// Cancel this generation.
    pub async fn cancel(&self) -> Result<(), MlxError> {
        // Clean up the listener
        let mut listeners = self.token_listeners.write().await;
        listeners.remove(&self.request_id);
        self.generation_complete.store(true, std::sync::atomic::Ordering::Relaxed);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Traces to: FR-INF-001, FR-INF-002, FR-INF-004
    #[tokio::test]
    async fn test_sidecar_config_default() {
        let config = MlxSidecarConfig::default();
        assert_eq!(config.python.to_string_lossy(), "python3");
        assert_eq!(config.omlx_module, "omlx.__main_hwledger__");
    }
}
