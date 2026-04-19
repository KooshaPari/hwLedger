//! Agent configuration.

use std::path::PathBuf;
use std::time::Duration;

/// Configuration for the hwLedger agent.
/// Traces to: FR-FLEET-002
#[derive(Debug, Clone)]
pub struct AgentConfig {
    /// Server URL (e.g., "https://central.example.com:6443").
    pub server_url: String,
    /// One-time bootstrap token for initial registration.
    pub bootstrap_token: String,
    /// Directory to persist agent state (keypair, cert, agent_id).
    pub state_dir: PathBuf,
    /// Interval between heartbeat submissions (default 30s).
    pub heartbeat_interval: Duration,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            server_url: "https://127.0.0.1:6443".to_string(),
            bootstrap_token: "dev-bootstrap-token".to_string(),
            state_dir: dirs::data_local_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join("hwledger-agent"),
            heartbeat_interval: Duration::from_secs(30),
        }
    }
}
