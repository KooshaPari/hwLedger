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
            ("X-Title", "hwledger-frame-describer"),
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
}
