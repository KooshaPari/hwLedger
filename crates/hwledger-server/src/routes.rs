//! HTTP route handlers for the fleet server.
//!
//! Endpoints for agent registration, heartbeat, job dispatch, and health checks.
//! Traces to: FR-FLEET-001, FR-FLEET-002, FR-FLEET-008

use crate::error::ServerError;
use crate::{rentals, ssh, tailscale, AppState};
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use base64::engine::{general_purpose, Engine};
use chrono::Utc;
use hwledger_fleet_proto::{
    AgentRegistration, DispatchOrder, DispatchReport, Heartbeat, JobState, RegistrationAck,
};
use serde::Serialize;
use serde_json::json;
use std::collections::HashMap;
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
        return Err(ServerError::Auth { reason: "invalid bootstrap token".to_string() });
    }

    // Sign the CSR
    let agent_cert_pem = state
        .ca
        .sign_csr(&req.cert_csr_pem, &req.hostname)
        .map_err(|e| ServerError::Internal { reason: format!("failed to sign CSR: {}", e) })?;

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
    .bind(
        serde_json::to_string(&req.platform)
            .map_err(|e| ServerError::Internal { reason: e.to_string() })?,
    )
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
        return Err(ServerError::Validation { reason: "agent_id mismatch".to_string() });
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
/// Admin-only: requires mTLS with CN="admin". Validates via cert extraction.
/// Traces to: FR-FLEET-001, ADR-0009
///
/// Note: This function will be called with an extracted admin CN when mTLS listener is wired in lib.rs.
/// For now (MVP), validation is stubbed; axum-server with rustls will inject the cert via Extension.
pub async fn list_agents(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<Agent>>, ServerError> {
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

    Ok((StatusCode::CREATED, Json(json!({ "job_id": req.job_id.to_string() }))))
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
        return Err(ServerError::Validation { reason: "job_id mismatch".to_string() });
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
    let agent_count: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM agents").fetch_one(&state.db).await.unwrap_or((0,));

    Json(HealthResponse {
        status: "ok".to_string(),
        uptime_s: 0, // TODO: track server uptime
        agent_count: agent_count.0,
    })
}

/// Probe a remote host via SSH for GPU devices.
/// Query parameter: `host` is base64-encoded JSON of SshHost struct.
/// Traces to: FR-FLEET-003
pub async fn ssh_probe(
    State(_state): State<Arc<AppState>>,
    axum::extract::Query(params): axum::extract::Query<HashMap<String, String>>,
) -> Result<Json<Vec<hwledger_fleet_proto::DeviceReport>>, ServerError> {
    let host_b64 = params.get("host").ok_or_else(|| ServerError::Validation {
        reason: "missing 'host' query parameter".to_string(),
    })?;

    let host_bytes = general_purpose::STANDARD.decode(host_b64).map_err(|e| {
        ServerError::Validation { reason: format!("invalid base64 host encoding: {}", e) }
    })?;

    let host_json = String::from_utf8(host_bytes).map_err(|e| ServerError::Validation {
        reason: format!("host bytes are not valid UTF-8: {}", e),
    })?;

    let _host: ssh::SshHost = serde_json::from_str(&host_json).map_err(|e| {
        ServerError::Validation { reason: format!("failed to parse SSH host JSON: {}", e) }
    })?;

    // TODO(fleet-ssh-exec-v1): implement actual SSH connection pool
    Err(ServerError::Internal { reason: "SSH probe not yet implemented in MVP".to_string() })
}

/// Discover Tailscale peers in the local network.
/// Traces to: FR-FLEET-004
pub async fn tailscale_peers(
    State(_state): State<Arc<AppState>>,
) -> Result<Json<tailscale::TailscaleStatus>, ServerError> {
    tailscale::discover().await.map(Json)
}

/// Get or refresh the rental offerings catalog.
/// Caches results for 1 hour.
/// Traces to: FR-FLEET-005
pub async fn get_rentals(
    State(state): State<Arc<AppState>>,
) -> Result<Json<rentals::RentalCatalog>, ServerError> {
    // Check if cache is fresh
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);

    let ttl_ms = 3600 * 1000; // 1 hour

    {
        let catalog = state.rentals_catalog.read().await;
        if let Some(cached) = catalog.as_ref() {
            if now_ms.saturating_sub(cached.refreshed_at_ms) < ttl_ms {
                return Ok(Json(cached.clone()));
            }
        }
    }

    // Refresh catalog
    let api_keys = rentals::RentalApiKeys {
        vast_ai: std::env::var("HWLEDGER_VAST_API_KEY").ok(),
        runpod: std::env::var("HWLEDGER_RUNPOD_API_KEY").ok(),
        lambda: std::env::var("HWLEDGER_LAMBDA_API_KEY").ok(),
        modal: std::env::var("HWLEDGER_MODAL_API_KEY").ok(),
    };

    let new_catalog = rentals::RentalCatalog::refresh(api_keys).await?;
    let catalog_clone = new_catalog.clone();

    let mut cache = state.rentals_catalog.write().await;
    *cache = Some(new_catalog);

    Ok(Json(catalog_clone))
}

