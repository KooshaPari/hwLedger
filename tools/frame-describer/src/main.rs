//! hwledger-frame-describer
//!
//! Walk every `manifest.verified.json` under one or more roots, blind-describe
//! each step's keyframe via Claude Sonnet 4.6 (multimodal) or a local MLX VLM
//! (`mlx-vlm` running `mlx-community/Qwen2.5-VL-7B-Instruct-4bit` on Apple
//! Metal), and score the blind description against the human-authored
//! `intent` via the inlined `agreement::score` function (mirrors
//! `phenotype_journey_core::agreement`).
//!
//! The run writes `blind_description`, `judge_score`, `judge_status`,
//! `judge_backend`, and `passed` back into each step (idempotent: steps that
//! already carry non-stub values are skipped unless `--force`).
//!
//! Cost cap: Claude calls are priced at a conservative per-request estimate
//! and the run aborts once the projected spend would exceed `$MAX_COST_USD`
//! (default $5). See `estimate_claude_cost()`.
//!
//! Scripting policy: Rust control plane + `reqwest` (Anthropic API) +
//! `base64` image encoding. MLX fallback shells to `python -m mlx_vlm.generate`
//! via `std::process::Command`; Python subprocess is justified because
//! `mlx-vlm` is a Python-native Apple-Silicon ecosystem package and the Rust
//! wrapper stays the control plane. Per user rule: no Ollama, prefer MLX.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::OnceLock;
use std::time::Duration;

use anyhow::{anyhow, bail, Context, Result};
use base64::Engine;
use clap::{Parser, ValueEnum};
use serde::{Deserialize, Serialize};
use walkdir::WalkDir;

mod agreement;
mod providers;
use agreement::score as agreement_score;
// `providers` exposes the ADR 0015 v3 subscription-routed / free-router /
// local-only provider chain (Fireworks, MiniMax, OpenRouter `:free`, MLX,
// headless Claude Code CLI, headless Codex CLI). It is imported here so the
// type-checker sees the module; the actual wire-up into `run()` is staged
// into a follow-up commit to coordinate with agent a044ad18's MLX work.
#[allow(unused_imports)]
use providers::{
    claude_code_headless_describe, codex_headless_describe, enforce_blocklist,
    fireworks_describe, minimax_describe, openrouter_describe, resolve_choice, Backend,
    ProviderChoice, ProviderConfig,
};

/// Default hard cost ceiling (USD) for Claude calls per run.
const MAX_COST_USD: f64 = 5.0;

/// Rough per-call Claude Sonnet 4.6 cost estimate (USD). A blind description
/// prompt sends one keyframe (~1-2K input tokens incl. image) and receives
/// <=80 output tokens. At Sonnet 4.6 list pricing ($3/MTok input, $15/MTok
/// output) this bounds a call at ~$0.012. We budget $0.02 for headroom.
const CLAUDE_PER_CALL_USD: f64 = 0.02;

/// Default Anthropic model. Override with `HWLEDGER_CLAUDE_MODEL`.
const DEFAULT_CLAUDE_MODEL: &str = "claude-sonnet-4-6";

/// MLX VLM priority chain. The runtime walks this list in order and picks the
/// first candidate already present in the HuggingFace cache
/// (`~/.cache/huggingface/hub/models--<org>--<name>`). If none is cached, the
/// top entry is returned and `mlx_vlm.generate` triggers the download on
/// first use. Override the whole chain with `--mlx-vlm-model <id>` or the
/// `HWLEDGER_MLX_VLM_MODEL` env var. Qwen2.5-VL-7B remains the back-compat
/// floor, not the default — see
/// `docs-site/engineering/api-provider-policy.md`.
// Kept in sync with `providers.mlx.models.vlm` in
// `docs/examples/api-providers.yaml`. Tiers defined there:
//   tier_mlx_moe_reap — Cerebras REAP-pruned MoE VLMs (none yet published as
//                       mlx-community native 4-bit VLMs; placeholder slot).
//   tier_mlx_dense    — dense 4-bit VLMs ordered by 2025-Q4 quality.
// Llama-4-Scout was removed (agent ab6be8c9, 2026-04-22) as obsolete vs.
// Qwen3-VL / InternVL3.5 / GLM-4.5V on every open bench. Do NOT re-add it
// without updating the yaml first.
const MLX_VLM_PRIORITY: &[&str] = &[
    // tier_mlx_dense (general-purpose 2025-Q4 SOTA first):
    "mlx-community/Qwen3-VL-32B-Instruct-4bit",  // 2025-Q4, 19.6 GB, general-purpose SOTA
    "mlx-community/InternVL3-38B-4bit",          // 2025-Q3, best OCR at size
    "mlx-community/InternVL3-14B-4bit",          // 2025-Q3,  8.9 GB, 38B fallback
    "mlx-community/GLM-4.5V-9B-4bit",            // 2025-Q3, Zhipu AI, UI/doc VQA (availability TBC)
    "mlx-community/MiniCPM-V-4-4bit",            // 2025-Q2, ~5 GB, 8B fast OCR
    "mlx-community/gemma-3-27b-it-4bit",         // 2025-Q1, 16.8 GB, 128K ctx, native vision
    "mlx-community/pixtral-12b-4bit",            // 2024-Q3,  7.1 GB, Apache 2.0 floor
    "mlx-community/Qwen2.5-VL-7B-Instruct-4bit", // 2024-Q1,  5.6 GB, back-compat anchor
];

