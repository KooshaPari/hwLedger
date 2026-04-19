//! Blackbox screenshot verification via Claude VLM + LLM-judge equivalence.
//!
//! This crate provides a verification harness for user-journey screenshots.
//! It leverages Claude Opus 4.7 for vision-based description and Claude Sonnet 4.6
//! for equivalence judging (intent vs. description match).
//!
//! See `docs/research/12-ui-journey-harness-2026.md` for full context.

use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use thiserror::Error;
use tracing::{debug, info, warn};


pub mod cache;
pub mod client;

pub use cache::Cache;
pub use client::AnthropicClient;

/// Configuration for the verifier.
#[derive(Debug, Clone)]
pub struct VerifierConfig {
    /// Anthropic API key (reads from ANTHROPIC_API_KEY or HWLEDGER_ANTHROPIC_API_KEY env).
    pub api_key: String,
    /// Model for vision-based description (default: "claude-opus-4-7").
    pub describe_model: String,
    /// Model for equivalence judging (default: "claude-sonnet-4-6").
    pub judge_model: String,
    /// Max tokens for describe call (default: 400).
    pub max_tokens_describe: u32,
    /// Max tokens for judge call (default: 150).
    pub max_tokens_judge: u32,
    /// Optional base URL override for testing (e.g., wiremock).
    pub base_url: Option<String>,
    /// Enable caching (default: true).
    pub cache_enabled: bool,
}

impl Default for VerifierConfig {
    fn default() -> Self {
        let api_key = std::env::var("ANTHROPIC_API_KEY")
            .or_else(|_| std::env::var("HWLEDGER_ANTHROPIC_API_KEY"))
            .unwrap_or_default();

        Self {
            api_key,
            describe_model: "claude-opus-4-7".to_string(),
            judge_model: "claude-sonnet-4-6".to_string(),
            max_tokens_describe: 400,
            max_tokens_judge: 150,
            base_url: None,
            cache_enabled: true,
        }
    }
}

impl VerifierConfig {
    /// Create a new configuration with the given API key.
    pub fn with_api_key(api_key: String) -> Self {
        Self {
            api_key,
            ..Default::default()
        }
    }

    /// Set the describe model.
    pub fn with_describe_model(mut self, model: String) -> Self {
        self.describe_model = model;
        self
    }

    /// Set the judge model.
    pub fn with_judge_model(mut self, model: String) -> Self {
        self.judge_model = model;
        self
    }

    /// Set base URL (for testing).
    pub fn with_base_url(mut self, url: String) -> Self {
        self.base_url = Some(url);
        self
    }

    /// Disable caching.
    pub fn with_cache_disabled(mut self) -> Self {
        self.cache_enabled = false;
        self
    }
}

/// Vision description result from Claude Opus.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Description {
    /// The human-readable description.
    pub text: String,
    /// Optional structured JSON response.
    pub structured: Option<serde_json::Value>,
    /// Tokens used in the describe call.
    pub tokens_used: u32,
}

/// Judge verdict result from Claude Sonnet.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JudgeVerdict {
    /// Score from 1 to 5: 5=full match, 1=complete mismatch.
    pub score_1_to_5: u8,
    /// Rationale for the score.
    pub rationale: String,
    /// Tokens used in the judge call.
    pub tokens_used: u32,
}

/// Verification result for a single step.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepVerification {
    /// Intent for this step.
    pub intent: String,
    /// VLM-generated description.
    pub description: Description,
    /// Judge verdict.
    pub verdict: JudgeVerdict,
}

/// Overall manifest verification result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestVerification {
    /// Journey ID.
    pub journey_id: String,
    /// Per-step verification results.
    pub steps: Vec<StepVerification>,
    /// Weighted average score across all steps.
    pub overall_score: f32,
    /// Total tokens used across all calls.
    pub total_tokens: u32,
    /// Timestamp of verification.
    pub verified_at: String,
}

/// Journey manifest (read from journeys/<id>/manifest.json).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JourneyManifest {
    /// Journey ID.
    pub id: String,
    /// Journey steps with intent and screenshot path.
    pub steps: Vec<ManifestStep>,
    /// Whether this journey was from a video recording.
    pub recording: Option<bool>,
    /// Number of keyframes extracted (if applicable).
    pub keyframe_count: Option<usize>,
    /// Overall journey pass/fail status.
    pub passed: Option<bool>,
}

/// A single step in the journey.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestStep {
    /// 0-indexed step number.
    pub index: usize,
    /// User intent for this step.
    pub intent: String,
    /// Path to the PNG screenshot (relative to manifest directory).
    pub screenshot_path: String,
    /// Optional slug/identifier.
    pub slug: Option<String>,
}

