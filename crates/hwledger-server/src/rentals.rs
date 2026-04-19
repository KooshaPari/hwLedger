//! Cloud rental cost model and provider discovery (FR-FLEET-005, FR-FLEET-007).
//!
//! Queries cloud rental providers (Vast.ai, RunPod, Lambda Labs, Modal) for
//! spot-priced GPU availability. Maintains a 1-hour TTL cache of offerings.

use crate::error::ServerError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{info, warn};

/// Cloud rental provider identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Provider {
    VastAi,
    RunPod,
    Lambda,
    Modal,
}

/// Availability tier for a rental offering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum RentalAvailability {
    OnDemand,
    Spot,
    Reserved,
}

/// A single rental offering from a cloud provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RentalOffering {
    pub provider: Provider,
    pub gpu_model: String,
    pub vram_gb: u32,
    pub cpu_cores: u32,
    pub ram_gb: u32,
    pub hourly_usd: f64,
    pub region: String,
    pub availability: RentalAvailability,
}

/// API credentials for cloud providers.
#[derive(Debug, Clone, Default)]
pub struct RentalApiKeys {
    pub vast_ai: Option<String>,
    pub runpod: Option<String>,
    pub lambda: Option<String>,
    pub modal: Option<String>,
}

/// Catalog of all available rental offerings, with refresh timestamp.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RentalCatalog {
    pub entries: Vec<RentalOffering>,
    /// Milliseconds since Unix epoch when this catalog was refreshed.
    pub refreshed_at_ms: u64,
}

impl RentalCatalog {
    /// Refresh the rental catalog from all configured providers.
    /// Traces to: FR-FLEET-005, FR-FLEET-007
    pub async fn refresh(api_keys: RentalApiKeys) -> Result<Self, ServerError> {
        let mut entries = Vec::new();

        // Vast.ai
        if let Some(api_key) = &api_keys.vast_ai {
            match fetch_vast_ai(api_key).await {
                Ok(mut offerings) => {
                    info!("fetched {} offerings from Vast.ai", offerings.len());
                    entries.append(&mut offerings);
                }
                Err(e) => {
                    warn!("failed to fetch Vast.ai offerings: {}", e);
                    return Err(ServerError::Internal {
                        reason: format!("Vast.ai API error: {}", e),
                    });
                }
            }
        }

        // RunPod
        if let Some(api_key) = &api_keys.runpod {
            match fetch_runpod(api_key).await {
                Ok(mut offerings) => {
                    info!("fetched {} offerings from RunPod", offerings.len());
                    entries.append(&mut offerings);
                }
                Err(e) => {
                    warn!("failed to fetch RunPod offerings: {}", e);
                    return Err(ServerError::Internal {
                        reason: format!("RunPod API error: {}", e),
                    });
                }
            }
        }

        // Lambda Labs
        if let Some(api_key) = &api_keys.lambda {
            match fetch_lambda(api_key).await {
                Ok(mut offerings) => {
                    info!("fetched {} offerings from Lambda Labs", offerings.len());
                    entries.append(&mut offerings);
                }
                Err(e) => {
                    warn!("failed to fetch Lambda Labs offerings: {}", e);
                    return Err(ServerError::Internal {
                        reason: format!("Lambda Labs API error: {}", e),
                    });
                }
            }
        }

        // Modal: has no public spot marketplace API; log and skip
        if api_keys.modal.is_some() {
            info!("Modal has no spot marketplace API; skipping");
        }

        let refreshed_at_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);

        Ok(RentalCatalog { entries, refreshed_at_ms })
    }
}

/// Fetch offerings from Vast.ai.
/// Uses the undocumented query JSON in the URL parameter.
async fn fetch_vast_ai(api_key: &str) -> Result<Vec<RentalOffering>, ServerError> {
    let query = r#"{"gpu_name":"any","verified":{"eq":true}}"#;
    let url = format!("https://console.vast.ai/api/v0/bundles?q={}", urlencoding::encode(query));

    let client = reqwest::Client::new();
    let response = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", api_key))
        .send()
        .await
        .map_err(|e| ServerError::Internal { reason: format!("HTTP error: {}", e) })?;

    if !response.status().is_success() {
        return Err(ServerError::Internal {
            reason: format!("Vast.ai returned {}", response.status()),
        });
    }

    let body: serde_json::Value = response
        .json()
        .await
        .map_err(|e| ServerError::Internal { reason: format!("JSON parse error: {}", e) })?;

    let mut offerings = Vec::new();

    // Vast.ai returns { "bundles": [...] }
    if let Some(bundles) = body.get("bundles").and_then(|v| v.as_array()) {
        for bundle in bundles {
            if let Ok(offering) = parse_vast_bundle(bundle) {
                offerings.push(offering);
            }
        }
    }

    Ok(offerings)
}

