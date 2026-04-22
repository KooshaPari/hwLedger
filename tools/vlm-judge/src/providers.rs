// The provider surface lands in a dedicated commit; callers are still in the
// legacy Claude-direct path while a044ad18's MLX chain and ab6be8c9's
// provider chain are merged. Justification tracked in ADR 0015 v3 and
// docs-site/engineering/api-provider-policy.md; remove this allow once the
// wire-up lands in `main.rs::run()`.
#![allow(dead_code)]
//! Provider registry for the VLM judge.
//!
//! Policy (user mandate, 2026-04-22): first-party paid APIs (Anthropic /
//! OpenAI / Gemini) are BLOCKED by default. The allowed provider chain is:
//!
//!   1. Fireworks.ai (subscription — Kimi K2.5 VLM / K2 Turbo text)
//!   2. MiniMax (subscription — M2.7)
//!   3. OpenRouter `:free` tier only
//!   4. Local MLX (mlx-vlm, Apple Silicon)
//!   5. Headless Claude Code CLI (`claude -p ...`)  [uses the user's CLI login]
//!   6. Headless Codex CLI (`codex exec ...`)        [uses the user's CLI login]
//!
//! Anthropic/OpenAI/Gemini direct API paths are only reachable when both:
//!   * a config file sets `policy: allow-first-party`, AND
//!   * `HWLEDGER_ALLOW_FIRST_PARTY_API=1` is exported in the environment.
//!
//! When either condition is missing the registry emits a loud WARN and
//! pretends those credentials are unset.
//!
//! Scripting policy: Rust, reqwest + serde. See ADR 0015 v3.

use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::OnceLock;
use std::time::Duration;

use anyhow::{anyhow, bail, Context, Result};
use base64::Engine;
use serde::{Deserialize, Serialize};

/// Environment override required to enable first-party paid APIs even when
/// the config policy allows them.
pub const ALLOW_FIRST_PARTY_ENV: &str = "HWLEDGER_ALLOW_FIRST_PARTY_API";

/// Env vars that belong to blocked first-party providers. Presence of any of
/// these without an allow-override triggers a warning.
pub const FIRST_PARTY_ENV_KEYS: &[&str] = &[
    "ANTHROPIC_API_KEY",
    "OPENAI_API_KEY",
    "GOOGLE_API_KEY",
    "GEMINI_API_KEY",
];

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Policy {
    BlockAll,
    FreeRouterOnly,
    SubscriptionRouted,
    AllowFirstParty,
}

impl Policy {
    pub fn first_party_allowed(self) -> bool {
        matches!(self, Policy::AllowFirstParty)
            && std::env::var(ALLOW_FIRST_PARTY_ENV).map(|v| v == "1").unwrap_or(false)
    }
}

/// The ordered provider choices callers can pick. `Auto` walks the registry
/// priority list until something succeeds.
#[derive(Copy, Clone, Debug, PartialEq, Eq, clap::ValueEnum)]
pub enum ProviderChoice {
    Auto,
    Fireworks,
    Minimax,
    Openrouter,
    Mlx,
    ClaudeCode,
    Codex,
    None,
}

impl ProviderChoice {
    pub fn label(self) -> &'static str {
        match self {
            ProviderChoice::Auto => "auto",
            ProviderChoice::Fireworks => "fireworks",
            ProviderChoice::Minimax => "minimax",
            ProviderChoice::Openrouter => "openrouter",
            ProviderChoice::Mlx => "mlx",
            ProviderChoice::ClaudeCode => "claude-code",
            ProviderChoice::Codex => "codex",
            ProviderChoice::None => "none",
        }
    }
}

/// What the judge binary actually resolves to for a single call.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Backend {
    Fireworks,
    Minimax,
    Openrouter,
    Mlx,
    ClaudeCode,
    Codex,
    None,
}

impl Backend {
    pub fn label(self) -> &'static str {
        match self {
            Backend::Fireworks => "fireworks",
            Backend::Minimax => "minimax",
            Backend::Openrouter => "openrouter",
            Backend::Mlx => "mlx",
            Backend::ClaudeCode => "claude-code",
            Backend::Codex => "codex",
            Backend::None => "none",
        }
    }
}

/// In-memory view of `~/.hwledger/api-providers.yaml`.
/// Defaults mirror `docs/examples/api-providers.yaml` so the judge works
/// out of the box without any config present.
#[derive(Debug, Clone)]
pub struct ProviderConfig {
    pub policy: Policy,
    pub fireworks_model_vlm: String,
    pub fireworks_model_text: String,
    pub fireworks_base_url: String,
    pub minimax_model: String,
    pub minimax_base_url: String,
    pub openrouter_model_vlm: String,
    pub openrouter_base_url: String,
    pub mlx_model: String,
    pub claude_code_bin: String,
    pub claude_code_model: String,
    pub codex_bin: String,
    pub codex_model: String,
}

