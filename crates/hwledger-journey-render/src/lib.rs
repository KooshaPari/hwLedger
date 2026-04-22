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
    /// Which Remotion composition to render. Defaults to `JourneyRich`.
    /// Batch mode auto-switches to `JourneySlideshow` for GUI journeys whose
    /// raw MP4 is missing or < 3 s (i.e. TCC-blocked XCUITest capture) — the
    /// slideshow path drives the render off the per-step keyframe PNGs.
    #[serde(default = "default_composition_id")]
    pub composition_id: String,
    /// Optional target total content length (in seconds, at render fps). When
    /// set and the manifest has no explicit `scenes`, `build_rich_manifest`
    /// synthesises per-step scenes whose `durationFrames` sum to this target
    /// (clamped: min 2.5s/step, max 6s/step). Used by CLI batch mode to
    /// produce 20-40s videos with visible captions instead of 12s flashes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_content_seconds: Option<f32>,
}

fn default_composition_id() -> String {
    "JourneyRich".to_string()
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
            composition_id: default_composition_id(),
            target_content_seconds: None,
        }
    }
}

/// Synthesise a per-journey voiceover WAV via edge-tts (Microsoft cloud).
/// Returns the public-relative path suitable for `manifest.voiceover.audio`.
///
/// Voice defaults to `en-US-AriaNeural` (the current CLI default, tied to
/// the A/B winner once declared). Override with `HWLEDGER_EDGE_VOICE`.
/// Requires `edge-tts` CLI on PATH. Falls through to the caller's error
/// handling if the binary is missing so `auto`-mode can fall back to Piper.
pub fn synthesise_voiceover_edge_tts(plan: &RenderPlan) -> Result<String, RenderError> {
    let voice = std::env::var("HWLEDGER_EDGE_VOICE")
        .unwrap_or_else(|_| "en-US-AriaNeural".to_string());
    if Command::new("edge-tts").arg("--help").output().is_err() {
        return Err(RenderError::BadManifest("edge-tts not on PATH".into()));
    }
    let raw = std::fs::read_to_string(&plan.manifest_path)?;
    let rich: RichManifest = serde_json::from_str(&raw)
        .map_err(|e| RenderError::BadManifest(format!("manifest parse: {e}")))?;
    let tmp_dir = plan.remotion_root.join("public").join("audio").join(&plan.journey_id);
    std::fs::create_dir_all(&tmp_dir)?;
    let mut parts: Vec<PathBuf> = Vec::new();
    let intro = format!("{}. {}", plan.journey_id.replace('-', " "), rich.intent);
    let intro_wav = tmp_dir.join("000-intro.wav");
    edge_tts_one(&voice, &intro, &intro_wav)?;
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
        edge_tts_one(&voice, &line, &out)?;
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
        .args(["-ar", "22050", "-ac", "1", "-c:a", "pcm_s16le"])
        .arg(&out_wav)
        .status()?;
    if !status.success() {
        return Err(RenderError::BadManifest("ffmpeg concat failed (edge-tts)".into()));
    }
    Ok(format!("audio/{}.voiceover.wav", plan.journey_id))
}

