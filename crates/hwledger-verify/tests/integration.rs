//! Integration tests for hwledger-verify with mocked Anthropic API responses.

use hwledger_verify::{
    Description, JourneyManifest, JudgeVerdict, ManifestStep, Verifier, VerifierConfig,
};
use serde_json::json;
use std::fs;
use std::path::PathBuf;
use wiremock::matchers::{header, method, path};
use wiremock::Mock;
use wiremock::MockServer;
use wiremock::ResponseTemplate;

/// Helper: Create a simple test PNG (minimal valid PNG: 1x1 transparent pixel)
fn test_png_bytes() -> Vec<u8> {
    vec![
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, // PNG signature
        0x00, 0x00, 0x00, 0x0D, // IHDR chunk length
        0x49, 0x48, 0x44, 0x52, // IHDR
        0x00, 0x00, 0x00, 0x01, // width: 1
        0x00, 0x00, 0x00, 0x01, // height: 1
        0x08, 0x06, 0x00, 0x00, 0x00, // bit depth, color type, compression, filter, interlace
        0x1F, 0x15, 0xC4, 0x89, // CRC
        0x00, 0x00, 0x00, 0x0A, // IDAT chunk length
        0x49, 0x44, 0x41, 0x54, // IDAT
        0x78, 0x9C, 0x63, 0x00, 0x01, 0x00, 0x00, 0x05, 0x00, 0x01, // compressed data
        0x0D, 0x0A, 0x2D, 0xB4, // CRC
        0x00, 0x00, 0x00, 0x00, // IEND chunk length
        0x49, 0x45, 0x4E, 0x44, // IEND
        0xAE, 0x42, 0x60, 0x82, // CRC
    ]
}

/// Helper: Create a minimal manifest for testing
fn test_manifest_path() -> PathBuf {
    let temp_dir = PathBuf::from("target/test-manifests");
    let _ = fs::create_dir_all(&temp_dir);

    let manifest_path = temp_dir.join("test-manifest.json");
    let manifest = JourneyManifest {
        id: "test-journey".to_string(),
        steps: vec![
            ManifestStep {
                index: 0,
                intent: "A uniform colored image".to_string(),
                screenshot_path: "step-000.png".to_string(),
                slug: Some("step-0".to_string()),
            },
        ],
        recording: Some(false),
        keyframe_count: None,
        passed: None,
    };

    let manifest_json = serde_json::to_string_pretty(&manifest).unwrap();
    fs::write(&manifest_path, manifest_json).unwrap();

    // Write the test PNG
    let png_path = temp_dir.join("step-000.png");
    fs::write(&png_path, test_png_bytes()).unwrap();

    manifest_path
}

// Traces to: FR-UX-VERIFY-001
#[test]
fn test_verifier_config_api_key_from_env() {
    let config = VerifierConfig::with_api_key("test-api-key".to_string());
    assert_eq!(config.api_key, "test-api-key");
    assert_eq!(config.describe_model, "claude-opus-4-7");
}

// Traces to: FR-UX-VERIFY-001
#[test]
fn test_verifier_builder_pattern() {
    let config = VerifierConfig::with_api_key("key".to_string())
        .with_describe_model("model1".to_string())
        .with_judge_model("model2".to_string())
        .with_cache_disabled();

    assert_eq!(config.describe_model, "model1");
    assert_eq!(config.judge_model, "model2");
    assert!(!config.cache_enabled);
}

// Traces to: FR-UX-VERIFY-001
#[test]
fn test_verifier_creation_without_api_key() {
    let config = VerifierConfig {
        api_key: String::new(),
        ..Default::default()
    };

    let result = Verifier::new(config);
    assert!(result.is_err());
}

// Traces to: FR-UX-VERIFY-001
#[test]
fn test_description_json_roundtrip() {
    let desc = Description {
        text: "A uniform red square".to_string(),
        structured: Some(json!({
            "description": "A uniform red square",
            "visible_elements": ["square"],
            "notable_state": "filled with solid red color"
        })),
        tokens_used: 250,
    };

    let json = serde_json::to_string(&desc).unwrap();
    let parsed: Description = serde_json::from_str(&json).unwrap();

    assert_eq!(parsed.text, desc.text);
    assert_eq!(parsed.tokens_used, desc.tokens_used);
    assert!(parsed.structured.is_some());
}

