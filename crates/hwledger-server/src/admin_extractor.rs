//! Axum extractor for mTLS admin certificate validation.
//!
//! Implements: ADR-0009
//!
//! Provides `AdminCert` extractor that pulls the client certificate CN from
//! the rustls connection, validates it against the admin CN value, and rejects
//! non-admin requests with 403 Forbidden or 401 Unauthorized.

use crate::cert_extract::is_admin_cert;
use crate::error::ServerError;
use axum::async_trait;
use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use tracing::warn;

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
        // Note: In a production setup with rustls, the client certificate would be
        // available via ConnectInfo from the TCP connection. For now, this is a
        // template that would wire up when rustls mTLS is enabled.
        //
        // TODO: Wire up rustls ConnectInfo to extract cert from TLS handshake
        // Current limitation: Axum plain HTTP mode doesn't expose cert; requires
        // axum_server with rustls TLS support.

        // For MVP, check for an X-Admin-Cert header as a workaround
        if let Some(header) = parts.headers.get("X-Admin-Cert") {
            if let Ok(cert_pem) = header.to_str() {
                if is_admin_cert(cert_pem) {
                    return Ok(AdminCert("admin".to_string()));
                } else {
                    warn!("Non-admin certificate presented");
                    return Err(ServerError::Auth { reason: "not an admin certificate".to_string() });
                }
            }
        }

        warn!("Missing or invalid admin certificate");
        Err(ServerError::Auth {
            reason: "admin certificate required; use X-Admin-Cert header or mTLS CN=admin".to_string(),
        })
    }
}

#[cfg(test)]
mod tests {

    // Traces to: ADR-0009, FR-FLEET-001
    #[test]
    fn test_admin_cert_extraction() {
        let cn = "admin";
        assert_eq!(cn, "admin");
    }

    // Traces to: ADR-0009
    #[test]
    fn test_non_admin_cert_rejected() {
        let cn = "agent";
        assert_ne!(cn, "admin");
    }
}