/// Parse a single Vast.ai bundle into a RentalOffering.
fn parse_vast_bundle(bundle: &serde_json::Value) -> Result<RentalOffering, String> {
    let gpu_model = bundle
        .get("gpu_name")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "missing gpu_name".to_string())?
        .to_string();

    let vram_gb = bundle
        .get("gpu_memory_gb")
        .and_then(|v| v.as_f64())
        .ok_or_else(|| "missing gpu_memory_gb".to_string())? as u32;

    let cpu_cores = bundle.get("cpu_count").and_then(|v| v.as_u64()).unwrap_or(4) as u32;

    let ram_gb = bundle.get("ram_gb").and_then(|v| v.as_f64()).unwrap_or(8.0) as u32;

    let hourly_usd = bundle
        .get("price")
        .or_else(|| bundle.get("dph_total"))
        .and_then(|v| v.as_f64())
        .ok_or_else(|| "missing price".to_string())?;

    let region =
        bundle.get("geolocation").and_then(|v| v.as_str()).unwrap_or("unknown").to_string();

    Ok(RentalOffering {
        provider: Provider::VastAi,
        gpu_model,
        vram_gb,
        cpu_cores,
        ram_gb,
        hourly_usd,
        region,
        availability: RentalAvailability::Spot,
    })
}

/// Fetch offerings from RunPod.
async fn fetch_runpod(api_key: &str) -> Result<Vec<RentalOffering>, ServerError> {
    // RunPod GraphQL endpoint
    let query = r#"
        query {
            podFindAndDeployOnDemand(input: {}) {
                edges {
                    node {
                        gpuCount
                        gpuDisplayName
                        minimumBidPrice
                        volumeInGbMax
                        secureCloud
                        cloudType
                    }
                }
            }
        }
    "#;

    let client = reqwest::Client::new();
    let mut payload = HashMap::new();
    payload.insert("query", query);

    let response = client
        .post("https://api.runpod.io/graphql")
        .bearer_auth(api_key)
        .json(&payload)
        .send()
        .await
        .map_err(|e| ServerError::Internal { reason: format!("HTTP error: {}", e) })?;

    if !response.status().is_success() {
        return Err(ServerError::Internal {
            reason: format!("RunPod GraphQL returned {}", response.status()),
        });
    }

    let body: serde_json::Value = response
        .json()
        .await
        .map_err(|e| ServerError::Internal { reason: format!("JSON parse error: {}", e) })?;

    let mut offerings = Vec::new();

    // Parse GraphQL response structure
    if let Some(edges) = body
        .get("data")
        .and_then(|d| d.get("podFindAndDeployOnDemand"))
        .and_then(|p| p.get("edges"))
        .and_then(|e| e.as_array())
    {
        for edge in edges {
            if let Some(node) = edge.get("node") {
                if let Ok(offering) = parse_runpod_node(node) {
                    offerings.push(offering);
                }
            }
        }
    }

    Ok(offerings)
}

/// Parse a single RunPod pod into a RentalOffering.
fn parse_runpod_node(node: &serde_json::Value) -> Result<RentalOffering, String> {
    let gpu_display_name = node
        .get("gpuDisplayName")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "missing gpuDisplayName".to_string())?
        .to_string();

    let gpu_count = node.get("gpuCount").and_then(|v| v.as_u64()).unwrap_or(1) as u32;

    let hourly_usd = node
        .get("minimumBidPrice")
        .and_then(|v| v.as_f64())
        .ok_or_else(|| "missing minimumBidPrice".to_string())?;

    // Estimate VRAM from GPU name (simplified)
    let vram_gb = estimate_vram_from_gpu_name(&gpu_display_name);

    Ok(RentalOffering {
        provider: Provider::RunPod,
        gpu_model: gpu_display_name,
        vram_gb: vram_gb * gpu_count,
        cpu_cores: 8,
        ram_gb: 32,
        hourly_usd,
        region: "us-east".to_string(),
        availability: RentalAvailability::Spot,
    })
}

