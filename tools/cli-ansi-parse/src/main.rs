//! CLI driver for `hwledger-cli-ansi-parse`.
//!
//! Usage:
//!
//! ```text
//! hwledger-cli-ansi-parse \
//!     --cast recordings/probe-list.cast \
//!     --out-dir docs-site/public/cli-journeys/probe-list/structural \
//!     --timestamp 0.5:frame_001 \
//!     --timestamp 1.2:frame_002 \
//!     --timestamp 3.0:frame_003
//! ```

use std::path::PathBuf;

use anyhow::{bail, Context, Result};
use clap::Parser;

use hwledger_cli_ansi_parse::{emit_snapshots, load_cast, replay_and_snapshot};

#[derive(Debug, Parser)]
#[command(
    name = "hwledger-cli-ansi-parse",
    about = "Asciicast v2 → per-keyframe structural terminal snapshot (Tier 0)."
)]
struct Cli {
    /// Path to the asciicast v2 `.cast` file.
    #[arg(long)]
    cast: PathBuf,
    /// Directory to write `<frame-id>.structural.json` files into.
    #[arg(long)]
    out_dir: PathBuf,
    /// Repeat per keyframe: `<seconds>:<frame-id>` (e.g. `1.25:frame_003`).
    #[arg(long = "timestamp", value_parser = parse_ts_spec)]
    timestamps: Vec<(f64, String)>,
}

fn parse_ts_spec(s: &str) -> Result<(f64, String), String> {
    let (t, id) = s
        .split_once(':')
        .ok_or_else(|| format!("expected <seconds>:<frame-id>, got {s:?}"))?;
    let t: f64 = t.parse().map_err(|e| format!("bad seconds {t:?}: {e}"))?;
    Ok((t, id.to_string()))
}

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .with_writer(std::io::stderr)
        .init();

    let cli = Cli::parse();
    if cli.timestamps.is_empty() {
        bail!("at least one --timestamp required");
    }
    // Ascending order.
    let mut ts_pairs = cli.timestamps;
    ts_pairs.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
    let times: Vec<f64> = ts_pairs.iter().map(|p| p.0).collect();
    let ids: Vec<String> = ts_pairs.iter().map(|p| p.1.clone()).collect();

    let (header, events) = load_cast(&cli.cast).with_context(|| "load cast")?;
    let snaps = replay_and_snapshot(&header, &events, &times);
    let written = emit_snapshots(&cli.out_dir, &ids, &snaps)?;

    for p in &written {
        println!("{}", p.display());
    }
    Ok(())
}