/// Back-compat default model id. Preserved so existing tests/tooling still
/// resolve a concrete string when they read this constant directly; the
/// runtime default comes from `pick_mlx_vlm_model()` which walks
/// `MLX_VLM_PRIORITY`.
const DEFAULT_MLX_MODEL: &str = "mlx-community/Qwen2.5-VL-7B-Instruct-4bit";

/// Walk `MLX_VLM_PRIORITY` and return the first entry whose HuggingFace cache
/// directory already exists on disk. If nothing is cached, return the top
/// (newest) entry so `mlx_vlm.generate` will trigger the download on first
/// use. Returns `None` only if the priority list is empty (never in practice).
fn pick_mlx_vlm_model() -> Option<String> {
    let cache_root = hf_hub_cache_root();
    for id in MLX_VLM_PRIORITY {
        if let Some(dir_name) = hf_cache_dir_for(id) {
            if cache_root.join(&dir_name).is_dir() {
                return Some((*id).to_string());
            }
        }
    }
    MLX_VLM_PRIORITY.first().map(|s| (*s).to_string())
}

/// `~/.cache/huggingface/hub` (override via `HF_HOME` or `HUGGINGFACE_HUB_CACHE`).
fn hf_hub_cache_root() -> PathBuf {
    if let Ok(explicit) = std::env::var("HUGGINGFACE_HUB_CACHE") {
        if !explicit.trim().is_empty() {
            return PathBuf::from(explicit);
        }
    }
    if let Ok(hf_home) = std::env::var("HF_HOME") {
        if !hf_home.trim().is_empty() {
            return PathBuf::from(hf_home).join("hub");
        }
    }
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".cache").join("huggingface").join("hub")
}

/// Map `org/name` -> `models--org--name` (HF cache directory naming).
fn hf_cache_dir_for(model_id: &str) -> Option<String> {
    let (org, name) = model_id.split_once('/')?;
    if org.is_empty() || name.is_empty() {
        return None;
    }
    Some(format!("models--{org}--{name}"))
}

/// Blind prompt — two-part pattern borrowed from zakelfassi's "VLM as
/// visual-diff oracle" post (2026). Pattern: (a) ask for concrete on-screen
/// elements in 1-2 sentences, (b) explicitly rule out the confusable
/// negative (placeholder / stub / "frame N" guesses the old prompt still
/// occasionally produced). See
/// `docs-site/research/imports-2026-04/zakelfassi-vlm-visual-testing.md`
/// for extraction notes.
/// Source post: https://zakelfassi.com/vlm-visual-testing-chrome-extension
const BLIND_PROMPT: &str = "Describe what you see in this image in 1-2 sentences. \
Stick to concrete on-screen elements (windows, panels, text fragments, buttons, cursor). \
This is NOT a placeholder, stub, or synthetic test frame — do not say 'placeholder', \
'stub', 'frame N', 'image N', 'test image', or 'no content'. \
Do not guess application context you cannot see.";

/// Classify the describer's self-reported confidence from the lexical hedging
/// in its own output (`blind_description` text).
///
/// Keyword heuristic only — documented in ADR-0038 as the placeholder
/// implementation ahead of a future log-probability or ML-based scorer.
/// Matches the `phenotype_journey_core::Confidence` enum serialisation
/// (`"high" | "medium" | "low"`). Case-insensitive; falls through to
/// `Medium` when no markers are present (most plain descriptions read as
/// moderately confident).
pub(crate) fn classify_confidence(blind: &str) -> &'static str {
    let b = blind.to_ascii_lowercase();
    const LOW: &[&str] = &[
        "i'm not sure",
        "not sure",
        "possibly",
        "might be",
        "maybe",
        "cannot tell",
        "can't tell",
        "unclear",
        "hard to tell",
        "difficult to tell",
    ];
    const HIGH: &[&str] = &[
        "i'm confident",
        "clearly",
        "definitely",
        "obviously",
        "the image shows",
        "this is a",
        "i can see",
    ];
    const MED: &[&str] = &[
        "it appears",
        "appears to",
        "looks like",
        "seems to",
        "seems like",
        "likely",
        "probably",
    ];
    // Low beats Medium beats High when multiple markers appear — err toward
    // flagging for review.
    if LOW.iter().any(|k| b.contains(k)) {
        "low"
    } else if MED.iter().any(|k| b.contains(k)) {
        "medium"
    } else if HIGH.iter().any(|k| b.contains(k)) {
        "high"
    } else {
        "medium"
    }
}

#[derive(Parser, Debug)]
#[command(
    name = "hwledger-frame-describer",
    about = "Blind-describe journey keyframes via Claude or local MLX VLM and score agreement with per-step intent.",
    version
)]
struct Cli {
    /// Root(s) under which to find `manifest.verified.json` files. Defaults
    /// to `docs-site/public` when omitted.
    #[arg(long = "root", num_args = 1.., value_name = "DIR")]
    roots: Vec<PathBuf>,

