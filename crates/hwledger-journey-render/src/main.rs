//! hwledger-journey-render CLI — thin binary on top of the library.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use clap::{Parser, Subcommand};
use hwledger_journey_render::{
    annotate as run_annotate, batch, build_rich_manifest, manifest::CustomAnchor, run, Annotation,
    RenderPlan,
};

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
    /// Voiceover backend — `auto` (default) uses Piper when available and
    /// silently falls back to no audio if the binary or voice model is
    /// missing. Explicit values: `piper`, `silent`.
    #[arg(long, default_value = "auto")]
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

        /// Post-render blind-judge phase. `auto` picks Claude when
        /// ANTHROPIC_API_KEY is set, else Ollama when reachable, else none.
        /// `none` disables the post-render judge phase entirely.
        #[arg(long, default_value = "auto")]
        judge: String,
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
        /// Override the Remotion composition id. Defaults to `JourneyRich`;
        /// pass `JourneySlideshow` to force the keyframe-slideshow fallback.
        #[arg(long, default_value = "JourneyRich")]
        composition: String,
    },

    /// Annotate keyframes for an already-projected manifest (no MP4 render).
    Annotate {
        /// Manifest file (manifest.verified.json) with steps[].annotations already populated.
        manifest: PathBuf,
        /// Keyframes directory (containing frame-NNN.png).
        #[arg(long)]
        keyframes: PathBuf,
        /// Remotion project root (tools/journey-remotion).
        #[arg(long)]
        remotion_root: PathBuf,
    },

    /// Project annotations from a shot-annotations.yaml into one or more manifests.
    ProjectAnnotations {
        /// Path to shot-annotations.yaml.
        #[arg(long)]
        yaml: PathBuf,
        /// One or more manifest.verified.json files to update in place.
        /// The journey id is read from the manifest `id` field.
        #[arg(long = "manifest", num_args = 1..)]
        manifests: Vec<PathBuf>,
    },
}

type YamlAnnotations = BTreeMap<String, BTreeMap<u32, Vec<YamlAnnotation>>>;

#[derive(serde::Deserialize, Debug, Clone)]
struct YamlAnnotation {
    bbox: [u32; 4],
    label: String,
    #[serde(default)]
    color: Option<String>,
    #[serde(default)]
    style: Option<String>,
    #[serde(default)]
    note: Option<String>,
    #[serde(default)]
    kind: Option<String>,
    /// Callout position hint. One of:
    /// auto | top-left | top-right | bottom-left | bottom-right |
    /// center | center-top | center-bottom | custom. Default: `auto`
    /// (renderer picks based on bbox anchor).
    #[serde(default)]
    position: Option<String>,
    /// Custom pixel anchor when `position: custom`.
    #[serde(default)]
    custom: Option<YamlCustomAnchor>,
}

#[derive(serde::Deserialize, Debug, Clone)]
struct YamlCustomAnchor {
    x: u32,
    y: u32,
}

fn project_annotations(yaml_path: &Path, manifests: &[PathBuf]) -> anyhow::Result<()> {
    let yaml_raw = std::fs::read_to_string(yaml_path)?;
    let all: YamlAnnotations = serde_yaml::from_str(&yaml_raw)?;
    let mut projected = 0usize;
    for m in manifests {
        let raw = std::fs::read_to_string(m)?;
        let mut manifest: serde_json::Value = serde_json::from_str(&raw)?;
        let id = manifest.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let Some(journey_entries) = all.get(&id) else {
            eprintln!("no entries for journey {id} in YAML (manifest {})", m.display());
            continue;
        };
        let Some(steps) = manifest.get_mut("steps").and_then(|s| s.as_array_mut()) else {
            eprintln!("manifest {} has no steps[]", m.display());
            continue;
        };
        for step in steps.iter_mut() {
            // `step.index` is 0-based; YAML `frame` is 1-based and matches frame-NNN.png.
            let Some(idx0) = step.get("index").and_then(|v| v.as_u64()) else { continue };
            let frame_index = (idx0 as u32) + 1;
            let Some(anns) = journey_entries.get(&frame_index) else { continue };
            let anns_json: Vec<Annotation> = anns
                .iter()
                .map(|a| Annotation {
                    bbox: a.bbox,
                    label: a.label.clone(),
                    color: a.color.clone(),
                    style: a.style.clone(),
                    note: a.note.clone(),
                    kind: a.kind.clone(),
                    position: a.position.clone(),
                    custom: a.custom.as_ref().map(|c| CustomAnchor { x: c.x, y: c.y }),
                })
                .collect();
            let step_obj = step.as_object_mut().expect("step is object");
            step_obj.insert("annotations".into(), serde_json::to_value(&anns_json)?);
            projected += anns.len();
        }
        let pretty = serde_json::to_string_pretty(&manifest)?;
        std::fs::write(m, pretty + "\n")?;
        println!("projected annotations -> {} ({} bbox total so far)", m.display(), projected);
    }
    println!("total annotations projected: {projected}");
    Ok(())
}

