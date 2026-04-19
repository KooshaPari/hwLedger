//! Tailscale peer discovery via shell-out (FR-FLEET-004).
//!
//! Discovers Tailscale network peers by shelling out to `tailscale status --json`
//! and parsing the JSON output into a structured peer list.

use crate::error::ServerError;
use serde::{Deserialize, Serialize};
use std::process::Stdio;
use tokio::process::Command;
use tracing::info;

/// A single Tailscale peer in the network.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TailscalePeer {
    /// Unique node ID within the tailnet.
    pub node_id: String,
    /// Hostname of the peer (e.g., "gpu-box-1").
    pub hostname: String,
    /// Tailscale IP address (100.x.y.z).
    pub tailscale_ip: String,
    /// Operating system (e.g., "linux", "darwin", "windows").
    pub os: String,
    /// Whether the peer is currently online.
    pub online: bool,
    /// Relay server in use, if any (e.g., "aws-us-west-1c").
    pub relay: String,
}

/// Complete Tailscale status snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TailscaleStatus {
    /// List of all peers in the tailnet.
    pub peers: Vec<TailscalePeer>,
    /// Information about this node itself.
    pub self_node: TailscalePeer,
}

/// Discover Tailscale peers by shelling out to `tailscale status --json`.
/// Traces to: FR-FLEET-004
pub async fn discover() -> Result<TailscaleStatus, ServerError> {
    let output = Command::new("tailscale")
        .arg("status")
        .arg("--json")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
        .map_err(|e| ServerError::Validation {
            reason: format!("failed to run 'tailscale status --json': {}", e),
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(ServerError::Validation {
            reason: format!("tailscale CLI not found or not authenticated: {}", stderr),
        });
    }

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Parse the JSON output
    let raw: serde_json::Value = serde_json::from_str(&stdout).map_err(|e| {
        ServerError::Validation { reason: format!("failed to parse tailscale JSON: {}", e) }
    })?;

    // Extract peers and self
    let mut peers = Vec::new();
    let mut self_node: Option<TailscalePeer> = None;

    if let Some(peers_obj) = raw.get("Peers").and_then(|v| v.as_object()) {
        for (_, peer_json) in peers_obj.iter() {
            if let Ok(peer) = parse_peer(peer_json) {
                peers.push(peer);
            }
        }
    }

    if let Some(self_json) = raw.get("Self") {
        if let Ok(peer) = parse_peer(self_json) {
            self_node = Some(peer);
        }
    }

    let self_node = self_node.ok_or_else(|| ServerError::Validation {
        reason: "missing 'Self' in tailscale status output".to_string(),
    })?;

    info!("discovered {} tailscale peers", peers.len());

    Ok(TailscaleStatus { peers, self_node })
}

/// Parse a single peer from the raw JSON object.
fn parse_peer(json: &serde_json::Value) -> Result<TailscalePeer, String> {
    let node_id = json
        .get("ID")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "missing ID".to_string())?
        .to_string();

    let hostname = json
        .get("HostName")
        .or_else(|| json.get("Hostnames"))
        .and_then(|v| v.as_str())
        .or_else(|| json.get("Name").and_then(|v| v.as_str()))
        .unwrap_or("unknown")
        .to_string();

    let tailscale_ip = json
        .get("TailscaleIPs")
        .and_then(|v| v.as_array())
        .and_then(|arr| arr.first())
        .and_then(|v| v.as_str())
        .or_else(|| {
            json.get("Addrs")
                .and_then(|v| v.as_array())
                .and_then(|arr| arr.first())
                .and_then(|v| v.as_str())
        })
        .unwrap_or("0.0.0.0")
        .to_string();

    let os = json.get("OS").and_then(|v| v.as_str()).unwrap_or("unknown").to_string();

    let online = json.get("Online").and_then(|v| v.as_bool()).unwrap_or(false);

    let relay = json.get("Relay").and_then(|v| v.as_str()).unwrap_or("").to_string();

    Ok(TailscalePeer { node_id, hostname, tailscale_ip, os, online, relay })
}

#[cfg(test)]
mod tests {
    use super::*;

    // Traces to: FR-FLEET-004
    #[test]
    fn test_parse_peer_basic() {
        let json = serde_json::json!({
            "ID": "1",
            "HostName": "gpu-box-1",
            "TailscaleIPs": ["100.100.100.1"],
            "OS": "linux",
            "Online": true,
            "Relay": "aws-us-west-1c"
        });

        let peer = parse_peer(&json).expect("parse failed");
        assert_eq!(peer.node_id, "1");
        assert_eq!(peer.hostname, "gpu-box-1");
        assert_eq!(peer.tailscale_ip, "100.100.100.1");
        assert_eq!(peer.os, "linux");
        assert!(peer.online);
        assert_eq!(peer.relay, "aws-us-west-1c");
    }

    // Traces to: FR-FLEET-004
    #[test]
    fn test_parse_peer_missing_fields() {
        let json = serde_json::json!({
            "ID": "2",
            "OS": "darwin"
        });

        let peer = parse_peer(&json).expect("parse failed");
        assert_eq!(peer.node_id, "2");
        assert_eq!(peer.hostname, "unknown");
        assert_eq!(peer.tailscale_ip, "0.0.0.0");
        assert!(!peer.online);
    }

    // Traces to: FR-FLEET-004
    #[test]
    fn test_tailscale_status_serde() {
        let peer = TailscalePeer {
            node_id: "1".to_string(),
            hostname: "box-1".to_string(),
            tailscale_ip: "100.100.100.1".to_string(),
            os: "linux".to_string(),
            online: true,
            relay: "".to_string(),
        };

        let status = TailscaleStatus { peers: vec![peer.clone()], self_node: peer };

        let json = serde_json::to_string(&status).expect("serialize");
        let status2: TailscaleStatus = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(status2.peers.len(), 1);
        assert_eq!(status2.self_node.node_id, "1");
    }
}