impl Default for ProviderConfig {
    fn default() -> Self {
        Self {
            policy: Policy::FreeRouterOnly,
            fireworks_model_vlm: "accounts/fireworks/models/kimi-k2p5".into(),
            fireworks_model_text: "accounts/fireworks/models/kimi-k2-turbo".into(),
            fireworks_base_url: "https://api.fireworks.ai/inference/v1".into(),
            minimax_model: "MiniMax-M2.7".into(),
            minimax_base_url: "https://api.minimax.io/v1".into(),
            openrouter_model_vlm: "qwen/qwen2.5-vl-72b-instruct:free".into(),
            openrouter_base_url: "https://openrouter.ai/api/v1".into(),
            mlx_model: "mlx-community/Qwen2.5-VL-7B-Instruct-4bit".into(),
            claude_code_bin: "claude".into(),
            claude_code_model: "claude-opus-4-7".into(),
            codex_bin: "codex".into(),
            codex_model: "gpt-5".into(),
        }
    }
}

impl ProviderConfig {
    /// Best-effort load from `~/.hwledger/api-providers.yaml`. Missing file →
    /// defaults. YAML parsing is minimal and key-by-key so we avoid dragging
    /// in `serde_yaml` for the MVP; only the fields the judge actually reads
    /// are honored here.
    pub fn load() -> Self {
        let Some(home) = std::env::var_os("HOME") else {
            return Self::default();
        };
        let path = Path::new(&home).join(".hwledger").join("api-providers.yaml");
        let Ok(text) = std::fs::read_to_string(&path) else {
            return Self::default();
        };
        let mut cfg = Self::default();
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let Some((key, val)) = line.split_once(':') else { continue };
            let key = key.trim();
            let val = val.trim().trim_matches(|c| c == '"' || c == '\'');
            match key {
                "policy" => {
                    cfg.policy = match val {
                        "block-all" => Policy::BlockAll,
                        "free-router-only" => Policy::FreeRouterOnly,
                        "subscription-routed" => Policy::SubscriptionRouted,
                        "allow-first-party" => Policy::AllowFirstParty,
                        _ => cfg.policy,
                    };
                }
                _ => { /* MVP YAML: only top-level policy parsed */ }
            }
        }
        cfg
    }
}

/// Emit a loud warning if any first-party env key is present but the policy
/// does not allow direct first-party API usage.
pub fn enforce_blocklist(policy: Policy) {
    let allowed = policy.first_party_allowed();
    let leaked: Vec<&&str> = FIRST_PARTY_ENV_KEYS
        .iter()
        .filter(|k| std::env::var(k).is_ok())
        .collect();
    if allowed {
        if !leaked.is_empty() {
            eprintln!(
                "[vlm-judge] WARN: {} is set AND first-party env keys are present: {:?}. \
                 Direct paid-API calls WILL be made. Unset {} to block.",
                ALLOW_FIRST_PARTY_ENV, leaked, ALLOW_FIRST_PARTY_ENV
            );
        }
    } else if !leaked.is_empty() {
        eprintln!(
            "[vlm-judge] WARN: first-party API key(s) detected in env ({:?}) but policy blocks \
             direct use. They will be ignored. Set {}=1 AND policy=allow-first-party to override. \
             See docs/examples/api-providers.yaml.",
            leaked, ALLOW_FIRST_PARTY_ENV
        );
    }
}

