//! Axum extractor for mTLS admin certificate validation.
//!
//! Implements: ADR-0009
//!
//! Provides `AdminCert` extractor that pulls the client certificate CN from
//! the rustls connection, validates it against the admin CN value, and rejects
//! non-admin requests with 403 Forbidden or 401 Unauthorized.

use crate::cert_extract::{extract_cn_from_pem, is_admin_cert};
use crate::error::ServerError;
use crate::tls::PeerCertInfo;
use axum::async_trait;
use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use base64::engine::{general_purpose, Engine};
use tracing::{debug, warn};

fn der_to_pem(der: &[u8]) -> String {
    let b64 = general_purpose::STANDARD.encode(der);
    let mut pem = String::from("-----BEGIN CERTIFICATE-----\n");
    for chunk in b64.as_bytes().chunks(64) {
        pem.push_str(std::str::from_utf8(chunk).unwrap());
        pem.push('\n');
    }
    pem.push_str("-----END CERTIFICATE-----\n");
    pem
}

/// Extractor that validates admin mTLS certificate.
///
/// Extracts the client certificate CN from the rustls connection and checks
/// that it matches "admin". Non-admin certificates result in 403 Forbidden.
/// Missing certificates (when required) result in 401 Unauthorized.
///
/// Traces to: ADR-0009, FR-FLEET-001
#[derive(Debug, Clone)]
pub struct AdminCert(pub String);

#[async_trait]
impl<S> FromRequestParts<S> for AdminCert
where
    S: Send + Sync,
{
    type Rejection = ServerError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        // Preferred path: rustls TLS handshake captured the peer cert and the
        // PeerCertAcceptor injected it as a request extension.
        if let Some(peer) = parts.extensions.get::<PeerCertInfo>() {
            if let Some(der) = peer.cert_der.as_deref() {
                let pem = der_to_pem(der);
                match extract_cn_from_pem(&pem) {
                    Some(cn) if cn == "admin" => {
                        debug!("AdminCert: mTLS CN=admin accepted");
                        return Ok(AdminCert(cn));
                    }
                    Some(cn) => {
                        warn!("AdminCert: mTLS CN={cn} rejected (not admin)");
                        return Err(ServerError::Auth {
                            reason: format!("mTLS cert CN={cn} is not admin"),
                        });
                    }
                    None => {
                        warn!("AdminCert: mTLS peer cert present but CN unparseable");
                        return Err(ServerError::Auth {
                            reason: "mTLS cert missing CN".to_string(),
                        });
                    }
                }
            }
        }

        // Plain-HTTP fallback: X-Admin-Cert header carries a PEM blob.
        if let Some(header) = parts.headers.get("X-Admin-Cert") {
            if let Ok(cert_pem) = header.to_str() {
                if is_admin_cert(cert_pem) {
                    debug!("AdminCert: X-Admin-Cert header CN=admin accepted");
                    return Ok(AdminCert("admin".to_string()));
                } else {
                    warn!("AdminCert: X-Admin-Cert header CN is not admin");
                    return Err(ServerError::Auth {
                        reason: "not an admin certificate".to_string(),
                    });
                }
            }
        }

        warn!("AdminCert: no mTLS peer cert and no X-Admin-Cert header");
        Err(ServerError::Auth {
            reason: "admin certificate required; use mTLS CN=admin or X-Admin-Cert header"
                .to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::extract::FromRequestParts;
    use axum::http::Request;

    fn mint_cert(cn: &str) -> Vec<u8> {
        use rcgen::{CertificateParams, DistinguishedName, DnType, KeyPair};
        let key_pair = KeyPair::generate().unwrap();
        let mut params = CertificateParams::default();
        let mut dn = DistinguishedName::new();
        dn.push(DnType::CommonName, cn);
        params.distinguished_name = dn;
        let cert = params.self_signed(&key_pair).unwrap();
        cert.der().to_vec()
    }

    // Traces to: ADR-0009, FR-FLEET-001
    #[tokio::test]
    async fn test_admin_cert_from_peer_cert_extension() {
        let mut req = Request::builder().body(()).unwrap();
        req.extensions_mut().insert(PeerCertInfo { cert_der: Some(mint_cert("admin")) });
        let (mut parts, _) = req.into_parts();
        let res = AdminCert::from_request_parts(&mut parts, &()).await;
        assert!(matches!(res, Ok(AdminCert(cn)) if cn == "admin"));
    }

    // Traces to: ADR-0009
    #[tokio::test]
    async fn test_non_admin_peer_cert_rejected() {
        let mut req = Request::builder().body(()).unwrap();
        req.extensions_mut().insert(PeerCertInfo { cert_der: Some(mint_cert("agent-001")) });
        let (mut parts, _) = req.into_parts();
        let res = AdminCert::from_request_parts(&mut parts, &()).await;
        assert!(res.is_err());
    }

    // Traces to: ADR-0009
    #[tokio::test]
    async fn test_no_cert_no_header_rejected() {
        let req = Request::builder().body(()).unwrap();
        let (mut parts, _) = req.into_parts();
        let res = AdminCert::from_request_parts(&mut parts, &()).await;
        assert!(res.is_err());
    }

    #[test]
    fn test_der_to_pem_roundtrip() {
        let der = mint_cert("admin");
        let pem = der_to_pem(&der);
        assert!(pem.contains("BEGIN CERTIFICATE"));
        assert_eq!(extract_cn_from_pem(&pem), Some("admin".to_string()));
    }
}