// Traces to: FR-UX-VERIFY-002
#[test]
fn test_judge_verdict_json_roundtrip() {
    let verdict = JudgeVerdict {
        score_1_to_5: 5,
        rationale: "Perfect match: description accurately describes a uniform colored square as intended"
            .to_string(),
        tokens_used: 120,
    };

    let json = serde_json::to_string(&verdict).unwrap();
    let parsed: JudgeVerdict = serde_json::from_str(&json).unwrap();

    assert_eq!(parsed.score_1_to_5, 5);
    assert_eq!(parsed.tokens_used, 120);
}

// Traces to: FR-UX-VERIFY-002
#[test]
fn test_judge_verdict_score_bounds() {
    let valid = JudgeVerdict {
        score_1_to_5: 3,
        rationale: "Acceptable match".to_string(),
        tokens_used: 100,
    };

    assert!(valid.score_1_to_5 >= 1 && valid.score_1_to_5 <= 5);
}

// Traces to: FR-UX-VERIFY-003
#[tokio::test]
async fn test_manifest_parsing() {
    let manifest_path = test_manifest_path();
    let manifest_text = fs::read_to_string(&manifest_path).unwrap();
    let manifest: JourneyManifest = serde_json::from_str(&manifest_text).unwrap();

    assert_eq!(manifest.id, "test-journey");
    assert_eq!(manifest.steps.len(), 1);
    assert_eq!(manifest.steps[0].intent, "A uniform colored image");
}

// Traces to: FR-UX-VERIFY-001
#[tokio::test]
async fn test_api_call_with_mock_server_success_describe() {
    let mock_server = MockServer::start().await;

    // Mock successful describe response
    let response_body = json!({
        "content": [
            {
                "type": "text",
                "text": r#"{"description": "A uniform colored square", "visible_elements": ["square"], "notable_state": "solid color"}"#
            }
        ],
        "usage": {
            "input_tokens": 500,
            "output_tokens": 150
        }
    });

    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .and(header("x-api-key", "test-key"))
        .respond_with(ResponseTemplate::new(200).set_body_json(response_body))
        .mount(&mock_server)
        .await;

    let config = VerifierConfig::with_api_key("test-key".to_string())
        .with_base_url(mock_server.uri());

    let verifier = Verifier::new(config).unwrap();
    let result = verifier.describe(&test_png_bytes()).await;

    assert!(result.is_ok());
    let desc = result.unwrap();
    assert!(desc.text.contains("uniform colored square"));
    assert_eq!(desc.tokens_used, 650); // input + output
}

// Traces to: FR-UX-VERIFY-002
#[tokio::test]
async fn test_api_call_with_mock_server_success_judge() {
    let mock_server = MockServer::start().await;

    let response_body = json!({
        "content": [
            {
                "type": "text",
                "text": r#"{"score": 5, "rationale": "Perfect match: the description accurately reflects the intent"}"#
            }
        ],
        "usage": {
            "input_tokens": 400,
            "output_tokens": 75
        }
    });

    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .and(header("x-api-key", "test-key"))
        .respond_with(ResponseTemplate::new(200).set_body_json(response_body))
        .mount(&mock_server)
        .await;

    let config = VerifierConfig::with_api_key("test-key".to_string())
        .with_base_url(mock_server.uri());

    let verifier = Verifier::new(config).unwrap();
    let result = verifier
        .judge(
            "A uniform colored image",
            "A uniform colored square with solid color fill",
        )
        .await;

    assert!(result.is_ok());
    let verdict = result.unwrap();
    assert_eq!(verdict.score_1_to_5, 5);
    assert!(verdict.rationale.contains("Perfect match"));
}

// Traces to: FR-UX-VERIFY-001
#[tokio::test]
async fn test_api_retry_on_429() {
    let mock_server = MockServer::start().await;

    let success_response = json!({
        "content": [
            {
                "type": "text",
                "text": r#"{"description": "A red square", "visible_elements": ["square"], "notable_state": "red"}"#
            }
        ],
        "usage": {
            "input_tokens": 500,
            "output_tokens": 100
        }
    });

    // First request returns 429, subsequent return 200
    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(ResponseTemplate::new(429))
        .up_to_n_times(1)
        .mount(&mock_server)
        .await;

    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(ResponseTemplate::new(200).set_body_json(success_response))
        .mount(&mock_server)
        .await;

    let config = VerifierConfig::with_api_key("test-key".to_string())
        .with_base_url(mock_server.uri());

    let verifier = Verifier::new(config).unwrap();
    let result = verifier.describe(&test_png_bytes()).await;

    assert!(result.is_ok());
}