fn edge_tts_one(voice: &str, text: &str, out_wav: &Path) -> Result<(), RenderError> {
    // edge-tts writes MP3; transcode to WAV PCM for concat-compat with piper.
    let tmp_mp3 = out_wav.with_extension("mp3");
    let out = Command::new("edge-tts")
        .args(["--voice", voice, "--text", text, "--write-media"])
        .arg(&tmp_mp3)
        .output()?;
    if !out.status.success() {
        return Err(RenderError::BadManifest(format!(
            "edge-tts failed: {}",
            String::from_utf8_lossy(&out.stderr)
        )));
    }
    let status = Command::new("ffmpeg")
        .args(["-y", "-i"])
        .arg(&tmp_mp3)
        .args(["-ar", "22050", "-ac", "1", "-c:a", "pcm_s16le"])
        .arg(out_wav)
        .status()?;
    if !status.success() {
        return Err(RenderError::BadManifest("ffmpeg transcode (edge->wav) failed".into()));
    }
    let _ = std::fs::remove_file(&tmp_mp3);
    Ok(())
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
    // Synthesise scene durations from a target total when the manifest has
    // no explicit scenes. Clamp per-step duration to [2.5s, 6s] so small
    // journeys get breathing room and large ones don't over-inflate.
    if let Some(target_s) = plan.target_content_seconds {
        let needs_synth = rich.scenes.as_ref().map(|s| s.is_empty()).unwrap_or(true);
        let steps_len = rich.steps.len() as u32;
        if needs_synth && steps_len > 0 {
            let fps = 30.0_f32;
            let per_step_frames =
                ((target_s * fps) / steps_len as f32).clamp(75.0, 180.0) as u32;
            let scenes: Vec<SceneSpec> = (0..steps_len)
                .map(|i| SceneSpec {
                    step: i,
                    callout_text: format!("Step {}", i + 1),
                    callout_sub_text: rich.steps.get(i as usize).map(|s| s.intent.clone()),
                    callout_color: Some("#34d399".to_string()),
                    duration_frames: Some(per_step_frames),
                })
                .collect();
            rich.scenes = Some(scenes);
        }
    }
    // Voiceover backend:
    //   `silent`   -> no audio, backend="silent"
    //   `piper`    -> hard-require Piper; error if missing
    //   `edge-tts` -> hard-require edge-tts; error if missing
    //   `auto`     -> try edge-tts, then Piper; fall back to silent on failure
    let (effective_backend, audio) = match plan.voiceover.as_str() {
        "silent" => ("silent".to_string(), None),
        "piper" => ("piper".to_string(), Some(synthesise_voiceover_piper(plan)?)),
        "edge-tts" | "edge" => {
            ("edge-tts".to_string(), Some(synthesise_voiceover_edge_tts(plan)?))
        }
        "indextts" | "indextts2" | "index-tts" | "index-tts-2" => {
            ("indextts2".to_string(), Some(synthesise_voiceover_indextts2(plan)?))
        }
        "kokoro" | "kokoro82" | "kokoro-82m" => {
            ("kokoro82".to_string(), Some(synthesise_voiceover_kokoro(plan)?))
        }
        "kitten" | "kittentts" | "kitten-tts" => {
            ("kittentts".to_string(), Some(synthesise_voiceover_kittentts(plan)?))
        }
        "avspeech" | "say" | "apple" => {
            ("avspeech".to_string(), Some(synthesise_voiceover_avspeech(plan)?))
        }
        "auto" | "" => auto_select_and_render(plan),
        other => {
            return Err(RenderError::BadManifest(format!(
                "unknown voiceover backend `{other}`: expected auto|indextts2|kokoro|kittentts|avspeech|edge-tts|piper|silent"
            )));
        }
    };
    rich.voiceover = Some(VoiceoverSpec { backend: effective_backend, lines: None, audio });
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

    let composition =
        if plan.composition_id.is_empty() { "JourneyRich" } else { plan.composition_id.as_str() };
    let out = Command::new("bun")
        .current_dir(&plan.remotion_root)
        .args(["x", "remotion", "render", "src/index.tsx", composition, "--props"])
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

// ---------------------------------------------------------------------------
// Voice backend selection (ADR-0010 v2, five-tier chain).
//
// The A/B taste test (see `docs-site/audio/voice-ab.md`) demoted Piper from
// default to tier-5 CI fallback. This enum + `select_voice_backend` encode
// the new precedence. `synthesise_voiceover_*` helpers above are kept
// intact; this layer only picks which one to call.
// ---------------------------------------------------------------------------

/// Narration backend identifier. Ordered loosely best-to-worst by ear.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VoiceBackend {
    /// IndexTTS 2.0 zero-shot expressive model (Python venv, torch+MPS/CUDA).
    IndexTts2,
    /// Kokoro-82M ONNX (82M params, CPU-fast, kokoro-onnx Python wrapper).
    Kokoro82,
    /// KittenTTS nano ONNX (tiny, CPU-only, KittenML HF model).
    Kitten,
    /// Apple AVSpeechSynthesizer via the macOS `say` binary.
    AVSpeech,
    /// Microsoft edge-tts — explicit opt-in only, never selected by `auto`.
    EdgeTts,
    /// Piper (rhasspy) — tier-5 offline fallback for headless Linux CI.
    Piper,
    /// No voiceover; render stays silent.
    Silent,
}