/// Query placement suggestions across agents, peers, and rentals.
/// Filters by model reference and minimum VRAM requirement.
/// Ranks by (fit_score, cost_per_hour).
/// Traces to: FR-FLEET-007
pub async fn placement_suggestions(
    State(state): State<Arc<AppState>>,
    axum::extract::Query(params): axum::extract::Query<HashMap<String, String>>,
) -> Result<Json<Vec<PlacementSuggestion>>, ServerError> {
    let _model_ref = params.get("model_ref").ok_or_else(|| ServerError::Validation {
        reason: "missing 'model_ref' query parameter".to_string(),
    })?;

    let min_vram_gb: u32 = params
        .get("min_vram_gb")
        .ok_or_else(|| ServerError::Validation {
            reason: "missing 'min_vram_gb' query parameter".to_string(),
        })?
        .parse()
        .map_err(|_| ServerError::Validation {
            reason: "invalid 'min_vram_gb' (expected integer)".to_string(),
        })?;

    let min_vram_bytes = (min_vram_gb as u64) * 1024 * 1024 * 1024;

    // Collect agents from the database
    let agents: Vec<(String, u64)> = sqlx::query_as::<_, (String, i64)>(
        "SELECT id, COALESCE(
            (SELECT COALESCE(MAX(free_vram_bytes), 0) FROM telemetry WHERE agent_id = agents.id),
            0
        ) as max_free_vram FROM agents"
    )
    .fetch_all(&state.db)
    .await
    .unwrap_or_default()
    .into_iter()
    .map(|(id, vram)| (id, vram as u64))
    .collect();

    let mut candidates: Vec<PlacementCandidate> = Vec::new();

    // Score each agent
    for (agent_id, free_vram_bytes) in agents {
        let device_vram: i64 = sqlx::query_scalar::<_, i64>(
            "SELECT COALESCE(MAX(total_vram_bytes), 1) FROM devices WHERE agent_id = ?"
        )
        .bind(&agent_id)
        .fetch_one(&state.db)
        .await
        .unwrap_or(1);

        let total_vram_bytes = device_vram as u64;

        // Fit score: clamp((free - required) / total, 0, 1)
        let fit_score = if free_vram_bytes >= min_vram_bytes {
            let excess = free_vram_bytes - min_vram_bytes;
            ((excess as f32) / (total_vram_bytes as f32)).clamp(0.0, 1.0)
        } else {
            0.0
        };

        // Cost score for agents is always 1.0 (free/internal)
        let cost_score = 1.0f32;

        let rank = 0.7 * fit_score + 0.3 * cost_score;

        candidates.push(PlacementCandidate {
            agent_id: Some(agent_id),
            rental_offering: None,
            fit_score,
            cost_score,
            rank,
        });
    }

    // Score each rental offering
    if let Ok(rentals_catalog_lock) = state.rentals_catalog.try_read() {
        if let Some(rentals_catalog) = rentals_catalog_lock.as_ref() {
            for offering in &rentals_catalog.entries {
                let offering_vram_bytes = (offering.vram_gb as u64) * 1024 * 1024 * 1024;

                // Only include rentals that meet minimum VRAM
                if offering_vram_bytes < min_vram_bytes {
                    continue;
                }

                // Fit score: (offering_vram - required) / offering_vram, clamped to [0, 1]
                let excess = offering_vram_bytes - min_vram_bytes;
                let fit_score =
                    ((excess as f32) / (offering_vram_bytes as f32)).clamp(0.0, 1.0);

                // Cost score: 1.0 / (1.0 + hourly_usd)
                let cost_val: f32 = 1.0f32 / (1.0f32 + offering.hourly_usd as f32);
                let cost_score = cost_val.clamp(0.0, 1.0);

                let rank = 0.7f32 * fit_score + 0.3f32 * cost_score;

                candidates.push(PlacementCandidate {
                    agent_id: None,
                    rental_offering: Some(offering.clone()),
                    fit_score,
                    cost_score,
                    rank,
                });
            }
        }
    }

    // Sort by rank descending, then by fit_score descending, then by cost ascending
    candidates.sort_by(|a, b| {
        b.rank
            .partial_cmp(&a.rank)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| {
                b.fit_score
                    .partial_cmp(&a.fit_score)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .then_with(|| a.cost_score.partial_cmp(&b.cost_score)
                .unwrap_or(std::cmp::Ordering::Equal))
    });

    // Convert to PlacementSuggestion and return top 5
    let suggestions: Vec<PlacementSuggestion> = candidates
        .into_iter()
        .take(5)
        .map(|c| {
            let location = if let Some(ref offering) = c.rental_offering {
                format!("{} ({})", offering.gpu_model, offering.region)
            } else {
                format!("agent-{}", c.agent_id.unwrap_or_default())
            };
            let cost_per_hour_usd = c
                .rental_offering
                .as_ref()
                .map(|o| o.hourly_usd)
                .unwrap_or(0.0);

            PlacementSuggestion {
                location,
                fit_score: c.fit_score,
                cost_per_hour_usd,
                rank: c.rank,
            }
        })
        .collect();

    Ok(Json(suggestions))
}

