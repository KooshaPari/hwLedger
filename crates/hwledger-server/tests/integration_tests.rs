//! Integration tests for hwledger-server routes, config, and CA.
//! Tests trace to FR-FLEET-001, FR-FLEET-002, FR-FLEET-008
//!
//! This module provides an in-memory test harness that:
//! - Creates a temporary in-memory SQLite pool
//! - Initializes a fresh CA for each test
//! - Builds test requests via tower's test facilities
//! - Exercises all major route handlers

use axum::body::Body;
use axum::extract::DefaultBodyLimit;
use axum::http::{Request, StatusCode};
use axum::routing::{get, post};
use axum::Router;
use chrono::Utc;
use hwledger_fleet_proto::{
    AgentRegistration, DispatchOrder, DispatchReport, Heartbeat, JobState, Platform,
};
use hwledger_server::config::ServerConfig;
use hwledger_server::routes;
use hwledger_server::{ca, AppState};
use std::collections::BTreeMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower::ServiceExt;
use uuid::Uuid;

/// Shared test helpers (for other crate tests to reuse).
mod common {
    use super::*;
    use std::path::PathBuf;
    use tempfile::TempDir;

    /// Initialize a fresh in-memory SQLite pool for testing.
    pub async fn create_test_db() -> Result<sqlx::SqlitePool, Box<dyn std::error::Error>> {
        let pool = sqlx::sqlite::SqlitePool::connect("sqlite::memory:").await?;

        // Run inline migrations (matching db::init)
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS agents (
                id TEXT PRIMARY KEY,
                hostname TEXT NOT NULL,
                platform_json TEXT NOT NULL,
                cert_pem TEXT NOT NULL,
                registered_at_ms INTEGER NOT NULL,
                last_seen_ms INTEGER
            )
            "#,
        )
        .execute(&pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS devices (
                agent_id TEXT NOT NULL,
                device_idx INTEGER NOT NULL,
                backend TEXT NOT NULL,
                name TEXT NOT NULL,
                uuid TEXT,
                total_vram_bytes INTEGER NOT NULL,
                PRIMARY KEY (agent_id, device_idx),
                FOREIGN KEY (agent_id) REFERENCES agents(id)
            )
            "#,
        )
        .execute(&pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS telemetry (
                agent_id TEXT NOT NULL,
                device_idx INTEGER NOT NULL,
                captured_at_ms INTEGER NOT NULL,
                free_vram_bytes INTEGER,
                util_percent REAL,
                temperature_c REAL,
                power_watts REAL,
                PRIMARY KEY (agent_id, device_idx, captured_at_ms),
                FOREIGN KEY (agent_id, device_idx) REFERENCES devices(agent_id, device_idx)
            )
            "#,
        )
        .execute(&pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS jobs (
                id TEXT PRIMARY KEY,
                agent_id TEXT NOT NULL,
                model_ref TEXT NOT NULL,
                state TEXT NOT NULL,
                started_at_ms INTEGER,
                finished_at_ms INTEGER,
                exit_code INTEGER,
                log_tail TEXT,
                created_at_ms INTEGER NOT NULL,
                FOREIGN KEY (agent_id) REFERENCES agents(id)
            )
            "#,
        )
        .execute(&pool)
        .await?;

