//! Dedicated tests for certificate authority functionality.
//! Traces to: FR-FLEET-001, FR-FLEET-002

use hwledger_server::ca::CertificateAuthority;
use tempfile::TempDir;

#[tokio::test]
async fn test_ca_load_or_create_new() {
    // Traces to: FR-FLEET-001
    let temp_dir = TempDir::new().expect("create temp dir");
    let cert_path = temp_dir.path().join("new_ca.crt");
    let key_path = temp_dir.path().join("new_ca.key");

    assert!(!cert_path.exists());
    assert!(!key_path.exists());

    let ca = CertificateAuthority::load_or_create(&cert_path, &key_path)
        .await
        .expect("create CA");

    assert!(cert_path.exists());
    assert!(key_path.exists());
    assert!(ca.ca_cert_pem.contains("BEGIN CERTIFICATE"));
    assert!(ca.ca_cert_pem.contains("END CERTIFICATE"));
}

#[tokio::test]
async fn test_ca_load_existing() {
    // Traces to: FR-FLEET-001
    let temp_dir = TempDir::new().expect("create temp dir");
    let cert_path = temp_dir.path().join("existing.crt");
    let key_path = temp_dir.path().join("existing.key");

    // Create initial CA
    let ca1 = CertificateAuthority::load_or_create(&cert_path, &key_path)
        .await
        .expect("create CA 1");
    let cert1 = ca1.ca_cert_pem.clone();

    // Load again
    let ca2 = CertificateAuthority::load_or_create(&cert_path, &key_path)
        .await
        .expect("create CA 2");

    // Should be the same certificate
    assert_eq!(cert1, ca2.ca_cert_pem);
}

#[tokio::test]
async fn test_ca_sign_csr_returns_valid_pem() {
    // Traces to: FR-FLEET-001, FR-FLEET-002
    let temp_dir = TempDir::new().expect("create temp dir");
    let cert_path = temp_dir.path().join("test.crt");
    let key_path = temp_dir.path().join("test.key");

    let ca = CertificateAuthority::load_or_create(&cert_path, &key_path)
        .await
        .expect("create CA");

    let signed = ca.sign_csr("any-csr-pem", "my-agent-hostname")
        .expect("sign CSR");

    assert!(signed.contains("BEGIN CERTIFICATE"));
    assert!(signed.contains("END CERTIFICATE"));
    assert!(signed.len() > 100); // Real certificate should be substantial
}

#[tokio::test]
async fn test_ca_sign_csr_different_hostnames() {
    // Traces to: FR-FLEET-002
    let temp_dir = TempDir::new().expect("create temp dir");
    let cert_path = temp_dir.path().join("test.crt");
    let key_path = temp_dir.path().join("test.key");

    let ca = CertificateAuthority::load_or_create(&cert_path, &key_path)
        .await
        .expect("create CA");

    let cert1 = ca.sign_csr("fake-csr", "agent-1").expect("sign CSR 1");
    let cert2 = ca.sign_csr("fake-csr", "agent-2").expect("sign CSR 2");

    // Both should be valid PEM but different (different agent names)
    assert!(cert1.contains("BEGIN CERTIFICATE"));
    assert!(cert2.contains("BEGIN CERTIFICATE"));
    assert_ne!(cert1, cert2);
}

