//! hwledger-journey-render: orchestrates the Remotion enrichment pipeline.
//!
//! Flow:
//!   1. Read the canonical manifest (from `phenotype-journeys`).
//!   2. Merge an optional sidecar scene-spec (callout texts, durations).
//!   3. Call `bun run annotate` to composite annotated keyframe PNGs.
//!   4. Call `bun run remotion render` with the enriched manifest inline.
//!   5. Write the rich manifest (with `recording_rich` + `annotated_keyframes`)
//!      back next to the source manifest as `manifest.rich.json`.
//!
//! The actual TS code lives in `tools/journey-remotion/`. This crate is a
//! thin, typed orchestrator so Rust tests/tools can invoke the pipeline.

use std::path::{Path, PathBuf};
use std::process::Command;

use serde::{Deserialize, Serialize};

pub mod batch;
pub mod manifest;

pub use manifest::{Annotation, JourneyStep, RichManifest, SceneSpec, VoiceoverSpec};

#[derive(Debug, thiserror::Error)]
pub enum RenderError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
    #[error("bun not on PATH — install bun.sh first")]
    BunMissing,
    #[error("remotion render failed (exit {code}): {stderr}")]
    RenderFailed { code: i32, stderr: String },
    #[error("annotate step failed (exit {code}): {stderr}")]
    AnnotateFailed { code: i32, stderr: String },
    #[error("bad manifest: {0}")]
    BadManifest(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenderPlan {
    pub journey_id: String,
    /// Path to the canonical manifest.json (phenotype-journeys shape).
    pub manifest_path: PathBuf,
    /// Directory containing the keyframe PNGs (frame-001.png, ...).
    pub keyframes_dir: PathBuf,
    /// Root of the Remotion project (`tools/journey-remotion`).
    pub remotion_root: PathBuf,
    /// Output MP4 path (absolute).
    pub output_mp4: PathBuf,
    /// Optional scene spec YAML/JSON sidecar; if missing, defaults are used.
    pub scene_spec: Option<PathBuf>,
    /// Voiceover backend ("silent" or "piper").
    pub voiceover: String,
}

impl RenderPlan {
    pub fn new(
        journey_id: impl Into<String>,
        manifest_path: impl Into<PathBuf>,
        keyframes_dir: impl Into<PathBuf>,
        remotion_root: impl Into<PathBuf>,
        output_mp4: impl Into<PathBuf>,
    ) -> Self {
        Self {
            journey_id: journey_id.into(),
            manifest_path: manifest_path.into(),
            keyframes_dir: keyframes_dir.into(),
            remotion_root: remotion_root.into(),
            output_mp4: output_mp4.into(),
            scene_spec: None,
            voiceover: "silent".to_string(),
        }
    }
}

/// Synthesise a per-journey voiceover WAV via Piper, concatenating one
/// utterance per step. Returns the public-relative path suitable for
/// `manifest.voiceover.audio` (i.e. consumable via Remotion `staticFile()`).
///
/// Requires `piper` on PATH and a voice model at the path given by
/// `HWLEDGER_PIPER_VOICE` (default: `~/.cache/piper/voices/en_US-lessac-medium.onnx`).
pub fn synthesise_voiceover_piper(plan: &RenderPlan) -> Result<String, RenderError> {
    let model = std::env::var("HWLEDGER_PIPER_VOICE")
        .ok()
        .map(PathBuf::from)
        .filter(|p| p.exists())
        .unwrap_or_else(|| {
            let home = std::env::var("HOME").unwrap_or_default();
            PathBuf::from(home).join(".cache/piper/voices/en_US-lessac-medium.onnx")
        });
    if !model.exists() {
        return Err(RenderError::BadManifest(format!(
            "piper voice model not found: {}",
            model.display()
        )));
    }
    if Command::new("piper").arg("--help").output().is_err() {
        return Err(RenderError::BadManifest("piper not on PATH".into()));
    }

    let raw = std::fs::read_to_string(&plan.manifest_path)?;
    let rich: RichManifest = serde_json::from_str(&raw)
        .map_err(|e| RenderError::BadManifest(format!("manifest parse: {e}")))?;

    let tmp_dir = plan.remotion_root.join("public").join("audio").join(&plan.journey_id);
    std::fs::create_dir_all(&tmp_dir)?;
    let mut parts: Vec<PathBuf> = Vec::new();
    let intro = format!("{}. {}", plan.journey_id.replace('-', " "), rich.intent);
    let intro_wav = tmp_dir.join("000-intro.wav");
    piper_one(&model, &intro, &intro_wav)?;
    parts.push(intro_wav);
    for (i, step) in rich.steps.iter().enumerate() {
        let line = step
            .description
            .clone()
            .or_else(|| step.blind_description.clone())
            .unwrap_or_else(|| step.intent.clone());
        if line.trim().is_empty() {
            continue;
        }
        let out = tmp_dir.join(format!("{i:03}-step.wav"));
        piper_one(&model, &line, &out)?;
        parts.push(out);
    }

    let list_path = tmp_dir.join("concat.txt");
    let mut list = String::new();
    for p in &parts {
        list.push_str(&format!("file '{}'\n", p.display()));
    }
    std::fs::write(&list_path, list)?;
    let out_wav = plan
        .remotion_root
        .join("public")
        .join("audio")
        .join(format!("{}.voiceover.wav", plan.journey_id));
    if let Some(parent) = out_wav.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let status = Command::new("ffmpeg")
        .args(["-y", "-f", "concat", "-safe", "0", "-i"])
        .arg(&list_path)
        .args(["-c", "copy"])
        .arg(&out_wav)
        .status()?;
    if !status.success() {
        return Err(RenderError::BadManifest("ffmpeg concat failed".into()));
    }
    Ok(format!("audio/{}.voiceover.wav", plan.journey_id))
}

fn piper_one(model: &Path, text: &str, out_wav: &Path) -> Result<(), RenderError> {
    use std::io::Write;
    let mut child = Command::new("piper")
        .arg("--model")
        .arg(model)
        .arg("--output_file")
        .arg(out_wav)
        .stdin(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()?;
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(text.as_bytes())?;
    }
    let out = child.wait_with_output()?;
    if !out.status.success() {
        return Err(RenderError::BadManifest(format!(
            "piper failed: {}",
            String::from_utf8_lossy(&out.stderr)
        )));
    }
    Ok(())
}

/// Build the enriched manifest (merge base + sidecar scene-spec + voiceover
/// metadata). Writes `<keyframes_dir>/manifest.rich.json` and returns it.
pub fn build_rich_manifest(plan: &RenderPlan) -> Result<PathBuf, RenderError> {
    let base_raw = std::fs::read_to_string(&plan.manifest_path)?;
    let mut rich: RichManifest = serde_json::from_str(&base_raw)
        .map_err(|e| RenderError::BadManifest(format!("parse base manifest: {e}")))?;

    // Merge sidecar scene spec if present.
    if let Some(spec_path) = &plan.scene_spec {
        if spec_path.exists() {
            let spec_raw = std::fs::read_to_string(spec_path)?;
            let scenes: Vec<SceneSpec> = serde_json::from_str(&spec_raw)
                .map_err(|e| RenderError::BadManifest(format!("scene spec: {e}")))?;
            rich.scenes = Some(scenes);
        }
    }
    // Voiceover backend:
    //   `silent` -> no audio, backend="silent"
    //   `piper`  -> hard-require Piper; error if missing
    //   `auto`   -> try Piper; log + fall back to silent on any error
    let (effective_backend, audio) = match plan.voiceover.as_str() {
        "silent" => ("silent".to_string(), None),
        "piper" => ("piper".to_string(), Some(synthesise_voiceover_piper(plan)?)),
        "auto" | "" => match synthesise_voiceover_piper(plan) {
            Ok(path) => ("piper".to_string(), Some(path)),
            Err(e) => {
                eprintln!(
                    "[journey-render] piper unavailable for {} ({e}); continuing silent",
                    plan.journey_id
                );
                ("silent".to_string(), None)
            }
        },
        other => {
            return Err(RenderError::BadManifest(format!(
                "unknown voiceover backend `{other}`: expected auto|piper|silent"
            )));
        }
    };
    rich.voiceover = Some(VoiceoverSpec {
        backend: effective_backend,
        lines: None,
        audio,
    });
    rich.recording_rich =
        Some(format!("recordings/{}/{}.rich.mp4", plan.journey_id, plan.journey_id));

    let out = plan.keyframes_dir.join("manifest.rich.json");
    std::fs::write(&out, serde_json::to_string_pretty(&rich)?)?;
    Ok(out)
}

/// Invoke `bun run src/annotate.ts` to composite annotated keyframes.
pub fn annotate(plan: &RenderPlan, rich_manifest_path: &Path) -> Result<(), RenderError> {
    ensure_bun()?;
    let out = Command::new("bun")
        .current_dir(&plan.remotion_root)
        .args(["run", "src/annotate.ts", "--manifest"])
        .arg(rich_manifest_path)
        .args(["--keyframes-dir"])
        .arg(&plan.keyframes_dir)
        .output()?;
    if !out.status.success() {
        return Err(RenderError::AnnotateFailed {
            code: out.status.code().unwrap_or(-1),
            stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
        });
    }
    Ok(())
}

/// Invoke `bun run remotion render` with inline props and write MP4.
pub fn render(plan: &RenderPlan, rich_manifest_path: &Path) -> Result<(), RenderError> {
    ensure_bun()?;
    let rich_raw = std::fs::read_to_string(rich_manifest_path)?;
    let rich: RichManifest = serde_json::from_str(&rich_raw)?;
    let props = serde_json::json!({
        "journeyId": plan.journey_id,
        "manifest": rich,
        "keyframeBase": format!("keyframes/{}", plan.journey_id),
    });
    let props_str = serde_json::to_string(&props)?;

    // Ensure parent dir exists.
    if let Some(parent) = plan.output_mp4.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let out = Command::new("bun")
        .current_dir(&plan.remotion_root)
        .args(["x", "remotion", "render", "src/index.tsx", "JourneyRich", "--props"])
        .arg(&props_str)
        .arg("--output")
        .arg(&plan.output_mp4)
        .output()?;
    if !out.status.success() {
        return Err(RenderError::RenderFailed {
            code: out.status.code().unwrap_or(-1),
            stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
        });
    }
    Ok(())
}

/// End-to-end: build rich manifest → annotate → render.
pub fn run(plan: &RenderPlan) -> Result<PathBuf, RenderError> {
    let rich = build_rich_manifest(plan)?;
    annotate(plan, &rich)?;
    render(plan, &rich)?;
    Ok(plan.output_mp4.clone())
}

fn ensure_bun() -> Result<(), RenderError> {
    match Command::new("bun").arg("--version").output() {
        Ok(o) if o.status.success() => Ok(()),
        _ => Err(RenderError::BunMissing),
    }
}
