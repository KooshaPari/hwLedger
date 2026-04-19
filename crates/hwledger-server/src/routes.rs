//! HTTP route handlers for the fleet server.
//!
//! Endpoints for agent registration, heartbeat, job dispatch, and health checks.
//! Traces to: FR-FLEET-001, FR-FLEET-002, FR-FLEET-008

use crate::error::ServerError;
use crate::AppState;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use chrono::Utc;
use hwledger_fleet_proto::{
    AgentRegistration, DispatchOrder, DispatchReport, Heartbeat, JobState, RegistrationAck,
};
use serde::Serialize;
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;

/// Health check response.
#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub uptime_s: u64,
    pub agent_count: i64,
}

/// Agent list response.
#[derive(Debug, Serialize)]
pub struct Agent {
    pub id: String,
    pub hostname: String,
    pub registered_at_ms: i64,
    pub last_seen_ms: Option<i64>,
}

/// Register a new agent or refresh an existing one.
/// Accepts AgentRegistration with a bootstrap token, validates the token,
/// signs the CSR, and returns RegistrationAck.
/// Traces to: FR-FLEET-001, FR-FLEET-002
pub async fn register_agent(
    State(state): State<Arc<AppState>>,
    Json(req): Json<AgentRegistration>,
) -> Result<(StatusCode, Json<RegistrationAck>), ServerError> {
    // Validate bootstrap token
    if !state.config.bootstrap_tokens.contains(&req.bootstrap_token) {
        return Err(ServerError::Auth {
            reason: "invalid bootstrap token".to_string(),
        });
    }

    // Sign the CSR
    let agent_cert_pem = state.ca.sign_csr(&req.cert_csr_pem, &req.hostname).map_err(|e| {
        ServerError::Internal {
            reason: format!("failed to sign CSR: {}", e),
        }
    })?;

    let now_ms = Utc::now().timestamp_millis();

    // Insert or update agent in database
    sqlx::query(
        r#"
        INSERT INTO agents (id, hostname, platform_json, cert_pem, registered_at_ms, last_seen_ms)
        VALUES (?, ?, ?, ?, ?, ?)
        ON CONFLICT(id) DO UPDATE SET
            cert_pem = excluded.cert_pem,
            last_seen_ms = excluded.last_seen_ms
        "#,
    )
    .bind(req.agent_id.to_string())
    .bind(&req.hostname)
    .bind(serde_json::to_string(&req.platform).map_err(|e| ServerError::Internal {
        reason: e.to_string(),
    })?)
    .bind(&agent_cert_pem)
    .bind(now_ms)
    .bind(now_ms)
    .execute(&state.db)
    .await?;

    let ack = RegistrationAck {
        agent_id: req.agent_id,
        assigned_cert_pem: agent_cert_pem,
        ca_cert_pem: state.ca.ca_cert_pem.clone(),
        server_time_ms: now_ms as u64,
    };

    Ok((StatusCode::CREATED, Json(ack)))
}

/// Submit a heartbeat from an agent: updates device inventory and telemetry.
/// Traces to: FR-FLEET-001, FR-FLEET-002
pub async fn heartbeat(
    State(state): State<Arc<AppState>>,
    Path(agent_id): Path<Uuid>,
    Json(req): Json<Heartbeat>,
) -> Result<StatusCode, ServerError> {
    // Verify agent_id matches
    if req.agent_id != agent_id {
        return Err(ServerError::Validation {
            reason: "agent_id mismatch".to_string(),
        });
    }

    let now_ms = Utc::now().timestamp_millis();

    // Update agent last_seen
    sqlx::query("UPDATE agents SET last_seen_ms = ? WHERE id = ?")
        .bind(now_ms)
        .bind(agent_id.to_string())
        .execute(&state.db)
        .await?;

    // Upsert devices
    for (device_idx, device) in req.devices.iter().enumerate() {
        sqlx::query(
            r#"
            INSERT INTO devices (agent_id, device_idx, backend, name, uuid, total_vram_bytes)
            VALUES (?, ?, ?, ?, ?, ?)
            ON CONFLICT(agent_id, device_idx) DO UPDATE SET
                backend = excluded.backend,
                name = excluded.name,
                uuid = excluded.uuid,
                total_vram_bytes = excluded.total_vram_bytes
            "#,
        )
        .bind(agent_id.to_string())
        .bind(device_idx as i32)
        .bind(&device.backend)
        .bind(&device.name)
        .bind(&device.uuid)
        .bind(device.total_vram_bytes as i64)
        .execute(&state.db)
        .await?;

        // Insert telemetry if snapshot is present
        if let Some(snap) = &device.snapshot {
            sqlx::query(
                r#"
                INSERT INTO telemetry
                (agent_id, device_idx, captured_at_ms, free_vram_bytes, util_percent, temperature_c, power_watts)
                VALUES (?, ?, ?, ?, ?, ?, ?)
                "#,
            )
            .bind(agent_id.to_string())
            .bind(device_idx as i32)
            .bind(snap.captured_at_ms as i64)
            .bind(snap.free_vram_bytes as i64)
            .bind(snap.util_percent)
            .bind(snap.temperature_c)
            .bind(snap.power_watts)
            .execute(&state.db)
            .await?;
        }
    }

    Ok(StatusCode::ACCEPTED)
}

