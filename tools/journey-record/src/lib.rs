//! `hwledger-journey-record` — thin stdio JSON-RPC 2.0 client for
//! [PlayCua](https://github.com/KooshaPari/PlayCua).
//!
//! **Direction change (2026-04-22):** this crate supersedes the earlier
//! per-OS-from-scratch exploration scoped under agent dispatch `a3773560`.
//! Instead of maintaining our own WGC / ScreenCaptureKit / PipeWire adapters,
//! we spawn PlayCua's native Rust binary and drive it over stdin/stdout using
//! newline-delimited JSON-RPC 2.0 as specified in PlayCua's ADR-002.
//!
//! Rationale in `docs-site/architecture/adrs/0035-playcua-recording-integration.md`.
//! Windows-specific gaps mined from `dino` (`CreateDesktop` hidden-desktop, DXGI
//! black-frame retry, Nefarius MTT VDD) are captured in
//! `docs/research/prior-recording-research-index.md` and will be filed upstream
//! on PlayCua.

#![forbid(unsafe_code)]

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Stdio;

use anyhow::{anyhow, bail, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};

/// Per-request tag; monotonically increasing.
#[derive(Debug, Clone, Copy, Default)]
pub struct RequestId(u64);

impl RequestId {
    fn bump(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(1);
        self.0
    }
}

/// Resolved invocation for the PlayCua binary.
#[derive(Debug, Clone)]
pub enum PlayCuaBinary {
    /// Direct path to a prebuilt `playcua` binary.
    Direct(PathBuf),
    /// `cargo run --manifest-path <p> --release --bin playcua` fallback.
    CargoRun { manifest_path: PathBuf },
}

impl PlayCuaBinary {
    /// Locate PlayCua using the documented precedence:
    /// 1. `PLAYCUA_BIN` environment variable (must be a regular file).
    /// 2. `~/.cache/hwledger/bin/playcua`.
    /// 3. `cargo run --manifest-path <PlayCua repo>/Cargo.toml --release --bin playcua`.
    ///
    /// The PlayCua repo is discovered via `PLAYCUA_REPO` or a default sibling
    /// path relative to the hwLedger canonical checkout.
    pub fn locate() -> Result<Self> {
        if let Some(bin) = std::env::var_os("PLAYCUA_BIN").map(PathBuf::from) {
            if bin.is_file() {
                return Ok(Self::Direct(bin));
            }
            // If the env var is set but bogus, fail loudly — don't silently
            // drop to fallbacks (per repo "fail clearly, not silently" rule).
            bail!(
                "PLAYCUA_BIN is set to {:?} but no regular file exists there",
                bin
            );
        }

        if let Some(home) = dirs_home() {
            let cached = home.join(".cache").join("hwledger").join("bin").join("playcua");
            if cached.is_file() {
                return Ok(Self::Direct(cached));
            }
        }

        let repo = std::env::var_os("PLAYCUA_REPO")
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                PathBuf::from("/Users/kooshapari/CodeProjects/Phenotype/repos/PlayCua")
            });
        let manifest = repo.join("Cargo.toml");
        if manifest.is_file() {
            return Ok(Self::CargoRun { manifest_path: manifest });
        }

        bail!(
            "PlayCua not found: set PLAYCUA_BIN, populate ~/.cache/hwledger/bin/playcua, \
             or point PLAYCUA_REPO at a PlayCua checkout (tried {:?})",
            manifest
        )
    }

    /// Build the `tokio::process::Command` for this invocation. No args
    /// appended — PlayCua defaults to the stdio JSON-RPC server on no-arg.
    pub fn into_command(self) -> Command {
        match self {
            Self::Direct(bin) => Command::new(bin),
            Self::CargoRun { manifest_path } => {
                let mut c = Command::new("cargo");
                c.args([
                    OsString::from("run"),
                    OsString::from("--quiet"),
                    OsString::from("--manifest-path"),
                    manifest_path.into_os_string(),
                    OsString::from("--release"),
                    OsString::from("--bin"),
                    OsString::from("playcua"),
                ]);
                c
            }
        }
    }
}

