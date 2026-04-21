//! Streamlit dev runner with FFI hot-reload.
//!
//! Boots `streamlit run apps/streamlit/app.py --server.port 8511`, watches the
//! built FFI dylib (`crates/hwledger-ffi/target/release/libhwledger_ffi.dylib`
//! or platform equivalent), and restarts Streamlit whenever the dylib changes.
//!
//! Streamlit itself hot-reloads Python source on save; this runner covers the
//! Rust side so Python bindings pick up fresh FFI behavior without a manual
//! kill + restart.
//!
//! Scripting policy: brief §5 mandates a Rust watcher (not watchdog.py).

use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::sync::mpsc::{channel, RecvTimeoutError};
use std::time::{Duration, Instant};

use anyhow::{bail, Context, Result};
use notify::{Event, EventKind, RecursiveMode, Watcher};

fn dylib_name() -> &'static str {
    if cfg!(target_os = "macos") {
        "libhwledger_ffi.dylib"
    } else if cfg!(target_os = "linux") {
        "libhwledger_ffi.so"
    } else {
        "hwledger_ffi.dll"
    }
}

fn repo_root() -> Result<PathBuf> {
    // Binary lives under <repo>/target/<profile>/hwledger-streamlit-dev.
    // Walk up until we find Cargo.toml alongside apps/streamlit.
    let mut cur = std::env::current_exe()?;
    while let Some(parent) = cur.parent().map(Path::to_path_buf) {
        cur = parent;
        if cur.join("apps").join("streamlit").exists() && cur.join("Cargo.toml").exists() {
            return Ok(cur);
        }
    }
    // Fallback to CWD.
    let cwd = std::env::current_dir()?;
    if cwd.join("apps").join("streamlit").exists() {
        return Ok(cwd);
    }
    bail!("could not locate hwLedger repo root");
}

fn spawn_streamlit(root: &Path, port: u16) -> Result<Child> {
    tracing::info!("spawning streamlit on :{port}");
    let app = root.join("apps/streamlit/app.py");
    let child = Command::new("uv")
        .args([
            "run",
            "--project",
            root.join("apps/streamlit").to_str().unwrap(),
            "streamlit",
            "run",
            app.to_str().unwrap(),
            "--server.port",
            &port.to_string(),
            "--server.headless",
            "true",
        ])
        .spawn()
        .context("failed to spawn `uv run streamlit`")?;
    Ok(child)
}

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let root = repo_root()?;
    tracing::info!(root = %root.display(), "repo root");

    let dylib_path = root.join("target").join("release").join(dylib_name());
    tracing::info!(dylib = %dylib_path.display(), "watching FFI dylib");

    let port =
        std::env::var("HWLEDGER_STREAMLIT_PORT").ok().and_then(|s| s.parse().ok()).unwrap_or(8511);

    let (tx, rx) = channel::<notify::Result<Event>>();
    let mut watcher = notify::recommended_watcher(tx)?;
    // Watch the parent dir so that first-time creation also fires.
    let watch_dir = dylib_path.parent().unwrap_or_else(|| Path::new(".")).to_path_buf();
    std::fs::create_dir_all(&watch_dir).ok();
    watcher.watch(&watch_dir, RecursiveMode::NonRecursive)?;

    let mut child = spawn_streamlit(&root, port)?;
    let debounce = Duration::from_millis(750);
    let mut last_trigger = Instant::now() - debounce;

    loop {
        match rx.recv_timeout(Duration::from_millis(500)) {
            Ok(Ok(ev)) => {
                if !matches!(
                    ev.kind,
                    EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_)
                ) {
                    continue;
                }
                if !ev.paths.iter().any(|p| p.ends_with(dylib_name())) {
                    continue;
                }
                if last_trigger.elapsed() < debounce {
                    continue;
                }
                last_trigger = Instant::now();
                tracing::info!("FFI dylib changed; restarting streamlit");
                let _ = child.kill();
                let _ = child.wait();
                child = spawn_streamlit(&root, port)?;
            }
            Ok(Err(e)) => tracing::warn!("watch error: {e}"),
            Err(RecvTimeoutError::Timeout) => {
                if let Ok(Some(status)) = child.try_wait() {
                    tracing::warn!(?status, "streamlit exited; respawning");
                    child = spawn_streamlit(&root, port)?;
                }
            }
            Err(RecvTimeoutError::Disconnected) => bail!("watcher channel closed"),
        }
    }
}
