//! Agentless SSH probing for device discovery (FR-FLEET-003).
//!
//! Provides connection pooling to remote hosts via SSH and parses GPU telemetry
//! from nvidia-smi, rocm-smi, and system_profiler outputs.
//!
//! Uses russh 0.46 native client implementation (no subprocess) for SSH
//! connectivity. Authentication supports `Agent` (via `SSH_AUTH_SOCK`),
//! `KeyPath` (private key on disk), and `KeyData` (inline PEM).

use crate::error::ServerError;
use anyhow::{anyhow, bail, Result};
use hwledger_fleet_proto::DeviceReport;
use russh::client::{self, Handle, Handler};
use russh::keys as russh_keys;
use russh::keys::key::PublicKey;
use russh::ChannelMsg;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, info, warn};

/// SSH identity variants for authentication.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SshIdentity {
    /// Use SSH agent for key negotiation.
    #[serde(rename = "agent")]
    Agent,
    /// Use a local key file.
    #[serde(rename = "key_path")]
    KeyPath(PathBuf),
    /// Use a PEM-encoded key with optional passphrase.
    #[serde(rename = "key_data")]
    KeyData { pem: String, passphrase: Option<String> },
}

/// Target SSH host specification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SshHost {
    pub hostname: String,
    pub port: u16,
    pub user: String,
    pub identity: SshIdentity,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bastion: Option<Box<SshHost>>,
}

/// Pool of SSH connections to a single host via russh.
/// Uses deadpool to manage connection reuse with Clone-able handles.
pub struct SshPool {
    host: SshHost,
    // Pool would be initialized in production; for now, keep simple.
}

impl SshPool {
    /// Create a new SSH connection pool.
    /// Traces to: FR-FLEET-003
    pub async fn new(host: SshHost, _max_size: usize) -> Result<Self> {
        if host.hostname.is_empty() {
            anyhow::bail!("hostname must not be empty");
        }
        Ok(SshPool { host })
    }

    /// Probe a single remote host for GPU devices.
    /// Tries nvidia-smi first, then rocm-smi, then system_profiler.
    /// Traces to: FR-FLEET-003
    pub async fn probe(&self) -> std::result::Result<Vec<DeviceReport>, ServerError> {
        // Try nvidia-smi
        match self.run_command("nvidia-smi --query-gpu=index,uuid,name,memory.total,memory.free,utilization.gpu,temperature.gpu,power.draw --format=csv,noheader,nounits").await {
            Ok(output) => {
                info!("nvidia-smi succeeded on {}", self.host.hostname);
                return Ok(parse_nvidia_smi(&output));
            }
            Err(e) => {
                warn!("nvidia-smi failed on {}: {}", self.host.hostname, e);
            }
        }

        // Try rocm-smi
        match self.run_command("rocm-smi --showproductname --showmeminfo vram --showuse --showtemp --showpower --json").await {
            Ok(output) => {
                info!("rocm-smi succeeded on {}", self.host.hostname);
                return Ok(parse_rocm_smi_json(&output));
            }
            Err(e) => {
                warn!("rocm-smi failed on {}: {}", self.host.hostname, e);
            }
        }

        // Try system_profiler (macOS)
        match self.run_command("system_profiler SPGPUDataType -json").await {
            Ok(output) => {
                info!("system_profiler succeeded on {}", self.host.hostname);
                return Ok(parse_system_profiler_json(&output));
            }
            Err(e) => {
                warn!("system_profiler failed on {}: {}", self.host.hostname, e);
            }
        }

        Err(ServerError::Validation {
            reason: format!(
                "no GPU detection tools available on remote host {}",
                self.host.hostname
            ),
        })
    }