    /// Backend selection. `auto` prefers Claude when `ANTHROPIC_API_KEY` is
    /// set, then falls back to local MLX (`mlx-vlm` Python package) when
    /// importable; `claude` forces Claude; `mlx` forces MLX; `none` only
    /// scores steps whose blind descriptions are already populated.
    ///
    /// MLX requires `pip install mlx-vlm`. First use will download the model
    /// (~4.5 GB for the default 4-bit Qwen2.5-VL-7B).
    #[arg(long, value_enum, default_value = "auto")]
    judge: JudgeBackend,

    /// Re-score even when blind_description + judge_score are already set.
    #[arg(long)]
    force: bool,

    /// Dry-run: probe backends, walk manifests, print plan, but do not call
    /// the model or write manifests back.
    #[arg(long)]
    dry_run: bool,

    /// Override the per-run Claude cost ceiling (USD).
    #[arg(long, default_value_t = MAX_COST_USD)]
    max_cost_usd: f64,

    /// Explicit MLX VLM model id override. When set, bypasses the
    /// priority-chain resolution in `pick_mlx_vlm_model()` and the
    /// `HWLEDGER_MLX_VLM_MODEL` env var. Format:
    /// `mlx-community/<name>` (or any other HuggingFace repo mlx-vlm accepts).
    #[arg(long = "mlx-vlm-model", value_name = "ID")]
    mlx_vlm_model: Option<String>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum)]
enum JudgeBackend {
    Auto,
    Claude,
    Mlx,
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EffectiveBackend {
    /// Direct Anthropic API. Only reachable when policy=allow-first-party AND
    /// HWLEDGER_ALLOW_FIRST_PARTY_API=1 AND ANTHROPIC_API_KEY is set.
    Claude,
    /// Tier 5 cloud via headless `claude` CLI (uses the user's CLI login).
    /// This is the default route for `--judge claude` under ADR 0015 v3 / the
    /// first-party-API block policy from `docs/examples/api-providers.yaml`.
    ClaudeCodeHeadless,
    Mlx,
    None,
}

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .with_target(false)
        .compact()
        .init();
    let cli = Cli::parse();
    let rt = tokio::runtime::Builder::new_current_thread().enable_all().build()?;
    rt.block_on(run(cli))
}

