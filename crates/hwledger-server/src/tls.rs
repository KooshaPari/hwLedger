//! Optional rustls-based mTLS listener with client-certificate extraction.
//!
//! Implements: ADR-0009, FR-FLEET-001
//!
//! When enabled, the server binds a rustls listener that:
//! - Serves a self-signed server certificate minted at boot (rcgen).
//! - Requests (optionally requires) a client certificate on every connection.
//! - Accepts any presented client cert (CN-gated, not chain-verified — matches
//!   the MVP CA posture where agent certs are self-signed).
//! - Captures the first peer certificate DER and exposes it to handlers via a
//!   request extension `PeerCertInfo`, which [`crate::admin_extractor::AdminCert`]
//!   reads to enforce `CN=admin` on privileged endpoints.
//!
//! This complements the `X-Admin-Cert` header MVP path used in plain-HTTP mode.

use std::io;
use std::sync::Arc;

use anyhow::Result;
use axum::middleware::AddExtension;
use axum::Extension;
use axum_server::accept::Accept;
use axum_server::tls_rustls::{RustlsAcceptor, RustlsConfig};
use futures_util::future::BoxFuture;
use rustls::pki_types::{CertificateDer, PrivatePkcs8KeyDer, UnixTime};
use rustls::server::danger::{ClientCertVerified, ClientCertVerifier};
use rustls::server::WebPkiClientVerifier;
use rustls::{DigitallySignedStruct, DistinguishedName, SignatureScheme};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio_rustls::server::TlsStream;
use tower::Layer;
use tracing::{debug, info, warn};

/// Per-connection TLS context injected into request extensions.
///
/// `cert_der` is the DER-encoded first end-entity certificate presented by the
/// client, or `None` if no client cert was offered.
#[derive(Debug, Clone, Default)]
pub struct PeerCertInfo {
    pub cert_der: Option<Vec<u8>>,
}

/// Verifier that accepts any client cert but still validates handshake
/// signatures using the default webpki algorithm set.
///
/// Trust decisions (i.e. "is this the admin CN?") are made downstream in
/// [`crate::admin_extractor::AdminCert`] after the CN is extracted from DER.
#[derive(Debug)]
struct AcceptAnyClientCert {
    subjects: Vec<DistinguishedName>,
    supported: rustls::crypto::WebPkiSupportedAlgorithms,
}

impl AcceptAnyClientCert {
    fn new() -> Self {
        Self {
            subjects: Vec::new(),
            supported: rustls::crypto::ring::default_provider().signature_verification_algorithms,
        }
    }
}

impl ClientCertVerifier for AcceptAnyClientCert {
    fn offer_client_auth(&self) -> bool {
        true
    }

    fn client_auth_mandatory(&self) -> bool {
        // Optional: mTLS admin enforcement still lives in AdminCert extractor,
        // which returns 401 if the extension is missing. Requiring a cert here
        // would break non-admin endpoints that work without mTLS.
        false
    }

    fn root_hint_subjects(&self) -> &[DistinguishedName] {
        &self.subjects
    }

    fn verify_client_cert(
        &self,
        _end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _now: UnixTime,
    ) -> Result<ClientCertVerified, rustls::Error> {
        Ok(ClientCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls12_signature(message, cert, dss, &self.supported)
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls13_signature(message, cert, dss, &self.supported)
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.supported.supported_schemes()
    }
}

/// Build a `rustls::ServerConfig` with an ephemeral server cert and the
/// accept-any client verifier.
///
/// The server cert is regenerated on every boot (matches the MVP CA posture).
/// TLS-ALPN is set to `h2` then `http/1.1` to align with axum-server defaults.
pub fn build_rustls_config() -> Result<RustlsConfig> {
    // Ensure the rustls crypto provider is installed before building a config.
    // Idempotent: install_default() returns Err if already installed — we ignore.
    let _ = rustls::crypto::ring::default_provider().install_default();

    // Generate a self-signed server cert for CN=localhost with SAN=127.0.0.1.
    let mut params =
        rcgen::CertificateParams::new(vec!["localhost".to_string(), "127.0.0.1".to_string()])?;
    params.distinguished_name.push(rcgen::DnType::CommonName, "hwledger-server");
    let key_pair = rcgen::KeyPair::generate()?;
    let cert = params.self_signed(&key_pair)?;

    let cert_der = CertificateDer::from(cert.der().to_vec());
    let key_der = PrivatePkcs8KeyDer::from(key_pair.serialize_der());

    let verifier: Arc<dyn ClientCertVerifier> = Arc::new(AcceptAnyClientCert::new());

    let mut config = rustls::ServerConfig::builder()
        .with_client_cert_verifier(verifier)
        .with_single_cert(vec![cert_der], key_der.into())
        .map_err(|e| anyhow::anyhow!("rustls server config: {e}"))?;
    config.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];

    info!("TLS: generated ephemeral server cert; client auth = optional (CN-gated)");
    // Silence unused-warning on WebPkiClientVerifier (imported for future use).
    let _ = std::any::type_name::<WebPkiClientVerifier>();
    Ok(RustlsConfig::from_config(Arc::new(config)))
}

/// Custom axum-server acceptor that wraps [`RustlsAcceptor`] and injects a
/// [`PeerCertInfo`] extension carrying the client's first end-entity cert.
#[derive(Clone)]
pub struct PeerCertAcceptor {
    inner: RustlsAcceptor,
}

impl PeerCertAcceptor {
    pub fn new(config: RustlsConfig) -> Self {
        Self { inner: RustlsAcceptor::new(config) }
    }
}

impl<I, S> Accept<I, S> for PeerCertAcceptor
where
    I: AsyncRead + AsyncWrite + Unpin + Send + 'static,
    S: Send + 'static,
{
    type Stream = TlsStream<I>;
    type Service = AddExtension<S, PeerCertInfo>;
    type Future = BoxFuture<'static, io::Result<(Self::Stream, Self::Service)>>;

    fn accept(&self, stream: I, service: S) -> Self::Future {
        let acceptor = self.inner.clone();
        Box::pin(async move {
            let (stream, service) = acceptor.accept(stream, service).await?;
            let cert_der = stream
                .get_ref()
                .1
                .peer_certificates()
                .and_then(|certs| certs.first())
                .map(|c| c.as_ref().to_vec());
            match &cert_der {
                Some(der) => debug!("TLS: captured client cert ({} bytes)", der.len()),
                None => warn!("TLS: no client certificate presented on this connection"),
            }
            let ext = PeerCertInfo { cert_der };
            let service = Extension(ext).layer(service);
            Ok((stream, service))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_peer_cert_info_default() {
        let info = PeerCertInfo::default();
        assert!(info.cert_der.is_none());
    }

    #[test]
    fn test_build_rustls_config_ok() {
        let cfg = build_rustls_config();
        assert!(cfg.is_ok(), "rustls config should build");
    }

    #[test]
    fn test_accept_any_verifier_offers_auth() {
        let v = AcceptAnyClientCert::new();
        assert!(v.offer_client_auth());
        assert!(!v.client_auth_mandatory());
        assert!(v.root_hint_subjects().is_empty());
    }
}
