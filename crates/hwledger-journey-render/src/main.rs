//! hwledger-journey-render CLI — thin binary on top of the library.

use std::path::PathBuf;

use clap::Parser;
use hwledger_journey_render::{run, RenderPlan};

#[derive(Parser, Debug)]
#[command(
    name = "hwledger-journey-render",
    about = "Render an enriched (rich) MP4 for a CLI journey via Remotion.",
    version,
)]
struct Cli {
    /// Journey id (e.g. "plan-deepseek").
    #[arg(long)]
    journey: String,

    /// Canonical manifest.json from phenotype-journeys.
    #[arg(long)]
    manifest: PathBuf,

    /// Directory of keyframe PNGs (frame-001.png, ...).
    #[arg(long)]
    keyframes: PathBuf,

    /// Remotion project root (tools/journey-remotion).
    #[arg(long)]
    remotion_root: PathBuf,

    /// Output MP4 path.
    #[arg(long)]
    output: PathBuf,

    /// Optional scene-spec sidecar JSON.
    #[arg(long)]
    scene_spec: Option<PathBuf>,

    /// Voiceover backend ("silent" | "piper"). Default "silent".
    #[arg(long, default_value = "silent")]
    voiceover: String,
}

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();
    let cli = Cli::parse();
    let mut plan = RenderPlan::new(
        cli.journey,
        cli.manifest,
        cli.keyframes,
        cli.remotion_root,
        cli.output,
    );
    plan.scene_spec = cli.scene_spec;
    plan.voiceover = cli.voiceover;
    let out = run(&plan)?;
    println!("{}", out.display());
    Ok(())
}