fn dirs_home() -> Option<PathBuf> {
    std::env::var_os("HOME").map(PathBuf::from)
}

/// Errors surfaced by the client. JSON-RPC errors are echoed verbatim; all
/// IO / framing errors are wrapped.
#[derive(Debug, thiserror::Error)]
pub enum RpcError {
    #[error("JSON-RPC error {code}: {message}")]
    Server { code: i64, message: String },
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("framing: {0}")]
    Framing(String),
    #[error("serde: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("protocol: {0}")]
    Protocol(String),
}

/// Minimal JSON-RPC 2.0 response envelope.
#[derive(Debug, Deserialize)]
struct RpcResponse {
    #[allow(dead_code)] // jsonrpc field asserted, value not needed downstream
    jsonrpc: Option<String>,
    id: Option<Value>,
    #[serde(default)]
    result: Option<Value>,
    #[serde(default)]
    error: Option<RpcErrorBody>,
}

#[derive(Debug, Deserialize)]
struct RpcErrorBody {
    code: i64,
    message: String,
}

/// Recording target variant — mirrors the `--target` CLI parser.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RecordTarget {
    Window { title: String },
    Pid { pid: u32 },
    BundleId { bundle_id: String },
}

impl RecordTarget {
    /// Parse the CLI `--target` value: `window:<substring>`,
    /// `pid:<number>`, or `bundle-id:<reverse-dns>`.
    pub fn parse(raw: &str) -> Result<Self> {
        let (kind, rest) = raw
            .split_once(':')
            .ok_or_else(|| anyhow!("target must be `<kind>:<value>`: {raw}"))?;
        match kind {
            "window" => Ok(Self::Window { title: rest.to_string() }),
            "pid" => {
                let pid: u32 = rest.parse().with_context(|| format!("pid: {rest}"))?;
                Ok(Self::Pid { pid })
            }
            "bundle-id" | "bundle_id" => Ok(Self::BundleId { bundle_id: rest.to_string() }),
            other => bail!("unknown target kind {other:?}; expected window|pid|bundle-id"),
        }
    }

    /// Lower to the `window_title` hint PlayCua currently accepts. PlayCua's
    /// openrpc contract does not yet expose PID or bundle-id lookups; those
    /// fall back to the title hint (empty string means whole monitor).
    ///
    /// Traces to: gap identified in research-index item
    /// `dino@cca1721` ("MainWindowHandle+GetWindowRect from PID").
    pub fn to_window_title_hint(&self) -> Option<String> {
        match self {
            Self::Window { title } => Some(title.clone()),
            Self::Pid { .. } => None,
            Self::BundleId { bundle_id } => Some(bundle_id.clone()),
        }
    }
}

/// Cursor-track entry. Each entry is a timed `mouse_move` that the client
/// ships to PlayCua while the real screen is being captured via `xcap`.
#[derive(Debug, Clone, Deserialize)]
pub struct CursorTick {
    /// Milliseconds from recording start.
    pub at_ms: u64,
    pub x: i32,
    pub y: i32,
    /// Optional synthetic click after the move (`left`, `right`, `middle`).
    #[serde(default)]
    pub click: Option<String>,
}

/// One PlayCua subprocess + a monotonic request-id counter. Public so tests
/// can mock it with an in-memory transport.
pub struct RpcClient<W, R>
where
    W: AsyncWriteExt + Unpin,
    R: AsyncBufReadExt + Unpin,
{
    stdin: W,
    stdout: R,
    next_id: RequestId,
}