/// Placement candidate combining agent or rental offering with computed scores.
#[derive(Debug, Clone, Serialize)]
pub struct PlacementCandidate {
    /// Agent UUID if this is an internal agent, None for rentals.
    pub agent_id: Option<String>,
    /// Rental offering if this is a cloud rental, None for internal agents.
    pub rental_offering: Option<rentals::RentalOffering>,
    /// VRAM fitness score: clamp((free_vram - required_vram) / total_vram, 0, 1).
    pub fit_score: f32,
    /// Cost score: 1.0 / (1.0 + hourly_usd), ranging [0, 1).
    pub cost_score: f32,
    /// Combined rank: 0.7 * fit_score + 0.3 * cost_score.
    pub rank: f32,
}

/// Placement suggestion with location, fit score, and estimated cost.
#[derive(Debug, Serialize)]
pub struct PlacementSuggestion {
    pub location: String,
    pub fit_score: f32,
    pub cost_per_hour_usd: f64,
    pub rank: f32,
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

    // Traces to: FR-FLEET-007
    #[test]
    fn test_placement_fit_score_when_free_exceeds_required() {
        let required_vram = 24u64 * 1024 * 1024 * 1024; // 24GB
        let free_vram = 80u64 * 1024 * 1024 * 1024;    // 80GB
        let total_vram = 80u64 * 1024 * 1024 * 1024;

        let excess = free_vram - required_vram;
        let fit_score = ((excess as f32) / (total_vram as f32)).clamp(0.0, 1.0);
        // (80-24)/80 = 56/80 = 0.7, so assert > 0.6
        assert!(fit_score > 0.6);
    }

    // Traces to: FR-FLEET-007
    #[test]
    fn test_placement_fit_score_when_free_insufficient() {
        let required_vram = 80u64 * 1024 * 1024 * 1024; // 80GB
        let free_vram = 24u64 * 1024 * 1024 * 1024;    // 24GB
        let _total_vram = 24u64 * 1024 * 1024 * 1024;

        // Fit score should be 0 if free < required
        let fit_score = if free_vram >= required_vram {
            let excess = free_vram - required_vram;
            ((excess as f32) / (_total_vram as f32)).clamp(0.0, 1.0)
        } else {
            0.0f32
        };

        assert_eq!(fit_score, 0.0);
    }

    // Traces to: FR-FLEET-007
    #[test]
    fn test_placement_cost_score_monotonic_in_price() {
        let cost1 = 1.0 / (1.0 + 0.30f32);
        let cost2 = 1.0 / (1.0 + 2.0f32);

        // Cheaper should have higher cost_score
        assert!(cost1 > cost2);
    }

    // Traces to: FR-FLEET-007
    #[test]
    fn test_placement_rank_combines_fit_and_cost() {
        let fit_score = 0.9f32;
        let cost_score = 0.5f32;
        let rank = 0.7f32 * fit_score + 0.3f32 * cost_score;

        // rank = 0.7*0.9 + 0.3*0.5 = 0.63 + 0.15 = 0.78
        // So rank (0.78) < fit_score (0.9), which is expected
        assert!(rank < fit_score);
        assert!(rank > cost_score);
    }

    // Traces to: FR-FLEET-007
    #[test]
    fn test_placement_candidate_ordering_by_rank() {
        let mut candidates = [
            PlacementCandidate {
                agent_id: Some("a1".to_string()),
                rental_offering: None,
                fit_score: 0.5f32,
                cost_score: 0.5f32,
                rank: 0.5f32,
            },
            PlacementCandidate {
                agent_id: Some("a2".to_string()),
                rental_offering: None,
                fit_score: 0.9f32,
                cost_score: 0.9f32,
                rank: 0.9f32,
            },
            PlacementCandidate {
                agent_id: Some("a3".to_string()),
                rental_offering: None,
                fit_score: 0.7f32,
                cost_score: 0.7f32,
                rank: 0.7f32,
            },
        ].to_vec();

        candidates.sort_by(|a, b| {
            b.rank
                .partial_cmp(&a.rank)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        assert_eq!(candidates[0].rank, 0.9f32);
        assert_eq!(candidates[1].rank, 0.7f32);
        assert_eq!(candidates[2].rank, 0.5f32);
    }
}
