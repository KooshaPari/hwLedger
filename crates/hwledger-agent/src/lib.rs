//! Per-host agent for hwLedger fleet telemetry & job execution (FR-FLEET-002, FR-FLEET-008).
//!
//! On startup: generate keypair + CSR, register with server, persist cert.
//! Runtime: periodically send heartbeat with device telemetry, poll for jobs, execute and report.

pub mod config;
pub mod error;
pub mod keypair;
pub mod registration;
pub mod state;

pub use config::AgentConfig;
pub use error::AgentError;
pub use state::AgentState;

use anyhow::Result;
use hwledger_probe::detect;
use std::time::Duration;
use tracing::info;

/// Main agent loop: register, then heartbeat + poll for jobs.
/// Traces to: FR-FLEET-002
pub async fn run(config: AgentConfig) -> Result<()> {
    info!("hwLedger agent starting with config: {:?}", config);

    // Initialize or load agent state (keypair, cert, agent_id)
    let mut state = AgentState::load_or_create(&config.state_dir).await?;
    info!("Agent ID: {}", state.agent_id);

    // If no cert, register with server
    if state.assigned_cert_pem.is_none() {
        registration::register(&config, &mut state).await?;
    }

    // Initialize GPU probes
    let probes = detect();
    info!("Initialized {} GPU probes", probes.len());

    // Main loop: heartbeat every 30s, poll for jobs every 10s
    let mut heartbeat_interval = tokio::time::interval(config.heartbeat_interval);
    let mut job_poll_interval = tokio::time::interval(Duration::from_secs(10));

    loop {
        tokio::select! {
            _ = heartbeat_interval.tick() => {
                if let Err(e) = send_heartbeat(&config, &state, &probes).await {
                    tracing::warn!("Heartbeat failed: {}", e);
                }
            }
            _ = job_poll_interval.tick() => {
                if let Err(e) = poll_jobs(&config, &state).await {
                    tracing::warn!("Job poll failed: {}", e);
                }
            }
        }
    }
}

/// Send a heartbeat to the server with current device telemetry.
/// Traces to: FR-FLEET-002
async fn send_heartbeat(
    config: &AgentConfig,
    state: &AgentState,
    probes: &[Box<dyn hwledger_probe::GpuProbe>],
) -> Result<()> {
    let mut devices = Vec::new();

    for probe in probes {
        let backend = probe.backend_name();

        match probe.enumerate() {
            Ok(device_list) => {
                for device in device_list {
                    // Attempt to query telemetry for this device
                    let snapshot = match (
                        probe.free_vram(device.id),
                        probe.utilization(device.id),
                        probe.temperature(device.id),
                        probe.power_draw(device.id),
                    ) {
                        (Ok(free), Ok(util), Ok(temp), Ok(power)) => {
                            Some(hwledger_fleet_proto::TelemetrySnapshot {
                                free_vram_bytes: free,
                                util_percent: util,
                                temperature_c: temp,
                                power_watts: power,
                                captured_at_ms: chrono::Utc::now().timestamp_millis() as u64,
                            })
                        }
                        _ => None,
                    };

                    devices.push(hwledger_fleet_proto::DeviceReport {
                        backend: backend.to_string(),
                        id: device.id,
                        name: device.name,
                        uuid: device.uuid,
                        total_vram_bytes: device.total_vram,
                        snapshot,
                    });
                }
            }
            Err(e) => {
                tracing::warn!("Failed to enumerate {} devices: {}", backend, e);
            }
        }
    }

    let heartbeat = hwledger_fleet_proto::Heartbeat {
        agent_id: state.agent_id,
        uptime_s: 0, // TODO: track agent uptime
        devices,
    };

    let client = reqwest::Client::new();
    let url = format!("{}/v1/agents/{}/heartbeat", config.server_url, state.agent_id);

    client.post(&url).json(&heartbeat).send().await?;
    info!("Heartbeat sent");

    Ok(())
}

/// Poll for pending jobs from the server.
/// Traces to: FR-FLEET-008
async fn poll_jobs(config: &AgentConfig, state: &AgentState) -> Result<()> {
    let client = reqwest::Client::new();
    let url = format!("{}/v1/agents/{}/jobs", config.server_url, state.agent_id);

    let response = client.get(&url).send().await?;
    let jobs: Vec<hwledger_fleet_proto::DispatchOrder> = response.json().await?;

    for job in jobs {
        // TODO(fleet-v2): implement job execution
        tracing::info!("Received job: {:?}", job.job_id);
    }

    Ok(())
}

pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        assert!(!version().is_empty());
    }
}