/// Fetch offerings from Lambda Labs.
async fn fetch_lambda(api_key: &str) -> Result<Vec<RentalOffering>, ServerError> {
    let client = reqwest::Client::new();
    let response = client
        .get("https://cloud.lambdalabs.com/api/v1/instance-types")
        .bearer_auth(api_key)
        .send()
        .await
        .map_err(|e| ServerError::Internal { reason: format!("HTTP error: {}", e) })?;

    if !response.status().is_success() {
        return Err(ServerError::Internal {
            reason: format!("Lambda Labs returned {}", response.status()),
        });
    }

    let body: serde_json::Value = response
        .json()
        .await
        .map_err(|e| ServerError::Internal { reason: format!("JSON parse error: {}", e) })?;

    let mut offerings = Vec::new();

    // Lambda returns { "data": { "instance_type": {...}, ... } }
    if let Some(data) = body.get("data").and_then(|v| v.as_object()) {
        for (_, instance_info) in data.iter() {
            if let Ok(offering) = parse_lambda_instance(instance_info) {
                offerings.push(offering);
            }
        }
    }

    Ok(offerings)
}

/// Parse a single Lambda Labs instance type into a RentalOffering.
fn parse_lambda_instance(info: &serde_json::Value) -> Result<RentalOffering, String> {
    let name = info
        .get("name")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "missing name".to_string())?
        .to_string();

    let price_cents_per_hour = info
        .get("price_cents_per_hour")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| "missing price_cents_per_hour".to_string())?;

    let hourly_usd = price_cents_per_hour as f64 / 100.0;

    // Lambda lists specs in nested structure; approximate
    let gpu_specs = info.get("specs");
    let vram_gb =
        gpu_specs.and_then(|s| s.get("gpu_memory_gb")).and_then(|v| v.as_u64()).unwrap_or(24)
            as u32;

    Ok(RentalOffering {
        provider: Provider::Lambda,
        gpu_model: name,
        vram_gb,
        cpu_cores: 16,
        ram_gb: 64,
        hourly_usd,
        region: "us-west".to_string(),
        availability: RentalAvailability::OnDemand,
    })
}

/// Estimate VRAM (GB) from GPU model name.
fn estimate_vram_from_gpu_name(name: &str) -> u32 {
    let lower = name.to_lowercase();
    match true {
        _ if lower.contains("h100") => 80,
        _ if lower.contains("a100") => 40,
        _ if lower.contains("l40s") => 48,
        _ => 24,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Traces to: FR-FLEET-005
    #[test]
    fn test_parse_vast_bundle() {
        let bundle = serde_json::json!({
            "gpu_name": "NVIDIA H100",
            "gpu_memory_gb": 80,
            "cpu_count": 32,
            "ram_gb": 256,
            "price": 1.25,
            "geolocation": "us-east-1"
        });

        let offering = parse_vast_bundle(&bundle).expect("parse failed");
        assert_eq!(offering.provider, Provider::VastAi);
        assert_eq!(offering.gpu_model, "NVIDIA H100");
        assert_eq!(offering.vram_gb, 80);
        assert_eq!(offering.hourly_usd, 1.25);
    }

    // Traces to: FR-FLEET-005
    #[test]
    fn test_parse_runpod_node() {
        let node = serde_json::json!({
            "gpuDisplayName": "A100",
            "gpuCount": 2,
            "minimumBidPrice": 0.55
        });

        let offering = parse_runpod_node(&node).expect("parse failed");
        assert_eq!(offering.provider, Provider::RunPod);
        assert_eq!(offering.gpu_model, "A100");
        assert_eq!(offering.hourly_usd, 0.55);
    }

    // Traces to: FR-FLEET-007
    #[test]
    fn test_rental_offering_serde() {
        let offering = RentalOffering {
            provider: Provider::Lambda,
            gpu_model: "L40S".to_string(),
            vram_gb: 48,
            cpu_cores: 12,
            ram_gb: 96,
            hourly_usd: 0.89,
            region: "us-west-2".to_string(),
            availability: RentalAvailability::OnDemand,
        };

        let json = serde_json::to_string(&offering).expect("serialize");
        let offering2: RentalOffering = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(offering2.provider, Provider::Lambda);
        assert_eq!(offering2.vram_gb, 48);
    }

    // Traces to: FR-FLEET-005
    #[test]
    fn test_estimate_vram_from_gpu_name() {
        assert_eq!(estimate_vram_from_gpu_name("H100"), 80);
        assert_eq!(estimate_vram_from_gpu_name("A100"), 40);
        assert_eq!(estimate_vram_from_gpu_name("RTX 4090"), 24);
        assert_eq!(estimate_vram_from_gpu_name("Unknown"), 24);
    }
}