/// Error type for verification operations.
#[derive(Debug, Error)]
pub enum VerifyError {
    #[error("API error (status {status}): {body}")]
    Api { status: u16, body: String },

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON parse error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Parse error: {0}")]
    Parse(String),

    #[error("Missing API key")]
    MissingApiKey,

    #[error("Missing screenshot file: {path}")]
    MissingScreenshot { path: String },

    #[error("Invalid image format: {0}")]
    InvalidImageFormat(String),

    #[error("Invalid response from API: {0}")]
    InvalidResponse(String),

    #[error("Retry exhausted: {0}")]
    RetryExhausted(String),

    #[error("Manifest error: {0}")]
    ManifestError(String),

    #[error("Cache error: {0}")]
    CacheError(String),
}

/// The main verification engine.
pub struct Verifier {
    config: VerifierConfig,
    client: AnthropicClient,
    cache: Option<Cache>,
}

impl Verifier {
    /// Create a new verifier with the given configuration.
    pub fn new(config: VerifierConfig) -> Result<Self, VerifyError> {
        if config.api_key.is_empty() {
            return Err(VerifyError::MissingApiKey);
        }

        let client = AnthropicClient::new(config.clone());

        let cache = if config.cache_enabled {
            Cache::new().ok()
        } else {
            None
        };

        Ok(Self {
            config,
            client,
            cache,
        })
    }

    /// Describe a screenshot using Claude Opus 4.7 vision.
    ///
    /// Traces to: FR-UX-VERIFY-001
    pub async fn describe(&self, screenshot_png: &[u8]) -> Result<Description, VerifyError> {
        // Check cache
        if let Some(ref cache) = self.cache {
            let key = cache.key_for_screenshot(screenshot_png, &self.config.describe_model);
            if let Ok(cached) = cache.get(&key) {
                debug!("Cache hit for describe");
                return Ok(cached);
            }
        }

        info!(
            "Calling {} for vision description",
            self.config.describe_model
        );

        let description = self
            .client
            .describe(screenshot_png, &self.config.describe_model)
            .await?;

        // Store in cache
        if let Some(ref cache) = self.cache {
            let key = cache.key_for_screenshot(screenshot_png, &self.config.describe_model);
            let _ = cache.set(&key, &description);
        }

        Ok(description)
    }

    /// Judge whether a description matches an intent.
    ///
    /// Traces to: FR-UX-VERIFY-002
    pub async fn judge(
        &self,
        intent: &str,
        description: &str,
    ) -> Result<JudgeVerdict, VerifyError> {
        // Check cache
        if let Some(ref cache) = self.cache {
            let key = cache.key_for_judge(intent, description, &self.config.judge_model);
            if let Ok(cached) = cache.get(&key) {
                debug!("Cache hit for judge");
                return Ok(cached);
            }
        }

        info!("Calling {} for equivalence judgment", self.config.judge_model);

        let verdict = self
            .client
            .judge(intent, description, &self.config.judge_model)
            .await?;

        // Store in cache
        if let Some(ref cache) = self.cache {
            let key = cache.key_for_judge(intent, description, &self.config.judge_model);
            let _ = cache.set(&key, &verdict);
        }

        Ok(verdict)
    }

    /// Verify a single step: describe screenshot, then judge against intent.
    pub async fn verify_step(
        &self,
        intent: &str,
        screenshot_png: &[u8],
    ) -> Result<StepVerification, VerifyError> {
        let description = self.describe(screenshot_png).await?;
        let verdict = self.judge(intent, &description.text).await?;

        Ok(StepVerification {
            intent: intent.to_string(),
            description,
            verdict,
        })
    }

