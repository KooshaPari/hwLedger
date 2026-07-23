//! `hwledger-mcp` — minimal MCP 2024-11-05 server over JSON-RPC 2.0.
//!
//! This crate implements just enough of the Model Context Protocol
//! specification to be a useful tool provider for an LLM client connected
//! over either stdio (`transport`) or HTTP+SSE (`transport_http`). It
//! deliberately bypasses the upstream `rmcp` SDK to avoid a
//! `PointeeSized` compatibility issue with Rust 1.95; the whole transport
//! layer is ~200 lines of straightforward `serde_json` + `std::io`.
//!
//! Layering:
//!
//! * [`error::McpError`]  — typed errors with JSON-RPC 2.0 error codes.
//! * [`backend::Backend`] — trait abstracting the search index; the real
//!   implementation ([`backend::ServiceBackend`])
//!   delegates to `hwledger_server::service::*`,
//!   and the in-test [`backend::MockBackend`]
//!   records calls without a live tantivy index.
//! * [`tools`]            — six tool implementations, each one a thin
//!   validator that forwards to the [`Backend`].
//! * `McpServer` / `McpState` (this module) — request validation +
//!   dispatch.
//! * [`transport`]        — stdio read/write loop and per-message handler.
//! * [`transport_http`]   — axum router exposing POST `/mcp` (per-request
//!   JSON-RPC) and GET `/sse` (Server-Sent Events for streaming
//!   notifications), reusing the same `McpServer::dispatch` core.
//!
//! Round-trip flow for a single request:
//!
//! 1. Client sends one JSON-RPC 2.0 request (stdin line or POST `/mcp`
//!    body).
//! 2. The transport layer parses it, validates the envelope, and
//!    dispatches to the matching method on `McpServer`.
//! 3. `McpServer` returns either a `result` value or an `error` object.
//! 4. The response (if any) is serialised and written back (stdout or
//!    HTTP response body).
#![deny(missing_docs)]
#![deny(rust_2018_idioms)]

pub mod backend;
pub mod error;
pub mod tools;
pub mod transport;
pub mod transport_http;

use std::sync::Arc;

use serde_json::{json, Value};

use crate::backend::Backend;
use crate::error::McpError;

/// Server-wide state that mutates over the lifetime of a connection.
///
/// `McpState` is the single object the transport, server, and tool layers
/// all share. It carries:
///
/// * the MCP protocol-version + `client_info` captured during
///   `initialize`;
/// * a `Backend` implementation — either the real
///   [`backend::ServiceBackend`] in production, or a
///   [`backend::MockBackend`] in tests.
pub struct McpState {
    /// The tool backend (`model_search`, `model_detail`, ...).
    pub backend: Arc<dyn Backend>,
    /// Set to `true` once the client has completed the `initialize` /
    /// `notifications/initialized` handshake.
    pub initialized: bool,
    /// Client identity fields captured during `initialize`, kept verbatim
    /// so the server can echo them back in logs/diagnostics.
    pub client_info: Option<Value>,
    /// Protocol version the client claimed in `initialize`.
    pub protocol_version: Option<String>,
}

impl std::fmt::Debug for McpState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("McpState")
            .field("backend", &"<dyn Backend>")
            .field("initialized", &self.initialized)
            .field("client_info", &self.client_info)
            .field("protocol_version", &self.protocol_version)
            .finish()
    }
}

impl McpState {
    /// Construct a fresh, uninitialized state bound to `backend`.
    pub fn new(backend: Arc<dyn Backend>) -> Self {
        Self {
            backend,
            initialized: false,
            client_info: None,
            protocol_version: None,
        }
    }
}

/// The MCP server itself.
///
/// `McpServer` is intentionally cheap and stateless — it holds no I/O,
/// no I/O handles, and no caches. All per-connection state lives in
/// `McpState`, which is passed mutably into each `handle_*` call.
#[derive(Debug, Default, Clone, Copy)]
pub struct McpServer;

impl McpServer {
    /// Construct a new server handle.
    pub fn new() -> Self {
        Self
    }