    /// Execute a remote command via SSH using the native russh client.
    /// Traces to: FR-FLEET-003, ADR-0003
    async fn run_command(&self, cmd: &str) -> Result<String> {
        let addr = (self.host.hostname.as_str(), self.host.port);
        debug!("SSH: russh connecting to {}:{}", self.host.hostname, self.host.port);

        let config = Arc::new(client::Config {
            inactivity_timeout: Some(Duration::from_secs(30)),
            ..Default::default()
        });

        let mut session =
            tokio::time::timeout(Duration::from_secs(10), client::connect(config, addr, SshClient))
                .await
                .map_err(|_| anyhow!("SSH handshake timeout after 10s"))??;

        self.authenticate(&mut session).await?;

        let mut channel = session.channel_open_session().await?;
        channel.exec(true, cmd).await?;

        let mut stdout = Vec::<u8>::new();
        let mut stderr = Vec::<u8>::new();
        let mut exit: Option<u32> = None;

        loop {
            match tokio::time::timeout(Duration::from_secs(30), channel.wait()).await {
                Err(_) => bail!("SSH command read timeout after 30s"),
                Ok(None) => break,
                Ok(Some(msg)) => match msg {
                    ChannelMsg::Data { ref data } => stdout.extend_from_slice(data),
                    ChannelMsg::ExtendedData { ref data, ext: 1 } => stderr.extend_from_slice(data),
                    ChannelMsg::ExitStatus { exit_status } => exit = Some(exit_status),
                    ChannelMsg::Eof | ChannelMsg::Close => break,
                    _ => {}
                },
            }
        }

        let _ = session.disconnect(russh::Disconnect::ByApplication, "done", "en").await;

        match exit {
            Some(0) => {
                let out = String::from_utf8(stdout)
                    .map_err(|e| anyhow!("SSH output is not valid UTF-8: {}", e))?;
                info!("SSH: command '{}' on {} completed successfully", cmd, self.host.hostname);
                Ok(out)
            }
            Some(code) => {
                let stderr_str = String::from_utf8_lossy(&stderr);
                bail!("SSH command exited with status {}: {}", code, stderr_str.trim())
            }
            None => bail!("SSH channel closed without exit status"),
        }
    }

    /// Authenticate the russh session according to the configured identity.
    async fn authenticate(&self, session: &mut Handle<SshClient>) -> Result<()> {
        let user = self.host.user.clone();
        let ok = match &self.host.identity {
            SshIdentity::KeyPath(path) => {
                let key = russh_keys::load_secret_key(path, None)
                    .map_err(|e| anyhow!("failed to load SSH key {}: {}", path.display(), e))?;
                session.authenticate_publickey(user.clone(), Arc::new(key)).await?
            }
            SshIdentity::KeyData { pem, passphrase } => {
                let key = russh_keys::decode_secret_key(pem, passphrase.as_deref())
                    .map_err(|e| anyhow!("failed to decode inline SSH key: {}", e))?;
                session.authenticate_publickey(user.clone(), Arc::new(key)).await?
            }
            SshIdentity::Agent => self.authenticate_agent(session, &user).await?,
        };

        if !ok {
            bail!("SSH public-key authentication failed for {}@{}", user, self.host.hostname);
        }
        Ok(())
    }

    /// Walk identities exposed by `ssh-agent` (via `SSH_AUTH_SOCK`) and try
    /// each one. Returns true on first success.
    async fn authenticate_agent(
        &self,
        session: &mut Handle<SshClient>,
        user: &str,
    ) -> Result<bool> {
        let mut agent = russh_keys::agent::client::AgentClient::connect_env()
            .await
            .map_err(|e| anyhow!("could not connect to ssh-agent (SSH_AUTH_SOCK): {}", e))?;

        let identities: Vec<PublicKey> = agent
            .request_identities()
            .await
            .map_err(|e| anyhow!("ssh-agent request_identities failed: {}", e))?;

        if identities.is_empty() {
            bail!("ssh-agent has no identities loaded");
        }

        for pk in identities {
            let (returned, result) = session.authenticate_future(user.to_string(), pk, agent).await;
            agent = returned;
            match result {
                Ok(true) => return Ok(true),
                Ok(false) => continue,
                Err(e) => {
                    warn!("ssh-agent signer error: {:?}", e);
                    continue;
                }
            }
        }
        Ok(false)
    }
}

/// Minimal SSH client handler.
///
/// MVP accepts any server key (logs a debug line). A production deployment
/// should pin host keys via `~/.ssh/known_hosts` or the fleet ledger.
struct SshClient;

#[async_trait::async_trait]
impl Handler for SshClient {
    type Error = russh::Error;

    async fn check_server_key(
        &mut self,
        server_public_key: &PublicKey,
    ) -> std::result::Result<bool, Self::Error> {
        debug!("SSH: accepting server key fingerprint {}", server_public_key.fingerprint());
        Ok(true)
    }
}