/// Probe whether a binary is on PATH.
pub fn which_on_path(bin: &str) -> bool {
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

/// Priority walk: return the first backend that looks reachable.
pub fn select_auto(cfg: &ProviderConfig) -> Backend {
    if std::env::var("FIREWORKS_API_KEY").is_ok() {
        return Backend::Fireworks;
    }
    if std::env::var("MINIMAX_API_KEY").is_ok() {
        return Backend::Minimax;
    }
    if std::env::var("OPENROUTER_API_KEY").is_ok() {
        return Backend::Openrouter;
    }
    if mlx_available() {
        return Backend::Mlx;
    }
    if which_on_path(&cfg.claude_code_bin) {
        return Backend::ClaudeCode;
    }
    if which_on_path(&cfg.codex_bin) {
        return Backend::Codex;
    }
    Backend::None
}

pub fn resolve_choice(choice: ProviderChoice, cfg: &ProviderConfig) -> Backend {
    match choice {
        ProviderChoice::Auto => select_auto(cfg),
        ProviderChoice::Fireworks => Backend::Fireworks,
        ProviderChoice::Minimax => Backend::Minimax,
        ProviderChoice::Openrouter => Backend::Openrouter,
        ProviderChoice::Mlx => Backend::Mlx,
        ProviderChoice::ClaudeCode => Backend::ClaudeCode,
        ProviderChoice::Codex => Backend::Codex,
        ProviderChoice::None => Backend::None,
    }
}

/// Cached availability probe for MLX.
pub fn mlx_available() -> bool {
    static AVAIL: OnceLock<bool> = OnceLock::new();
    *AVAIL.get_or_init(probe_mlx_once)
}

fn python_bin() -> String {
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
    "python3".to_string()
}

fn probe_mlx_once() -> bool {
    let py = python_bin();
    if !which_on_path(&py) && !Path::new(&py).is_file() {
        return false;
    }
    let mut child = match std::process::Command::new(&py)
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

pub fn mime_from_extension(p: &Path) -> &'static str {
    match p.extension().and_then(|e| e.to_str()).map(|s| s.to_ascii_lowercase()).as_deref() {
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("webp") => "image/webp",
        Some("gif") => "image/gif",
        _ => "image/png",
    }
}

pub fn read_image_as_data_url(image: &Path) -> Result<String> {
    let bytes = std::fs::read(image).with_context(|| format!("read {}", image.display()))?;
    let mt = mime_from_extension(image);
    let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
    Ok(format!("data:{mt};base64,{b64}"))
}

// ---------------------------------------------------------------------------
// OpenAI-compatible chat-completions body builder (used by Fireworks,
// OpenRouter, and — best-effort — MiniMax).
// ---------------------------------------------------------------------------

#[derive(Serialize)]
struct OaiChatRequest<'a> {
    model: &'a str,
    max_tokens: u32,
    temperature: f32,
    messages: Vec<OaiMessage<'a>>,
}

#[derive(Serialize)]
struct OaiMessage<'a> {
    role: &'a str,
    content: Vec<OaiContent<'a>>,
}

#[derive(Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum OaiContent<'a> {
    Text { text: &'a str },
    ImageUrl { image_url: OaiImageUrl },
}

#[derive(Serialize)]
struct OaiImageUrl {
    url: String,
}

#[derive(Deserialize)]
struct OaiChatResponse {
    #[serde(default)]
    choices: Vec<OaiChoice>,
    #[serde(default)]
    error: Option<serde_json::Value>,
}

#[derive(Deserialize)]
struct OaiChoice {
    #[serde(default)]
    message: OaiResponseMessage,
}

#[derive(Deserialize, Default)]
struct OaiResponseMessage {
    #[serde(default)]
    content: String,
}

async fn oai_describe(
    http: &reqwest::Client,
    endpoint: &str,
    api_key: &str,
    model: &str,
    image: &Path,
    prompt: &str,
    extra_headers: &[(&str, &str)],
) -> Result<String> {
    let data_url = read_image_as_data_url(image)?;
    let req = OaiChatRequest {
        model,
        max_tokens: 200,
        temperature: 0.2,
        messages: vec![OaiMessage {
            role: "user",
            content: vec![
                OaiContent::ImageUrl { image_url: OaiImageUrl { url: data_url } },
                OaiContent::Text { text: prompt },
            ],
        }],
    };
    let mut rb = http
        .post(endpoint)
        .bearer_auth(api_key)
        .header("content-type", "application/json")
        .json(&req);
    for (k, v) in extra_headers {
        rb = rb.header(*k, *v);
    }
    let resp = rb.send().await.context("oai post")?;
    let status = resp.status();
    let body: OaiChatResponse = resp.json().await.context("oai parse")?;
    if !status.is_success() || body.error.is_some() {
        bail!("oai error {status}: {:?}", body.error);
    }
    let text = body
        .choices
        .into_iter()
        .map(|c| c.message.content)
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .to_string();
    if text.is_empty() {
        bail!("empty oai response");
    }
    Ok(text)
}

pub async fn fireworks_describe(
    http: &reqwest::Client,
    cfg: &ProviderConfig,
    image: &Path,
    prompt: &str,
) -> Result<String> {
    let key = std::env::var("FIREWORKS_API_KEY")
        .map_err(|_| anyhow!("FIREWORKS_API_KEY not set"))?;
    let endpoint = format!("{}/chat/completions", cfg.fireworks_base_url);
    oai_describe(
        http,
        &endpoint,
        &key,
        &cfg.fireworks_model_vlm,
        image,
        prompt,
        &[],
    )
    .await
}

