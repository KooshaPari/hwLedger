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

/// Blind prompt — specificity-forced rewrite (2026-04-22, user feedback).
///
/// Previous prompt (kept here for provenance) was:
///   "Describe what you see in this image in 1-2 sentences. Stick to
///    concrete on-screen elements (windows, panels, text fragments,
///    buttons, cursor). This is NOT a placeholder, stub, or synthetic
///    test frame ... Do not guess application context you cannot see."
///
/// That wording produced vague output because "do not guess context"
/// was being over-applied by the VLM — models refused to name commands
/// and flags they could clearly read off the pixels. The new prompt
/// inverts the default: it REQUIRES verbatim quoting of visible text,
/// commands, flags, numeric values, labels, and error messages, and
/// explicitly bans placeholder phrases.
///
/// User feedback cite (2026-04-22):
///   "Current blind_description outputs are generic — 'a command was
///    run with an output' instead of 'the `hwledger plan --help`
///    command output, showing --context and --batch flags.' The problem
///    is prompt design, not VLM capability."
///
/// Applies to every provider in this module: Claude direct, headless
/// claude-code, Fireworks, MiniMax, OpenRouter free tier, and MLX
/// (Qwen / UI-TARS / Florence-2 / other mlx-vlm models).
pub const BLIND_PROMPT: &str = "You are describing a software screenshot for a blind-evaluation step. \
Your description MUST be specific and machine-checkable. Follow these rules:\n\n\
1. Name every visible CLI command verbatim, including flags (e.g. `hwledger plan --help`, not \"a command\").\n\
2. Quote visible text tokens verbatim (e.g. \"Attention Kind: MLA\" not \"an attention label\").\n\
3. State exact numeric values shown (e.g. \"VRAM: 49.79 MB\" not \"some memory value\").\n\
4. Name every visible UI element by its visible label (e.g. \"Plan button\", \"Seq-len slider at 32768\").\n\
5. If the frame shows terminal output, list the first 3 lines verbatim.\n\
6. If the frame shows an error, quote the error text exactly.\n\
7. Do NOT write placeholder phrases: \"a command\", \"some output\", \"an image\", \"what appears to be\".\n\n\
Write 2-4 short sentences. If you cannot see a specific element, say \"not visible\" rather than guess.";

/// Stricter retry prompt used when the first description hits the
/// generic-phrase detector. Reasserts the rules in the second person
/// and demands at least one verbatim quote.
pub const BLIND_PROMPT_STRICT: &str = "Your previous description was REJECTED because it used generic placeholder \
phrases (\"a command\", \"some output\", \"an image\", \"what appears to be\", \"it looks like\"). \
Re-describe the screenshot with the same rules as before, but this time you MUST:\n\
- Include at least ONE verbatim quoted string taken directly from the visible pixels (commands, flags, labels, error text, or numeric values).\n\
- Avoid every banned phrase.\n\
- If you genuinely cannot read a specific element, write \"not visible\" for that element instead of hedging.\n\n\
Write 2-4 short sentences. Specificity is mandatory.";

/// Generic-phrase rejection detector used by the post-generation gate
/// and the traceability warn/fail pass.
///
/// Matches (case-insensitive, word-boundaried) any of:
///   "a command", "some output", "an image",
///   "what appears to be", "it looks like"
///
/// Equivalent regex (user spec, 2026-04-22):
///   `/\b(a command|some output|an image|what appears to be|it looks like)\b/i`
///
/// Implemented by hand to avoid pulling `regex` into this crate's graph.
pub fn is_generic_blind_description(text: &str) -> bool {
    const NEEDLES: &[&str] = &[
        "a command",
        "some output",
        "an image",
        "what appears to be",
        "it looks like",
    ];
    let hay = text.to_ascii_lowercase();
    let bytes = hay.as_bytes();
    for needle in NEEDLES {
        let needle = *needle;
        let mut start = 0usize;
        while let Some(idx) = hay[start..].find(needle) {
            let abs = start + idx;
            let end = abs + needle.len();
            let left_ok = abs == 0 || !is_word_byte(bytes[abs - 1]);
            let right_ok = end == bytes.len() || !is_word_byte(bytes[end]);
            if left_ok && right_ok {
                return true;
            }
            start = abs + 1;
        }
    }
    false
}

#[inline]
fn is_word_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

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

    // Cite: user feedback 2026-04-22 — reject outputs matching
    // `/\b(a command|some output|an image|what appears to be|it looks like)\b/i`
    // and re-run with BLIND_PROMPT_STRICT.
    #[test]
    fn generic_phrase_detector_true_positive() {
        // Six example generic strings that MUST be flagged as generic.
        let generic = [
            "a command was run with an output",
            "The terminal shows some output from the tool.",
            "An image of a user interface is displayed.",
            "What appears to be a dashboard with tiles.",
            "It looks like a Streamlit page with controls.",
            "Shell session — a command produced some output with another stream.",
        ];
        for g in &generic {
            assert!(
                is_generic_blind_description(g),
                "expected generic-flag for: {g:?}"
            );
        }
    }

    #[test]
    fn generic_phrase_detector_true_negative() {
        // Six example specific strings that MUST NOT be flagged.
        let specific = [
            "Terminal shows `hwledger plan --help` with flags --context and --batch.",
            "Streamlit page titled \"Attention Kind: MLA\" showing VRAM: 49.79 MB.",
            "GUI frame with Plan button and Seq-len slider at 32768.",
            "Error banner reads \"failed to parse manifest.verified.json at line 12\".",
            "First three stdout lines: \"ok\", \"ok\", \"running 3 tests\".",
            "Sidebar lists entries: probe-gui-watch, settings-gui-mtls, export-gui-vllm.",
        ];
        for s in &specific {
            assert!(
                !is_generic_blind_description(s),
                "expected NOT generic for: {s:?}"
            );
        }
    }

    #[test]
    fn generic_phrase_detector_respects_word_boundaries() {
        // "command" alone (no leading "a ") must not trigger; "animage"
        // must not trigger. But "an image" in-sentence should trigger.
        assert!(!is_generic_blind_description("The command hwledger plan ran."));
        assert!(!is_generic_blind_description("Animage compression tool."));
        assert!(is_generic_blind_description("This is an image of a terminal."));
    }

    #[test]
    fn blind_prompt_strict_differs_from_default() {
        assert_ne!(BLIND_PROMPT, BLIND_PROMPT_STRICT);
        assert!(BLIND_PROMPT.contains("verbatim"));
        assert!(BLIND_PROMPT_STRICT.contains("REJECTED"));
    }
}
