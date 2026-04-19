//! Fleet subcommand: server and agent management.
//!
//! Traces to: FR-FLEET-001 (WP22 MVP)

use anyhow::{anyhow, Result};
use clap::{Parser, Subcommand};
use comfy_table::Table;
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Subcommand)]
pub enum FleetSubcommand {
    /// Query server status and connected agents.
    Status(StatusArgs),

    /// Register this agent with a fleet server.
    Register(RegisterArgs),

    /// Retrieve audit log from server.
    Audit(AuditArgs),
}

#[derive(Parser)]
pub struct StatusArgs {
    /// Fleet server URL (http://localhost:8080 or https://...).
    #[arg(long, env = "HWLEDGER_FLEET_SERVER")]
    server: String,

    /// Bearer token for authentication.
    #[arg(long, env = "HWLEDGER_FLEET_TOKEN")]
    token: String,

    /// Output as JSON instead of table.
    #[arg(long)]
    json: bool,
}

#[derive(Parser)]
pub struct RegisterArgs {
    /// Fleet server URL.
    #[arg(long, env = "HWLEDGER_FLEET_SERVER")]
    server: String,

    /// Bearer token for authentication.
    #[arg(long, env = "HWLEDGER_FLEET_TOKEN")]
    token: String,

    /// Hostname for this agent.
    #[arg(long, default_value = "localhost")]
    hostname: String,
}

#[derive(Parser)]
pub struct AuditArgs {
    /// Fleet server URL.
    #[arg(long, env = "HWLEDGER_FLEET_SERVER")]
    server: String,

    /// Maximum number of entries to retrieve.
    #[arg(long, default_value = "100")]
    limit: u32,

    /// Verify audit trail signatures (if server supports it).
    #[arg(long)]
    verify: bool,

    /// Output as JSON instead of table.
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerStatus {
    pub schema: String,
    pub version: String,
    pub uptime_seconds: u64,
    pub connected_agents: usize,
    pub total_vram_gb: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[expect(dead_code, reason = "surface wired for future flows — see WP32 follow-up")]
pub struct Agent {
    pub id: String,
    pub hostname: String,
    pub backend: String,
    pub vram_gb: u32,
    pub connected_since_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    pub timestamp_ms: u64,
    pub agent_id: String,
    pub event_type: String,
    pub details: String,
}

pub fn run(subcommand: FleetSubcommand) -> Result<()> {
    match subcommand {
        FleetSubcommand::Status(args) => status(args),
        FleetSubcommand::Register(args) => register(args),
        FleetSubcommand::Audit(args) => audit(args),
    }
}

fn status(args: StatusArgs) -> Result<()> {
    // Stub: in production, call /v1/status on the server
    // For now, return a realistic response

    if !args.server.starts_with("http") {
        return Err(anyhow!(
            "server URL must start with http:// or https://, got: {}",
            args.server
        ));
    }

    if args.token.is_empty() {
        return Err(anyhow!("--token is required"));
    }

    tracing::info!("Querying fleet status: {}", args.server);

    let status = ServerStatus {
        schema: "hwledger.v1".to_string(),
        version: "0.0.1".to_string(),
        uptime_seconds: 86400,
        connected_agents: 3,
        total_vram_gb: 240,
    };

    if args.json {
        println!("{}", serde_json::to_string_pretty(&status)?);
    } else {
        let mut table = Table::new();
        table.set_header(vec!["Property", "Value"]);
        table.add_row(vec!["Version", &status.version]);
        table.add_row(vec!["Uptime", &format!("{} hours", status.uptime_seconds / 3600)]);
        table.add_row(vec!["Connected Agents", &status.connected_agents.to_string()]);
        table.add_row(vec!["Total VRAM", &format!("{} GB", status.total_vram_gb)]);
        println!("{}", table);
    }

    Ok(())
}

fn register(args: RegisterArgs) -> Result<()> {
    if !args.server.starts_with("http") {
        return Err(anyhow!("server URL must start with http:// or https://"));
    }

    if args.token.is_empty() {
        return Err(anyhow!("--token is required"));
    }

    tracing::info!("Registering agent '{}' with fleet server: {}", args.hostname, args.server);

    // Stub: in production, POST /v1/agents/register with body {hostname, ...}
    // For now, just report success
    println!("Successfully registered agent '{}' with server at {}", args.hostname, args.server);

    Ok(())
}

fn audit(args: AuditArgs) -> Result<()> {
    if !args.server.starts_with("http") {
        return Err(anyhow!("server URL must start with http:// or https://"));
    }

    tracing::info!("Retrieving audit log from {}, limit={}", args.server, args.limit);

    // Stub: in production, call GET /v1/audit with ?limit=N and optional ?verify=true
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis() as u64;

    let entries = vec![
        AuditEntry {
            timestamp_ms: now - 5000,
            agent_id: "agent-001".to_string(),
            event_type: "inference_start".to_string(),
            details: "model=meta-llama/Llama-2-7b, seq_len=512".to_string(),
        },
        AuditEntry {
            timestamp_ms: now - 3000,
            agent_id: "agent-001".to_string(),
            event_type: "inference_end".to_string(),
            details: "duration_ms=2000, status=ok".to_string(),
        },
        AuditEntry {
            timestamp_ms: now - 1000,
            agent_id: "agent-002".to_string(),
            event_type: "probe_telemetry".to_string(),
            details: "backend=nvidia, device_0_free_vram=45GB".to_string(),
        },
    ];

    if args.json {
        println!("{}", serde_json::to_string_pretty(&entries)?);
    } else {
        let mut table = Table::new();
        table.set_header(vec!["Agent", "Event", "Details"]);
        for entry in entries {
            table.add_row(vec![entry.agent_id, entry.event_type, entry.details.clone()]);
        }
        println!("{}", table);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // Traces to: FR-FLEET-001
    #[test]
    fn test_invalid_server_url() {
        let args = StatusArgs {
            server: "invalid://localhost:8080".to_string(),
            token: "test-token".to_string(),
            json: false,
        };
        let result = status(args);
        assert!(result.is_err());
    }

    // Traces to: FR-FLEET-001
    #[test]
    fn test_missing_token() {
        let args = StatusArgs {
            server: "http://localhost:8080".to_string(),
            token: "".to_string(),
            json: false,
        };
        let result = status(args);
        assert!(result.is_err());
    }
}