    /// Verify all steps in a journey manifest.
    ///
    /// Traces to: FR-UX-VERIFY-003
    pub async fn verify_manifest(
        &self,
        manifest_path: &Path,
    ) -> Result<ManifestVerification, VerifyError> {
        let manifest_text = fs::read_to_string(manifest_path)
            .map_err(VerifyError::Io)?;
        let manifest: JourneyManifest = serde_json::from_str(&manifest_text)
            .map_err(VerifyError::Json)?;

        let manifest_dir = manifest_path.parent()
            .ok_or_else(|| VerifyError::ManifestError("No parent directory".to_string()))?;

        let mut steps_verified = Vec::new();
        let mut total_tokens = 0u32;
        let mut scores = Vec::new();

        for step in &manifest.steps {
            let screenshot_path = manifest_dir.join(&step.screenshot_path);

            if !screenshot_path.exists() {
                warn!(
                    "Screenshot missing for step {}: {}",
                    step.index, step.screenshot_path
                );
                return Err(VerifyError::MissingScreenshot {
                    path: step.screenshot_path.clone(),
                });
            }

            let png_bytes = fs::read(&screenshot_path)
                .map_err(VerifyError::Io)?;

            let step_verification = self.verify_step(&step.intent, &png_bytes).await?;

            total_tokens += step_verification.description.tokens_used
                + step_verification.verdict.tokens_used;
            scores.push(step_verification.verdict.score_1_to_5 as f32);

            steps_verified.push(step_verification);
        }

        let overall_score = if scores.is_empty() {
            0.0
        } else {
            scores.iter().sum::<f32>() / scores.len() as f32
        };

        let verification = ManifestVerification {
            journey_id: manifest.id.clone(),
            steps: steps_verified,
            overall_score,
            total_tokens,
            verified_at: Utc::now().to_rfc3339(),
        };

        info!(
            "Journey {} verified: overall_score={:.2}, total_tokens={}",
            manifest.id, overall_score, total_tokens
        );

        Ok(verification)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Traces to: FR-UX-VERIFY-001
    #[test]
    fn test_verify_config_defaults() {
        let config = VerifierConfig::default();
        assert_eq!(config.describe_model, "claude-opus-4-7");
        assert_eq!(config.judge_model, "claude-sonnet-4-6");
        assert_eq!(config.max_tokens_describe, 400);
        assert_eq!(config.max_tokens_judge, 150);
        assert!(config.cache_enabled);
    }

    // Traces to: FR-UX-VERIFY-001
    #[test]
    fn test_config_builder() {
        let config = VerifierConfig::default()
            .with_describe_model("custom-opus".to_string())
            .with_judge_model("custom-sonnet".to_string())
            .with_cache_disabled();

        assert_eq!(config.describe_model, "custom-opus");
        assert_eq!(config.judge_model, "custom-sonnet");
        assert!(!config.cache_enabled);
    }

    // Traces to: FR-UX-VERIFY-001
    #[test]
    fn test_missing_api_key() {
        let config = VerifierConfig {
            api_key: String::new(),
            ..Default::default()
        };

        let result = Verifier::new(config);
        assert!(matches!(result, Err(VerifyError::MissingApiKey)));
    }

    // Traces to: FR-UX-VERIFY-001
    #[test]
    fn test_description_serialization() {
        let desc = Description {
            text: "A red square".to_string(),
            structured: Some(serde_json::json!({
                "visible_elements": ["square"],
                "color": "red"
            })),
            tokens_used: 150,
        };

        let json = serde_json::to_string(&desc).unwrap();
        let deserialized: Description = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.text, "A red square");
        assert_eq!(deserialized.tokens_used, 150);
    }

    // Traces to: FR-UX-VERIFY-002
    #[test]
    fn test_judge_verdict_serialization() {
        let verdict = JudgeVerdict {
            score_1_to_5: 5,
            rationale: "Perfect match".to_string(),
            tokens_used: 80,
        };

        let json = serde_json::to_string(&verdict).unwrap();
        let deserialized: JudgeVerdict = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.score_1_to_5, 5);
        assert_eq!(deserialized.rationale, "Perfect match");
    }

    // Traces to: FR-UX-VERIFY-002
    #[test]
    fn test_step_verification_serialization() {
        let step = StepVerification {
            intent: "User sees a red square".to_string(),
            description: Description {
                text: "A red square is displayed".to_string(),
                structured: None,
                tokens_used: 120,
            },
            verdict: JudgeVerdict {
                score_1_to_5: 4,
                rationale: "Close match, slightly different wording".to_string(),
                tokens_used: 75,
            },
        };

        let json = serde_json::to_string(&step).unwrap();
        let deserialized: StepVerification = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.intent, "User sees a red square");
        assert_eq!(deserialized.verdict.score_1_to_5, 4);
    }

    // Traces to: FR-UX-VERIFY-003
    #[test]
    fn test_manifest_verification_serialization() {
        let verification = ManifestVerification {
            journey_id: "test-journey".to_string(),
            steps: vec![],
            overall_score: 4.5,
            total_tokens: 500,
            verified_at: Utc::now().to_rfc3339(),
        };

        let json = serde_json::to_string(&verification).unwrap();
        let deserialized: ManifestVerification = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.journey_id, "test-journey");
        assert_eq!(deserialized.overall_score, 4.5);
    }
}