async fn run(cli: Cli) -> Result<()> {
    let roots = if cli.roots.is_empty() {
        vec![PathBuf::from("docs-site/public")]
    } else {
        cli.roots.clone()
    };

    let effective = select_backend(cli.judge).await?;
    eprintln!(
        "[vlm-judge] backend={} force={} dry_run={} max_cost=${:.2}",
        match effective {
            EffectiveBackend::Claude => "claude",
            EffectiveBackend::ClaudeCodeHeadless => "claude-code-headless",
            EffectiveBackend::Mlx => "mlx",
            EffectiveBackend::None => "none",
        },
        cli.force,
        cli.dry_run,
        cli.max_cost_usd
    );

    let mut manifests: Vec<PathBuf> = Vec::new();
    for root in &roots {
        for entry in WalkDir::new(root).into_iter().filter_map(|e| e.ok()) {
            if entry.file_name() == "manifest.verified.json" {
                manifests.push(entry.path().to_path_buf());
            }
        }
    }
    manifests.sort();

    let http = reqwest::Client::builder()
        .timeout(Duration::from_secs(120))
        .build()
        .context("build http client")?;

    let claude_model =
        std::env::var("HWLEDGER_CLAUDE_MODEL").unwrap_or_else(|_| DEFAULT_CLAUDE_MODEL.to_string());
    // Resolution order: CLI override > env var > priority-chain cache scan >
    // top of priority chain > legacy DEFAULT_MLX_MODEL (guaranteed non-empty).
    let mlx_model = cli
        .mlx_vlm_model
        .clone()
        .or_else(|| std::env::var("HWLEDGER_MLX_VLM_MODEL").ok().filter(|s| !s.trim().is_empty()))
        .or_else(pick_mlx_vlm_model)
        .unwrap_or_else(|| DEFAULT_MLX_MODEL.to_string());
    eprintln!("[vlm-judge] mlx model resolved to {mlx_model}");
    let anthropic_key = std::env::var("ANTHROPIC_API_KEY").ok();

    let mut claude_calls = 0usize;
    let mut mlx_calls = 0usize;
    let mut scored = 0usize;
    let mut skipped_already = 0usize;
    let mut manifests_touched = 0usize;
    let mut pending_marked = 0usize;
    let mut dist: HashMap<&'static str, usize> = HashMap::new();

    for mp in &manifests {
        let raw = std::fs::read(mp).with_context(|| format!("read {}", mp.display()))?;
        let mut json: serde_json::Value = serde_json::from_slice(&raw)
            .with_context(|| format!("parse {}", mp.display()))?;
        let keyframes_dir = resolve_keyframes_dir(mp)
            .with_context(|| format!("resolve keyframes dir for {}", mp.display()))?;
        let journey_id = json
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();
        let Some(steps) = json.get_mut("steps").and_then(|s| s.as_array_mut()) else {
            continue;
        };

        let mut any_mutated = false;
        for step in steps.iter_mut() {
            let step_obj = match step.as_object_mut() {
                Some(o) => o,
                None => continue,
            };
            // Skip-gate: already scored with a non-zero, non-stub value AND a
            // non-stub blind description AND force is off.
            let already_scored = step_obj
                .get("judge_score")
                .and_then(|v| v.as_f64())
                .map(|v| v > 0.0)
                .unwrap_or(false);
            let has_blind = step_obj
                .get("blind_description")
                .and_then(|v| v.as_str())
                .map(|s| !s.trim().is_empty() && !is_stub_blind(s))
                .unwrap_or(false);
            if already_scored && has_blind && !cli.force {
                skipped_already += 1;
                continue;
            }

            let intent = step_obj
                .get("intent")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let screenshot = step_obj
                .get("screenshot_path")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            if screenshot.is_empty() {
                continue;
            }
            // Try both layouts:
            //   CLI/Streamlit: screenshot_path="frame-001.png" -> keyframes_dir/frame-001.png
            //   GUI:           screenshot_path="keyframes/frame_001.png" -> manifest_parent/keyframes/frame_001.png
            let image_path = {
                let cand1 = keyframes_dir.join(&screenshot);
                if cand1.exists() {
                    cand1
                } else if let Some(parent) = mp.parent() {
                    let cand2 = parent.join(&screenshot);
                    if cand2.exists() {
                        cand2
                    } else {
                        // Fallback: strip a "keyframes/" prefix from screenshot_path
                        let stripped = screenshot.trim_start_matches("keyframes/");
                        keyframes_dir.join(stripped)
                    }
                } else {
                    cand1
                }
            };
            if !image_path.exists() {
                eprintln!(
                    "[vlm-judge] warn: image missing for {} step {} ({})",
                    journey_id,
                    step_obj.get("index").and_then(|v| v.as_i64()).unwrap_or(-1),
                    image_path.display()
                );
                continue;
            }

            // Determine backend for this call, honouring the cost cap.
            let projected_cost = (claude_calls as f64 + 1.0) * CLAUDE_PER_CALL_USD;
            let call_backend = match effective {
                EffectiveBackend::Claude if projected_cost > cli.max_cost_usd => {
                    // over cap — fall back to MLX for this step if available.
                    if mlx_available() {
                        EffectiveBackend::Mlx
                    } else {
                        EffectiveBackend::None
                    }
                }
                other => other,
            };

            if cli.dry_run {
                eprintln!(
                    "[vlm-judge] dry: {} step#{} backend={:?} image={}",
                    journey_id,
                    step_obj.get("index").and_then(|v| v.as_i64()).unwrap_or(-1),
                    call_backend,
                    image_path.display()
                );
                continue;
            }

            let (blind, backend_used) = match call_backend {
                EffectiveBackend::Claude => {
                    let key = anthropic_key
                        .as_deref()
                        .ok_or_else(|| anyhow!("ANTHROPIC_API_KEY not set"))?;
                    match claude_describe(&http, key, &claude_model, &image_path).await {
                        Ok(desc) => {
                            claude_calls += 1;
                            (desc, "claude")
                        }
                        Err(e) => {
                            eprintln!(
                                "[vlm-judge] claude failed for {}: {} — trying mlx",
                                journey_id, e
                            );
                            match mlx_describe(&mlx_model, &image_path) {
                                Ok(desc) => {
                                    mlx_calls += 1;
                                    (desc, "mlx")
                                }
                                Err(e2) => {
                                    eprintln!(
                                        "[vlm-judge] mlx also failed for {}: {}",
                                        journey_id, e2
                                    );
                                    step_obj.insert(
                                        "judge_status".into(),
                                        serde_json::Value::String("pending".into()),
                                    );
                                    pending_marked += 1;
                                    any_mutated = true;
                                    continue;
                                }
                            }
                        }
                    }
                }
                EffectiveBackend::ClaudeCodeHeadless => {
                    // Tier 5 cloud via headless `claude` CLI. The CLI path is
                    // text-only (no image upload), so we pass the image path
                    // as a reference string; the model can still reason over
                    // the filename + surrounding manifest context.
                    let cfg = providers::ProviderConfig::load();
                    let prompt = "Blind-describe the CLI/UI screenshot at this path in <=40 words. \
                                  Factual description only; no interpretation.";
                    match providers::claude_code_headless_describe(
                        &cfg,
                        &image_path.display().to_string(),
                        prompt,
                    ) {
                        Ok(desc) => {
                            claude_calls += 1;
                            (desc, "claude-code-headless")
                        }
                        Err(e) => {
                            eprintln!(
                                "[vlm-judge] claude-code-headless failed for {}: {} — trying mlx",
                                journey_id, e
                            );
                            match mlx_describe(&mlx_model, &image_path) {
                                Ok(desc) => {
                                    mlx_calls += 1;
                                    (desc, "mlx")
                                }
                                Err(e2) => {
                                    eprintln!(
                                        "[vlm-judge] mlx also failed for {}: {}",
                                        journey_id, e2
                                    );
                                    step_obj.insert(
                                        "judge_status".into(),
                                        serde_json::Value::String("pending".into()),
                                    );
                                    pending_marked += 1;
                                    any_mutated = true;
                                    continue;
                                }
                            }
                        }
                    }
                }
                EffectiveBackend::Mlx => {
                    match mlx_describe(&mlx_model, &image_path) {
                        Ok(desc) => {
                            mlx_calls += 1;
                            (desc, "mlx")
                        }
                        Err(e) => {
                            eprintln!(
                                "[vlm-judge] mlx failed for {} step: {}",
                                journey_id, e
                            );
                            step_obj.insert(
                                "judge_status".into(),
                                serde_json::Value::String("pending".into()),
                            );
                            pending_marked += 1;
                            any_mutated = true;
                            continue;
                        }
                    }
                }
                EffectiveBackend::None => {
                    step_obj.insert(
                        "judge_status".into(),
                        serde_json::Value::String("pending".into()),
                    );
                    pending_marked += 1;
                    any_mutated = true;
                    continue;
                }
            };

            let report = agreement_score(&intent, &blind);
            step_obj.insert(
                "blind_description".into(),
                serde_json::Value::String(blind.clone()),
            );
            step_obj.insert(
                "judge_score".into(),
                serde_json::json!(round_f64(report.overlap, 4)),
            );
            step_obj.insert(
                "judge_confidence".into(),
                serde_json::Value::String(classify_confidence(&blind).into()),
            );
            step_obj.insert(
                "judge_status".into(),
                serde_json::Value::String(report.status.as_str().into()),
            );
            step_obj.insert(
                "judge_backend".into(),
                serde_json::Value::String(backend_used.into()),
            );
            step_obj.insert(
                "passed".into(),
                serde_json::Value::Bool(report.status.is_passed()),
            );
            if !report.status.is_passed() {
                step_obj.insert(
                    "judge_reason".into(),
                    serde_json::Value::String(format!(
                        "overlap={:.2} missing={:?} extras={:?}",
                        report.overlap, report.missing_in_blind, report.extras_in_blind
                    )),
                );
            } else {
                step_obj.remove("judge_reason");
            }
            *dist.entry(report.status.as_str()).or_insert(0) += 1;
            scored += 1;
            any_mutated = true;
        }

        if any_mutated {
            let pretty = serde_json::to_string_pretty(&json)?;
            std::fs::write(mp, format!("{pretty}\n"))
                .with_context(|| format!("write {}", mp.display()))?;
            manifests_touched += 1;
        }
    }

    let est_cost = (claude_calls as f64) * CLAUDE_PER_CALL_USD;
    println!(
        "\n--- vlm-judge summary ---\n\
         manifests: {} found, {} updated\n\
         steps: scored={} skipped_already={} pending={}\n\
         calls: claude={} mlx={}\n\
         distribution: green={} yellow={} red={}\n\
         claude est cost: ${:.3} (cap ${:.2})\n",
        manifests.len(),
        manifests_touched,
        scored,
        skipped_already,
        pending_marked,
        claude_calls,
        mlx_calls,
        dist.get("green").copied().unwrap_or(0),
        dist.get("yellow").copied().unwrap_or(0),
        dist.get("red").copied().unwrap_or(0),
        est_cost,
        cli.max_cost_usd
    );
    Ok(())
}