        Ok(pool)
    }

    /// Create a test CA in a temporary directory.
    pub async fn create_test_ca(dir: &TempDir) -> Result<ca::CertificateAuthority, anyhow::Error> {
        let cert_path = dir.path().join("ca.crt");
        let key_path = dir.path().join("ca.key");
        ca::CertificateAuthority::load_or_create(&cert_path, &key_path).await
    }

    /// Build a test AppState with in-memory DB and CA.
    pub async fn create_test_app_state() -> Result<Arc<AppState>, Box<dyn std::error::Error>> {
        let db = create_test_db().await?;
        let temp_dir = TempDir::new()?;
        let ca = create_test_ca(&temp_dir).await?;

        let config = ServerConfig {
            bind: ([127, 0, 0, 1], 6443).into(),
            db_path: PathBuf::from(":memory:"),
            ca_cert_path: temp_dir.path().join("ca.crt"),
            ca_key_path: temp_dir.path().join("ca.key"),
            bootstrap_tokens: vec!["test-bootstrap-token".to_string()],
            require_admin_cert: false,
        };

        let state = Arc::new(AppState { db, ca, config, rentals_catalog: RwLock::new(None) });

        Ok(state)
    }

    /// Build a test router with all routes.
    pub fn create_test_router(state: Arc<AppState>) -> Router {
        Router::new()
            .route("/v1/agents/register", post(routes::register_agent))
            .route("/v1/agents/:agent_id/heartbeat", post(routes::heartbeat))
            .route("/v1/agents", get(routes::list_agents))
            .route("/v1/jobs", post(routes::create_job))
            .route("/v1/agents/:agent_id/jobs", get(routes::get_pending_jobs))
            .route("/v1/jobs/:job_id/report", post(routes::report_job))
            .route("/v1/fleet/ssh_probe", get(routes::ssh_probe))
            .route("/v1/fleet/tailscale", get(routes::tailscale_peers))
            .route("/v1/fleet/rentals", get(routes::get_rentals))
            .route("/v1/fleet/placement", get(routes::placement_suggestions))
            .route("/v1/health", get(routes::health))
            .layer(DefaultBodyLimit::max(10 * 1024 * 1024))
            .with_state(state)
    }
}

// ============================================================================
// Route Handler Tests
// ============================================================================

#[tokio::test]
async fn test_register_agent_valid() {
    // Traces to: FR-FLEET-001, FR-FLEET-002
    let state = common::create_test_app_state().await.expect("create state");
    let router = common::create_test_router(state.clone());

    let agent_id = Uuid::new_v4();
    let req = AgentRegistration {
        agent_id,
        bootstrap_token: "test-bootstrap-token".to_string(),
        hostname: "test-agent-1".to_string(),
        cert_csr_pem: "-----BEGIN CERTIFICATE REQUEST-----\nMIICpTCCAY0CAQAwDzENMAsGA1UEAwwKdGVzdC1jc3IK\n-----END CERTIFICATE REQUEST-----".to_string(),
        platform: Platform {
            os: "Linux".to_string(),
            arch: "x86_64".to_string(),
            kernel: "5.10.0".to_string(),
            total_ram_bytes: 32 * 1024 * 1024 * 1024,
            cpu_model: "Intel Xeon".to_string(),
        },
    };

    let body = Body::from(serde_json::to_string(&req).unwrap());
    let request = Request::builder()
        .method("POST")
        .uri("/v1/agents/register")
        .header("content-type", "application/json")
        .body(body)
        .unwrap();

    let response = router.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);
}

