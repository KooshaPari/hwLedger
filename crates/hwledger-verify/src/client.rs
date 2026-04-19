//! Anthropic Messages API client for vision and judgment calls.

use base64::Engine;
use reqwest::Client;
use serde_json::json;
use std::time::Duration;
use tracing::{debug, warn};

use crate::{Description, JudgeVerdict, VerifierConfig, VerifyError};

const MAX_RETRIES: usize = 3;
const BASE_RETRY_DELAY_MS: u64 = 250;
const ANTHROPIC_API_VERSION: &str = "2023-06-01";

/// Anthropic Messages API client.
pub struct AnthropicClient {
    http_client: Client,
    config: VerifierConfig,
}

impl AnthropicClient {
    /// Create a new client with the given configuration.
    pub fn new(config: VerifierConfig) -> Self {
        let http_client = Client::new();
        Self { http_client, config }
    }

    /// Call the Messages API to describe a screenshot.
    pub async fn describe(
        &self,
        screenshot_png: &[u8],
        model: &str,
    ) -> Result<Description, VerifyError> {
        let base64_image = base64::engine::general_purpose::STANDARD.encode(screenshot_png);

        let system_prompt = r#"You are a precise UI observer. You will be shown a single PNG screenshot from a desktop app journey.
Describe exactly what you see in 3-5 sentences. Do not speculate about what the user might do next.
Respond with a JSON object: {"description": str, "visible_elements": [str], "notable_state": str}.
No prose outside the JSON."#;

        let user_message = "Describe what you see.";

        let body = json!({
            "model": model,
            "max_tokens": self.config.max_tokens_describe,
            "system": system_prompt,
            "messages": [
                {
                    "role": "user",
                    "content": [
                        {
                            "type": "text",
                            "text": user_message
                        },
                        {
                            "type": "image",
                            "source": {
                                "type": "base64",
                                "media_type": "image/png",
                                "data": base64_image
                            }
                        }
                    ]
                }
            ]
        });

        let response = self.call_with_retry(&body, self.config.max_tokens_describe).await?;

        // Parse the response
        let text = response
            .get("content")
            .and_then(|c| if let serde_json::Value::Array(arr) = c { arr.first() } else { None })
            .and_then(|c| c.get("text"))
            .and_then(|t| t.as_str())
            .ok_or_else(|| VerifyError::InvalidResponse("Missing text in content".to_string()))?;

        let tokens_used = response
            .get("usage")
            .and_then(|u| u.get("input_tokens").and_then(|t| t.as_u64()))
            .unwrap_or(0) as u32
            + response
                .get("usage")
                .and_then(|u| u.get("output_tokens").and_then(|t| t.as_u64()))
                .unwrap_or(0) as u32;

        // Try to parse structured JSON from the response
        let structured = serde_json::from_str::<serde_json::Value>(text).ok();

        debug!("describe response: {} tokens, structured={}", tokens_used, structured.is_some());

        Ok(Description { text: text.to_string(), structured, tokens_used })
    }

