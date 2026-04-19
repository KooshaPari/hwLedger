//! Ollama REST API adapter for model metadata ingestion.
//!
//! Ollama exposes models via a simple REST API:
//! - `GET /api/tags` — list all loaded/available models
//! - `POST /api/show` — fetch detailed metadata for a model

use crate::error::IngestError;
use serde::{Deserialize, Serialize};

/// Default Ollama REST endpoint.
pub const DEFAULT_OLLAMA_BASE_URL: &str = "http://localhost:11434";

/// Ollama model entry from `GET /api/tags`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OllamaModel {
    /// Model name (e.g., "llama2", "mistral:7b-instruct").
    pub name: String,
    /// Modified timestamp.
    pub modified_at: String,
    /// Model size in bytes.
    pub size: u64,
}

/// Ollama model details from `POST /api/show`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OllamaModelDetails {
    /// Model family (e.g., "llama", "mistral").
    pub format: Option<String>,
    /// Modelfile text content.
    pub modelfile: Option<String>,
    /// Parameters as a dict-like structure.
    pub parameters: Option<String>,
    /// Template or other metadata.
    pub template: Option<String>,
    /// Details subobject.
    pub details: Option<OllamaDetails>,
}

/// Sub-object containing model architecture details.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OllamaDetails {
    /// Model family (e.g., "llama").
    pub family: Option<String>,
    /// Parameter count as string (e.g., "7B", "13B").
    pub parameter_size: Option<String>,
    /// Quantization level (e.g., "Q4_0", "Q5_K_M").
    pub quantization_level: Option<String>,
}

/// List models available in Ollama.
///
/// Hits `GET {base_url}/api/tags` and returns a parsed list of [`OllamaModel`].
///
/// # Arguments
/// * `base_url` — Ollama REST endpoint (default: `http://localhost:11434`)
///
/// # Traces
/// FR-PLAN-001: Ingest model metadata from Ollama
#[cfg(feature = "rest")]
pub async fn list(base_url: &str) -> Result<Vec<OllamaModel>, IngestError> {
    let url = format!("{}/api/tags", base_url);
    let client = reqwest::Client::new();
    let response = client.get(&url).send().await?;
    let body = response.text().await?;
    let parsed: serde_json::Value = serde_json::from_str(&body)?;

    let models = parsed
        .get("models")
        .and_then(|m| m.as_array())
        .ok_or_else(|| {
            IngestError::Parse(
                "Expected 'models' array in Ollama /api/tags response".to_string(),
            )
        })?;

    let result: Result<Vec<OllamaModel>, serde_json::Error> =
        models.iter().map(|m| serde_json::from_value(m.clone())).collect();

    result.map_err(IngestError::from)
}

/// Fetch detailed metadata for a specific Ollama model.
///
/// Hits `POST {base_url}/api/show` with `{"name": model}` and returns parsed [`OllamaModelDetails`].
///
/// # Arguments
/// * `base_url` — Ollama REST endpoint (default: `http://localhost:11434`)
/// * `model` — Model name (e.g., "llama2", "mistral:7b-instruct")
///
/// # Traces
/// FR-PLAN-001: Ingest model metadata from Ollama
#[cfg(feature = "rest")]
pub async fn show(base_url: &str, model: &str) -> Result<OllamaModelDetails, IngestError> {
    let url = format!("{}/api/show", base_url);
    let client = reqwest::Client::new();
    let payload = serde_json::json!({ "name": model });

    let response = client.post(&url).json(&payload).send().await?;
    let body = response.text().await?;
    let details: OllamaModelDetails = serde_json::from_str(&body)?;

    Ok(details)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Traces to: FR-PLAN-001
    #[test]
    fn test_ollama_model_structure() {
        let model = OllamaModel {
            name: "llama2".to_string(),
            modified_at: "2024-01-15T10:30:00Z".to_string(),
            size: 3_825_305_600,
        };
        assert_eq!(model.name, "llama2");
        assert_eq!(model.size, 3_825_305_600);
    }

    // Traces to: FR-PLAN-001
    #[test]
    fn test_ollama_details_structure() {
        let details = OllamaDetails {
            family: Some("llama".to_string()),
            parameter_size: Some("7B".to_string()),
            quantization_level: Some("Q4_0".to_string()),
        };
        assert_eq!(details.family, Some("llama".to_string()));
        assert_eq!(details.parameter_size, Some("7B".to_string()));
    }

    // Traces to: FR-PLAN-001
    #[test]
    fn test_ollama_model_details_serialization() {
        let details = OllamaModelDetails {
            format: Some("gguf".to_string()),
            modelfile: Some("FROM base-model".to_string()),
            parameters: None,
            template: None,
            details: Some(OllamaDetails {
                family: Some("mistral".to_string()),
                parameter_size: Some("7B".to_string()),
                quantization_level: Some("Q5_K_M".to_string()),
            }),
        };

        let json = serde_json::to_string(&details).expect("serialization failed");
        let deserialized: OllamaModelDetails =
            serde_json::from_str(&json).expect("deserialization failed");

        assert_eq!(deserialized.format, details.format);
        assert_eq!(deserialized.details.as_ref().unwrap().family, Some("mistral".to_string()));
    }

    // Traces to: FR-PLAN-001
    #[cfg(feature = "rest")]
    #[tokio::test]
    async fn test_ollama_list_with_mock() {
        use wiremock::matchers::method;
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;
        let response_body = r#"{
            "models": [
                {
                    "name": "llama2",
                    "modified_at": "2024-01-15T10:30:00Z",
                    "size": 3825305600
                },
                {
                    "name": "mistral:7b-instruct",
                    "modified_at": "2024-01-16T14:20:00Z",
                    "size": 4294967296
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
        assert_eq!(models[0].name, "llama2");
        assert_eq!(models[1].name, "mistral:7b-instruct");
    }

    // Traces to: FR-PLAN-001
    #[cfg(feature = "rest")]
    #[tokio::test]
    async fn test_ollama_show_with_mock() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;
        let response_body = r#"{
            "format": "gguf",
            "modelfile": "FROM base-model",
            "parameters": null,
            "template": null,
            "details": {
                "family": "llama",
                "parameter_size": "7B",
                "quantization_level": "Q4_0"
            }
        }"#;

        Mock::given(method("POST"))
            .and(path("/api/show"))
            .respond_with(ResponseTemplate::new(200).set_body_string(response_body))
            .mount(&mock_server)
            .await;

        let base_url = mock_server.uri();
        let details = show(&base_url, "llama2").await.expect("show failed");

        assert_eq!(details.format, Some("gguf".to_string()));
        assert_eq!(
            details.details.unwrap().family,
            Some("llama".to_string())
        );
    }

    // Traces to: FR-PLAN-001
    #[cfg(feature = "rest")]
    #[tokio::test]
    async fn test_ollama_list_malformed_response() {
        use wiremock::matchers::method;
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(200).set_body_string(r#"{"bad": "structure"}"#))
            .mount(&mock_server)
            .await;

        let base_url = mock_server.uri();
        let result = list(&base_url).await;

        assert!(result.is_err());
    }
}
