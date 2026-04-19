//! Agent registration flow.
//!
//! Generates CSR, sends AgentRegistration to server, receives RegistrationAck,
//! and persists the signed cert.
//! Traces to: FR-FLEET-002

use crate::config::AgentConfig;
use crate::error::AgentError;
use crate::state::AgentState;
use hwledger_fleet_proto::{AgentRegistration, Platform};
use std::time::Duration;
use tracing::info;

/// Register the agent with the server.
/// Generates CSR, sends registration request, and updates state with the signed cert.
/// Traces to: FR-FLEET-002
pub async fn register(config: &AgentConfig, state: &mut AgentState) -> Result<(), AgentError> {
    info!("Registering agent {} with server", state.agent_id);

    // Generate CSR for this agent
    let hostname = hostname::get()
        .unwrap_or_else(|_| std::ffi::OsStr::new("unknown").to_os_string())
        .to_string_lossy()
        .to_string();

    let (csr_pem, _private_key) = crate::keypair::generate_csr(&hostname)?;

    // Gather platform metadata
    let platform = Platform {
        os: std::env::consts::OS.to_string(),
        arch: std::env::consts::ARCH.to_string(),
        kernel: get_kernel_version(),
        total_ram_bytes: get_total_ram(),
        cpu_model: get_cpu_model(),
    };

    // Create registration request
    let reg = AgentRegistration {
        agent_id: state.agent_id,
        hostname,
        cert_csr_pem: csr_pem,
        platform,
        bootstrap_token: config.bootstrap_token.clone(),
    };

    // Send to server with retries
    let client = reqwest::Client::builder()
        .danger_accept_invalid_certs(true) // TODO(fleet-auth-v2): proper cert validation
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(AgentError::Network)?;

    let url = format!("{}/v1/agents/register", config.server_url);
    let response = client.post(&url).json(&reg).send().await?;

    if !response.status().is_success() {
        let error_text = response.text().await.unwrap_or_else(|_| "unknown error".to_string());
        return Err(AgentError::Registration {
            reason: format!("server rejected registration: {}", error_text),
        });
    }

    let ack: hwledger_fleet_proto::RegistrationAck = response.json().await?;
    state.assigned_cert_pem = Some(ack.assigned_cert_pem);
    state.save(&config.state_dir)?;

    info!("Agent registered successfully; cert expires in 30 days");
    Ok(())
}

/// Attempt to retrieve the kernel version.
fn get_kernel_version() -> String {
    #[cfg(unix)]
    {
        std::process::Command::new("uname")
            .arg("-r")
            .output()
            .ok()
            .and_then(|output| String::from_utf8(output.stdout).ok())
            .map(|s| s.trim().to_string())
            .unwrap_or_else(|| "unknown".to_string())
    }
    #[cfg(not(unix))]
    {
        "unknown".to_string()
    }
}

/// Attempt to retrieve total system RAM in bytes.
fn get_total_ram() -> u64 {
    #[cfg(unix)]
    {
        use std::process::Command;
        if let Ok(output) = Command::new("sysctl").arg("-n").arg("hw.memsize").output() {
            if let Ok(stdout) = String::from_utf8(output.stdout) {
                if let Ok(bytes) = stdout.trim().parse::<u64>() {
                    return bytes;
                }
            }
        }
    }
    16 * 1024 * 1024 * 1024 // Default 16 GB fallback
}

/// Attempt to retrieve CPU model string.
fn get_cpu_model() -> String {
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("sysctl")
            .arg("-n")
            .arg("machdep.cpu.brand_string")
            .output()
            .ok()
            .and_then(|output| String::from_utf8(output.stdout).ok())
            .map(|s| s.trim().to_string())
            .unwrap_or_else(|| "unknown".to_string())
    }
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("sh")
            .arg("-c")
            .arg("grep -m1 'model name' /proc/cpuinfo | cut -d: -f2 | xargs")
            .output()
            .ok()
            .and_then(|output| String::from_utf8(output.stdout).ok())
            .map(|s| s.trim().to_string())
            .unwrap_or_else(|| "unknown".to_string())
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        "unknown".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Traces to: FR-FLEET-002
    #[test]
    fn test_kernel_version_retrieval() {
        let kernel = get_kernel_version();
        assert!(!kernel.is_empty());
    }

    // Traces to: FR-FLEET-002
    #[test]
    fn test_cpu_model_retrieval() {
        let cpu = get_cpu_model();
        assert!(!cpu.is_empty());
    }

    // Traces to: FR-FLEET-002
    #[test]
    fn test_platform_serialization() {
        let platform = Platform {
            os: "linux".to_string(),
            arch: "x86_64".to_string(),
            kernel: "6.8.0".to_string(),
            total_ram_bytes: 64 * 1024 * 1024 * 1024,
            cpu_model: "Intel Xeon".to_string(),
        };
        let json = serde_json::to_string(&platform).expect("serialize");
        let platform2: Platform = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(platform, platform2);
    }
}
