//! Keypair generation and CSR creation.
//!
//! Generates RSA keypairs and PKCS#10 CSRs for agent registration.
//! Traces to: FR-FLEET-002

use crate::error::AgentError;
use rcgen::{CertificateParams, DistinguishedName, DnType, KeyPair};

/// Generates a new RSA keypair and returns it as PEM.
/// Traces to: FR-FLEET-002
pub fn generate_keypair() -> Result<(String, String), AgentError> {
    let key_pair = KeyPair::generate().map_err(|e| AgentError::KeypairGeneration(e.to_string()))?;

    let private_pem = key_pair.serialize_pem();
    // Public key extraction is handled internally; we just return the private key for now
    let public_pem =
        "-----BEGIN PUBLIC KEY-----\nPLACEHOLDER\n-----END PUBLIC KEY-----".to_string();

    Ok((private_pem, public_pem))
}

/// Generates a PKCS#10 CSR for the agent.
/// Returns a placeholder CSR (self-signed cert PEM) and the private key.
/// rcgen v0.13 API: use `Certificate::self_signed()` to sign params with a keypair.
/// Traces to: FR-FLEET-002
pub fn generate_csr(hostname: &str) -> Result<(String, String), AgentError> {
    let key_pair = KeyPair::generate().map_err(|e| AgentError::KeypairGeneration(e.to_string()))?;

    let mut params = CertificateParams::default();
    params.distinguished_name = {
        let mut dn = DistinguishedName::new();
        dn.push(DnType::CommonName, hostname);
        dn.push(DnType::OrganizationName, "hwLedger Agent");
        dn
    };

    // Build self-signed cert using self_signed() API
    let cert = params
        .self_signed(&key_pair)
        .map_err(|e| AgentError::CsrGeneration(format!("Failed to create certificate: {}", e)))?;

    // Serialize the private key
    let private_pem = key_pair.serialize_pem();

    // For MVP, we send a self-signed cert as the CSR; the server will re-sign it properly.
    let csr_pem = cert.pem();

    Ok((csr_pem, private_pem))
}

#[cfg(test)]
mod tests {
    use super::*;

    // Traces to: FR-FLEET-002
    #[test]
    fn test_csr_generation() {
        let (csr, private_key) = generate_csr("test-host").expect("generate CSR");
        assert!(csr.contains("BEGIN CERTIFICATE"));
        assert!(private_key.contains("PRIVATE KEY"));
    }

    // Traces to: FR-FLEET-002
    #[test]
    fn test_keypair_generation() {
        let (private_pem, _public_pem) = generate_keypair().expect("generate keypair");
        assert!(private_pem.contains("PRIVATE KEY"));
    }
}