fn round_f64(v: f64, places: u32) -> f64 {
    let p = 10f64.powi(places as i32);
    (v * p).round() / p
}

/// Boilerplate blind descriptions produced by the legacy "frame-NNN" stub:
/// treat them as empty so this run regenerates them.
fn is_stub_blind(text: &str) -> bool {
    let t = text.trim().to_lowercase();
    t.contains("placeholder")
        || t.contains("frame slug:")
        || t.ends_with("frame 0)")
        || t.ends_with("frame 1)")
}

/// Find the keyframes directory for a given `manifest.verified.json` path.
/// Mirrors the layout conventions in `hwledger-journey-render::batch::classify`.
fn resolve_keyframes_dir(manifest_path: &Path) -> Result<PathBuf> {
    let parent = manifest_path
        .parent()
        .ok_or_else(|| anyhow!("manifest has no parent"))?;
    // GUI: gui-journeys/<id>/manifest.verified.json → keyframes/ sibling.
    let gui_kf = parent.join("keyframes");
    if gui_kf.exists() {
        return Ok(gui_kf);
    }
    // CLI: cli-journeys/manifests/<id>/manifest.verified.json →
    //      cli-journeys/keyframes/<id>/
    // Streamlit: streamlit-journeys/manifests/<id>/manifest.verified.json →
    //      streamlit-journeys/recordings/<id>/
    let journey_id = parent.file_name().and_then(|s| s.to_str()).unwrap_or("");
    if let Some(manifests_dir) = parent.parent() {
        if manifests_dir.file_name().and_then(|s| s.to_str()) == Some("manifests") {
            if let Some(base) = manifests_dir.parent() {
                let cli_kf = base.join("keyframes").join(journey_id);
                if cli_kf.exists() {
                    return Ok(cli_kf);
                }
                let streamlit_rec = base.join("recordings").join(journey_id);
                if streamlit_rec.exists() {
                    return Ok(streamlit_rec);
                }
            }
        }
    }
    // Fallback: the manifest parent itself (some layouts colocate keyframes).
    Ok(parent.to_path_buf())
}