fn annotate_only(manifest: &Path, keyframes: &Path, remotion_root: &Path) -> anyhow::Result<()> {
    let manifest_abs = std::fs::canonicalize(manifest)?;
    let keyframes_abs = std::fs::canonicalize(keyframes)?;
    let remotion_abs = std::fs::canonicalize(remotion_root)?;
    let plan = RenderPlan {
        journey_id: read_id(&manifest_abs)?,
        manifest_path: manifest_abs,
        keyframes_dir: keyframes_abs,
        remotion_root: remotion_abs,
        output_mp4: PathBuf::new(),
        scene_spec: None,
        voiceover: "auto".to_string(),
        composition_id: "JourneyRich".to_string(),
        target_content_seconds: None,
    };
    let rich_path = build_rich_manifest(&plan)?;
    run_annotate(&plan, &rich_path)?;
    println!("annotated keyframes written for {}", plan.journey_id);
    Ok(())
}

fn read_id(manifest: &Path) -> anyhow::Result<String> {
    let raw = std::fs::read_to_string(manifest)?;
    let v: serde_json::Value = serde_json::from_str(&raw)?;
    Ok(v.get("id").and_then(|s| s.as_str()).unwrap_or("unknown").to_string())
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
        Some(Cmd::All { root, remotion_root, force, voiceover, judge }) => {
            let remotion_root = remotion_root.unwrap_or_else(default_remotion_root);
            batch::render_all(&root, &remotion_root, force, &voiceover)?;
            if judge != "none" {
                invoke_vlm_judge(&root, &judge)?;
            }
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
            composition,
        }) => run_single(
            journey,
            manifest,
            keyframes,
            remotion_root,
            output,
            scene_spec,
            voiceover,
            composition,
        ),
        Some(Cmd::Annotate { manifest, keyframes, remotion_root }) => {
            annotate_only(&manifest, &keyframes, &remotion_root)
        }
        Some(Cmd::ProjectAnnotations { yaml, manifests }) => project_annotations(&yaml, &manifests),
        None => {
            let journey = cli.journey.ok_or_else(|| {
                anyhow::anyhow!(
                    "either use subcommand (`all`/`one`/`annotate`/`project-annotations`) or provide legacy flags"
                )
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
                "JourneyRich".to_string(),
            )
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn run_single(
    journey: String,
    manifest: PathBuf,
    keyframes: PathBuf,
    remotion_root: PathBuf,
    output: PathBuf,
    scene_spec: Option<PathBuf>,
    voiceover: String,
    composition: String,
) -> anyhow::Result<()> {
    let manifest_abs = std::fs::canonicalize(&manifest).unwrap_or(manifest);
    let keyframes_abs = std::fs::canonicalize(&keyframes).unwrap_or(keyframes);
    let remotion_abs = std::fs::canonicalize(&remotion_root).unwrap_or(remotion_root);
    let output_abs =
        if output.is_absolute() { output.clone() } else { std::env::current_dir()?.join(&output) };
    let mut plan = RenderPlan::new(journey, manifest_abs, keyframes_abs, remotion_abs, output_abs);
    plan.scene_spec = scene_spec;
    plan.voiceover = voiceover;
    plan.composition_id = composition;
    let out = run(&plan)?;
    println!("{}", out.display());
    Ok(())
}

/// Shell out to the sibling `hwledger-vlm-judge` binary as a post-render
/// phase. Best-effort: a failure here logs but does not abort the render run.
fn invoke_vlm_judge(root: &Path, judge: &str) -> anyhow::Result<()> {
    let bin_name = "hwledger-vlm-judge";
    let candidates: Vec<PathBuf> = std::iter::once(PathBuf::from(bin_name))
        .chain(std::env::current_exe().ok().and_then(|p| p.parent().map(|dir| dir.join(bin_name))))
        .collect();
    let mut last_err: Option<String> = None;
    for cand in &candidates {
        let mut cmd = std::process::Command::new(cand);
        cmd.arg("--judge").arg(judge).arg("--root").arg(root);
        match cmd.status() {
            Ok(s) if s.success() => {
                return Ok(());
            }
            Ok(s) => {
                last_err = Some(format!("exit {s}"));
            }
            Err(e) => {
                last_err = Some(format!("spawn {cand:?}: {e}"));
                continue;
            }
        }
    }
    eprintln!(
        "[journey-render] vlm-judge phase skipped ({}): install `hwledger-vlm-judge` (cargo build -p hwledger-vlm-judge)",
        last_err.unwrap_or_else(|| "binary not found".into())
    );
    Ok(())
}

fn default_remotion_root() -> PathBuf {
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
