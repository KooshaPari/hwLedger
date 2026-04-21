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
    /// Pre-pinned SSH host key fingerprint (SHA-256, openssh format like
    /// `SHA256:abc…`). When set, the server key presented at handshake must
    /// match this value exactly — `~/.ssh/known_hosts` is bypassed. Use for
    /// programmatic trust loops where the fingerprint ships alongside the
    /// ledger's `HostEnrolled` event.
    ///
    /// Traces to: FR-FLEET-003
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected_fingerprint: Option<String>,
}

/// Structured host-key verification errors surfaced through
/// [`SshClient::check_server_key`]. Rich enough for the CLI and server
/// layers to drive a trust decision.
///
/// Traces to: FR-FLEET-003
#[derive(Debug, thiserror::Error)]
pub enum SshError {
    /// Host is not in `known_hosts` and TOFU is disabled.
    #[error("unknown SSH host {hostname}: fingerprint {fingerprint} not in known_hosts")]
    KnownHostsPrompt { hostname: String, fingerprint: String },

    /// Host is in `known_hosts` but the key has changed — possible MITM.
    #[error("SSH host key for {hostname} CHANGED (possible MITM): new fingerprint {fingerprint}")]
    KnownHostsKeyChanged { hostname: String, fingerprint: String },

    /// Expected fingerprint was supplied but the actual one differs.
    #[error("SSH host {hostname} fingerprint mismatch: expected {expected}, got {actual}")]
    FingerprintMismatch { hostname: String, expected: String, actual: String },

    /// Wrapped lower-level russh error (required by russh's Handler trait).
    #[error("russh error: {0}")]
    Russh(#[from] russh::Error),
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

        let policy = HostKeyPolicy::from_env(&self.host);
        let (handler, last_err) = SshClient::new(policy);

        let connect_res =
            tokio::time::timeout(Duration::from_secs(10), client::connect(config, addr, handler))
                .await
                .map_err(|_| anyhow!("SSH handshake timeout after 10s"))?;

        let mut session = match connect_res {
            Ok(s) => s,
            Err(e) => {
                // Surface the structured host-key verdict if present.
                if let Some(host_err) = last_err.lock().unwrap().take() {
                    return Err(anyhow!("{host_err}"));
                }
                return Err(e.into());
            }
        };

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

/// Host-key verification policy consumed by [`SshClient`].
///
/// Traces to: FR-FLEET-003
#[derive(Debug, Clone)]
pub struct HostKeyPolicy {
    pub hostname: String,
    pub port: u16,
    /// Optional pinned fingerprint; when `Some`, takes precedence over
    /// `known_hosts`.
    pub expected_fingerprint: Option<String>,
    /// Override for the `known_hosts` path (defaults to `~/.ssh/known_hosts`).
    pub known_hosts_path: Option<PathBuf>,
    /// Trust-on-first-use: when the key is absent, auto-append and continue.
    /// Enabled via env var `HWLEDGER_SSH_TOFU=1`.
    pub tofu: bool,
}

impl HostKeyPolicy {
    fn from_env(host: &SshHost) -> Self {
        let tofu = std::env::var("HWLEDGER_SSH_TOFU").map(|v| v == "1").unwrap_or(false);
        HostKeyPolicy {
            hostname: host.hostname.clone(),
            port: host.port,
            expected_fingerprint: host.expected_fingerprint.clone(),
            known_hosts_path: None,
            tofu,
        }
    }
}

/// SSH client handler with host-key pinning.
///
/// The handshake rejects any server whose key is not:
/// - byte-equal to `policy.expected_fingerprint` (when set), OR
/// - recorded in `known_hosts` (default `~/.ssh/known_hosts`), OR
/// - newly encountered under `HWLEDGER_SSH_TOFU=1` (in which case it is
///   appended to `known_hosts` before continuing).
///
/// Rejection reasons are stashed in `last_error` so the caller can surface
/// them after `client::connect` returns `Err`.
///
/// Traces to: FR-FLEET-003
pub struct SshClient {
    policy: HostKeyPolicy,
    last_error: Arc<std::sync::Mutex<Option<SshError>>>,
}

impl SshClient {
    /// Construct a policy-bound client. The returned `last_error` slot lets
    /// the caller introspect the rejection reason after handshake failure.
    pub fn new(policy: HostKeyPolicy) -> (Self, Arc<std::sync::Mutex<Option<SshError>>>) {
        let slot = Arc::new(std::sync::Mutex::new(None));
        (SshClient { policy, last_error: slot.clone() }, slot)
    }

    fn default_known_hosts_path() -> Option<PathBuf> {
        std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".ssh").join("known_hosts"))
    }