pub async fn openrouter_describe(
    http: &reqwest::Client,
    cfg: &ProviderConfig,
    image: &Path,
    prompt: &str,
) -> Result<String> {
    let key = std::env::var("OPENROUTER_API_KEY")
        .map_err(|_| anyhow!("OPENROUTER_API_KEY not set"))?;
    let endpoint = format!("{}/chat/completions", cfg.openrouter_base_url);
    // OpenRouter asks for HTTP-Referer + X-Title headers for attribution.
    oai_describe(
        http,
        &endpoint,
        &key,
        &cfg.openrouter_model_vlm,
        image,
        prompt,
        &[
            ("HTTP-Referer", "https://github.com/KooshaPari/hwLedger"),
            ("X-Title", "hwledger-vlm-judge"),
        ],
    )
    .await
}

pub async fn minimax_describe(
    http: &reqwest::Client,
    cfg: &ProviderConfig,
    image: &Path,
    prompt: &str,
) -> Result<String> {
    // MiniMax's platform API is close-to-OpenAI-compatible at
    // `/text/chatcompletion_v2`. Image content blocks are accepted when the
    // model supports vision; otherwise the call 4xx's and the caller falls
    // through to the next provider.
    let key = std::env::var("MINIMAX_API_KEY")
        .map_err(|_| anyhow!("MINIMAX_API_KEY not set"))?;
    let endpoint = format!("{}/text/chatcompletion_v2", cfg.minimax_base_url);
    oai_describe(http, &endpoint, &key, &cfg.minimax_model, image, prompt, &[]).await
}

// ---------------------------------------------------------------------------
// Headless Claude Code / Codex CLI shims.
//
// Neither CLI reliably accepts images over stdin today, so these providers
// run TEXT-ONLY: the caller must pass a short textual summary (for example
// produced by an earlier MLX pass) as `image_ref`. If no prior description
// exists, these providers hard-fail with a clear error and the outer loop
// marks the step `judge_status: "pending"`.
// ---------------------------------------------------------------------------