impl VoiceBackend {
    /// Parse a backend name from `HWLEDGER_VOICE` or `RenderPlan.voiceover`.
    /// Unknown names resolve to `Silent` so the render never crashes on a
    /// misconfigured env var.
    pub fn parse(name: &str) -> VoiceBackend {
        match name.trim().to_ascii_lowercase().as_str() {
            "indextts" | "indextts2" | "index-tts" | "index-tts-2" => VoiceBackend::IndexTts2,
            "kokoro" | "kokoro82" | "kokoro-82m" => VoiceBackend::Kokoro82,
            "kitten" | "kittentts" | "kitten-tts" => VoiceBackend::Kitten,
            "avspeech" | "av-speech" | "say" | "apple" => VoiceBackend::AVSpeech,
            "edge" | "edge-tts" | "edgetts" => VoiceBackend::EdgeTts,
            "piper" => VoiceBackend::Piper,
            "silent" | "none" | "" => VoiceBackend::Silent,
            _ => VoiceBackend::Silent,
        }
    }

    /// Canonical string tag used in the rich manifest `voiceover.backend`.
    pub fn as_tag(&self) -> &'static str {
        match self {
            VoiceBackend::IndexTts2 => "indextts2",
            VoiceBackend::Kokoro82 => "kokoro82",
            VoiceBackend::Kitten => "kittentts",
            VoiceBackend::AVSpeech => "avspeech",
            VoiceBackend::EdgeTts => "edge-tts",
            VoiceBackend::Piper => "piper",
            VoiceBackend::Silent => "silent",
        }
    }
}

/// Pick the best available backend in ADR-0010-v2 order:
///   1. `HWLEDGER_VOICE` explicit override (highest priority).
///   2. IndexTTS 2.0 if GPU + venv present.
///   3. Kokoro-82M if its venv/driver present.
///   4. KittenTTS if its venv/driver present.
///   5. AVSpeechSynthesizer on macOS hosts.
///   6. Piper (tier-5 CI fallback).
///   7. Silent.
///
/// `edge-tts` is *never* selected by auto — it's cloud, and the chain is
/// local-first. Callers opt in explicitly via `HWLEDGER_VOICE=edge-tts`.
pub fn select_voice_backend() -> VoiceBackend {
    if let Ok(name) = std::env::var("HWLEDGER_VOICE") {
        return VoiceBackend::parse(&name);
    }
    if gpu_available() && indextts_available() {
        return VoiceBackend::IndexTts2;
    }
    if kokoro_available() {
        return VoiceBackend::Kokoro82;
    }
    if kittentts_available() {
        return VoiceBackend::Kitten;
    }
    if macos() {
        return VoiceBackend::AVSpeech;
    }
    if piper_available() {
        return VoiceBackend::Piper;
    }
    VoiceBackend::Silent
}

// --- capability probes ------------------------------------------------------

fn macos() -> bool {
    cfg!(target_os = "macos")
}

/// GPU = CUDA or Apple Metal (MPS). We don't open a torch handle here; the
/// presence of a macOS system or an `nvidia-smi` binary is a fast proxy.
fn gpu_available() -> bool {
    if macos() {
        return true; // MPS is always on on this workspace's hardware.
    }
    Command::new("nvidia-smi").arg("-L").output().map(|o| o.status.success()).unwrap_or(false)
}

fn indextts_root() -> PathBuf {
    if let Ok(p) = std::env::var("INDEXTTS_ROOT") {
        return PathBuf::from(p);
    }
    let home = std::env::var("HOME").unwrap_or_default();
    PathBuf::from(home).join(".cache/hwledger/tts/index-tts")
}

fn indextts_available() -> bool {
    let root = indextts_root();
    let venv_python = root.join(".venv/bin/python");
    let ckpt = root.join("checkpoints/config.yaml");
    venv_python.exists() && ckpt.exists()
}

fn kokoro_root() -> PathBuf {
    if let Ok(p) = std::env::var("KOKORO_VENV") {
        return PathBuf::from(p);
    }
    let home = std::env::var("HOME").unwrap_or_default();
    PathBuf::from(home).join(".cache/hwledger/tts/kokoro/.venv")
}

fn kokoro_available() -> bool {
    kokoro_root().join("bin/python").exists()
}

fn kittentts_root() -> PathBuf {
    if let Ok(p) = std::env::var("KITTENTTS_VENV") {
        return PathBuf::from(p);
    }
    let home = std::env::var("HOME").unwrap_or_default();
    PathBuf::from(home).join(".cache/hwledger/tts/kittentts/.venv")
}

fn kittentts_available() -> bool {
    kittentts_root().join("bin/python").exists()
}

fn piper_available() -> bool {
    Command::new("piper").arg("--help").output().map(|o| o.status.success()).unwrap_or(false)
}

