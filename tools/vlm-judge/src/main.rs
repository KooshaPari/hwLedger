//! hwledger-vlm-judge
//!
//! Walk every `manifest.verified.json` under one or more roots, blind-describe
//! each step's keyframe via Claude Sonnet 4.6 (multimodal) or Ollama
//! `qwen2.5vl:7b`, and score the blind description against the human-authored
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
//! Scripting policy: pure Rust + `reqwest` (Anthropic API, Ollama HTTP) +
//! `base64` image encoding. No shell glue.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{anyhow, bail, Context, Result};
use base64::Engine;
use clap::{Parser, ValueEnum};
use serde::{Deserialize, Serialize};
use walkdir::WalkDir;

mod agreement;
use agreement::score as agreement_score;

/// Default hard cost ceiling (USD) for Claude calls per run.
const MAX_COST_USD: f64 = 5.0;

/// Rough per-call Claude Sonnet 4.6 cost estimate (USD). A blind description
/// prompt sends one keyframe (~1-2K input tokens incl. image) and receives
/// <=80 output tokens. At Sonnet 4.6 list pricing ($3/MTok input, $15/MTok
/// output) this bounds a call at ~$0.012. We budget $0.02 for headroom.
const CLAUDE_PER_CALL_USD: f64 = 0.02;

/// Default Anthropic model. Override with `HWLEDGER_CLAUDE_MODEL`.
const DEFAULT_CLAUDE_MODEL: &str = "claude-sonnet-4-6";

/// Default Ollama model tag used for the fallback backend.
const DEFAULT_OLLAMA_MODEL: &str = "qwen2.5vl:7b";

/// Default Ollama endpoint.
const DEFAULT_OLLAMA_URL: &str = "http://127.0.0.1:11434";

/// Blind prompt — mirrors the instructions in the task brief.
const BLIND_PROMPT: &str = "Describe what you see in this image, 2 sentences max. Do not guess context. Do not mention 'placeholder' or similar.";

#[derive(Parser, Debug)]
#[command(
    name = "hwledger-vlm-judge",
    about = "Blind-describe journey keyframes via Claude/Ollama and score agreement with per-step intent.",
    version
)]
struct Cli {
    /// Root(s) under which to find `manifest.verified.json` files. Defaults
    /// to `docs-site/public` when omitted.
    #[arg(long = "root", num_args = 1.., value_name = "DIR")]
    roots: Vec<PathBuf>,

    /// Backend selection. `auto` prefers Claude when `ANTHROPIC_API_KEY` is
    /// set, then falls back to Ollama; `claude` forces Claude; `ollama`
    /// forces Ollama; `none` only scores steps whose blind descriptions are
    /// already populated.
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
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, ValueEnum)]
enum JudgeBackend {
    Auto,
    Claude,
    Ollama,
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EffectiveBackend {
    Claude,
    Ollama,
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
            EffectiveBackend::Ollama => "ollama",
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
    let ollama_url =
        std::env::var("OLLAMA_URL").unwrap_or_else(|_| DEFAULT_OLLAMA_URL.to_string());
    let ollama_model =
        std::env::var("HWLEDGER_OLLAMA_MODEL").unwrap_or_else(|_| DEFAULT_OLLAMA_MODEL.to_string());
    let anthropic_key = std::env::var("ANTHROPIC_API_KEY").ok();

    let mut claude_calls = 0usize;
    let mut ollama_calls = 0usize;
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
                    // over cap — fall back to Ollama for this step if available.
                    if probe_ollama(&http, &ollama_url).await {
                        EffectiveBackend::Ollama
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
                                "[vlm-judge] claude failed for {}: {} — trying ollama",
                                journey_id, e
                            );
                            match ollama_describe(&http, &ollama_url, &ollama_model, &image_path)
                                .await
                            {
                                Ok(desc) => {
                                    ollama_calls += 1;
                                    (desc, "ollama")
                                }
                                Err(e2) => {
                                    eprintln!(
                                        "[vlm-judge] ollama also failed for {}: {}",
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
                EffectiveBackend::Ollama => {
                    match ollama_describe(&http, &ollama_url, &ollama_model, &image_path).await {
                        Ok(desc) => {
                            ollama_calls += 1;
                            (desc, "ollama")
                        }
                        Err(e) => {
                            eprintln!(
                                "[vlm-judge] ollama failed for {} step: {}",
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
         calls: claude={} ollama={}\n\
         distribution: green={} yellow={} red={}\n\
         claude est cost: ${:.3} (cap ${:.2})\n",
        manifests.len(),
        manifests_touched,
        scored,
        skipped_already,
        pending_marked,
        claude_calls,
        ollama_calls,
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

async fn select_backend(choice: JudgeBackend) -> Result<EffectiveBackend> {
    match choice {
        JudgeBackend::None => Ok(EffectiveBackend::None),
        JudgeBackend::Claude => {
            if std::env::var("ANTHROPIC_API_KEY").is_err() {
                bail!("--judge claude selected but ANTHROPIC_API_KEY is not set");
            }
            Ok(EffectiveBackend::Claude)
        }
        JudgeBackend::Ollama => {
            let http = reqwest::Client::new();
            let url =
                std::env::var("OLLAMA_URL").unwrap_or_else(|_| DEFAULT_OLLAMA_URL.to_string());
            if !probe_ollama(&http, &url).await {
                bail!("--judge ollama selected but Ollama is not reachable at {url}");
            }
            Ok(EffectiveBackend::Ollama)
        }
        JudgeBackend::Auto => {
            if std::env::var("ANTHROPIC_API_KEY").is_ok() {
                return Ok(EffectiveBackend::Claude);
            }
            let http = reqwest::Client::new();
            let url =
                std::env::var("OLLAMA_URL").unwrap_or_else(|_| DEFAULT_OLLAMA_URL.to_string());
            if probe_ollama(&http, &url).await {
                return Ok(EffectiveBackend::Ollama);
            }
            Ok(EffectiveBackend::None)
        }
    }
}

async fn probe_ollama(http: &reqwest::Client, base_url: &str) -> bool {
    http.get(format!("{}/api/tags", base_url.trim_end_matches('/')))
        .timeout(Duration::from_secs(2))
        .send()
        .await
        .map(|r| r.status().is_success())
        .unwrap_or(false)
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
// Ollama
// ---------------------------------------------------------------------------

#[derive(Serialize)]
struct OllamaRequest<'a> {
    model: &'a str,
    prompt: &'a str,
    images: Vec<String>,
    stream: bool,
}

#[derive(Deserialize)]
struct OllamaResponse {
    #[serde(default)]
    response: String,
}

async fn ollama_describe(
    http: &reqwest::Client,
    base_url: &str,
    model: &str,
    image: &Path,
) -> Result<String> {
    let bytes = std::fs::read(image)?;
    let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
    let req = OllamaRequest { model, prompt: BLIND_PROMPT, images: vec![b64], stream: false };
    let resp = http
        .post(format!("{}/api/generate", base_url.trim_end_matches('/')))
        .json(&req)
        .send()
        .await
        .context("ollama post")?;
    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        bail!("ollama http {status}: {body}");
    }
    let body: OllamaResponse = resp.json().await.context("ollama parse")?;
    let text = body.response.trim().to_string();
    if text.is_empty() {
        bail!("empty ollama response");
    }
    Ok(text)
}