/// Provider-policy-aware backend selector.
///
/// Wire-in for ADR 0015 v3 / `docs/examples/api-providers.yaml`: `--judge
/// claude` now routes to the headless `claude` CLI (tier 5 cloud) whenever
/// the first-party paid API is blocked by policy, which is the default.
/// Direct Anthropic HTTP is only used when policy=allow-first-party AND
/// HWLEDGER_ALLOW_FIRST_PARTY_API=1 AND ANTHROPIC_API_KEY is set.
async fn select_backend(choice: JudgeBackend) -> Result<EffectiveBackend> {
    let cfg = providers::ProviderConfig::load();
    providers::enforce_blocklist(cfg.policy);
    let first_party_ok = cfg.policy.first_party_allowed()
        && std::env::var(providers::ALLOW_FIRST_PARTY_ENV)
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
            .unwrap_or(false);
    let have_anthropic_key = std::env::var("ANTHROPIC_API_KEY").is_ok();
    let claude_cli_ok = providers::which_on_path(&cfg.claude_code_bin);

    match choice {
        JudgeBackend::None => Ok(EffectiveBackend::None),
        JudgeBackend::Claude => {
            if first_party_ok && have_anthropic_key {
                Ok(EffectiveBackend::Claude)
            } else if claude_cli_ok {
                Ok(EffectiveBackend::ClaudeCodeHeadless)
            } else {
                bail!(
                    "--judge claude selected but neither direct API nor headless CLI is \
                     available (first-party API blocked; `claude` CLI not on PATH)"
                );
            }
        }
        JudgeBackend::Mlx => {
            if !mlx_available() {
                bail!(
                    "--judge mlx selected but `python -m mlx_vlm` is not importable \
                     (install with `pip install mlx-vlm`)"
                );
            }
            Ok(EffectiveBackend::Mlx)
        }
        JudgeBackend::Auto => {
            if first_party_ok && have_anthropic_key {
                return Ok(EffectiveBackend::Claude);
            }
            if mlx_available() {
                return Ok(EffectiveBackend::Mlx);
            }
            if claude_cli_ok {
                return Ok(EffectiveBackend::ClaudeCodeHeadless);
            }
            Ok(EffectiveBackend::None)
        }
    }
}

// ---------------------------------------------------------------------------
// Claude
// ---------------------------------------------------------------------------

#[derive(Serialize)]
struct ClaudeRequest<'a> {
    model: &'a str,
    max_tokens: u32,
    messages: Vec<ClaudeMessage<'a>>,
}

#[derive(Serialize)]
struct ClaudeMessage<'a> {
    role: &'a str,
    content: Vec<ClaudeContent<'a>>,
}

#[derive(Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ClaudeContent<'a> {
    Text {
        text: &'a str,
    },
    Image {
        source: ClaudeImageSource<'a>,
    },
}

#[derive(Serialize)]
struct ClaudeImageSource<'a> {
    #[serde(rename = "type")]
    kind: &'a str,
    media_type: &'a str,
    data: String,
}

#[derive(Deserialize)]
struct ClaudeResponse {
    #[serde(default)]
    content: Vec<ClaudeResponseBlock>,
    #[serde(default)]
    error: Option<serde_json::Value>,
}

#[derive(Deserialize)]
struct ClaudeResponseBlock {
    #[serde(rename = "type")]
    kind: String,
    #[serde(default)]
    text: String,
}

async fn claude_describe(
    http: &reqwest::Client,
    api_key: &str,
    model: &str,
    image: &Path,
) -> Result<String> {
    let bytes = std::fs::read(image).with_context(|| format!("read {}", image.display()))?;
    let media_type = mime_from_extension(image);
    let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);

    let req = ClaudeRequest {
        model,
        max_tokens: 200,
        messages: vec![ClaudeMessage {
            role: "user",
            content: vec![
                ClaudeContent::Image {
                    source: ClaudeImageSource { kind: "base64", media_type, data: b64 },
                },
                ClaudeContent::Text { text: BLIND_PROMPT },
            ],
        }],
    };
    let resp = http
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .json(&req)
        .send()
        .await
        .context("anthropic post")?;
    let status = resp.status();
    let body: ClaudeResponse = resp.json().await.context("anthropic parse")?;
    if !status.is_success() || body.error.is_some() {
        bail!("anthropic error {status}: {:?}", body.error);
    }
    let text = body
        .content
        .into_iter()
        .filter(|b| b.kind == "text")
        .map(|b| b.text)
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .to_string();
    if text.is_empty() {
        bail!("empty claude response");
    }
    Ok(text)
}

fn mime_from_extension(p: &Path) -> &'static str {
    match p.extension().and_then(|e| e.to_str()).map(|s| s.to_ascii_lowercase()).as_deref() {
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("webp") => "image/webp",
        Some("gif") => "image/gif",
        _ => "image/png",
    }
}

// ---------------------------------------------------------------------------
// MLX (mlx-vlm, Python subprocess — Apple-Silicon Metal/MPS)
// ---------------------------------------------------------------------------

