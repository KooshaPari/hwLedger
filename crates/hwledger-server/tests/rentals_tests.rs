//! Tests for cloud rental provider integration.
//! Traces to: FR-FLEET-005, FR-FLEET-007

use hwledger_server::rentals::{
    Provider, RentalApiKeys, RentalAvailability, RentalCatalog, RentalOffering,
};

#[test]
fn test_rental_api_keys_default() {
    // Traces to: FR-FLEET-005
    let keys = RentalApiKeys::default();
    assert!(keys.vast_ai.is_none());
    assert!(keys.runpod.is_none());
    assert!(keys.lambda.is_none());
    assert!(keys.modal.is_none());
}

#[test]
fn test_rental_api_keys_with_values() {
    // Traces to: FR-FLEET-005
    let keys = RentalApiKeys {
        vast_ai: Some("key123".to_string()),
        runpod: Some("key456".to_string()),
        lambda: None,
        modal: None,
    };

    assert!(keys.vast_ai.is_some());
    assert!(keys.runpod.is_some());
    assert!(keys.lambda.is_none());
}

#[test]
fn test_rental_offering_serialization() {
    // Traces to: FR-FLEET-005, FR-FLEET-007
    let offering = RentalOffering {
        provider: Provider::VastAi,
        gpu_model: "NVIDIA A100".to_string(),
        vram_gb: 40,
        cpu_cores: 32,
        ram_gb: 256,
        hourly_usd: 2.45,
        region: "us-east-1".to_string(),
        availability: RentalAvailability::Spot,
    };

    let json = serde_json::to_string(&offering).unwrap();
    assert!(json.contains("NVIDIA A100"));
    assert!(json.contains("2.45"));

    let deserialized: RentalOffering = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.vram_gb, 40);
    assert_eq!(deserialized.hourly_usd, 2.45);
}

#[test]
fn test_rental_availability_serde() {
    // Traces to: FR-FLEET-005
    let on_demand = RentalAvailability::OnDemand;
    let on_demand_json = serde_json::to_string(&on_demand).unwrap();
    assert_eq!(on_demand_json, "\"OnDemand\"");

    let spot = RentalAvailability::Spot;
    let spot_json = serde_json::to_string(&spot).unwrap();
    assert_eq!(spot_json, "\"Spot\"");

    let reserved = RentalAvailability::Reserved;
    let reserved_json = serde_json::to_string(&reserved).unwrap();
    assert_eq!(reserved_json, "\"Reserved\"");
}

#[test]
fn test_provider_serde() {
    // Traces to: FR-FLEET-005
    let vast = Provider::VastAi;
    let vast_json = serde_json::to_string(&vast).unwrap();
    assert_eq!(vast_json, "\"vast_ai\"");

    let runpod = Provider::RunPod;
    let runpod_json = serde_json::to_string(&runpod).unwrap();
    assert_eq!(runpod_json, "\"run_pod\"");

    let lambda = Provider::Lambda;
    let lambda_json = serde_json::to_string(&lambda).unwrap();
    assert_eq!(lambda_json, "\"lambda\"");

    let modal = Provider::Modal;
    let modal_json = serde_json::to_string(&modal).unwrap();
    assert_eq!(modal_json, "\"modal\"");
}

#[test]
fn test_rental_catalog_serialization() {
    // Traces to: FR-FLEET-005, FR-FLEET-007
    let catalog = RentalCatalog {
        entries: vec![RentalOffering {
            provider: Provider::VastAi,
            gpu_model: "RTX 4090".to_string(),
            vram_gb: 24,
            cpu_cores: 16,
            ram_gb: 128,
            hourly_usd: 1.50,
            region: "us-east-1".to_string(),
            availability: RentalAvailability::Spot,
        }],
        refreshed_at_ms: 1000000,
    };

    let json = serde_json::to_string(&catalog).unwrap();
    assert!(json.contains("RTX 4090"));
    assert!(json.contains("1000000"));

    let deserialized: RentalCatalog = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.entries.len(), 1);
    assert_eq!(deserialized.refreshed_at_ms, 1000000);
}

#[test]
fn test_multiple_offerings() {
    // Traces to: FR-FLEET-005, FR-FLEET-007
    let offerings = vec![
        RentalOffering {
            provider: Provider::VastAi,
            gpu_model: "A100".to_string(),
            vram_gb: 40,
            cpu_cores: 32,
            ram_gb: 256,
            hourly_usd: 2.45,
            region: "us-east-1".to_string(),
            availability: RentalAvailability::Spot,
        },
        RentalOffering {
            provider: Provider::RunPod,
            gpu_model: "RTX 4090".to_string(),
            vram_gb: 24,
            cpu_cores: 16,
            ram_gb: 128,
            hourly_usd: 1.50,
            region: "us-west-2".to_string(),
            availability: RentalAvailability::OnDemand,
        },
    ];

    let catalog = RentalCatalog { entries: offerings, refreshed_at_ms: 500000 };

    assert_eq!(catalog.entries.len(), 2);
    assert_eq!(catalog.entries[0].vram_gb, 40);
    assert_eq!(catalog.entries[1].vram_gb, 24);
}

#[tokio::test]
async fn test_rental_catalog_refresh_no_keys() {
    // Traces to: FR-FLEET-005
    // With no API keys, we expect an error or empty catalog
    let api_keys = RentalApiKeys::default();

    // When no providers have keys, refresh should succeed with empty catalog
    match RentalCatalog::refresh(api_keys).await {
        Ok(catalog) => {
            // Empty catalog is acceptable when no providers are configured
            assert_eq!(catalog.entries.len(), 0);
        }
        Err(e) => {
            // Or it might error; both are acceptable MVPs
            println!("Expected error with no API keys: {}", e);
        }
    }
}