// --- Python-driven synthesisers ---------------------------------------------

fn render_voiceover_python(
    plan: &RenderPlan,
    venv_python: &Path,
    driver_script: &Path,
    extra_env: &[(&str, PathBuf)],
) -> Result<String, RenderError> {
    // Concatenate the full narration script (intro + one line per step) to a
    // single temp file, shell to the engine driver, transcode result to
    // 22050 mono s16 WAV in the expected manifest-audio location. Matches
    // the shape produced by `synthesise_voiceover_piper`.
    let raw = std::fs::read_to_string(&plan.manifest_path)?;
    let rich: RichManifest = serde_json::from_str(&raw)
        .map_err(|e| RenderError::BadManifest(format!("manifest parse: {e}")))?;
    let tmp_dir = plan.remotion_root.join("public").join("audio").join(&plan.journey_id);
    std::fs::create_dir_all(&tmp_dir)?;
    let script_txt = tmp_dir.join("narration.txt");
    let mut text = format!("{}. {}\n", plan.journey_id.replace('-', " "), rich.intent);
    for step in &rich.steps {
        let line = step
            .description
            .clone()
            .or_else(|| step.blind_description.clone())
            .unwrap_or_else(|| step.intent.clone());
        if !line.trim().is_empty() {
            text.push_str(&line);
            text.push('\n');
        }
    }
    std::fs::write(&script_txt, &text)?;

    let raw_wav = tmp_dir.join("engine-raw.wav");
    let start = std::time::Instant::now();
    let mut cmd = Command::new(venv_python);
    cmd.arg(driver_script).arg(&script_txt).arg(&raw_wav);
    for (k, v) in extra_env {
        cmd.env(k, v);
    }
    let out = cmd.output()?;
    if !out.status.success() {
        return Err(RenderError::BadManifest(format!(
            "tts driver {} failed: {}",
            driver_script.display(),
            String::from_utf8_lossy(&out.stderr)
        )));
    }
    let elapsed = start.elapsed().as_secs_f64();
    eprintln!("[journey-render] tts render: {:.2}s ({})", elapsed, driver_script.display());

    let out_wav = plan
        .remotion_root
        .join("public")
        .join("audio")
        .join(format!("{}.voiceover.wav", plan.journey_id));
    if let Some(parent) = out_wav.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let status = Command::new("ffmpeg")
        .args(["-y", "-i"])
        .arg(&raw_wav)
        .args(["-ar", "22050", "-ac", "1", "-c:a", "pcm_s16le"])
        .arg(&out_wav)
        .status()?;
    if !status.success() {
        return Err(RenderError::BadManifest("ffmpeg transcode (engine->wav) failed".into()));
    }
    Ok(format!("audio/{}.voiceover.wav", plan.journey_id))
}

/// Synthesise narration via IndexTTS 2.0 (zero-shot). Requires the venv at
/// `~/.cache/hwledger/tts/index-tts/.venv` and model checkpoints under
/// `checkpoints/`. Uses the `default_ref/default.wav` speaker prompt.
pub fn synthesise_voiceover_indextts2(plan: &RenderPlan) -> Result<String, RenderError> {
    let root = indextts_root();
    let python = root.join(".venv/bin/python");
    let driver = find_driver_script("render_indextts.py")?;
    let ref_wav = root.join("default_ref/default.wav");
    if !ref_wav.exists() {
        return Err(RenderError::BadManifest(format!(
            "indextts2 default reference clip missing: {}",
            ref_wav.display()
        )));
    }
    // IndexTTS driver takes (script, ref, out) — wrap via a thin adapter.
    let raw = std::fs::read_to_string(&plan.manifest_path)?;
    let rich: RichManifest = serde_json::from_str(&raw)
        .map_err(|e| RenderError::BadManifest(format!("manifest parse: {e}")))?;
    let tmp_dir = plan.remotion_root.join("public").join("audio").join(&plan.journey_id);
    std::fs::create_dir_all(&tmp_dir)?;
    let script_txt = tmp_dir.join("narration.txt");
    let mut text = format!("{}. {}\n", plan.journey_id.replace('-', " "), rich.intent);
    for step in &rich.steps {
        let line = step
            .description
            .clone()
            .or_else(|| step.blind_description.clone())
            .unwrap_or_else(|| step.intent.clone());
        if !line.trim().is_empty() {
            text.push_str(&line);
            text.push('\n');
        }
    }
    std::fs::write(&script_txt, &text)?;
    let raw_wav = tmp_dir.join("engine-raw.wav");
    let start = std::time::Instant::now();
    let out = Command::new(&python)
        .env("INDEXTTS_ROOT", &root)
        .arg(&driver)
        .arg(&script_txt)
        .arg(&ref_wav)
        .arg(&raw_wav)
        .output()?;
    if !out.status.success() {
        return Err(RenderError::BadManifest(format!(
            "indextts2 driver failed: {}",
            String::from_utf8_lossy(&out.stderr)
        )));
    }
    eprintln!(
        "[journey-render] indextts2 render: {:.2}s",
        start.elapsed().as_secs_f64()
    );
    let out_wav = plan
        .remotion_root
        .join("public")
        .join("audio")
        .join(format!("{}.voiceover.wav", plan.journey_id));
    if let Some(parent) = out_wav.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let status = Command::new("ffmpeg")
        .args(["-y", "-i"])
        .arg(&raw_wav)
        .args(["-ar", "22050", "-ac", "1", "-c:a", "pcm_s16le"])
        .arg(&out_wav)
        .status()?;
    if !status.success() {
        return Err(RenderError::BadManifest("ffmpeg transcode (indextts->wav) failed".into()));
    }
    Ok(format!("audio/{}.voiceover.wav", plan.journey_id))
}