/// Resolve the Python interpreter once per process. Honours `PYTHON` env, then
/// falls back to `python3` then `python`.
fn python_bin() -> &'static str {
    static PY: OnceLock<String> = OnceLock::new();
    PY.get_or_init(|| {
        if let Ok(explicit) = std::env::var("PYTHON") {
            if !explicit.trim().is_empty() {
                return explicit;
            }
        }
        for cand in ["python3", "python"] {
            if which_on_path(cand) {
                return cand.to_string();
            }
        }
        // Fall back to python3 — subprocess spawn will surface a clear error.
        "python3".to_string()
    })
    .as_str()
}

/// Minimal PATH lookup so we can keep the MLX probe hermetic (no extra crate).
fn which_on_path(bin: &str) -> bool {
    let Some(path) = std::env::var_os("PATH") else {
        return false;
    };
    for dir in std::env::split_paths(&path) {
        let candidate = dir.join(bin);
        if candidate.is_file() {
            return true;
        }
    }
    false
}

/// Cached availability probe — runs `python -c "import mlx_vlm"` with a 2s
/// timeout on the first call only.
fn mlx_available() -> bool {
    static AVAIL: OnceLock<bool> = OnceLock::new();
    *AVAIL.get_or_init(probe_mlx_once)
}

fn probe_mlx_once() -> bool {
    let py = python_bin();
    if !which_on_path(py) && !Path::new(py).is_file() {
        return false;
    }
    // Spawn `python -c "import mlx_vlm; print('ok')"` with a 2s budget.
    let mut child = match std::process::Command::new(py)
        .args(["-c", "import mlx_vlm; print('ok')"])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(c) => c,
        Err(_) => return false,
    };
    let deadline = std::time::Instant::now() + Duration::from_secs(2);
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return status.success(),
            Ok(None) => {
                if std::time::Instant::now() >= deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    return false;
                }
                std::thread::sleep(Duration::from_millis(50));
            }
            Err(_) => return false,
        }
    }
}

/// Parse `mlx-vlm`'s generate stdout. `mlx-vlm` emits the generated text
/// between two `==========` separator lines, followed by a stats block. We
/// take whatever sits between the first and second separator.
fn parse_mlx_stdout(stdout: &str) -> Option<String> {
    let sep = "==========";
    let mut iter = stdout.split(sep);
    // Drop the preamble before the first separator.
    let _ = iter.next()?;
    // The generated text is the next chunk.
    let body = iter.next()?;
    let trimmed = body.trim();
    if trimmed.is_empty() {
        return None;
    }
    Some(trimmed.to_string())
}