#[tokio::test]
async fn test_register_agent_invalid_token() {
    // Traces to: FR-FLEET-001, FR-FLEET-002
    let state = common::create_test_app_state().await.expect("create state");
    let router = common::create_test_router(state.clone());

    let agent_id = Uuid::new_v4();
    let req = AgentRegistration {
        agent_id,
        bootstrap_token: "invalid-token".to_string(),
        hostname: "test-agent".to_string(),
        cert_csr_pem: "fake-csr".to_string(),
        platform: Platform {
            os: "Linux".to_string(),
            arch: "x86_64".to_string(),
            kernel: "5.10.0".to_string(),
            total_ram_bytes: 32 * 1024 * 1024 * 1024,
            cpu_model: "Intel Xeon".to_string(),
        },
    };

    let body = Body::from(serde_json::to_string(&req).unwrap());
    let request = Request::builder()
        .method("POST")
        .uri("/v1/agents/register")
        .header("content-type", "application/json")
        .body(body)
        .unwrap();

    let response = router.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_health_check() {
    // Traces to: FR-FLEET-001
    let state = common::create_test_app_state().await.expect("create state");
    let router = common::create_test_router(state.clone());

    let request = Request::builder().method("GET").uri("/v1/health").body(Body::empty()).unwrap();

    let response = router.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(json["status"], "ok");
    assert!(json["agent_count"].is_number());
}

#[tokio::test]
async fn test_list_agents_empty() {
    // Traces to: FR-FLEET-001
    let state = common::create_test_app_state().await.expect("create state");
    let router = common::create_test_router(state.clone());

    let request = Request::builder().method("GET").uri("/v1/agents").body(Body::empty()).unwrap();

    let response = router.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let agents: Vec<serde_json::Value> = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(agents.len(), 0);
}

#[tokio::test]
async fn test_heartbeat_valid() {
    // Traces to: FR-FLEET-001, FR-FLEET-002
    let state = common::create_test_app_state().await.expect("create state");

    // First, register an agent
    let agent_id = Uuid::new_v4();
    let reg_req = AgentRegistration {
        agent_id,
        bootstrap_token: "test-bootstrap-token".to_string(),
        hostname: "test-agent".to_string(),
        cert_csr_pem: "fake-csr".to_string(),
        platform: Platform {
            os: "Linux".to_string(),
            arch: "x86_64".to_string(),
            kernel: "5.10.0".to_string(),
            total_ram_bytes: 32 * 1024 * 1024 * 1024,
            cpu_model: "Intel Xeon".to_string(),
        },
    };

    sqlx::query(
        r#"
        INSERT INTO agents (id, hostname, platform_json, cert_pem, registered_at_ms, last_seen_ms)
        VALUES (?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(agent_id.to_string())
    .bind("test-agent")
    .bind(serde_json::to_string(&reg_req.platform).unwrap())
    .bind("test-cert")
    .bind(Utc::now().timestamp_millis())
    .bind(Utc::now().timestamp_millis())
    .execute(&state.db)
    .await
    .unwrap();

    // Now send heartbeat
    let router = common::create_test_router(state.clone());
    let heartbeat = Heartbeat { agent_id, uptime_s: 1000, devices: vec![] };

    let body = Body::from(serde_json::to_string(&heartbeat).unwrap());
    let request = Request::builder()
        .method("POST")
        .uri(format!("/v1/agents/{}/heartbeat", agent_id))
        .header("content-type", "application/json")
        .body(body)
        .unwrap();

    let response = router.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::ACCEPTED);
}

#[tokio::test]
async fn test_create_job() {
    // Traces to: FR-FLEET-008
    let state = common::create_test_app_state().await.expect("create state");

    // First, create an agent to satisfy the foreign key constraint
    let agent_id = Uuid::new_v4();
    sqlx::query(
        r#"
        INSERT INTO agents (id, hostname, platform_json, cert_pem, registered_at_ms, last_seen_ms)
        VALUES (?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(agent_id.to_string())
    .bind("test-agent")
    .bind("{\"os\":\"Linux\"}")
    .bind("test-cert")
    .bind(Utc::now().timestamp_millis())
    .bind(Utc::now().timestamp_millis())
    .execute(&state.db)
    .await
    .unwrap();

    let router = common::create_test_router(state.clone());

    let job_id = Uuid::new_v4();

    let req = DispatchOrder {
        job_id,
        agent_id,
        model_ref: "llama-7b".to_string(),
        backend_hint: None,
        command: vec!["llama.cpp".to_string()],
        env: BTreeMap::new(),
        deadline_ms: None,
    };

    let body = Body::from(serde_json::to_string(&req).unwrap());
    let request = Request::builder()
        .method("POST")
        .uri("/v1/jobs")
        .header("content-type", "application/json")
        .body(body)
        .unwrap();

    let response = router.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(json["job_id"], job_id.to_string());
}

#[tokio::test]
async fn test_report_job() {
    // Traces to: FR-FLEET-008
    let state = common::create_test_app_state().await.expect("create state");

    let job_id = Uuid::new_v4();
    let agent_id = Uuid::new_v4();

    // Create agent first (required for foreign key constraint)
    sqlx::query(
        r#"
        INSERT INTO agents (id, hostname, platform_json, cert_pem, registered_at_ms, last_seen_ms)
        VALUES (?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(agent_id.to_string())
    .bind("test-agent")
    .bind("{\"os\":\"Linux\"}")
    .bind("test-cert")
    .bind(Utc::now().timestamp_millis())
    .bind(Utc::now().timestamp_millis())
    .execute(&state.db)
    .await
    .unwrap();

    // Create job
    sqlx::query(
        r#"
        INSERT INTO jobs (id, agent_id, model_ref, state, created_at_ms)
        VALUES (?, ?, ?, ?, ?)
        "#,
    )
    .bind(job_id.to_string())
    .bind(agent_id.to_string())
    .bind("llama-7b")
    .bind("Pending")
    .bind(Utc::now().timestamp_millis())
    .execute(&state.db)
    .await
    .unwrap();

    let router = common::create_test_router(state.clone());
    let report = DispatchReport {
        job_id,
        agent_id,
        state: JobState::Running,
        started_at_ms: Some(1000),
        finished_at_ms: None,
        exit_code: None,
        log_tail: "starting...".to_string(),
    };

    let body = Body::from(serde_json::to_string(&report).unwrap());
    let request = Request::builder()
        .method("POST")
        .uri(format!("/v1/jobs/{}/report", job_id))
        .header("content-type", "application/json")
        .body(body)
        .unwrap();

    let response = router.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn test_get_pending_jobs_empty() {
    // Traces to: FR-FLEET-008
    let state = common::create_test_app_state().await.expect("create state");
    let router = common::create_test_router(state.clone());

    let agent_id = Uuid::new_v4();
    let request = Request::builder()
        .method("GET")
        .uri(format!("/v1/agents/{}/jobs", agent_id))
        .body(Body::empty())
        .unwrap();

    let response = router.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let jobs: Vec<serde_json::Value> = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(jobs.len(), 0);
}

#[tokio::test]
async fn test_ssh_probe_missing_host() {
    // Traces to: FR-FLEET-003
    let state = common::create_test_app_state().await.expect("create state");
    let router = common::create_test_router(state.clone());

    let request =
        Request::builder().method("GET").uri("/v1/fleet/ssh_probe").body(Body::empty()).unwrap();

    let response = router.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_placement_suggestions_missing_params() {
    // Traces to: FR-FLEET-007
    let state = common::create_test_app_state().await.expect("create state");
    let router = common::create_test_router(state.clone());

    let request =
        Request::builder().method("GET").uri("/v1/fleet/placement").body(Body::empty()).unwrap();

    let response = router.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_placement_suggestions_valid() {
    // Traces to: FR-FLEET-007
    let state = common::create_test_app_state().await.expect("create state");
    let router = common::create_test_router(state.clone());

    let request = Request::builder()
        .method("GET")
        .uri("/v1/fleet/placement?model_ref=llama-7b&min_vram_gb=8")
        .body(Body::empty())
        .unwrap();

    let response = router.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let suggestions: Vec<serde_json::Value> = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(suggestions.len(), 0); // MVP returns empty
}

// ============================================================================
// Error Response Tests
// ============================================================================

#[tokio::test]
async fn test_heartbeat_id_mismatch() {
    // Traces to: FR-FLEET-001, FR-FLEET-002
    let state = common::create_test_app_state().await.expect("create state");
    let router = common::create_test_router(state.clone());

    let agent_id = Uuid::new_v4();
    let wrong_id = Uuid::new_v4();

    let heartbeat = Heartbeat { agent_id: wrong_id, uptime_s: 1000, devices: vec![] };

    let body = Body::from(serde_json::to_string(&heartbeat).unwrap());
    let request = Request::builder()
        .method("POST")
        .uri(format!("/v1/agents/{}/heartbeat", agent_id))
        .header("content-type", "application/json")
        .body(body)
        .unwrap();

    let response = router.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_report_job_id_mismatch() {
    // Traces to: FR-FLEET-008
    let state = common::create_test_app_state().await.expect("create state");
    let router = common::create_test_router(state.clone());

    let job_id = Uuid::new_v4();
    let wrong_id = Uuid::new_v4();

    let report = DispatchReport {
        job_id: wrong_id,
        agent_id: Uuid::new_v4(),
        state: JobState::Running,
        started_at_ms: Some(1000),
        finished_at_ms: None,
        exit_code: None,
        log_tail: "test".to_string(),
    };

    let body = Body::from(serde_json::to_string(&report).unwrap());
    let request = Request::builder()
        .method("POST")
        .uri(format!("/v1/jobs/{}/report", job_id))
        .header("content-type", "application/json")
        .body(body)
        .unwrap();

    let response = router.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

// ============================================================================
// Config Tests
// ============================================================================

#[test]
fn test_server_config_default() {
    // Traces to: FR-FLEET-001
    let config = ServerConfig::default();
    assert_eq!(config.bind.to_string(), "127.0.0.1:6443");
    assert_eq!(config.db_path.to_string_lossy(), "./ledger.db");
    assert_eq!(config.bootstrap_tokens.len(), 1);
    assert_eq!(config.bootstrap_tokens[0], "dev-bootstrap-token");
}

#[test]
fn test_server_config_custom() {
    // Traces to: FR-FLEET-001
    use std::path::PathBuf;

    let config = ServerConfig {
        bind: ([192, 168, 1, 1], 8080).into(),
        db_path: PathBuf::from("/tmp/custom.db"),
        ca_cert_path: PathBuf::from("/etc/ca.crt"),
        ca_key_path: PathBuf::from("/etc/ca.key"),
        bootstrap_tokens: vec!["custom-token-1".to_string(), "custom-token-2".to_string()],
        require_admin_cert: true,
    };

    assert_eq!(config.bind.port(), 8080);
    assert_eq!(config.db_path.to_string_lossy(), "/tmp/custom.db");
    assert_eq!(config.bootstrap_tokens.len(), 2);
}

// ============================================================================
// Certificate Authority Tests
// ============================================================================

#[tokio::test]
async fn test_ca_generation_and_csr_signing() {
    // Traces to: FR-FLEET-001, FR-FLEET-002
    let temp_dir = tempfile::TempDir::new().expect("create temp dir");
    let ca = common::create_test_ca(&temp_dir).await.expect("create CA");

    // CA cert should be valid PEM
    assert!(ca.ca_cert_pem.contains("BEGIN CERTIFICATE"));

    // Sign a CSR
    let signed_cert = ca.sign_csr("fake-csr-pem", "test-agent").expect("sign CSR");
    assert!(signed_cert.contains("BEGIN CERTIFICATE"));
}

#[tokio::test]
async fn test_ca_persistence() {
    // Traces to: FR-FLEET-001
    let temp_dir = tempfile::TempDir::new().expect("create temp dir");

    // Create CA
    let ca1 = common::create_test_ca(&temp_dir).await.expect("create CA 1");
    let pem1 = ca1.ca_cert_pem.clone();

    // Load CA again (should reload from disk)
    let ca2 = common::create_test_ca(&temp_dir).await.expect("create CA 2");
    let pem2 = ca2.ca_cert_pem.clone();

    assert_eq!(pem1, pem2);
}

// ============================================================================
// Job State Serialization Tests
// ============================================================================

#[test]
fn test_job_state_all_variants() {
    // Traces to: FR-FLEET-008
    let states = vec![
        JobState::Pending,
        JobState::Running,
        JobState::Succeeded,
        JobState::Failed,
        JobState::TimedOut,
    ];

    for state in states {
        let json = serde_json::to_string(&state).expect("serialize");
        let deserialized: JobState = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(state, deserialized);
    }
}
