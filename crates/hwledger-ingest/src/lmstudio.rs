//! LM Studio REST API adapter for model metadata ingestion.
//!
//! LM Studio exposes an OpenAI-compatible models API at `GET {base_url}/v1/models`.
//! Each model entry contains architecture and parameter information.

use crate::error::IngestError;
use serde::{Deserialize, Serialize};

/// Default LM Studio REST endpoint.
pub const DEFAULT_LMSTUDIO_BASE_URL: &str = "http://localhost:1234";

/// LM Studio model entry from `GET /v1/models`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LmStudioModel {
    /// Unique model key/identifier.
    #[serde(rename = "modelKey")]
    pub model_key: String,

    /// Human-readable model name.
    #[serde(rename = "displayName")]
    pub display_name: String,

    /// Model architecture (e.g., "llama", "mistral", "qwen").
    pub architecture: Option<String>,

    /// Parameter count as a string (e.g., "7B", "13B", "70B").
    #[serde(rename = "paramsString")]
    pub params_string: Option<String>,

    /// Maximum context length the model supports.
    #[serde(rename = "maxContextLength")]
    pub max_context_length: Option<u32>,

    /// Quantization description if applicable.
    pub quantization: Option<String>,
}

/// List models available in LM Studio.
///
/// Hits `GET {base_url}/v1/models` and returns a parsed list of [`LmStudioModel`].
///
/// # Arguments
/// * `base_url` — LM Studio REST endpoint (default: `http://localhost:1234`)
///
/// # Traces
/// FR-PLAN-001: Ingest model metadata from LM Studio
#[cfg(feature = "rest")]
pub async fn list(base_url: &str) -> Result<Vec<LmStudioModel>, IngestError> {
    let url = format!("{}/v1/models", base_url);
    let client = reqwest::Client::new();
    let response = client.get(&url).send().await?;
    let body = response.text().await?;
    let parsed: serde_json::Value = serde_json::from_str(&body)?;

    let data = parsed
        .get("data")
        .and_then(|d| d.as_array())
        .ok_or_else(|| {
            IngestError::Parse(
                "Expected 'data' array in LM Studio /v1/models response".to_string(),
            )
        })?;

    let result: Result<Vec<LmStudioModel>, serde_json::Error> =
        data.iter().map(|m| serde_json::from_value(m.clone())).collect();

    result.map_err(IngestError::from)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Traces to: FR-PLAN-001
    #[test]
    fn test_lmstudio_model_structure() {
        let model = LmStudioModel {
            model_key: "local-model-abc123".to_string(),
            display_name: "Llama 2 7B Q4".to_string(),
            architecture: Some("llama".to_string()),
            params_string: Some("7B".to_string()),
            max_context_length: Some(4096),
            quantization: Some("Q4_0".to_string()),
        };

        assert_eq!(model.model_key, "local-model-abc123");
        assert_eq!(model.display_name, "Llama 2 7B Q4");
        assert_eq!(model.architecture, Some("llama".to_string()));
    }

    // Traces to: FR-PLAN-001
    #[test]
    fn test_lmstudio_model_deserialization() {
        let json = r#"{
            "modelKey": "local-model-xyz",
            "displayName": "Mistral 7B Instruct",
            "architecture": "mistral",
            "paramsString": "7B",
            "maxContextLength": 8192,
            "quantization": "Q5_K_M"
        }"#;

        let model: LmStudioModel = serde_json::from_str(json).expect("deserialization failed");

        assert_eq!(model.model_key, "local-model-xyz");
        assert_eq!(model.display_name, "Mistral 7B Instruct");
        assert_eq!(model.params_string, Some("7B".to_string()));
        assert_eq!(model.max_context_length, Some(8192));
    }

    // Traces to: FR-PLAN-001
    #[test]
    fn test_lmstudio_model_optional_fields() {
        let json = r#"{
            "modelKey": "model-minimal",
            "displayName": "Model with Minimal Fields"
        }"#;

        let model: LmStudioModel = serde_json::from_str(json).expect("deserialization failed");

        assert_eq!(model.model_key, "model-minimal");
        assert_eq!(model.display_name, "Model with Minimal Fields");
        assert_eq!(model.architecture, None);
        assert_eq!(model.params_string, None);
        assert_eq!(model.max_context_length, None);
    }

    // Traces to: FR-PLAN-001
    #[cfg(feature = "rest")]
    #[tokio::test]
    async fn test_lmstudio_list_with_mock() {
        use wiremock::matchers::method;
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;
        let response_body = r#"{
            "data": [
                {
                    "modelKey": "llama2-7b",
                    "displayName": "Llama 2 7B",
                    "architecture": "llama",
                    "paramsString": "7B",
                    "maxContextLength": 4096,
                    "quantization": "Q4_0"
                },
                {
                    "modelKey": "mistral-7b",
                    "displayName": "Mistral 7B Instruct",
                    "architecture": "mistral",
                    "paramsString": "7B",
                    "maxContextLength": 8192,
                    "quantization": "Q5_K_M"
                }
            ]
        }"#;

        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(200).set_body_string(response_body))
            .mount(&mock_server)
            .await;

        let base_url = mock_server.uri();
        let models = list(&base_url).await.expect("list failed");

        assert_eq!(models.len(), 2);
        assert_eq!(models[0].model_key, "llama2-7b");
        assert_eq!(models[1].display_name, "Mistral 7B Instruct");
    }

    // Traces to: FR-PLAN-001
    #[cfg(feature = "rest")]
    #[tokio::test]
    async fn test_lmstudio_list_empty() {
        use wiremock::matchers::method;
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(200).set_body_string(r#"{"data": []}"#))
            .mount(&mock_server)
            .await;

        let base_url = mock_server.uri();
        let models = list(&base_url).await.expect("list failed");

        assert!(models.is_empty());
    }

    // Traces to: FR-PLAN-001
    #[cfg(feature = "rest")]
    #[tokio::test]
    async fn test_lmstudio_list_malformed_response() {
        use wiremock::matchers::method;
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(200).set_body_string(r#"{"models": []}"#))
            .mount(&mock_server)
            .await;

        let base_url = mock_server.uri();
        let result = list(&base_url).await;

        assert!(result.is_err());
    }
}