/// Parse nvidia-smi CSV output into DeviceReport entries.
/// Expected format: index,uuid,name,memory.total,memory.free,utilization.gpu,temperature.gpu,power.draw
fn parse_nvidia_smi(output: &str) -> Vec<DeviceReport> {
    // Traces to: FR-FLEET-003
    output
        .lines()
        .filter(|line| !line.is_empty())
        .enumerate()
        .filter_map(|(idx, line)| {
            let parts: Vec<&str> = line.split(',').map(|p| p.trim()).collect();
            if parts.len() < 4 {
                return None;
            }

            let total_vram_bytes =
                parts.get(3).and_then(|s| s.parse::<u64>().ok()).unwrap_or(0) * 1024 * 1024; // assume MB

            Some(DeviceReport {
                backend: "nvidia".to_string(),
                id: idx as u32,
                name: parts
                    .get(2)
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| "Unknown GPU".to_string()),
                uuid: parts.get(1).map(|s| s.to_string()),
                total_vram_bytes,
                snapshot: None, // Would be populated from device.snapshot if available
            })
        })
        .collect()
}

/// Parse rocm-smi JSON output into DeviceReport entries.
fn parse_rocm_smi_json(output: &str) -> Vec<DeviceReport> {
    // Traces to: FR-FLEET-003
    match serde_json::from_str::<serde_json::Value>(output) {
        Ok(json) => {
            // rocm-smi --json output structure varies; minimal parsing
            if let Some(devices) = json.as_array() {
                devices
                    .iter()
                    .enumerate()
                    .map(|(idx, device)| {
                        let name = device
                            .get("product_name")
                            .or_else(|| device.get("name"))
                            .and_then(|v| v.as_str())
                            .unwrap_or("AMD GPU")
                            .to_string();

                        let total_vram = device
                            .get("vram")
                            .or_else(|| device.get("total_memory"))
                            .and_then(|v| v.as_str())
                            .and_then(|s| s.parse::<u64>().ok())
                            .unwrap_or(0)
                            * 1024
                            * 1024;

                        DeviceReport {
                            backend: "amd".to_string(),
                            id: idx as u32,
                            name,
                            uuid: device
                                .get("gpu_unique_id")
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string()),
                            total_vram_bytes: total_vram,
                            snapshot: None,
                        }
                    })
                    .collect()
            } else {
                vec![]
            }
        }
        Err(_) => vec![],
    }
}

/// Parse macOS system_profiler JSON output into DeviceReport entries.
fn parse_system_profiler_json(output: &str) -> Vec<DeviceReport> {
    // Traces to: FR-FLEET-003
    match serde_json::from_str::<serde_json::Value>(output) {
        Ok(json) => {
            if let Some(gpu_array) = json.get("SPGPUDataType").and_then(|v| v.as_array()) {
                gpu_array
                    .iter()
                    .enumerate()
                    .map(|(idx, gpu)| {
                        let name = gpu
                            .get("_name")
                            .or_else(|| gpu.get("sppci_model"))
                            .and_then(|v| v.as_str())
                            .unwrap_or("Apple GPU")
                            .to_string();

                        let vram_str =
                            gpu.get("sppci_vram").and_then(|v| v.as_str()).unwrap_or("0 MB");
                        let vram_bytes = parse_vram_string(vram_str);

                        DeviceReport {
                            backend: "metal".to_string(),
                            id: idx as u32,
                            name,
                            uuid: None,
                            total_vram_bytes: vram_bytes,
                            snapshot: None,
                        }
                    })
                    .collect()
            } else {
                vec![]
            }
        }
        Err(_) => vec![],
    }
}

