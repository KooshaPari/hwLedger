//! `hwledger-journey-record` — per-OS recording orchestrator for hwLedger
//! GUI + web journeys.
//!
//! Exposes a single CLI over three backends:
//!
//! | backend | OS      | capture stack                          | status  |
//! |---------|---------|----------------------------------------|---------|
//! | `scsk`  | macOS   | `ScreenCaptureKit` via `hwledger-gui-recorder` / Swift bridge | full    |
//! | `xvfb`  | Linux   | Xvfb + `ffmpeg -f x11grab` (+ bubblewrap, xdotool) | stub    |
//! | `winrdp`| Windows | `Windows.Graphics.Capture` (`windows-capture` crate), Windows Sandbox | stub    |
//! | `auto`  | any     | picks best for current host                      | routing |
//!
//! XCUITest (macOS) / Playwright (web) still drive UI events; this tool only
//! owns the capture layer.
//!
//! See `docs-site/reference/recording-backends.md` for the per-OS feature matrix.
//!
//! Traces to: G-recording-backends, scripting-policy.

use std::path::PathBuf;
use std::time::Duration;

use anyhow::Result;
use clap::{Parser, ValueEnum};

mod backends;

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
#[clap(rename_all = "lower")]
pub enum Backend {
    /// macOS ScreenCaptureKit — full implementation.
    Scsk,
    /// Linux Xvfb + ffmpeg x11grab — stub (TODO: implement).
    Xvfb,
    /// Windows.Graphics.Capture + RDP isolation — stub (TODO: implement).
    Winrdp,
    /// Pick the best backend for the current host.
    Auto,
}

#[derive(Debug, Parser)]
#[command(
    name = "hwledger-journey-record",
    about = "Per-OS recording orchestrator for hwLedger GUI + web journeys.",
    version
)]
pub struct Cli {
    /// Capture target. Either an app bundle-id (macOS), a browser URL, or a PID.
    #[arg(long)]
    pub target: String,

    /// Output MP4 path.
    #[arg(long)]
    pub output: PathBuf,

    /// Recording duration in seconds. 0 means "record until Ctrl+C".
    #[arg(long, default_value_t = 0)]
    pub duration: u64,

    /// Backend selection.
    #[arg(long, value_enum, default_value_t = Backend::Auto)]
    pub backend: Backend,

    /// Render a synthetic cursor into the capture stream instead of the user's
    /// real OS cursor (lets the user keep driving their own desktop).
    #[arg(long)]
    pub virtual_cursor: bool,

    /// Capture on an off-screen / virtual display so the user's main desktop is
    /// untouched. Falls back to primary display with a warning if unsupported.
    #[arg(long)]
    pub headless: bool,

    /// Run the target app inside a per-OS sandbox (sandbox-exec / bubblewrap /
    /// Windows Sandbox).
    #[arg(long)]
    pub sandbox: bool,

    /// Capture width in pixels.
    #[arg(long, default_value_t = 1440)]
    pub width: u32,

    /// Capture height in pixels.
    #[arg(long, default_value_t = 900)]
    pub height: u32,

    /// Frame rate.
    #[arg(long, default_value_t = 30)]
    pub fps: u32,
}

#[derive(Debug, Clone)]
pub struct RecordRequest {
    pub target: String,
    pub output: PathBuf,
    pub duration: Option<Duration>,
    pub virtual_cursor: bool,
    pub headless: bool,
    pub sandbox: bool,
    pub width: u32,
    pub height: u32,
    pub fps: u32,
}

impl From<&Cli> for RecordRequest {
    fn from(c: &Cli) -> Self {
        Self {
            target: c.target.clone(),
            output: c.output.clone(),
            duration: if c.duration == 0 { None } else { Some(Duration::from_secs(c.duration)) },
            virtual_cursor: c.virtual_cursor,
            headless: c.headless,
            sandbox: c.sandbox,
            width: c.width,
            height: c.height,
            fps: c.fps,
        }
    }
}

/// Resolve `Backend::Auto` to the concrete backend for the current host.
pub fn resolve_backend(requested: Backend) -> Backend {
    match requested {
        Backend::Auto => {
            if cfg!(target_os = "macos") {
                Backend::Scsk
            } else if cfg!(target_os = "linux") {
                Backend::Xvfb
            } else if cfg!(target_os = "windows") {
                Backend::Winrdp
            } else {
                // Unknown OS — surface scsk so error messaging is clear.
                Backend::Scsk
            }
        }
        other => other,
    }
}

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let cli = Cli::parse();
    let req = RecordRequest::from(&cli);
    let backend = resolve_backend(cli.backend);

    tracing::info!(
        ?backend,
        target = %req.target,
        output = %req.output.display(),
        duration_s = cli.duration,
        virtual_cursor = req.virtual_cursor,
        headless = req.headless,
        sandbox = req.sandbox,
        "hwledger-journey-record starting"
    );

    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async move {
        match backend {
            Backend::Scsk => backends::scsk::run(&req).await,
            Backend::Xvfb => backends::xvfb::run(&req).await,
            Backend::Winrdp => backends::winrdp::run(&req).await,
            Backend::Auto => unreachable!("resolve_backend removes Auto"),
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn cli_verifies() {
        Cli::command().debug_assert();
    }

    #[test]
    fn parses_minimal() {
        let cli = Cli::try_parse_from([
            "hwledger-journey-record",
            "--target",
            "com.kooshapari.hwLedger",
            "--output",
            "/tmp/out.mp4",
        ])
        .expect("parse");
        assert_eq!(cli.backend, Backend::Auto);
        assert_eq!(cli.duration, 0);
        assert!(!cli.virtual_cursor);
        assert!(!cli.headless);
        assert!(!cli.sandbox);
    }

    #[test]
    fn parses_full() {
        let cli = Cli::try_parse_from([
            "hwledger-journey-record",
            "--target",
            "com.kooshapari.hwLedger",
            "--output",
            "/tmp/out.mp4",
            "--duration",
            "3",
            "--backend",
            "scsk",
            "--virtual-cursor",
            "--headless",
            "--sandbox",
        ])
        .expect("parse");
        assert_eq!(cli.backend, Backend::Scsk);
        assert_eq!(cli.duration, 3);
        assert!(cli.virtual_cursor);
        assert!(cli.headless);
        assert!(cli.sandbox);
    }

    #[test]
    fn auto_resolves_to_host_backend() {
        let resolved = resolve_backend(Backend::Auto);
        // On any of our three supported hosts, Auto must collapse to a
        // concrete variant.
        assert!(matches!(resolved, Backend::Scsk | Backend::Xvfb | Backend::Winrdp));
    }

    #[test]
    fn explicit_backend_passthrough() {
        assert_eq!(resolve_backend(Backend::Scsk), Backend::Scsk);
        assert_eq!(resolve_backend(Backend::Xvfb), Backend::Xvfb);
        assert_eq!(resolve_backend(Backend::Winrdp), Backend::Winrdp);
    }
}
