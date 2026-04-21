//! hwledger-journey-render CLI — thin binary on top of the library.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use clap::{Parser, Subcommand};
use hwledger_journey_render::{
    annotate as run_annotate, build_rich_manifest, run, Annotation, RenderPlan,
};

#[derive(Parser, Debug)]
#[command(
    name = "hwledger-journey-render",
    about = "Project annotations, render annotated keyframes, and enrich journey MP4s via Remotion.",
    version
)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand, Debug)]
enum Cmd {
    /// End-to-end render (project annotations + annotate keyframes + render rich MP4).
    Render(RenderArgs),
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

#[derive(clap::Args, Debug)]
struct RenderArgs {
    /// Journey id (e.g. "plan-deepseek").
    #[arg(long)]
    journey: String,

    /// Canonical manifest.json (or manifest.verified.json) from phenotype-journeys.
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

/// Run annotate step only (requires manifest with annotations already populated).
fn annotate_only(manifest: &Path, keyframes: &Path, remotion_root: &Path) -> anyhow::Result<()> {
    // Build a minimal RenderPlan to reuse annotate() helper.
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
        voiceover: "silent".to_string(),
    };
    // annotate.ts expects a RichManifest shape — the real manifest already has steps[]; write
    // a rich-shaped copy next to keyframes_dir so annotate.ts can read it.
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

fn do_render(args: RenderArgs) -> anyhow::Result<()> {
    let manifest_abs = std::fs::canonicalize(&args.manifest)?;
    let keyframes_abs = std::fs::canonicalize(&args.keyframes)?;
    let remotion_abs = std::fs::canonicalize(&args.remotion_root)?;
    // Output may not yet exist; canonicalize parent.
    let output_abs = if args.output.is_absolute() {
        args.output.clone()
    } else {
        std::env::current_dir()?.join(&args.output)
    };
    let mut plan =
        RenderPlan::new(args.journey, manifest_abs, keyframes_abs, remotion_abs, output_abs);
    plan.scene_spec = args.scene_spec;
    plan.voiceover = args.voiceover;
    let out = run(&plan)?;
    println!("{}", out.display());
    Ok(())
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
        Cmd::Render(args) => do_render(args),
        Cmd::Annotate { manifest, keyframes, remotion_root } => {
            annotate_only(&manifest, &keyframes, &remotion_root)
        }
        Cmd::ProjectAnnotations { yaml, manifests } => project_annotations(&yaml, &manifests),
    }
}