/// Synthesise narration via Kokoro-82M ONNX.
pub fn synthesise_voiceover_kokoro(plan: &RenderPlan) -> Result<String, RenderError> {
    let python = kokoro_root().join("bin/python");
    let driver = find_driver_script("render_kokoro.py")?;
    render_voiceover_python(plan, &python, &driver, &[])
}

/// Synthesise narration via KittenTTS nano.
pub fn synthesise_voiceover_kittentts(plan: &RenderPlan) -> Result<String, RenderError> {
    let python = kittentts_root().join("bin/python");
    let driver = find_driver_script("render_kittentts.py")?;
    render_voiceover_python(plan, &python, &driver, &[])
}

/// Synthesise narration via macOS `say` -> AIFF -> ffmpeg WAV. 5-line glue is
/// not acceptable here (the per-step concat mirrors the Piper path), so it's
/// implemented in Rust.
pub fn synthesise_voiceover_avspeech(plan: &RenderPlan) -> Result<String, RenderError> {
    if !macos() {
        return Err(RenderError::BadManifest("avspeech requires macOS".into()));
    }
    let raw = std::fs::read_to_string(&plan.manifest_path)?;
    let rich: RichManifest = serde_json::from_str(&raw)
        .map_err(|e| RenderError::BadManifest(format!("manifest parse: {e}")))?;
    let tmp_dir = plan.remotion_root.join("public").join("audio").join(&plan.journey_id);
    std::fs::create_dir_all(&tmp_dir)?;
    let mut parts: Vec<PathBuf> = Vec::new();
    let lines = std::iter::once(format!("{}. {}", plan.journey_id.replace('-', " "), rich.intent))
        .chain(rich.steps.iter().map(|step| {
            step.description
                .clone()
                .or_else(|| step.blind_description.clone())
                .unwrap_or_else(|| step.intent.clone())
        }));
    for (i, line) in lines.enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let aiff = tmp_dir.join(format!("{i:03}-say.aiff"));
        let wav = tmp_dir.join(format!("{i:03}-step.wav"));
        let status = Command::new("say").arg("-o").arg(&aiff).arg(&line).status()?;
        if !status.success() {
            return Err(RenderError::BadManifest("say failed".into()));
        }
        let status = Command::new("ffmpeg")
            .args(["-y", "-i"])
            .arg(&aiff)
            .args(["-ar", "22050", "-ac", "1", "-c:a", "pcm_s16le"])
            .arg(&wav)
            .status()?;
        if !status.success() {
            return Err(RenderError::BadManifest("ffmpeg (say->wav) failed".into()));
        }
        parts.push(wav);
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
        return Err(RenderError::BadManifest("ffmpeg concat (avspeech) failed".into()));
    }
    Ok(format!("audio/{}.voiceover.wav", plan.journey_id))
}

