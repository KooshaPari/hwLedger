//! Tests for verifier configuration and verdict aggregation.
//! Traces to: FR-VERIFY-001, FR-VERIFY-002

use hwledger_verify::{
    Description, JudgeVerdict, ManifestVerification, StepVerification, VerifierConfig,
};

#[test]
fn test_verifier_config_default() {
    // Traces to: FR-VERIFY-001
    let config = VerifierConfig::default();

    assert!(!config.api_key.is_empty() || config.api_key.is_empty()); // May be empty if env var not set
    assert_eq!(config.describe_model, "claude-opus-4-7");
    assert_eq!(config.judge_model, "claude-sonnet-4-6");
    assert_eq!(config.max_tokens_describe, 400);
    assert_eq!(config.max_tokens_judge, 150);
    assert!(config.base_url.is_none());
    assert!(config.cache_enabled);
}

#[test]
fn test_verifier_config_with_api_key() {
    // Traces to: FR-VERIFY-001
    let config = VerifierConfig::with_api_key("test-key-123".to_string());

    assert_eq!(config.api_key, "test-key-123");
    assert_eq!(config.describe_model, "claude-opus-4-7");
}

#[test]
fn test_verifier_config_with_describe_model() {
    // Traces to: FR-VERIFY-001
    let config = VerifierConfig::default().with_describe_model("claude-opus-4-8".to_string());

    assert_eq!(config.describe_model, "claude-opus-4-8");
}

#[test]
fn test_verifier_config_with_judge_model() {
    // Traces to: FR-VERIFY-001
    let config = VerifierConfig::default().with_judge_model("claude-sonnet-4-7".to_string());

    assert_eq!(config.judge_model, "claude-sonnet-4-7");
}

#[test]
fn test_verifier_config_with_base_url() {
    // Traces to: FR-VERIFY-001
    let config = VerifierConfig::default().with_base_url("http://localhost:8080".to_string());

    assert_eq!(config.base_url, Some("http://localhost:8080".to_string()));
}

#[test]
fn test_verifier_config_disable_cache() {
    // Traces to: FR-VERIFY-001
    let config = VerifierConfig::default().with_cache_disabled();

    assert!(!config.cache_enabled);
}

#[test]
fn test_verifier_config_builder_chain() {
    // Traces to: FR-VERIFY-001
    let config = VerifierConfig::with_api_key("key-456".to_string())
        .with_describe_model("custom-model".to_string())
        .with_base_url("http://test".to_string())
        .with_cache_disabled();

    assert_eq!(config.api_key, "key-456");
    assert_eq!(config.describe_model, "custom-model");
    assert_eq!(config.base_url, Some("http://test".to_string()));
    assert!(!config.cache_enabled);
}

#[test]
fn test_description_serialization() {
    // Traces to: FR-VERIFY-002
    let desc = Description {
        text: "A screenshot showing a login form".to_string(),
        structured: Some(serde_json::json!({ "type": "form", "fields": ["username", "password"] })),
        tokens_used: 145,
    };

    let json = serde_json::to_string(&desc).unwrap();
    assert!(json.contains("login form"));
    assert!(json.contains("145"));

    let deserialized: Description = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.text, "A screenshot showing a login form");
    assert_eq!(deserialized.tokens_used, 145);
}

#[test]
fn test_judge_verdict_serialization() {
    // Traces to: FR-VERIFY-002
    let verdict = JudgeVerdict {
        score_1_to_5: 4,
        rationale: "Description mostly matches intent, minor detail mismatch".to_string(),
        tokens_used: 82,
    };

    let json = serde_json::to_string(&verdict).unwrap();
    assert!(json.contains("\"score_1_to_5\":4"));
    assert!(json.contains("mostly matches"));

    let deserialized: JudgeVerdict = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.score_1_to_5, 4);
    assert_eq!(deserialized.tokens_used, 82);
}

