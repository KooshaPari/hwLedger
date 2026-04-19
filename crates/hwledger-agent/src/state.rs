//! Agent state persistence (agent_id, keypair, certificate).
//!
//! Stores agent identity across restarts. First run generates keypair + agent_id,
//! registers, and persists the signed cert.
//! Traces to: FR-FLEET-002

use crate::error::AgentError;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use uuid::Uuid;

/// Persistent agent state.
/// Traces to: FR-FLEET-002
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentState {
    /// Unique agent identifier (generated on first run).
    pub agent_id: Uuid,
    /// Private key in PEM format.
    pub private_key_pem: String,
    /// Assigned certificate from server (None until registration completes).
    pub assigned_cert_pem: Option<String>,
}

impl AgentState {
    /// Loads agent state from disk or creates a new one.
    /// Traces to: FR-FLEET-002
    pub async fn load_or_create(state_dir: &Path) -> Result<Self, AgentError> {
        fs::create_dir_all(state_dir)?;
        let state_file = state_dir.join("agent.json");

        if state_file.exists() {
            let json = fs::read_to_string(&state_file)?;
            let state: AgentState = serde_json::from_str(&json)?;
            tracing::info!("Loaded agent state from {}", state_file.display());
            return Ok(state);
        }

        // Generate new state
        tracing::info!("Generating new agent state");
        let (_csr_pem, private_key_pem) = crate::keypair::generate_csr(&hostname::get()?.to_string_lossy())?;

        let agent_id = Uuid::new_v4();
        let state = AgentState {
            agent_id,
            private_key_pem,
            assigned_cert_pem: None,
        };

        // Persist to disk
        let json = serde_json::to_string_pretty(&state)?;
        fs::write(&state_file, json)?;
        tracing::info!("Saved agent state to {}", state_file.display());

        Ok(state)
    }

    /// Persists updated state to disk.
    /// Traces to: FR-FLEET-002
    pub fn save(&self, state_dir: &Path) -> Result<(), AgentError> {
        fs::create_dir_all(state_dir)?;
        let state_file = state_dir.join("agent.json");
        let json = serde_json::to_string_pretty(self)?;
        fs::write(&state_file, json)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    // Traces to: FR-FLEET-002
    #[tokio::test]
    async fn test_state_creation() {
        let temp = TempDir::new().expect("create temp dir");
        let state = AgentState::load_or_create(temp.path()).await.expect("load or create state");
        assert!(!state.agent_id.to_string().is_empty());
        assert!(state.private_key_pem.contains("PRIVATE KEY"));
        assert!(state.assigned_cert_pem.is_none());
    }

    // Traces to: FR-FLEET-002
    #[tokio::test]
    async fn test_state_persistence() {
        let temp = TempDir::new().expect("create temp dir");
        let state1 = AgentState::load_or_create(temp.path()).await.expect("load or create");
        let agent_id_1 = state1.agent_id;

        let state2 = AgentState::load_or_create(temp.path()).await.expect("reload");
        assert_eq!(state2.agent_id, agent_id_1);
    }
}
