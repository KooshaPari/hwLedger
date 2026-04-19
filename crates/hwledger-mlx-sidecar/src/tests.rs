// Integration tests using a fake Python RPC server.
// Traces to: FR-INF-001, FR-INF-002, FR-INF-004, FR-INF-005

#![cfg(test)]

use crate::{MlxSidecar, MlxSidecarConfig};
use std::path::PathBuf;

#[tokio::test]
#[ignore] // Requires fake_sidecar.py to be in scope; can be run manually
async fn test_sidecar_health() {
    // Create a config pointing to our fake server
    let config = MlxSidecarConfig {
        python: PathBuf::from("python3"),
        venv: None,
        omlx_module: "tests.fake_sidecar".to_string(),
        cwd: None,
        env: vec![],
    };

    // Spawn the sidecar
    match MlxSidecar::spawn(config).await {
        Ok(sidecar) => {
            // Wait a moment for startup
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;

            // Test health
            match sidecar.health().await {
                Ok(health) => {
                    assert_eq!(health.status, "ok");
                    println!("Health: {:?}", health);
                }
                Err(e) => panic!("Health check failed: {}", e),
            }

            // Shutdown
            let _ = sidecar.shutdown().await;
        }
        Err(e) => panic!("Failed to spawn sidecar: {}", e),
    }
}

#[tokio::test]
#[ignore]
async fn test_sidecar_load_model() {
    let config = MlxSidecarConfig {
        python: PathBuf::from("python3"),
        venv: None,
        omlx_module: "tests.fake_sidecar".to_string(),
        cwd: None,
        env: vec![],
    };

    if let Ok(sidecar) = MlxSidecar::spawn(config).await {
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;

        match sidecar.load_model("llama-3b".to_string(), 8192).await {
            Ok(result) => {
                assert!(result.loaded);
                assert_eq!(result.model, "llama-3b");
                assert!(result.context_length > 0);
            }
            Err(e) => panic!("Load failed: {}", e),
        }

        let _ = sidecar.shutdown().await;
    }
}

#[tokio::test]
#[ignore]
async fn test_sidecar_generate_tokens() {
    let config = MlxSidecarConfig {
        python: PathBuf::from("python3"),
        venv: None,
        omlx_module: "tests.fake_sidecar".to_string(),
        cwd: None,
        env: vec![],
    };

    if let Ok(sidecar) = MlxSidecar::spawn(config).await {
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;

        match sidecar.generate("Hello world".to_string(), "llama-3b".to_string(), 5, 0.7).await {
            Ok(mut stream) => {
                let mut token_count = 0;
                while let Some(result) = stream.next_token().await {
                    match result {
                        Ok(token) => {
                            token_count += 1;
                            println!("Token {}: {}", token_count, token);
                        }
                        Err(e) => panic!("Token error: {}", e),
                    }
                }
                assert!(token_count > 0);
            }
            Err(e) => panic!("Generate failed: {}", e),
        }

        let _ = sidecar.shutdown().await;
    }
}

#[tokio::test]
#[ignore]
async fn test_sidecar_memory_report() {
    let config = MlxSidecarConfig {
        python: PathBuf::from("python3"),
        venv: None,
        omlx_module: "tests.fake_sidecar".to_string(),
        cwd: None,
        env: vec![],
    };

    if let Ok(sidecar) = MlxSidecar::spawn(config).await {
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;

        match sidecar.memory_report().await {
            Ok(report) => {
                assert!(report.total_unified_mb > 0.0);
                println!("Memory: {:?}", report);
            }
            Err(e) => panic!("Memory report failed: {}", e),
        }

        let _ = sidecar.shutdown().await;
    }
}

#[tokio::test]
#[ignore]
async fn test_sidecar_cancel() {
    let config = MlxSidecarConfig {
        python: PathBuf::from("python3"),
        venv: None,
        omlx_module: "tests.fake_sidecar".to_string(),
        cwd: None,
        env: vec![],
    };

    if let Ok(sidecar) = MlxSidecar::spawn(config).await {
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;

        match sidecar.generate("Hello world".to_string(), "llama-3b".to_string(), 100, 0.7).await {
            Ok(stream) => {
                let request_id = stream.request_id.clone();
                let _ = sidecar.cancel(request_id).await;
                let _ = stream.cancel().await;
            }
            Err(e) => panic!("Generate failed: {}", e),
        }

        let _ = sidecar.shutdown().await;
    }
}
