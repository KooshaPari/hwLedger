//! Extract Common Name (CN) from client certificates for mTLS admin validation.
//!
//! Traces to: FR-FLEET-001, ADR-0009

use base64::engine::{general_purpose, Engine};
use tracing::debug;
use x509_parser::prelude::*;

/// Extract the Common Name (CN) from an X.509 certificate in PEM format.
///
/// Returns the CN value if found, or None if parsing fails.
/// Traces to: ADR-0009
pub fn extract_cn_from_pem(pem_cert: &str) -> Option<String> {
    // Parse PEM: extract the base64 content between BEGIN and END markers
    let pem_lines: Vec<&str> = pem_cert.lines().collect();
    if pem_lines.is_empty() || !pem_lines[0].contains("BEGIN") {
        debug!("Invalid PEM format");
        return None;
    }

    // Find END marker
    let end_idx = pem_lines.iter().position(|line| line.contains("END"))?;

    // Reconstruct DER from PEM base64 (skip BEGIN and END lines)
    let base64_content = pem_lines[1..end_idx].join("");
    let der_bytes = match general_purpose::STANDARD.decode(&base64_content) {
        Ok(bytes) => bytes,
        Err(e) => {
            debug!("Failed to decode PEM base64: {}", e);
            return None;
        }
    };

    // Parse X.509 certificate from DER
    let (_, cert) = match parse_x509_certificate(&der_bytes) {
        Ok(result) => result,
        Err(e) => {
            debug!("Failed to parse X.509 certificate: {}", e);
            return None;
        }
    };

    // Extract CN from the subject's Distinguished Name
    // Use iter_common_name() to directly access CN
    for cn_attr in cert.subject.iter_common_name() {
        // cn_attr is an AttributeTypeAndValue
        // Convert to string
        if let Ok(cn_str) = cn_attr.as_str() {
            return Some(cn_str.to_string());
        }
    }

    None
}

/// Check if a certificate's CN matches "admin".
///
/// Returns true if CN == "admin", false otherwise.
/// Traces to: ADR-0009
pub fn is_admin_cert(pem_cert: &str) -> bool {
    match extract_cn_from_pem(pem_cert) {
        Some(cn) => cn == "admin",
        None => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Generate a test certificate with CN "admin"
    fn generate_test_admin_cert() -> String {
        use rcgen::{CertificateParams, DistinguishedName, DnType, KeyPair};

        let key_pair = KeyPair::generate().unwrap();
        let mut params = CertificateParams::default();
        params.distinguished_name = {
            let mut dn = DistinguishedName::new();
            dn.push(DnType::CommonName, "admin");
            dn
        };

        let cert = params.self_signed(&key_pair).unwrap();
        cert.pem()
    }

    // Generate a test certificate with CN "agent"
    fn generate_test_agent_cert() -> String {
        use rcgen::{CertificateParams, DistinguishedName, DnType, KeyPair};

        let key_pair = KeyPair::generate().unwrap();
        let mut params = CertificateParams::default();
        params.distinguished_name = {
            let mut dn = DistinguishedName::new();
            dn.push(DnType::CommonName, "agent");
            dn
        };

        let cert = params.self_signed(&key_pair).unwrap();
        cert.pem()
    }

    // Traces to: ADR-0009, FR-FLEET-001
    #[test]
    fn test_extract_cn_from_admin_cert() {
        let admin_cert = generate_test_admin_cert();
        let cn = extract_cn_from_pem(&admin_cert);
        assert_eq!(cn, Some("admin".to_string()));
    }

    // Traces to: ADR-0009, FR-FLEET-001
    #[test]
    fn test_extract_cn_from_agent_cert() {
        let agent_cert = generate_test_agent_cert();
        let cn = extract_cn_from_pem(&agent_cert);
        assert_eq!(cn, Some("agent".to_string()));
    }

    // Traces to: ADR-0009
    #[test]
    fn test_is_admin_cert_true() {
        let admin_cert = generate_test_admin_cert();
        assert!(is_admin_cert(&admin_cert));
    }

    // Traces to: ADR-0009
    #[test]
    fn test_is_admin_cert_false() {
        let agent_cert = generate_test_agent_cert();
        assert!(!is_admin_cert(&agent_cert));
    }

    // Traces to: ADR-0009
    #[test]
    fn test_is_admin_cert_invalid_pem() {
        assert!(!is_admin_cert("not a cert"));
    }
}