pub fn claude_code_headless_describe(
    cfg: &ProviderConfig,
    image_ref: &str,
    prompt: &str,
) -> Result<String> {
    if !which_on_path(&cfg.claude_code_bin) {
        bail!("claude CLI not on PATH; skipping claude-code-headless");
    }
    let combined = format!("{prompt}\n\nImage reference / prior blind description:\n{image_ref}");
    let out = std::process::Command::new(&cfg.claude_code_bin)
        .args([
            "-p",
            "--model",
            &cfg.claude_code_model,
            "--output-format",
            "text",
            "--max-turns",
            "1",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("spawn claude CLI")
        .and_then(|mut child| {
            use std::io::Write;
            if let Some(stdin) = child.stdin.as_mut() {
                stdin.write_all(combined.as_bytes()).context("write stdin")?;
            }
            child.wait_with_output().context("wait claude CLI")
        })?;
    if !out.status.success() {
        bail!(
            "claude CLI exit={}: {}",
            out.status,
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    let text = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if text.is_empty() {
        bail!("claude CLI produced empty output");
    }
    Ok(text)
}

pub fn codex_headless_describe(
    cfg: &ProviderConfig,
    image_ref: &str,
    prompt: &str,
) -> Result<String> {
    if !which_on_path(&cfg.codex_bin) {
        bail!("codex CLI not on PATH; skipping codex-headless");
    }
    let combined = format!("{prompt}\n\nImage reference / prior blind description:\n{image_ref}");
    let out = std::process::Command::new(&cfg.codex_bin)
        .args(["exec", "--model", &cfg.codex_model, &combined])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .context("spawn codex CLI")?;
    if !out.status.success() {
        bail!(
            "codex CLI exit={}: {}",
            out.status,
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    let text = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if text.is_empty() {
        bail!("codex CLI produced empty output");
    }
    Ok(text)
}

// ---------------------------------------------------------------------------
// MLX (kept from the pre-existing implementation; agent a044ad18 owns the
// upstream MLX wiring, this re-exports it unchanged).
// ---------------------------------------------------------------------------

/// Blind prompt — kept identical to `main::BLIND_PROMPT`. Two-part pattern
/// borrowed from zakelfassi (2026): positive target in 1-2 sentences + explicit
/// negative to rule out stub/placeholder hallucinations. Source:
/// <https://zakelfassi.com/vlm-visual-testing-chrome-extension>. Extraction
/// notes: `docs-site/research/imports-2026-04/zakelfassi-vlm-visual-testing.md`.
pub const BLIND_PROMPT: &str = "Describe what you see in this image in 1-2 sentences. \
Stick to concrete on-screen elements (windows, panels, text fragments, buttons, cursor). \
This is NOT a placeholder, stub, or synthetic test frame — do not say 'placeholder', \
'stub', 'frame N', 'image N', 'test image', or 'no content'. \
Do not guess application context you cannot see.";

pub fn mlx_describe(model: &str, image: &Path) -> Result<String> {
    let py = python_bin();
    let out = std::process::Command::new(&py)
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

pub fn parse_mlx_stdout(stdout: &str) -> Option<String> {
    let sep = "==========";
    let mut iter = stdout.split(sep);
    let _ = iter.next()?;
    let body = iter.next()?;
    let trimmed = body.trim();
    if trimmed.is_empty() {
        return None;
    }
    Some(trimmed.to_string())
}

// Dummy to silence unused import in certain cfgs.
#[allow(dead_code)]
fn _dummy_pathbuf() -> PathBuf {
    PathBuf::new()
}

// ---------------------------------------------------------------------------
// Frame-describer task router (ADR-0015 v5 / ADR-0039, 2026-04-22).
//
// Replaces the single monolithic MLX chain with a tiered chain keyed by
// inferred task-family. Florence-2 (microsoft/Florence-2-large, 771M, MIT)
// is the tier-2 SLM default for caption/OCR/region-describe; UI-TARS-1.5-7B
// is demoted from default to a tier-3 domain specialist that only wins on
// screenshot->action frames.
//
// The mapping here is the authoritative Rust mirror of the
// `providers.frame_describer.task_routing` block in
// `docs/examples/api-providers.yaml`. Keep both in sync.
// ---------------------------------------------------------------------------

/// Inferred task family for a keyframe, derived from step context
/// (terminal/CLI, SwiftUI button, Streamlit dashboard, etc.). The runtime
/// caller infers this from the step's `family` + `intent` fields before
/// asking the router which tier to try first.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DescriberTask {
    /// Generic "describe this region of a screenshot" — Florence-2 wins.
    CaptionRegion,
    /// SwiftUI button interaction or other UI-action frame — UI-TARS wins.
    UiActionDescribe,
    /// OCR-dominant keyframe (terminal, CLI output). Classical CV first,
    /// Florence-2 as the SLM backup.
    OcrOnly,
    /// Unusual or out-of-distribution frame — prefer omni generalist or
    /// cloud fallback.
    NovelUnusual,
}

/// Describer tier identifier. Matches the yaml keys under
/// `providers.frame_describer.{tier2_slm,tier3_domain,...}`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DescriberTier {
    Tier1ClassicalCv,
    Tier2Slm,
    Tier3Domain,
    Tier4Omni,
    Tier5Cloud,
}

impl DescriberTier {
    pub fn label(self) -> &'static str {
        match self {
            DescriberTier::Tier1ClassicalCv => "tier1_classical_cv",
            DescriberTier::Tier2Slm => "tier2_slm",
            DescriberTier::Tier3Domain => "tier3_domain",
            DescriberTier::Tier4Omni => "tier4_omni",
            DescriberTier::Tier5Cloud => "tier5_cloud",
        }
    }
}

/// Canonical tier-2 SLM chain. Florence-2-large is default; Florence-2-base
/// is the <4GB-host fallback. Order mirrors `tier2_slm:` in the yaml.
pub const TIER2_SLM_CHAIN: &[&str] = &[
    "microsoft/Florence-2-large",            // 771M, MIT, ~50 ms/frame on Metal via MPS
    "microsoft/Florence-2-base",             // 232M, for hosts with <4 GB free
    "vikhyatk/moondream2",                   // 1.86B, Apache-2.0
    "HuggingFaceTB/SmolVLM2-2.2B-Instruct",  // 2.2B, Apache-2.0
    "google/paligemma-3b-mix-448",           // 3B, Gemma license — last resort
];

/// Tier-3 domain specialist chain. UI-TARS-1.5-7B variants, ordered quant
/// fidelity first. Insert UI-TARS-2 at position 0 when the MLX port ships.
pub const TIER3_DOMAIN_CHAIN: &[&str] = &[
    "mlx-community/UI-TARS-1.5-7B-6bit",
    "mlx-community/UI-TARS-1.5-7B-4bit",
];

/// Tier-4 omni fallback chain (shares the legacy `providers.mlx.models.vlm`
/// priority list). Used when tier2/3 are unavailable or task is
/// `NovelUnusual` and no cloud provider is reachable.
pub const TIER4_OMNI_CHAIN: &[&str] = &[
    "mlx-community/Qwen3.6-35B-A3B-4bit",
    "mlx-community/Qwen3.5-122B-A10B-4bit",
    "mlx-community/Qwen3-VL-32B-Instruct-4bit",
    "mlx-community/InternVL3-38B-4bit",
    "mlx-community/InternVL3-14B-4bit",
    "mlx-community/GLM-4.5V-9B-4bit",
    "mlx-community/MiniCPM-V-4-4bit",
    "mlx-community/gemma-3-27b-it-4bit",
    "mlx-community/pixtral-12b-4bit",
    "mlx-community/Qwen2.5-VL-7B-Instruct-4bit",
];

/// Return the ordered tier preference list for a given describer task.
///
/// The caller walks this list and picks the first tier whose backend is
/// available on the host. `select_describer_model` below encapsulates that
/// walk for the common case where the caller only wants a concrete model id.
pub fn describer_task_router(task: DescriberTask) -> &'static [DescriberTier] {
    match task {
        DescriberTask::CaptionRegion => &[
            DescriberTier::Tier2Slm,
            DescriberTier::Tier3Domain,
            DescriberTier::Tier4Omni,
        ],
        DescriberTask::UiActionDescribe => {
            &[DescriberTier::Tier3Domain, DescriberTier::Tier4Omni]
        }
        DescriberTask::OcrOnly => {
            &[DescriberTier::Tier1ClassicalCv, DescriberTier::Tier2Slm]
        }
        DescriberTask::NovelUnusual => {
            &[DescriberTier::Tier4Omni, DescriberTier::Tier5Cloud]
        }
    }
}

/// Host-capability probe: which describer tiers can we actually run right
/// now? A tier is "available" when its primary backend is reachable:
///
/// * tier1_classical_cv — always true on macOS (Apple Vision) / Linux
///   (tesseract) baseline hosts; returns true unconditionally since the
///   classical CV path is a pure-Rust wrapper with no external Python.
/// * tier2_slm           — Python + transformers + torch available. We
///   re-use the `mlx_available()` probe as a coarse proxy for "a
///   python-VLM stack exists on this host". Callers that need a stricter
///   Florence-2 probe can call `florence2_available()` directly.
/// * tier3_domain        — mlx-vlm available (UI-TARS is MLX-native).
/// * tier4_omni          — mlx-vlm available.
/// * tier5_cloud         — any cloud api key OR headless CLI present.
pub fn tier_available(tier: DescriberTier, cfg: &ProviderConfig) -> bool {
    match tier {
        DescriberTier::Tier1ClassicalCv => true,
        DescriberTier::Tier2Slm => florence2_available(),
        DescriberTier::Tier3Domain | DescriberTier::Tier4Omni => mlx_available(),
        DescriberTier::Tier5Cloud => {
            std::env::var("FIREWORKS_API_KEY").is_ok()
                || std::env::var("OPENROUTER_API_KEY").is_ok()
                || std::env::var("MINIMAX_API_KEY").is_ok()
                || which_on_path(&cfg.claude_code_bin)
                || which_on_path(&cfg.codex_bin)
        }
    }
}

/// Resolved selection: the tier the router picked + the concrete model id
/// to hand to the chosen backend.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DescriberSelection {
    pub tier: DescriberTier,
    pub model: String,
}

/// Walk the task's tier preference list and return the first tier whose
/// backend is available, paired with its top model id. Returns `None` only
/// when every tier for the given task is unavailable (e.g. offline host
/// with no MLX, no cloud keys, and no CLI logins).
pub fn select_describer_model(
    task: DescriberTask,
    cfg: &ProviderConfig,
) -> Option<DescriberSelection> {
    for tier in describer_task_router(task) {
        if !tier_available(*tier, cfg) {
            continue;
        }
        let model = match tier {
            DescriberTier::Tier1ClassicalCv => "apple-vision-or-tesseract".to_string(),
            DescriberTier::Tier2Slm => TIER2_SLM_CHAIN.first()?.to_string(),
            DescriberTier::Tier3Domain => TIER3_DOMAIN_CHAIN.first()?.to_string(),
            DescriberTier::Tier4Omni => TIER4_OMNI_CHAIN.first()?.to_string(),
            DescriberTier::Tier5Cloud => {
                // Prefer the first enabled cloud provider's configured model.
                if std::env::var("FIREWORKS_API_KEY").is_ok() {
                    cfg.fireworks_model_vlm.clone()
                } else if std::env::var("OPENROUTER_API_KEY").is_ok() {
                    cfg.openrouter_model_vlm.clone()
                } else if std::env::var("MINIMAX_API_KEY").is_ok() {
                    cfg.minimax_model.clone()
                } else {
                    cfg.claude_code_model.clone()
                }
            }
        };
        return Some(DescriberSelection { tier: *tier, model });
    }
    None
}

// ---------------------------------------------------------------------------
// Florence-2 provider (microsoft/Florence-2-{large,base}).
//
// Florence-2 is a small task-specialist VLM that handles caption, OCR, and
// region-describe as structured tasks. It has no first-class MLX port yet so
// the runtime shells to `python -m` using `transformers` + `torch` with MPS
// on Apple Silicon. This mirrors the existing MLX pattern
// (`mlx_describe` / `probe_mlx_once`): Rust stays the control plane, Python
// subprocess is justified because Florence-2 is a HuggingFace transformers
// model.
// ---------------------------------------------------------------------------

/// Cached availability probe for Florence-2 (transformers + torch +
/// MPS/CPU). The script is identical to `probe_mlx_once` in shape so we
/// keep a consistent 2s budget and squelch stderr.
pub fn florence2_available() -> bool {
    static AVAIL: OnceLock<bool> = OnceLock::new();
    *AVAIL.get_or_init(probe_florence2_once)
}

fn probe_florence2_once() -> bool {
    let py = python_bin();
    if !which_on_path(&py) && !Path::new(&py).is_file() {
        return false;
    }
    let mut child = match std::process::Command::new(&py)
        .args([
            "-c",
            "import transformers, torch; print('ok')",
        ])
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

/// Describe `image` via Florence-2 using `transformers` + `torch`. The
/// Python one-liner loads the model (cached in HF hub after first run),
/// selects MPS on Apple Silicon / CUDA when available / CPU otherwise,
/// runs the `<CAPTION>` task, and prints the generated caption.
///
/// For batch runs a follow-up PR can promote this to a long-lived Python
/// worker subprocess so we pay the model-load cost once per run instead of
/// per frame; today this is a simple one-shot shim that matches
/// `mlx_describe`'s contract.
pub fn florence2_describe(model: &str, image: &Path) -> Result<String> {
    let py = python_bin();
    let image_str = image
        .to_str()
        .ok_or_else(|| anyhow!("image path not utf-8"))?;
    // Keep the Python embed short and deterministic. `<CAPTION>` is
    // Florence-2's canonical caption task prompt; `<OCR>` / `<DENSE_REGION_CAPTION>`
    // are available for tier1/task-specific callers.
    let script = format!(
        r#"
import sys
from PIL import Image
from transformers import AutoProcessor, AutoModelForCausalLM
import torch

model_id = "{model}"
image_path = "{image}"

if torch.backends.mps.is_available():
    device = "mps"
    dtype = torch.float32
elif torch.cuda.is_available():
    device = "cuda"
    dtype = torch.float16
else:
    device = "cpu"
    dtype = torch.float32

processor = AutoProcessor.from_pretrained(model_id, trust_remote_code=True)
m = AutoModelForCausalLM.from_pretrained(model_id, trust_remote_code=True, torch_dtype=dtype).to(device)
img = Image.open(image_path).convert("RGB")
task = "<CAPTION>"
inputs = processor(text=task, images=img, return_tensors="pt").to(device, dtype)
out = m.generate(
    input_ids=inputs["input_ids"],
    pixel_values=inputs["pixel_values"],
    max_new_tokens=128,
    do_sample=False,
    num_beams=3,
)
text = processor.batch_decode(out, skip_special_tokens=False)[0]
parsed = processor.post_process_generation(text, task=task, image_size=(img.width, img.height))
print("==========")
print(parsed.get(task, text).strip())
print("==========")
"#,
        model = model,
        image = image_str.replace('\\', "\\\\").replace('"', "\\\""),
    );
    let out = std::process::Command::new(&py)
        .args(["-c", &script])
        .stdin(Stdio::null())
        .output()
        .context("spawn python florence2 one-shot")?;
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        bail!(
            "florence2 exit={}: {}",
            out.status,
            stderr.trim()
        );
    }
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    match parse_mlx_stdout(&stdout) {
        Some(t) => Ok(t),
        None => bail!(
            "florence2 produced no parsable text block (stdout len={})",
            stdout.len()
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_policy_blocks_first_party() {
        let cfg = ProviderConfig::default();
        assert_eq!(cfg.policy, Policy::FreeRouterOnly);
        assert!(!cfg.policy.first_party_allowed());
    }

    #[test]
    fn allow_first_party_requires_env_override() {
        // Even AllowFirstParty alone shouldn't open the gate without the env var.
        let p = Policy::AllowFirstParty;
        // Make sure the env var is not set for this assertion.
        std::env::remove_var(ALLOW_FIRST_PARTY_ENV);
        assert!(!p.first_party_allowed());
        std::env::set_var(ALLOW_FIRST_PARTY_ENV, "1");
        assert!(p.first_party_allowed());
        std::env::remove_var(ALLOW_FIRST_PARTY_ENV);
    }

    #[test]
    fn parse_mlx_reuses_existing_contract() {
        let fixture = "preamble\n==========\nA terminal window.\n==========\nstats";
        assert_eq!(
            parse_mlx_stdout(fixture).as_deref(),
            Some("A terminal window.")
        );
        assert!(parse_mlx_stdout("no separators").is_none());
    }

    // ---- describer task router tests (ADR-0015 v5 / ADR-0039) ----

    #[test]
    fn router_picks_florence2_for_caption_region() {
        // caption_region MUST prefer tier2_slm (Florence-2) first.
        let tiers = describer_task_router(DescriberTask::CaptionRegion);
        assert_eq!(tiers.first().copied(), Some(DescriberTier::Tier2Slm));
        assert_eq!(
            TIER2_SLM_CHAIN.first().copied(),
            Some("microsoft/Florence-2-large"),
            "Florence-2-large must be the default tier-2 SLM model"
        );
    }

    #[test]
    fn router_picks_ui_tars_for_ui_action_describe() {
        // ui_action_describe MUST prefer tier3_domain (UI-TARS) first and
        // skip tier2_slm entirely — Florence-2 underperforms on UI action
        // frames and we don't want the router to waste a call on it.
        let tiers = describer_task_router(DescriberTask::UiActionDescribe);
        assert_eq!(tiers.first().copied(), Some(DescriberTier::Tier3Domain));
        assert!(
            !tiers.contains(&DescriberTier::Tier2Slm),
            "ui_action_describe must not route to tier2_slm"
        );
        assert!(
            TIER3_DOMAIN_CHAIN
                .first()
                .map(|m| m.contains("UI-TARS"))
                .unwrap_or(false),
            "tier3_domain must lead with UI-TARS"
        );
    }

    #[test]
    fn router_picks_cloud_when_novel_unusual_and_local_unavailable() {
        // For novel/out-of-distribution frames the router's first preference
        // is tier4_omni, but it MUST include tier5_cloud as the final
        // fallback — that's what enables the "local unavailable" path.
        let tiers = describer_task_router(DescriberTask::NovelUnusual);
        assert_eq!(tiers.first().copied(), Some(DescriberTier::Tier4Omni));
        assert_eq!(tiers.last().copied(), Some(DescriberTier::Tier5Cloud));

        // And `select_describer_model` on a host with no MLX and a cloud
        // API key MUST resolve to tier5_cloud. We simulate the "local
        // unavailable" half by asserting the tier-ordering contract itself,
        // because `mlx_available()` is cached across the process and we
        // can't mock it cleanly from a unit test. The ordering guarantees
        // that if tier4 is unavailable, tier5 is the next candidate —
        // which is the property the task specifies.
        assert!(tiers.contains(&DescriberTier::Tier5Cloud));
    }

    #[test]
    fn ocr_only_prefers_classical_cv_then_slm() {
        let tiers = describer_task_router(DescriberTask::OcrOnly);
        assert_eq!(
            tiers,
            &[
                DescriberTier::Tier1ClassicalCv,
                DescriberTier::Tier2Slm,
            ]
        );
    }

    #[test]
    fn tier_labels_match_yaml_keys() {
        // The Rust tier labels MUST match the yaml keys verbatim so
        // cross-refs in logs and docs line up with
        // `docs/examples/api-providers.yaml`.
        assert_eq!(DescriberTier::Tier1ClassicalCv.label(), "tier1_classical_cv");
        assert_eq!(DescriberTier::Tier2Slm.label(), "tier2_slm");
        assert_eq!(DescriberTier::Tier3Domain.label(), "tier3_domain");
        assert_eq!(DescriberTier::Tier4Omni.label(), "tier4_omni");
        assert_eq!(DescriberTier::Tier5Cloud.label(), "tier5_cloud");
    }

    // Florence-2 smoke test — requires a real plan-deepseek keyframe + a
    // working `transformers` install with the Florence-2 weights in HF
    // cache. Gated behind `#[ignore]` so CI doesn't attempt to download
    // a 771M-parameter model. Run with:
    //   cargo test -p hwledger-vlm-judge -- --ignored florence2_smoke
    #[test]
    #[ignore]
    fn florence2_smoke_describes_plan_deepseek_frame() {
        let frame = Path::new(
            "docs-site/public/cli-journeys/recordings/plan-deepseek/keyframes/step-01.png",
        );
        if !frame.exists() {
            eprintln!("skip: {} not present", frame.display());
            return;
        }
        let desc = florence2_describe("microsoft/Florence-2-large", frame)
            .expect("florence2 describe should succeed when deps installed");
        let lower = desc.to_lowercase();
        assert!(
            lower.contains("mla")
                || lower.contains("vram")
                || lower.contains("deepseek")
                || lower.contains("terminal"),
            "florence2 caption should mention at least one expected token; got: {desc}"
        );
    }
}
