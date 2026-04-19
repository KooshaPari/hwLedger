//! Agentless SSH probing for device discovery (FR-FLEET-003).
//!
//! Provides connection pooling to remote hosts via SSH and parses GPU telemetry
//! from nvidia-smi, rocm-smi, and system_profiler outputs.

use crate::error::ServerError;
use anyhow::Result;
use hwledger_fleet_proto::DeviceReport;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tracing::{info, warn};

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

/// Pool of SSH connections to a single host.
/// MVP: placeholder for russh integration.
pub struct SshPool {
    host: SshHost,
    #[expect(dead_code)]
    max_size: usize,
}

impl SshPool {
    /// Create a new SSH connection pool.
    /// Traces to: FR-FLEET-003
    pub async fn new(host: SshHost, max_size: usize) -> Result<Self> {
        if max_size == 0 {
            anyhow::bail!("max_size must be > 0");
        }
        Ok(SshPool { host, max_size })
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

    async fn run_command(&self, _cmd: &str) -> Result<String> {
        // TODO(fleet-ssh-exec-v1): implement russh channel setup
        // Placeholder: actual russh integration with russh 0.46 requires:
        // 1. Client config setup with key loading (identity variants)
        // 2. Async SSH session connection (TCP + cipher negotiation)
        // 3. Channel open for exec with command
        // 4. Read from channel until EOF
        // 5. Handle keep-alive with 30s timeout
        // For now, fail loudly as per spec.
        anyhow::bail!("SSH command execution not yet implemented in MVP")
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
}