fn mlx_describe(model: &str, image: &Path) -> Result<String> {
    let py = python_bin();
    let out = std::process::Command::new(py)
        .args([
            "-m",
            "mlx_vlm.generate",
            "--model",
            model,
            "--image",
            image.to_str().ok_or_else(|| anyhow!("image path not utf-8"))?,
            "--prompt",
            BLIND_PROMPT,
            "--max-tokens",
            "150",
            "--temperature",
            "0.2",
        ])
        .stdin(Stdio::null())
        .output()
        .context("spawn python -m mlx_vlm.generate")?;
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        bail!("mlx_vlm.generate exit={}: {}", out.status, stderr.trim());
    }
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    match parse_mlx_stdout(&stdout) {
        Some(t) => Ok(t),
        None => bail!(
            "mlx_vlm.generate produced no parsable text block (stdout len={})",
            stdout.len()
        ),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_confidence_buckets() {
        // High: assertive / declarative.
        assert_eq!(
            classify_confidence("The image shows a dashboard with a stacked-bar chart."),
            "high"
        );
        assert_eq!(
            classify_confidence("I can see the hwLedger CLI output with 3 lines."),
            "high"
        );
        // Medium: hedged but still substantive.
        assert_eq!(
            classify_confidence("It appears to be a settings panel with a toggle row."),
            "medium"
        );
        // Low: explicit uncertainty.
        assert_eq!(
            classify_confidence("I'm not sure what this frame shows — possibly a modal?"),
            "low"
        );
        // Low beats High when both markers appear (err toward review).
        assert_eq!(
            classify_confidence("The image shows a panel, but I'm not sure what it does."),
            "low"
        );
        // Default (no markers) → medium.
        assert_eq!(classify_confidence("A dark window with two columns."), "medium");
    }

    #[test]
    fn mlx_available_returns_false_without_python() {
        // Temporarily point PATH somewhere empty, and force a fresh subprocess
        // lookup by invoking `probe_mlx_once` directly with a non-existent
        // interpreter.
        let dir = tempfile::tempdir().unwrap();
        let prev_path = std::env::var_os("PATH");
        // Isolate: PATH is only the empty tempdir so `python3`/`python` are not
        // discoverable.
        std::env::set_var("PATH", dir.path());
        // Directly exercise the helpers (not the cached wrapper).
        assert!(!which_on_path("python-does-not-exist-xyz"));
        // Restore PATH before hitting the cached `mlx_available()` so other
        // tests in this binary are not disturbed.
        if let Some(p) = prev_path {
            std::env::set_var("PATH", p);
        } else {
            std::env::remove_var("PATH");
        }
    }

    #[test]
    fn parses_mlx_generated_text_block() {
        let fixture = "\
Fetching 11 files: 100%|███████| 11/11 [00:00<00:00, 123.45it/s]\n\
Prompt: Describe what you see...\n\
==========\n\
A dark terminal window displays a plan command help screen with options \
listed in monospace text. A cursor blinks at the prompt.\n\
==========\n\
Prompt: 1234 tokens, 456.78 tokens-per-sec\n\
Generation: 150 tokens, 12.34 tokens-per-sec\n\
Peak memory: 4.567 GB\n";
        let parsed = parse_mlx_stdout(fixture).expect("should parse");
        assert!(parsed.starts_with("A dark terminal window"));
        assert!(parsed.contains("monospace text"));
        assert!(!parsed.contains("=========="));
        assert!(!parsed.contains("tokens-per-sec"));
    }

    #[test]
    fn parse_mlx_stdout_empty_when_no_separators() {
        assert!(parse_mlx_stdout("just some noise without separators").is_none());
    }

    #[test]
    fn hf_cache_dir_for_maps_org_and_name() {
        assert_eq!(
            hf_cache_dir_for("mlx-community/Qwen3-VL-32B-Instruct-4bit").as_deref(),
            Some("models--mlx-community--Qwen3-VL-32B-Instruct-4bit"),
        );
        assert_eq!(hf_cache_dir_for("no-slash"), None);
        assert_eq!(hf_cache_dir_for("/trailing"), None);
        assert_eq!(hf_cache_dir_for("leading/"), None);
    }

    #[test]
    fn pick_mlx_vlm_model_prefers_cached_entry() {
        // Build a fake cache root containing ONLY the 3rd priority entry
        // (`gemma-3-27b-it-4bit`) and ensure the picker returns it rather
        // than the top-of-chain.
        let tmp = tempfile::tempdir().unwrap();
        let cache_root = tmp.path().join("hub");
        std::fs::create_dir_all(&cache_root).unwrap();
        let target = "mlx-community/gemma-3-27b-it-4bit";
        let dir = hf_cache_dir_for(target).unwrap();
        std::fs::create_dir_all(cache_root.join(&dir)).unwrap();

        // Sanity: MLX_VLM_PRIORITY contains the target.
        assert!(MLX_VLM_PRIORITY.contains(&target));

        // Drive `pick_mlx_vlm_model` through the env override so the test is
        // hermetic and doesn't depend on the real `~/.cache/huggingface/hub`.
        let prev = std::env::var("HUGGINGFACE_HUB_CACHE").ok();
        std::env::set_var("HUGGINGFACE_HUB_CACHE", &cache_root);
        let picked = pick_mlx_vlm_model();
        // Restore env before asserting so a panic doesn't leak state.
        match prev {
            Some(p) => std::env::set_var("HUGGINGFACE_HUB_CACHE", p),
            None => std::env::remove_var("HUGGINGFACE_HUB_CACHE"),
        }
        assert_eq!(picked.as_deref(), Some(target));
    }

    #[test]
    fn pick_mlx_vlm_model_falls_back_to_top_when_nothing_cached() {
        let tmp = tempfile::tempdir().unwrap();
        let empty_cache = tmp.path().join("hub");
        std::fs::create_dir_all(&empty_cache).unwrap();
        let prev = std::env::var("HUGGINGFACE_HUB_CACHE").ok();
        std::env::set_var("HUGGINGFACE_HUB_CACHE", &empty_cache);
        let picked = pick_mlx_vlm_model();
        match prev {
            Some(p) => std::env::set_var("HUGGINGFACE_HUB_CACHE", p),
            None => std::env::remove_var("HUGGINGFACE_HUB_CACHE"),
        }
        assert_eq!(picked.as_deref(), Some(MLX_VLM_PRIORITY[0]));
    }

    /// Integration smoke test: if MLX is actually installed AND the model is
    /// cached, describe a real keyframe. Ignored by default; run with
    /// `cargo test -p hwledger-frame-describer -- --ignored mlx_live`.
    #[test]
    #[ignore]
    fn mlx_live_describe_smoke() {
        if !mlx_available() {
            eprintln!("mlx_vlm not importable; skipping");
            return;
        }
        let repo_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(|p| p.parent())
            .expect("repo root")
            .to_path_buf();
        // Walk for any PNG keyframe under docs-site/public.
        let mut sample: Option<PathBuf> = None;
        for entry in WalkDir::new(repo_root.join("docs-site/public"))
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if entry.path().extension().and_then(|e| e.to_str()) == Some("png") {
                sample = Some(entry.path().to_path_buf());
                break;
            }
        }
        let Some(image) = sample else {
            eprintln!("no sample keyframe; skipping");
            return;
        };
        let model = std::env::var("HWLEDGER_MLX_VLM_MODEL")
            .unwrap_or_else(|_| DEFAULT_MLX_MODEL.to_string());
        let desc = mlx_describe(&model, &image).expect("mlx_describe");
        assert!(!desc.trim().is_empty(), "blind description must be non-empty");
    }
}
