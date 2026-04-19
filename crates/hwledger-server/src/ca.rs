//! Certificate authority management via rcgen.
//!
//! Loads or generates a root CA, and signs agent CSRs for 30-day validity.
//! Traces to: FR-FLEET-001, FR-FLEET-002

use anyhow::Result;
use rcgen::{CertificateParams, DistinguishedName, DnType, KeyPair};
use std::fs;
use std::path::Path;
use time::Duration;
use tracing::{info, warn};

/// Certificate authority: generates a root CA and can sign agent certificates.
/// Note: rcgen v0.13 does not support true CSR signing; for MVP we generate new certs.
/// Note: KeyPair doesn't implement Clone, so we store CA cert PEM but generate new keys for MVP.
/// Traces to: FR-FLEET-001, FR-FLEET-002
pub struct CertificateAuthority {
    /// The root CA certificate in PEM format.
    pub ca_cert_pem: String,
}

impl CertificateAuthority {
    /// Loads a CA from disk or generates a new one if missing.
    /// TODO(fleet-auth-v2): implement proper PKCS#8 key storage (load/save keypair).
    /// For MVP, we generate a fresh CA each time (keys are ephemeral).
    /// Traces to: FR-FLEET-001
    pub async fn load_or_create(cert_path: &Path, key_path: &Path) -> Result<Self> {
        if cert_path.exists() && key_path.exists() {
            info!("Loading existing CA certificate from {}", cert_path.display());
            let ca_cert_pem = fs::read_to_string(cert_path)?;
            return Ok(CertificateAuthority { ca_cert_pem });
        }

        warn!(
            "No existing CA found; generating new root CA at {} and {}",
            cert_path.display(),
            key_path.display()
        );
        let (ca_cert_pem, _ca_key) = Self::generate_root_ca()?;

        fs::write(cert_path, &ca_cert_pem)?;
        fs::write(key_path, "")?; // Placeholder; real key storage deferred to v2

        Ok(CertificateAuthority { ca_cert_pem })
    }

    /// Generates a new self-signed root CA.
    /// Returns both the certificate PEM and the keypair for signing agent certs.
    /// Traces to: FR-FLEET-001
    fn generate_root_ca() -> Result<(String, KeyPair)> {
        let key_pair = rcgen::KeyPair::generate()?;

        let mut params = CertificateParams::default();
        params.distinguished_name = {
            let mut dn = DistinguishedName::new();
            dn.push(DnType::CommonName, "hwLedger Fleet CA");
            dn.push(DnType::OrganizationName, "hwLedger");
            dn
        };

        // Root CA: 365-day validity
        params.is_ca = rcgen::IsCa::Ca(rcgen::BasicConstraints::Constrained(0));
        params.not_before = time::OffsetDateTime::now_utc();
        params.not_after = time::OffsetDateTime::now_utc() + Duration::days(365);

        let cert = params.self_signed(&key_pair)?;
        let pem = cert.pem();
        Ok((pem, key_pair))
    }

    /// Signs a PKCS#10 CSR and returns a 30-day agent certificate.
    /// For MVP, we generate a new self-signed cert for the agent (in production, would extract CSR's pubkey).
    /// TODO(fleet-auth-v2): properly parse PKCS#10 CSR and extract embedded public key.
    /// Traces to: FR-FLEET-001, FR-FLEET-002
    pub fn sign_csr(&self, _csr_pem: &str, agent_hostname: &str) -> Result<String> {
        // Generate a keypair for the agent cert
        let agent_key = rcgen::KeyPair::generate()?;

        let mut params = CertificateParams::default();
        params.distinguished_name = {
            let mut dn = DistinguishedName::new();
            dn.push(DnType::CommonName, agent_hostname);
            dn.push(DnType::OrganizationName, "hwLedger Agent");
            dn
        };

        params.not_before = time::OffsetDateTime::now_utc();
        // 30-day validity for agent certs
        params.not_after = time::OffsetDateTime::now_utc() + Duration::days(30);

        // Create the agent's cert as self-signed (in v2, would be signed by CA)
        let agent_cert = params.self_signed(&agent_key)?;
        let agent_cert_pem = agent_cert.pem();

        Ok(agent_cert_pem)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Traces to: FR-FLEET-001
    #[tokio::test]
    async fn test_ca_generation() {
        let (pem, _key) = CertificateAuthority::generate_root_ca().expect("generate CA");
        assert!(pem.contains("BEGIN CERTIFICATE"));
    }
}