    /// Core decision routine — factored out for direct unit testing.
    /// Returns `Ok(true)` on accept, `Err(SshError::*)` with rich reason
    /// on reject. `Ok(false)` is not used (reject paths always carry info).
    fn decide(
        policy: &HostKeyPolicy,
        server_pub: &PublicKey,
    ) -> std::result::Result<bool, SshError> {
        let fingerprint = server_pub.fingerprint();

        // 1) Programmatic pin always wins.
        if let Some(expected) = &policy.expected_fingerprint {
            return if fingerprint == *expected {
                Ok(true)
            } else {
                Err(SshError::FingerprintMismatch {
                    hostname: policy.hostname.clone(),
                    expected: expected.clone(),
                    actual: fingerprint,
                })
            };
        }

        // 2) known_hosts lookup.
        let path =
            policy.known_hosts_path.clone().or_else(Self::default_known_hosts_path).ok_or_else(
                || SshError::KnownHostsPrompt {
                    hostname: policy.hostname.clone(),
                    fingerprint: fingerprint.clone(),
                },
            )?;

        if path.exists() {
            match russh_keys::check_known_hosts_path(
                &policy.hostname,
                policy.port,
                server_pub,
                &path,
            ) {
                Ok(true) => return Ok(true),
                Ok(false) => { /* fall through to TOFU / prompt */ }
                Err(russh_keys::Error::KeyChanged { .. }) => {
                    return Err(SshError::KnownHostsKeyChanged {
                        hostname: policy.hostname.clone(),
                        fingerprint,
                    });
                }
                Err(e) => {
                    warn!("known_hosts scan failed for {}: {}", policy.hostname, e);
                }
            }
        }

        // 3) TOFU: auto-learn and append.
        if policy.tofu {
            if let Some(parent) = path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            match russh_keys::known_hosts::learn_known_hosts_path(
                &policy.hostname,
                policy.port,
                server_pub,
                &path,
            ) {
                Ok(()) => {
                    info!(
                        "SSH TOFU: appended {} ({}) to {}",
                        policy.hostname,
                        fingerprint,
                        path.display()
                    );
                    return Ok(true);
                }
                Err(e) => {
                    warn!("failed to append to {}: {}", path.display(), e);
                }
            }
        }

        // 4) Unknown host — hard reject with fingerprint for prompt.
        Err(SshError::KnownHostsPrompt { hostname: policy.hostname.clone(), fingerprint })
    }
}

#[async_trait::async_trait]
impl Handler for SshClient {
    type Error = SshError;

