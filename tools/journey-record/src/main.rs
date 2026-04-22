//! `hwledger-journey-record` — thin stdio JSON-RPC client that wraps
//! [PlayCua](https://github.com/KooshaPari/PlayCua) for GUI journey recording.
//!
//! Replaces the earlier per-OS from-scratch direction (agent dispatch
//! `a3773560`). Contract: see `docs-site/architecture/adrs/0035-playcua-recording-integration.md`.

use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use hwledger_journey_record::{
    parse_cursor_track, run_record, PlayCuaBinary, RecordTarget,
};

#[derive(Debug, Parser)]
#[command(
    name = "hwledger-journey-record",
    about = "Record per-OS GUI journeys by wrapping PlayCua over stdio JSON-RPC 2.0."
)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Debug, Subcommand)]
enum Cmd {
    /// Record an MP4 of the target window / process for `--duration` seconds.
    Record {
        /// `window:<substring>`, `pid:<pid>`, or `bundle-id:<reverse-dns>`.
        #[arg(long)]
        target: String,

        /// Output path (MP4).
        #[arg(long)]
        out: PathBuf,

        /// Duration in seconds.
        #[arg(long)]
        duration: u64,

        /// Inline JSON array or path to a cursor-track JSON file. Each entry:
        /// `{"at_ms": u64, "x": i32, "y": i32, "click": "left"?}`.
        #[arg(long)]
        cursor_track: Option<String>,

        /// Reserved: run PlayCua with its sandbox primitive enabled. Wiring
        /// lives in PlayCua upstream; we forward the intent once the
        /// contract stabilises (see research index §1 `afaa70d`).
        #[arg(long, default_value_t = false)]
        sandbox: bool,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .with_writer(std::io::stderr)
        .init();

    let cli = Cli::parse();
    match cli.cmd {
        Cmd::Record { target, out, duration, cursor_track, sandbox } => {
            let target = RecordTarget::parse(&target)?;
            let ticks = match cursor_track {
                Some(s) => parse_cursor_track(&s)?,
                None => Vec::new(),
            };
            if sandbox {
                tracing::warn!(
                    "--sandbox requested but PlayCua's sandbox primitive is not yet surfaced \
                     via JSON-RPC; continuing without sandbox (see ADR 0035 §Consequences)."
                );
            }
            let bin = PlayCuaBinary::locate().context("locate PlayCua")?;
            let outcome = run_record(bin, &target, &out, duration, &ticks).await?;
            tracing::info!(
                session_id = %outcome.session_id,
                out = %outcome.out_path.display(),
                duration_secs = outcome.duration_secs,
                "recording complete"
            );
            println!(
                "{}",
                serde_json::json!({
                    "session_id": outcome.session_id,
                    "out": outcome.out_path.display().to_string(),
                    "duration_secs": outcome.duration_secs,
                })
            );
            Ok(())
        }
    }
}