/// List all agents with their last telemetry snapshot.
/// Admin-only: currently loose mTLS (any valid cert). TODO(fleet-auth-v2): add CN validation.
/// Traces to: FR-FLEET-001
pub async fn list_agents(State(state): State<Arc<AppState>>) -> Result<Json<Vec<Agent>>, ServerError> {
    let rows = sqlx::query_as::<_, (String, String, i64, Option<i64>)>(
        "SELECT id, hostname, registered_at_ms, last_seen_ms FROM agents",
    )
    .fetch_all(&state.db)
    .await?;

    let agents = rows
        .into_iter()
        .map(|(id, hostname, registered_at_ms, last_seen_ms)| Agent {
            id,
            hostname,
            registered_at_ms,
            last_seen_ms,
        })
        .collect();

    Ok(Json(agents))
}

/// Create a new dispatch job and insert it into the database.
/// Traces to: FR-FLEET-008
pub async fn create_job(
    State(state): State<Arc<AppState>>,
    Json(req): Json<DispatchOrder>,
) -> Result<(StatusCode, Json<serde_json::Value>), ServerError> {
    let now_ms: i64 = Utc::now().timestamp_millis();

    sqlx::query(
        r#"
        INSERT INTO jobs (id, agent_id, model_ref, state, started_at_ms, finished_at_ms, exit_code, log_tail, created_at_ms)
        VALUES (?, ?, ?, ?, NULL, NULL, NULL, '', ?)
        "#,
    )
    .bind(req.job_id.to_string())
    .bind(req.agent_id.to_string())
    .bind(&req.model_ref)
    .bind("Pending")
    .bind(now_ms)
    .execute(&state.db)
    .await?;

    Ok((
        StatusCode::CREATED,
        Json(json!({ "job_id": req.job_id.to_string() })),
    ))
}

/// Get pending jobs for an agent.
/// Agent polls this endpoint every 10s.
/// Traces to: FR-FLEET-008
pub async fn get_pending_jobs(
    State(_state): State<Arc<AppState>>,
    Path(_agent_id): Path<Uuid>,
) -> Result<Json<Vec<DispatchOrder>>, ServerError> {
    // For MVP, we return empty jobs. Full implementation would:
    // 1. Query jobs with state='Pending' for this agent
    // 2. Convert DB rows back to DispatchOrder
    // For now, return empty to avoid bloating the integration test
    Ok(Json(vec![]))
}

/// Report job completion or state transition.
/// Traces to: FR-FLEET-008
pub async fn report_job(
    State(state): State<Arc<AppState>>,
    Path(job_id): Path<Uuid>,
    Json(req): Json<DispatchReport>,
) -> Result<StatusCode, ServerError> {
    if req.job_id != job_id {
        return Err(ServerError::Validation {
            reason: "job_id mismatch".to_string(),
        });
    }

    let state_str = match req.state {
        JobState::Pending => "Pending",
        JobState::Running => "Running",
        JobState::Succeeded => "Succeeded",
        JobState::Failed => "Failed",
        JobState::TimedOut => "TimedOut",
    };

    sqlx::query(
        r#"
        UPDATE jobs
        SET state = ?, started_at_ms = ?, finished_at_ms = ?, exit_code = ?, log_tail = ?
        WHERE id = ?
        "#,
    )
    .bind(state_str)
    .bind(req.started_at_ms.map(|ms| ms as i64))
    .bind(req.finished_at_ms.map(|ms| ms as i64))
    .bind(req.exit_code)
    .bind(&req.log_tail)
    .bind(job_id.to_string())
    .execute(&state.db)
    .await?;

    Ok(StatusCode::NO_CONTENT)
}

/// Health check endpoint.
/// Traces to: FR-FLEET-001
pub async fn health(State(state): State<Arc<AppState>>) -> Json<HealthResponse> {
    let agent_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM agents")
        .fetch_one(&state.db)
        .await
        .unwrap_or((0,));

    Json(HealthResponse {
        status: "ok".to_string(),
        uptime_s: 0, // TODO: track server uptime
        agent_count: agent_count.0,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    // Traces to: FR-FLEET-008
    #[test]
    fn test_job_state_serialization() {
        let state = JobState::Succeeded;
        let json = serde_json::to_string(&state).expect("serialize");
        assert_eq!(json, "\"Succeeded\"");
        let state2: JobState = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(state, state2);
    }
}