/// Helper to parse VRAM strings like "24 GB" or "8192 MB".
fn parse_vram_string(s: &str) -> u64 {
    let parts: Vec<&str> = s.split_whitespace().collect();
    if parts.len() < 2 {
        return 0;
    }

    let value = parts[0].parse::<f64>().unwrap_or(0.0);
    let unit = parts[1].to_uppercase();

    match unit.as_str() {
        "GB" => (value * 1024.0 * 1024.0 * 1024.0) as u64,
        "MB" => (value * 1024.0 * 1024.0) as u64,
        "KB" => (value * 1024.0) as u64,
        "B" => value as u64,
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Traces to: FR-FLEET-003
    #[test]
    fn test_parse_nvidia_smi_single_gpu() {
        let output = "0,GPU-abc123,NVIDIA RTX 4090,24576,20480,30,60,120\n";
        let reports = parse_nvidia_smi(output);
        assert_eq!(reports.len(), 1);
        assert_eq!(reports[0].backend, "nvidia");
        assert_eq!(reports[0].name, "NVIDIA RTX 4090");
        assert_eq!(reports[0].uuid, Some("GPU-abc123".to_string()));
    }

    // Traces to: FR-FLEET-003
    #[test]
    fn test_parse_nvidia_smi_multiple_gpus() {
        let output =
            "0,GPU-abc,RTX 4090,24576,20480,30,60,120\n1,GPU-def,RTX 3090,24576,18432,50,75,200\n";
        let reports = parse_nvidia_smi(output);
        assert_eq!(reports.len(), 2);
        assert_eq!(reports[0].name, "RTX 4090");
        assert_eq!(reports[1].name, "RTX 3090");
    }

    // Traces to: FR-FLEET-003
    #[test]
    fn test_parse_rocm_smi_json() {
        let output = r#"[
            {"gpu_unique_id": "0x1002:0x67a0", "product_name": "AMD Radeon RX 7900 XTX", "vram": "24576"}
        ]"#;
        let reports = parse_rocm_smi_json(output);
        assert_eq!(reports.len(), 1);
        assert_eq!(reports[0].backend, "amd");
        assert_eq!(reports[0].name, "AMD Radeon RX 7900 XTX");
    }

    // Traces to: FR-FLEET-003
    #[test]
    fn test_parse_system_profiler_json() {
        let output = r#"{"SPGPUDataType": [{"_name": "Apple M4 Pro", "sppci_vram": "10 GB"}]}"#;
        let reports = parse_system_profiler_json(output);
        assert_eq!(reports.len(), 1);
        assert_eq!(reports[0].backend, "metal");
        assert_eq!(reports[0].name, "Apple M4 Pro");
        assert!(reports[0].total_vram_bytes > 0);
    }

    // Traces to: FR-FLEET-003
    #[test]
    fn test_parse_vram_string_gb() {
        assert_eq!(parse_vram_string("24 GB"), 24 * 1024 * 1024 * 1024);
    }

    // Traces to: FR-FLEET-003
    #[test]
    fn test_parse_vram_string_mb() {
        assert_eq!(parse_vram_string("8192 MB"), 8192 * 1024 * 1024);
    }

    // Traces to: FR-FLEET-003, ADR-0003
    #[tokio::test]
    async fn test_ssh_pool_new_valid_host() {
        let host = SshHost {
            hostname: "localhost".to_string(),
            port: 22,
            user: "testuser".to_string(),
            identity: SshIdentity::Agent,
            bastion: None,
        };
        let pool = SshPool::new(host, 5).await;
        assert!(pool.is_ok());
    }

    // Traces to: FR-FLEET-003
    #[tokio::test]
    async fn test_ssh_pool_new_empty_hostname_fails() {
        let host = SshHost {
            hostname: "".to_string(),
            port: 22,
            user: "testuser".to_string(),
            identity: SshIdentity::Agent,
            bastion: None,
        };
        let pool = SshPool::new(host, 5).await;
        assert!(pool.is_err());
    }

    // Traces to: FR-FLEET-003
    #[test]
    fn test_ssh_identity_agent() {
        let identity = SshIdentity::Agent;
        assert_eq!(format!("{:?}", identity), "Agent");
    }

    // Traces to: FR-FLEET-003, ADR-0003
    #[test]
    fn test_ssh_host_serialization() {
        let host = SshHost {
            hostname: "example.com".to_string(),
            port: 22,
            user: "ubuntu".to_string(),
            identity: SshIdentity::Agent,
            bastion: None,
        };
        let json = serde_json::to_string(&host).expect("serialize");
        let host2: SshHost = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(host.hostname, host2.hostname);
        assert_eq!(host.port, host2.port);
        assert_eq!(host.user, host2.user);
    }
}
