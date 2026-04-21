//! hwledger-journey-render CLI — thin binary on top of the library.

use std::path::PathBuf;

use clap::{Parser, Subcommand};
use hwledger_journey_render::{batch, run, RenderPlan};

#[derive(Parser, Debug)]
#[command(
    name = "hwledger-journey-render",
    about = "Render enriched (rich) MP4s for hwLedger journeys via Remotion.",
    version
)]
struct Cli {
    #[command(subcommand)]
    cmd: Option<Cmd>,

    /// (legacy single-journey mode) journey id.
    #[arg(long, global = false)]
    journey: Option<String>,
    #[arg(long)]
    manifest: Option<PathBuf>,
    #[arg(long)]
    keyframes: Option<PathBuf>,
    #[arg(long)]
    remotion_root: Option<PathBuf>,
    #[arg(long)]
    output: Option<PathBuf>,
    #[arg(long)]
    scene_spec: Option<PathBuf>,
    #[arg(long, default_value = "silent")]
    voiceover: String,
}

#[derive(Subcommand, Debug)]
enum Cmd {
    /// Batch-render every `manifest.verified.json` under <root>. Idempotent —
    /// journeys whose manifest hash already matches their
    /// `recording_rich_manifest_sha256` are skipped.
    All {
        /// Root directory under which to find manifests (e.g. `docs-site/public`).
        root: PathBuf,

        /// Remotion project root (defaults to `<repo>/tools/journey-remotion`).
        #[arg(long)]
        remotion_root: Option<PathBuf>,

        /// Force re-render even if manifest hash matches.
        #[arg(long)]
        force: bool,

        /// Voiceover backend ("silent" or "piper").
        #[arg(long, default_value = "silent")]
        voiceover: String,
    },

    /// Single journey (same as the legacy flag-only invocation).
    One {
        #[arg(long)]
        journey: String,
        #[arg(long)]
        manifest: PathBuf,
        #[arg(long)]
        keyframes: PathBuf,
        #[arg(long)]
        remotion_root: PathBuf,
        #[arg(long)]
        output: PathBuf,
        #[arg(long)]
        scene_spec: Option<PathBuf>,
        #[arg(long, default_value = "silent")]
        voiceover: String,
    },
}

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();
    let cli = Cli::parse();

    match cli.cmd {
        Some(Cmd::All { root, remotion_root, force, voiceover }) => {
            let remotion_root = remotion_root.unwrap_or_else(default_remotion_root);
            batch::render_all(&root, &remotion_root, force, &voiceover)?;
            Ok(())
        }
        Some(Cmd::One {
            journey,
            manifest,
            keyframes,
            remotion_root,
            output,
            scene_spec,
            voiceover,
        }) => {
            run_single(journey, manifest, keyframes, remotion_root, output, scene_spec, voiceover)
        }
        None => {
            // legacy: flags must all be provided.
            let journey = cli.journey.ok_or_else(|| {
                anyhow::anyhow!("either use subcommand (`all`/`one`) or provide legacy flags")
            })?;
            let manifest = cli.manifest.ok_or_else(|| anyhow::anyhow!("--manifest required"))?;
            let keyframes = cli.keyframes.ok_or_else(|| anyhow::anyhow!("--keyframes required"))?;
            let remotion_root =
                cli.remotion_root.ok_or_else(|| anyhow::anyhow!("--remotion-root required"))?;
            let output = cli.output.ok_or_else(|| anyhow::anyhow!("--output required"))?;
            run_single(
                journey,
                manifest,
                keyframes,
                remotion_root,
                output,
                cli.scene_spec,
                cli.voiceover,
            )
        }
    }
}

fn run_single(
    journey: String,
    manifest: PathBuf,
    keyframes: PathBuf,
    remotion_root: PathBuf,
    output: PathBuf,
    scene_spec: Option<PathBuf>,
    voiceover: String,
) -> anyhow::Result<()> {
    let mut plan = RenderPlan::new(journey, manifest, keyframes, remotion_root, output);
    plan.scene_spec = scene_spec;
    plan.voiceover = voiceover;
    let out = run(&plan)?;
    println!("{}", out.display());
    Ok(())
}

fn default_remotion_root() -> PathBuf {
    // Walk up from CWD to find tools/journey-remotion (workspace convention).
    let mut cur = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    for _ in 0..6 {
        let cand = cur.join("tools").join("journey-remotion");
        if cand.exists() {
            return cand;
        }
        if !cur.pop() {
            break;
        }
    }
    PathBuf::from("tools/journey-remotion")
}