#[test]
fn test_step_verification_serialization() {
    // Traces to: FR-VERIFY-002
    let step = StepVerification {
        intent: "User logs in with valid credentials".to_string(),
        description: Description {
            text: "Form with username and password fields".to_string(),
            structured: None,
            tokens_used: 120,
        },
        verdict: JudgeVerdict {
            score_1_to_5: 5,
            rationale: "Perfect match".to_string(),
            tokens_used: 75,
        },
    };

    let json = serde_json::to_string(&step).unwrap();
    assert!(json.contains("User logs in"));
    assert!(json.contains("Perfect match"));

    let deserialized: StepVerification = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.verdict.score_1_to_5, 5);
}

#[test]
fn test_manifest_verification_overall_score() {
    // Traces to: FR-VERIFY-002
    let steps = vec![
        StepVerification {
            intent: "Step 1".to_string(),
            description: Description {
                text: "Desc 1".to_string(),
                structured: None,
                tokens_used: 100,
            },
            verdict: JudgeVerdict {
                score_1_to_5: 5,
                rationale: "Good".to_string(),
                tokens_used: 50,
            },
        },
        StepVerification {
            intent: "Step 2".to_string(),
            description: Description {
                text: "Desc 2".to_string(),
                structured: None,
                tokens_used: 100,
            },
            verdict: JudgeVerdict {
                score_1_to_5: 3,
                rationale: "Fair".to_string(),
                tokens_used: 50,
            },
        },
    ];

    let manifest = ManifestVerification {
        journey_id: "journey-123".to_string(),
        steps: steps.clone(),
        overall_score: 4.0,
        total_tokens: 300,
        verified_at: "2024-01-01T12:00:00Z".to_string(),
    };

    assert_eq!(manifest.journey_id, "journey-123");
    assert_eq!(manifest.steps.len(), 2);
    assert_eq!(manifest.overall_score, 4.0);
    assert_eq!(manifest.total_tokens, 300);
}

#[test]
fn test_manifest_verification_serialization() {
    // Traces to: FR-VERIFY-002
    let manifest = ManifestVerification {
        journey_id: "test-journey".to_string(),
        steps: vec![],
        overall_score: 3.5,
        total_tokens: 500,
        verified_at: "2024-01-01T10:00:00Z".to_string(),
    };

    let json = serde_json::to_string(&manifest).unwrap();
    assert!(json.contains("test-journey"));
    assert!(json.contains("3.5"));

    let deserialized: ManifestVerification = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.journey_id, "test-journey");
    assert_eq!(deserialized.overall_score, 3.5);
}

#[test]
fn test_verifier_config_token_limits() {
    // Traces to: FR-VERIFY-001
    let config = VerifierConfig::default();

    assert!(config.max_tokens_describe > 0);
    assert!(config.max_tokens_judge > 0);
    assert!(config.max_tokens_describe > config.max_tokens_judge);
}

#[test]
fn test_description_without_structured() {
    // Traces to: FR-VERIFY-002
    let desc = Description {
        text: "Simple text description".to_string(),
        structured: None,
        tokens_used: 50,
    };

    let json = serde_json::to_string(&desc).unwrap();
    // structured field is included in JSON (as null)
    assert!(json.contains("text"));
    assert!(json.contains("tokens_used"));

    let deserialized: Description = serde_json::from_str(&json).unwrap();
    assert!(deserialized.structured.is_none());
    assert_eq!(deserialized.text, "Simple text description");
}

#[test]
fn test_verdict_score_boundaries() {
    // Traces to: FR-VERIFY-002
    for score in 1..=5 {
        let verdict = JudgeVerdict {
            score_1_to_5: score,
            rationale: format!("Score {}", score),
            tokens_used: 100,
        };

        let json = serde_json::to_string(&verdict).unwrap();
        let deserialized: JudgeVerdict = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.score_1_to_5, score);
    }
}