    /// Handle the MCP `initialize` request and return the `result` payload
    /// the spec expects (`protocolVersion`, `serverInfo`, `capabilities`).
    ///
    /// Also mutates [`McpState`] to record the client info and protocol
    /// version; the actual `initialized = true` flip happens on receipt of
    /// the `notifications/initialized` notification.
    pub fn handle_initialize(
        &self,
        state: &mut McpState,
        params: Option<&Value>,
    ) -> Result<Value, McpError> {
        let params = params.unwrap_or(&Value::Null);
        let obj = params
            .as_object()
            .ok_or_else(|| McpError::invalid_params("`initialize` params must be an object"))?;

        state.protocol_version = obj
            .get("protocolVersion")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        state.client_info = obj.get("clientInfo").cloned();

        // MCP 2024-11-05 protocol version we speak.
        Ok(json!({
            "protocolVersion": "2024-11-05",
            "serverInfo": {
                "name": "hwledger-mcp",
                "version": env!("CARGO_PKG_VERSION"),
            },
            "capabilities": {
                "tools": {"listChanged": false}
            }
        }))
    }

    /// Handle the MCP `tools/list` request and return the array of tool
    /// descriptors advertised by this server.
    pub fn handle_tools_list(&self) -> Result<Value, McpError> {
        Ok(json!({ "tools": tools::tool_definitions() }))
    }

    /// Handle the MCP `tools/call` request: dispatch to the named tool and
    /// return its result wrapped in the MCP `content` envelope
    /// (`{ content: [{ type: "text", text: "<json>" }], isError: false }`).
    ///
    /// Per the task spec, tool-level "method not found" / "invalid params"
    /// errors propagate up as JSON-RPC error codes (`-32601` / `-32602`)
    /// rather than being downgraded to a soft `isError: true` payload —
    /// that mirrors how the reference MCP implementations surface
    /// `tools/call` failures at the JSON-RPC layer.
    pub fn handle_tools_call(
        &self,
        state: &McpState,
        params: Option<&Value>,
    ) -> Result<Value, McpError> {
        let params = params.unwrap_or(&Value::Null);
        let obj = params
            .as_object()
            .ok_or_else(|| McpError::invalid_params("`tools/call` params must be an object"))?;

        let name = obj
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError::invalid_params("`tools/call` requires a `name` string"))?;

        let args = obj.get("arguments").cloned().unwrap_or_else(|| json!({}));

        let value = tools::call_tool(state.backend.as_ref(), name, &args)?;

        Ok(json!({
            "content": [{
                "type": "text",
                "text": serde_json::to_string(&value)
                    .unwrap_or_else(|_| value.to_string()),
            }],
            "isError": false,
        }))
    }

    /// Handle the `notifications/initialized` notification. Per the spec
    /// this is a notification (no `id`) and we'd return `None`; here we
    /// just flip the state flag.
    pub fn handle_notification_initialized(&self, state: &mut McpState) {
        state.initialized = true;
    }

    /// Top-level dispatch for a single JSON-RPC 2.0 request, returning
    /// either a successful `result` JSON value or an [`McpError`].
    ///
    /// `method` is the JSON-RPC method string; `params` is the optional
    /// `params` field, already validated to be either missing, null, or an
    /// object by the transport layer.
    pub fn dispatch(
        &self,
        state: &mut McpState,
        method: &str,
        params: Option<&Value>,
    ) -> Result<Value, McpError> {
        match method {
            "initialize" => self.handle_initialize(state, params),
            "tools/list" => self.handle_tools_list(),
            "tools/call" => self.handle_tools_call(state, params),
            "notifications/initialized" => {
                self.handle_notification_initialized(state);
                // Notifications don't produce a result; the transport layer
                // is responsible for not sending a response. We return
                // `Null` as a sentinel "nothing to reply with".
                Ok(Value::Null)
            }
            // `notifications/*` are explicitly ignored (not an error).
            other if other.starts_with("notifications/") => Ok(Value::Null),
            other => Err(McpError::method_not_found(other)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::MockBackend;

    #[test]
    fn mcp_state_new_is_uninitialized() {
        let state = McpState::new(Arc::new(MockBackend::new()));
        assert!(!state.initialized);
        assert!(state.client_info.is_none());
        assert!(state.protocol_version.is_none());
    }

    #[test]
    fn mcp_server_new_is_a_zero_sized_handle() {
        let server = McpServer::new();
        // `McpServer` is `Copy` + `Default`; a sanity check that we can
        // still construct it with the new constructor.
        let _ = server;
    }
}