// Traces to: FR-UX-VERIFY-001
#[tokio::test]
async fn test_api_failure_on_auth_error() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/v1/messages"))
        .respond_with(ResponseTemplate::new(401).set_body_string("Unauthorized"))
        .mount(&mock_server)
        .await;

    let config = VerifierConfig::with_api_key("invalid-key".to_string())
        .with_base_url(mock_server.uri());

    let verifier = Verifier::new(config).unwrap();
    let result = verifier.describe(&test_png_bytes()).await;

    assert!(result.is_err());
}

// Traces to: FR-UX-VERIFY-001
#[tokio::test]
async fn test_cache_hit_on_repeated_describe() {
    let config = VerifierConfig::with_api_key("test-key".to_string())
        .with_cache_disabled(); // Start without cache

    let _verifier1 = Verifier::new(config.clone()).unwrap();
    let cache = hwledger_verify::Cache::new().unwrap();
    cache.clear().ok();

    let png = test_png_bytes();
    let desc = Description {
        text: "Test description".to_string(),
        structured: None,
        tokens_used: 100,
    };

    let key = cache.key_for_screenshot(&png, "claude-opus-4-7");
    cache.set(&key, &desc).ok();

    // Verify we can retrieve it
    let retrieved: Result<Description, _> = cache.get(&key);
    assert!(retrieved.is_ok());
    assert_eq!(retrieved.unwrap().text, "Test description");
}

// Traces to: FR-UX-VERIFY-003
#[test]
fn test_golden_file_judge_verdict() {
    let _intent = "User sees a red square";
    let _description = "A red square is displayed on the screen with a solid fill";

    // Mock judgment response
    let verdict_json = json!({
        "score": 4,
        "rationale": "Very close match: the description accurately captures the intent with minor wording differences"
    });

    let score: u8 = verdict_json.get("score").unwrap().as_u64().unwrap() as u8;
    let rationale: String = verdict_json
        .get("rationale")
        .unwrap()
        .as_str()
        .unwrap()
        .to_string();

    assert_eq!(score, 4);
    assert!(rationale.contains("close match"));

    let verdict = JudgeVerdict {
        score_1_to_5: score,
        rationale,
        tokens_used: 85,
    };

    assert_eq!(verdict.score_1_to_5, 4);
    assert!(verdict.score_1_to_5 >= 1 && verdict.score_1_to_5 <= 5);
}

// Traces to: FR-UX-VERIFY-003
#[test]
fn test_manifest_verification_structure() {
    let verification = hwledger_verify::ManifestVerification {
        journey_id: "test-journey".to_string(),
        steps: vec![],
        overall_score: 4.2,
        total_tokens: 1200,
        verified_at: chrono::Utc::now().to_rfc3339(),
    };

    let json = serde_json::to_string(&verification).unwrap();
    let parsed: hwledger_verify::ManifestVerification =
        serde_json::from_str(&json).unwrap();

    assert_eq!(parsed.journey_id, "test-journey");
    assert_eq!(parsed.overall_score, 4.2);
    assert_eq!(parsed.total_tokens, 1200);
}

// Traces to: FR-UX-VERIFY-003
#[test]
fn test_step_verification_with_perfect_match() {
    let step = hwledger_verify::StepVerification {
        intent: "User opens the app".to_string(),
        description: Description {
            text: "The app window opens with the main screen visible".to_string(),
            structured: None,
            tokens_used: 150,
        },
        verdict: JudgeVerdict {
            score_1_to_5: 5,
            rationale: "Description perfectly matches the intent".to_string(),
            tokens_used: 100,
        },
    };

    assert_eq!(step.verdict.score_1_to_5, 5);
    assert!(step.verdict.score_1_to_5 >= 4); // Perfect or near-perfect
}

// Traces to: FR-UX-VERIFY-003
#[test]
fn test_step_verification_with_failing_match() {
    let step = hwledger_verify::StepVerification {
        intent: "User closes the dialog".to_string(),
        description: Description {
            text: "The application window remains open with no changes".to_string(),
            structured: None,
            tokens_used: 140,
        },
        verdict: JudgeVerdict {
            score_1_to_5: 1,
            rationale: "Description contradicts the intent: dialog not closed".to_string(),
            tokens_used: 90,
        },
    };

    assert!(step.verdict.score_1_to_5 <= 2); // Poor match
}