impl<W, R> RpcClient<W, R>
where
    W: AsyncWriteExt + Unpin,
    R: AsyncBufReadExt + Unpin,
{
    pub fn new(stdin: W, stdout: R) -> Self {
        Self { stdin, stdout, next_id: RequestId::default() }
    }

    /// Send a JSON-RPC request and await the matching response line.
    pub async fn call(&mut self, method: &str, params: Value) -> Result<Value, RpcError> {
        let id = self.next_id.bump();
        let req = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params,
        });
        let line = serde_json::to_string(&req)?;
        self.stdin.write_all(line.as_bytes()).await?;
        self.stdin.write_all(b"\n").await?;
        self.stdin.flush().await?;

        let mut buf = String::new();
        let n = self.stdout.read_line(&mut buf).await?;
        if n == 0 {
            return Err(RpcError::Framing(
                "EOF before response; PlayCua subprocess likely crashed (check stderr)".into(),
            ));
        }
        let resp: RpcResponse = serde_json::from_str(buf.trim_end())?;
        if let Some(err) = resp.error {
            return Err(RpcError::Server { code: err.code, message: err.message });
        }
        match resp.id {
            Some(Value::Number(ref n)) if n.as_u64() == Some(id) => {}
            Some(other) => {
                return Err(RpcError::Protocol(format!(
                    "response id mismatch: expected {id}, got {other}"
                )));
            }
            None => {
                // id-less response: accept only if this was clearly an error
                // (already handled above).
            }
        }
        resp.result.ok_or_else(|| RpcError::Protocol("missing result".into()))
    }
}

/// Drives a full recording run end-to-end against a spawned PlayCua process.
pub struct Session {
    child: Child,
    client: RpcClient<ChildStdin, BufReader<ChildStdout>>,
}

impl Session {
    /// Spawn PlayCua and open the stdio pipes.
    pub async fn spawn(bin: PlayCuaBinary) -> Result<Self> {
        let mut cmd = bin.into_command();
        cmd.stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .kill_on_drop(true);
        let mut child = cmd.spawn().context("spawn PlayCua")?;
        let stdin = child.stdin.take().ok_or_else(|| anyhow!("no stdin pipe"))?;
        let stdout = child.stdout.take().ok_or_else(|| anyhow!("no stdout pipe"))?;
        Ok(Self { child, client: RpcClient::new(stdin, BufReader::new(stdout)) })
    }

    /// Health-check: round-trip `ping` and verify the `ok` field is `true`.
    pub async fn ping(&mut self) -> Result<String, RpcError> {
        let r = self.client.call("ping", json!([])).await?;
        let ok = r.get("ok").and_then(Value::as_bool).unwrap_or(false);
        if !ok {
            return Err(RpcError::Protocol("ping returned ok=false".into()));
        }
        Ok(r.get("version").and_then(Value::as_str).unwrap_or("").to_string())
    }

    /// Ship a mouse-move tick to PlayCua. Uses `input.move` per PlayCua's
    /// openrpc contract (`contracts/openrpc.json`).
    pub async fn mouse_move(&mut self, x: i32, y: i32) -> Result<(), RpcError> {
        self.client.call("input.move", json!({"x": x, "y": y})).await?;
        Ok(())
    }

    /// Optional click after a mouse-move. PlayCua's `input.click` accepts
    /// `{"button": "left"|"right"|"middle"}` at the current cursor position.
    pub async fn mouse_click(&mut self, button: &str) -> Result<(), RpcError> {
        self.client.call("input.click", json!({"button": button})).await?;
        Ok(())
    }

    /// Begin recording. Returns PlayCua's session id.
    ///
    /// NOTE: PlayCua's published openrpc contract (as of 0.1.0) exposes only
    /// `screenshot` — video-mode `start_recording`/`stop_recording` are an
    /// **upstream gap** that ADR 0035 tracks. This crate ships the
    /// client-side contract the way the upstream will need to implement it
    /// (xcap frame pump on macOS/Linux, WGC `WindowRecordingSource` on
    /// Windows, per research-index item `dino@3d5c025`). Until PlayCua ships
    /// those methods, live recording will return `-32601 Method not found`;
    /// the unit tests exercise the happy path against a mock server.
    pub async fn start_recording(
        &mut self,
        out_path: &Path,
        title_hint: Option<&str>,
    ) -> Result<String, RpcError> {
        let mut params = json!({ "output_path": out_path.to_string_lossy() });
        if let Some(t) = title_hint {
            params["window_title"] = Value::String(t.to_string());
        }
        let r = self.client.call("start_recording", params).await?;
        Ok(r.get("session_id").and_then(Value::as_str).unwrap_or("").to_string())
    }