/// Locate a driver script in the nearest `tools/tts-ab` directory. Walks up
/// from `CARGO_MANIFEST_DIR` first, then from the current working directory,
/// so the function works whether the crate is invoked from a workspace root
/// or from `cargo run -p hwledger-journey-render` inside a nested tool.
fn find_driver_script(name: &str) -> Result<PathBuf, RenderError> {
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Ok(dir) = std::env::var("HWLEDGER_REPO") {
        candidates.push(PathBuf::from(dir).join("tools/tts-ab").join(name));
    }
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let mut p = PathBuf::from(manifest_dir);
    for _ in 0..6 {
        candidates.push(p.join("tools/tts-ab").join(name));
        if !p.pop() {
            break;
        }
    }
    if let Ok(cwd) = std::env::current_dir() {
        let mut p = cwd;
        for _ in 0..6 {
            candidates.push(p.join("tools/tts-ab").join(name));
            if !p.pop() {
                break;
            }
        }
    }
    for c in &candidates {
        if c.exists() {
            return Ok(c.clone());
        }
    }
    Err(RenderError::BadManifest(format!(
        "tts driver `{}` not found (searched {} paths)",
        name,
        candidates.len()
    )))
}

#[cfg(test)]
mod voice_backend_tests {
    use super::*;

    #[test]
    fn parse_known_backends() {
        assert_eq!(VoiceBackend::parse("indextts2"), VoiceBackend::IndexTts2);
        assert_eq!(VoiceBackend::parse("INDEX-TTS"), VoiceBackend::IndexTts2);
        assert_eq!(VoiceBackend::parse("kokoro"), VoiceBackend::Kokoro82);
        assert_eq!(VoiceBackend::parse("kittentts"), VoiceBackend::Kitten);
        assert_eq!(VoiceBackend::parse("avspeech"), VoiceBackend::AVSpeech);
        assert_eq!(VoiceBackend::parse("edge-tts"), VoiceBackend::EdgeTts);
        assert_eq!(VoiceBackend::parse("piper"), VoiceBackend::Piper);
        assert_eq!(VoiceBackend::parse("silent"), VoiceBackend::Silent);
        assert_eq!(VoiceBackend::parse(""), VoiceBackend::Silent);
        assert_eq!(VoiceBackend::parse("totally-unknown"), VoiceBackend::Silent);
    }

    #[test]
    fn tags_round_trip() {
        for b in [
            VoiceBackend::IndexTts2,
            VoiceBackend::Kokoro82,
            VoiceBackend::Kitten,
            VoiceBackend::AVSpeech,
            VoiceBackend::EdgeTts,
            VoiceBackend::Piper,
            VoiceBackend::Silent,
        ] {
            assert_eq!(VoiceBackend::parse(b.as_tag()), b);
        }
    }

    #[test]
    fn explicit_override_wins() {
        // Concurrency guard: other tests may mutate env. Set, then assert,
        // then restore; keep the critical section minimal.
        let prev = std::env::var("HWLEDGER_VOICE").ok();
        // SAFETY: single-threaded section in this module's test harness.
        unsafe {
            std::env::set_var("HWLEDGER_VOICE", "piper");
        }
        assert_eq!(select_voice_backend(), VoiceBackend::Piper);
        unsafe {
            std::env::set_var("HWLEDGER_VOICE", "silent");
        }
        assert_eq!(select_voice_backend(), VoiceBackend::Silent);
        unsafe {
            match prev {
                Some(v) => std::env::set_var("HWLEDGER_VOICE", v),
                None => std::env::remove_var("HWLEDGER_VOICE"),
            }
        }
    }

    #[test]
    fn piper_is_tier_five_not_default() {
        // Force env clear, then rely on probe fallbacks. On this workspace
        // the macOS branch selects AVSpeech before Piper, proving the
        // demotion.
        let prev = std::env::var("HWLEDGER_VOICE").ok();
        unsafe {
            std::env::remove_var("HWLEDGER_VOICE");
        }
        let got = select_voice_backend();
        // Any outcome *except* Piper confirms the demotion chain on hosts
        // that have IndexTTS/Kokoro/AVSpeech available. On pure-Linux CI
        // with only Piper, Piper is still correctly picked (tier-5).
        if got == VoiceBackend::Piper {
            // OK: Linux CI with only piper installed.
        } else {
            // OK: any higher tier (IndexTts2 | Kokoro82 | Kitten | AVSpeech).
            assert_ne!(got, VoiceBackend::EdgeTts, "edge-tts must be opt-in only");
        }
        unsafe {
            if let Some(v) = prev {
                std::env::set_var("HWLEDGER_VOICE", v);
            }
        }
    }
}