    async fn check_server_key(
        &mut self,
        server_public_key: &PublicKey,
    ) -> std::result::Result<bool, Self::Error> {
        match Self::decide(&self.policy, server_public_key) {
            Ok(accepted) => {
                debug!(
                    "SSH: host key decision accept={} fingerprint={}",
                    accepted,
                    server_public_key.fingerprint()
                );
                Ok(accepted)
            }
            Err(e) => {
                // Stash a clone of the structured reason so the connect
                // caller can surface it verbatim. SshError is not Clone
                // because russh::Error isn't, so hand-copy the informative
                // variants and fall back to the typed err for russh wraps.
                let stash = match &e {
                    SshError::KnownHostsPrompt { hostname, fingerprint } => {
                        Some(SshError::KnownHostsPrompt {
                            hostname: hostname.clone(),
                            fingerprint: fingerprint.clone(),
                        })
                    }
                    SshError::KnownHostsKeyChanged { hostname, fingerprint } => {
                        Some(SshError::KnownHostsKeyChanged {
                            hostname: hostname.clone(),
                            fingerprint: fingerprint.clone(),
                        })
                    }
                    SshError::FingerprintMismatch { hostname, expected, actual } => {
                        Some(SshError::FingerprintMismatch {
                            hostname: hostname.clone(),
                            expected: expected.clone(),
                            actual: actual.clone(),
                        })
                    }
                    SshError::Russh(_) => None,
                };
                if let Some(s) = stash {
                    *self.last_error.lock().unwrap() = Some(s);
                }
                Err(e)
            }
        }
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
            expected_fingerprint: None,
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
            expected_fingerprint: None,
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

    fn generate_pub_key() -> PublicKey {
        let kp = russh_keys::key::KeyPair::generate_ed25519();
        match kp {
            russh_keys::key::KeyPair::Ed25519(sk) => PublicKey::Ed25519(sk.verifying_key()),
            #[allow(unreachable_patterns)]
            _ => unreachable!("generate_ed25519 always returns Ed25519"),
        }
    }

    fn policy_for(path: &std::path::Path, hostname: &str, tofu: bool) -> HostKeyPolicy {
        HostKeyPolicy {
            hostname: hostname.to_string(),
            port: 22,
            expected_fingerprint: None,
            known_hosts_path: Some(path.to_path_buf()),
            tofu,
        }
    }

    /// Traces to: FR-FLEET-003 — known host: fingerprint already in
    /// `known_hosts` so the policy accepts the server key silently.
    #[test]
    fn host_key_decide_known_host_accepts() {
        let tmp = tempfile::tempdir().unwrap();
        let kh = tmp.path().join("known_hosts");
        let pk = generate_pub_key();
        russh_keys::known_hosts::learn_known_hosts_path("host-known.example", 22, &pk, &kh)
            .unwrap();

        let policy = policy_for(&kh, "host-known.example", /*tofu=*/ false);
        let outcome = SshClient::decide(&policy, &pk).expect("known host must accept");
        assert!(outcome);
    }

    /// Traces to: FR-FLEET-003 — TOFU: unknown host is auto-appended and
    /// subsequent visits re-accept from the freshly-written record.
    #[test]
    fn host_key_decide_tofu_appends_and_accepts() {
        let tmp = tempfile::tempdir().unwrap();
        let kh = tmp.path().join("known_hosts");
        let pk = generate_pub_key();

        let policy = policy_for(&kh, "host-new.example", /*tofu=*/ true);
        let outcome = SshClient::decide(&policy, &pk).expect("tofu must accept first sight");
        assert!(outcome);

        let contents = std::fs::read_to_string(&kh).unwrap();
        assert!(
            contents.contains("host-new.example") || contents.contains("|1|"),
            "known_hosts should contain a hashed or plain record: {contents}"
        );
        let outcome2 = SshClient::decide(&policy, &pk).expect("TOFU-learned key must re-accept");
        assert!(outcome2);
    }

    /// Traces to: FR-FLEET-003 — strict mode (no tofu, no pin, unseen host):
    /// the handler returns `KnownHostsPrompt` with the fingerprint so the
    /// caller can drive a trust decision.
    #[test]
    fn host_key_decide_unknown_host_rejects_with_prompt() {
        let tmp = tempfile::tempdir().unwrap();
        let kh = tmp.path().join("known_hosts");
        let pk = generate_pub_key();

        let policy = policy_for(&kh, "host-unseen.example", /*tofu=*/ false);
        let err = SshClient::decide(&policy, &pk).expect_err("unknown host must reject");
        match err {
            SshError::KnownHostsPrompt { hostname, fingerprint } => {
                assert_eq!(hostname, "host-unseen.example");
                assert_eq!(fingerprint, pk.fingerprint());
            }
            other => panic!("expected KnownHostsPrompt, got {other:?}"),
        }
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
            expected_fingerprint: None,
        };
        let json = serde_json::to_string(&host).expect("serialize");
        let host2: SshHost = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(host.hostname, host2.hostname);
        assert_eq!(host.port, host2.port);
        assert_eq!(host.user, host2.user);
    }
}