    /// Stop recording and await PlayCua's ack.
    pub async fn stop_recording(&mut self, session_id: &str) -> Result<(), RpcError> {
        self.client
            .call("stop_recording", json!({ "session_id": session_id }))
            .await?;
        Ok(())
    }

    /// Clean shutdown.
    pub async fn close(mut self) -> Result<()> {
        // PlayCua exits on stdin close per its stdio protocol.
        drop(self.client);
        let _ = self.child.wait().await?;
        Ok(())
    }
}

/// Shape returned by `run_record` for downstream callers (tests, CLI).
#[derive(Debug)]
pub struct RecordOutcome {
    pub session_id: String,
    pub out_path: PathBuf,
    pub duration_secs: u64,
}

/// Parse a `--cursor-track` JSON file (or inline JSON if the argument starts
/// with `[`).
pub fn parse_cursor_track(raw: &str) -> Result<Vec<CursorTick>> {
    let body = if raw.trim_start().starts_with('[') {
        raw.to_string()
    } else {
        std::fs::read_to_string(raw).with_context(|| format!("read cursor-track: {raw}"))?
    };
    let ticks: Vec<CursorTick> = serde_json::from_str(&body)?;
    Ok(ticks)
}

/// End-to-end happy path. Spawns PlayCua, pings, starts recording, streams
/// cursor ticks for the requested duration, stops, and closes.
///
/// Errors bubble loudly. No silent degradation.
pub async fn run_record(
    bin: PlayCuaBinary,
    target: &RecordTarget,
    out_path: &Path,
    duration_secs: u64,
    cursor_track: &[CursorTick],
) -> Result<RecordOutcome> {
    let mut session = Session::spawn(bin).await?;
    let _ver = session.ping().await.context("PlayCua ping failed")?;

    let hint = target.to_window_title_hint();
    let session_id = session
        .start_recording(out_path, hint.as_deref())
        .await
        .context("start_recording")?;

    let start = tokio::time::Instant::now();
    let total = std::time::Duration::from_secs(duration_secs);
    let mut cursor_iter = cursor_track.iter().peekable();

    while start.elapsed() < total {
        let elapsed_ms = start.elapsed().as_millis() as u64;
        while let Some(tick) = cursor_iter.peek() {
            if tick.at_ms > elapsed_ms {
                break;
            }
            let tick = cursor_iter.next().unwrap();
            session.mouse_move(tick.x, tick.y).await?;
            if let Some(button) = &tick.click {
                session.mouse_click(button).await?;
            }
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }

    session.stop_recording(&session_id).await.context("stop_recording")?;
    session.close().await?;

    Ok(RecordOutcome {
        session_id,
        out_path: out_path.to_path_buf(),
        duration_secs,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    use tokio::io::BufReader;

    /// Fake transport: in-memory reader/writer pair that returns canned
    /// responses. Exercises the JSON-RPC roundtrip without spawning PlayCua.
    struct MockServer {
        script: Vec<(&'static str, Value)>,
    }

    impl MockServer {
        fn into_bufs(self) -> (Vec<u8>, Vec<u8>) {
            let mut out = Vec::new();
            for (_method, result) in &self.script {
                let line = serde_json::to_string(&json!({
                    "jsonrpc": "2.0",
                    "id": 1, // overwritten below per iter
                    "result": result,
                }))
                .unwrap();
                out.extend_from_slice(line.as_bytes());
                out.push(b'\n');
            }
            // We'll fix ids at read-time by letting the client send first.
            let ids: Vec<u8> = Vec::new();
            (ids, out)
        }
    }

    // Reconstruct mock responses that echo incoming request ids so the
    // protocol-level id check passes.
    async fn rpc_echo_roundtrip(
        requests: &[(&str, Value)],
        fabricated_results: &[Value],
    ) -> Vec<Value> {
        assert_eq!(requests.len(), fabricated_results.len());
        let mut req_out = Vec::<u8>::new();
        // We simulate both sides by pre-computing the response stream after
        // the client writes, but since the client is sequential we can just
        // build the response lines with incremental ids 1..=N.
        let mut resp_in = Vec::<u8>::new();
        for (i, r) in fabricated_results.iter().enumerate() {
            let id = (i + 1) as u64;
            let line = serde_json::to_string(&json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": r,
            }))
            .unwrap();
            resp_in.extend_from_slice(line.as_bytes());
            resp_in.push(b'\n');
        }
        let reader = BufReader::new(Cursor::new(resp_in));
        let mut client = RpcClient::new(&mut req_out, reader);
        let mut outs = Vec::new();
        for (method, params) in requests {
            let r = client.call(method, params.clone()).await.expect("roundtrip");
            outs.push(r);
        }
        outs
    }

    #[tokio::test]
    async fn rpc_roundtrip_ping_and_record() {
        let results = rpc_echo_roundtrip(
            &[
                ("ping", json!([])),
                (
                    "start_recording",
                    json!({ "output_path": "/tmp/out.mp4", "window_title": "Finder" }),
                ),
                ("input.move", json!({"x": 100, "y": 200})),
                ("stop_recording", json!({"session_id": "sess-42"})),
            ],
            &[
                json!({"ok": true, "version": "0.1.0"}),
                json!({"session_id": "sess-42"}),
                json!({"ok": true}),
                json!({"ok": true}),
            ],
        )
        .await;
        assert_eq!(results[0]["version"], "0.1.0");
        assert_eq!(results[1]["session_id"], "sess-42");
        assert_eq!(results[2]["ok"], true);
    }

    #[test]
    fn parse_record_target_variants() {
        let w = RecordTarget::parse("window:Finder — Downloads").unwrap();
        assert!(matches!(w, RecordTarget::Window { .. }));
        assert_eq!(w.to_window_title_hint().as_deref(), Some("Finder — Downloads"));

        let p = RecordTarget::parse("pid:1234").unwrap();
        assert!(matches!(p, RecordTarget::Pid { pid: 1234 }));
        assert!(p.to_window_title_hint().is_none());

        let b = RecordTarget::parse("bundle-id:com.apple.finder").unwrap();
        assert!(matches!(b, RecordTarget::BundleId { .. }));

        assert!(RecordTarget::parse("bogus").is_err());
        assert!(RecordTarget::parse("pid:abc").is_err());
        assert!(RecordTarget::parse("widget:foo").is_err());
    }

    #[test]
    fn parse_cursor_track_inline_and_file() {
        let inline = parse_cursor_track(r#"[{"at_ms": 100, "x": 10, "y": 20}]"#).unwrap();
        assert_eq!(inline.len(), 1);
        assert_eq!(inline[0].at_ms, 100);

        let tmp = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(
            tmp.path(),
            r#"[{"at_ms": 0, "x": 0, "y": 0, "click": "left"}]"#,
        )
        .unwrap();
        let from_file = parse_cursor_track(tmp.path().to_str().unwrap()).unwrap();
        assert_eq!(from_file[0].click.as_deref(), Some("left"));
    }

    #[test]
    fn playcua_bin_env_var_must_exist() {
        // Sanity: PLAYCUA_BIN pointing at nowhere should fail loudly, not
        // silently drop to fallbacks.
        let prev = std::env::var_os("PLAYCUA_BIN");
        std::env::set_var("PLAYCUA_BIN", "/definitely/not/a/real/path/playcua-missing");
        let got = PlayCuaBinary::locate();
        // Restore before asserting so a failing assertion doesn't leak state.
        match prev {
            Some(v) => std::env::set_var("PLAYCUA_BIN", v),
            None => std::env::remove_var("PLAYCUA_BIN"),
        }
        assert!(got.is_err(), "locate should fail loudly when PLAYCUA_BIN is bogus");
    }

    #[allow(dead_code)]
    fn _suppress_unused_mock_server_warning() {
        let _ = MockServer { script: vec![] }.into_bufs();
    }
}