    /// Call the Messages API to judge equivalence between intent and description.
    pub async fn judge(
        &self,
        intent: &str,
        description: &str,
        model: &str,
    ) -> Result<JudgeVerdict, VerifyError> {
        let system_prompt = r#"You are evaluating whether a UI-journey observer's description matches the journey's declared intent.
Respond with a JSON object: {"score": 1-5, "rationale": str}.
Score 5 = description fully satisfies the intent; 1 = description contradicts or misses the intent entirely.
Be strict."#;

        let user_message = format!("Intent: {}\n\nObserver description: {}", intent, description);

        let body = json!({
            "model": model,
            "max_tokens": self.config.max_tokens_judge,
            "system": system_prompt,
            "messages": [
                {
                    "role": "user",
                    "content": user_message
                }
            ]
        });

        let response = self.call_with_retry(&body, self.config.max_tokens_judge).await?;

        // Parse the response
        let text = response
            .get("content")
            .and_then(|c| if let serde_json::Value::Array(arr) = c { arr.first() } else { None })
            .and_then(|c| c.get("text"))
            .and_then(|t| t.as_str())
            .ok_or_else(|| VerifyError::InvalidResponse("Missing text in content".to_string()))?;

        let parsed: serde_json::Value = serde_json::from_str(text)
            .map_err(|_| VerifyError::Parse(format!("Invalid JSON: {}", text)))?;

        let score =
            parsed.get("score").and_then(|s| s.as_u64()).ok_or_else(|| {
                VerifyError::Parse(format!("Missing or invalid score in: {}", text))
            })? as u8;

        if score == 0 || score > 5 {
            return Err(VerifyError::Parse(format!("Score out of range [1-5]: {}", score)));
        }

        let rationale = parsed
            .get("rationale")
            .and_then(|r| r.as_str())
            .unwrap_or("No rationale provided")
            .to_string();

        let tokens_used = response
            .get("usage")
            .and_then(|u| u.get("input_tokens").and_then(|t| t.as_u64()))
            .unwrap_or(0) as u32
            + response
                .get("usage")
                .and_then(|u| u.get("output_tokens").and_then(|t| t.as_u64()))
                .unwrap_or(0) as u32;

        debug!("judge response: {} tokens, score={}", tokens_used, score);

        Ok(JudgeVerdict { score_1_to_5: score, rationale, tokens_used })
    }

    /// Call the Anthropic Messages API with exponential backoff retry on 429/5xx.
    async fn call_with_retry(
        &self,
        body: &serde_json::Value,
        _max_tokens: u32,
    ) -> Result<serde_json::Value, VerifyError> {
        let url = match &self.config.base_url {
            Some(base) => format!("{}/v1/messages", base),
            None => "https://api.anthropic.com/v1/messages".to_string(),
        };

        for attempt in 0..MAX_RETRIES {
            debug!("API call attempt {}/{}", attempt + 1, MAX_RETRIES);

            let response = self
                .http_client
                .post(&url)
                .header("x-api-key", &self.config.api_key)
                .header("anthropic-version", ANTHROPIC_API_VERSION)
                .header("content-type", "application/json")
                .json(body)
                .send()
                .await
                .map_err(|e| VerifyError::Api {
                    status: 0,
                    body: format!("Request failed: {}", e),
                })?;

            let status = response.status();

            if status.is_success() {
                let json =
                    response.json::<serde_json::Value>().await.map_err(|e| VerifyError::Api {
                        status: 500,
                        body: format!("Failed to parse JSON response: {}", e),
                    })?;
                return Ok(json);
            }

            if status.as_u16() == 429 || status.is_server_error() {
                let delay_ms = BASE_RETRY_DELAY_MS * (2_u64.pow(attempt as u32));
                warn!("Rate limit or server error ({}), retrying in {}ms", status, delay_ms);

                if attempt < MAX_RETRIES - 1 {
                    tokio::time::sleep(Duration::from_millis(delay_ms)).await;
                    continue;
                }
            }

            let body_text =
                response.text().await.unwrap_or_else(|_| "(unable to read body)".to_string());

            return Err(VerifyError::Api { status: status.as_u16(), body: body_text });
        }

        Err(VerifyError::RetryExhausted("Max retries exceeded".to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Traces to: FR-UX-VERIFY-001
    #[test]
    fn test_client_creation() {
        let config = VerifierConfig { api_key: "test-key".to_string(), ..Default::default() };
        let client = AnthropicClient::new(config);
        // Just verify it creates without panicking
        assert!(!client.config.api_key.is_empty());
    }

    // Traces to: FR-UX-VERIFY-001
    #[test]
    fn test_base64_encoding() {
        let png_bytes = b"PNG-data";
        let encoded = base64::engine::general_purpose::STANDARD.encode(png_bytes);
        let expected = "UE5HLWRhdGE=";
        assert_eq!(encoded, expected);
    }
}
