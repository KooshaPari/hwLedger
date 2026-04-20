# ADR-0009: Fleet mTLS admin authentication via CN extraction

**Date**: 2026-04-19  
**Status**: Accepted  
**Traces to**: WP22, FR-FLEET-001, FR-FLEET-007, ADR-0003

## Problem

The hwLedger fleet server exposes admin-only endpoints (`GET /v1/agents`, `GET /v1/audit`) that list all agents and audit events. These must not be accessible to non-admin users. Initial design (ADR-0003) specifies mTLS for agent↔server transport, but does not differentiate between admin and non-admin certificate holders.

**Current state (MVP)**: All endpoints accept any valid mTLS cert.

## Decision

1. **Admin certs are issued out-of-band** with CN="admin" by the server operator
2. **Admin endpoints validate CN** via X.509 certificate extraction in Rust before handling requests
3. **Non-admin agent certs** carry hostname or UUID as CN; they are rejected on admin endpoints with 403 Forbidden
4. **Regular endpoints** (register, heartbeat, job dispatch) accept any valid cert

## Implementation

### Certificate structure

| Role | CN | Issuer | Validity | Distribution |
|------|-----|--------|----------|--------------|
| Admin | "admin" | Self-signed (MVP) | 90 days | Out-of-band via `mint-admin-cert` subcommand |
| Agent | hostname | Self-signed (MVP) | 30 days | Bootstrap via CSR signing at registration |

### Admin cert generation

New CLI subcommand in `hwledger-server`:

```bash
hwledger-server mint-admin-cert --cn admin --out admin.p12
```

Outputs PKCS#12 bundle (cert + key) for import into agent tools.

### CN extraction & validation

**File**: `crates/hwledger-server/src/cert_extract.rs`

1. `extract_cn_from_pem(pem: &str) -> Option<String>` — Parse X.509 PEM, extract CN from Subject DN
2. `is_admin_cert(pem: &str) -> bool` — Returns true iff CN == "admin"

Uses `x509-parser` crate for robust PEM/DER parsing.

### Route validation

In `crates/hwledger-server/src/routes.rs`:

- `list_agents()` — requires mTLS with CN="admin"
- `get_audit_log()` — requires mTLS with CN="admin"
- Other endpoints (register, heartbeat) — accept any valid cert

When axum mTLS listener is wired in `lib.rs`, request extractors will inject the client cert as an axum::Extension, enabling route handlers to call:

```rust
pub async fn list_agents(
    admin_cert: AdminCertExtension,  // validated CN="admin"
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<Agent>>, ServerError> {
    // Only reachable if CN="admin"
    ...
}
```

### Testing

**Unit tests**: `cert_extract.rs`
- [x] Extract CN from admin cert (CN="admin")
- [x] Extract CN from agent cert (CN="hostname")
- [x] `is_admin_cert()` returns true for admin, false for agent
- [x] `is_admin_cert()` returns false for invalid PEM

**Integration tests**: `tests/integration.rs`
- [ ] Admin endpoint with valid admin cert → 200 OK
- [ ] Admin endpoint with non-admin agent cert → 403 Forbidden
- [ ] Admin endpoint without cert (HTTP) → 401 Unauthorized

### Deferral to v2

- **Full CSR signing**: Currently all certs are self-signed; v2 will use CA-signed certs
- **PKCS#12 export**: MVP returns PEM only; v2 adds PKCS#12 packaging for easy browser/app import
- **Certificate revocation**: No CRL or OCSP in MVP
- **Token-based fallback**: Alternative auth (JWT) deferred to v2

## Rationale

- **CN extraction is standard**: X.509 Subject DN parsing is stable across all cert libraries
- **No new infrastructure**: Reuses existing mTLS TLS/rustls setup
- **Fail-safe**: Invalid certs (malformed PEM, missing CN) are rejected as non-admin
- **Audit trail**: All cert-based access decisions are loggable (future: audit log in ADR-0006)

## Trade-offs

- **Single admin role**: No fine-grained role-based access control (RBAC). v2 can extend to role values in cert extensions.
- **Out-of-band distribution**: Admin must manually manage cert lifecycle (generate, distribute, rotate). v2 can add CA-backed renewal.
- **No revocation**: Admin cert cannot be revoked without re-issuing all certs. v2 adds CRL.

## Acceptance criteria

- [x] CN extraction code is tested
- [x] `mint-admin-cert` generates cert with CN="admin"
- [x] `is_admin_cert()` correctly identifies admin certs
- [x] Routes document CN validation requirements
- [ ] Integration tests verify 403 on non-admin cert (awaits mTLS listener wiring)
- [x] ADR indexed in ADR.md

## References

- ADR-0003: Fleet wire (Axum + mTLS)
- FR-FLEET-001: Agent registration and heartbeat
- FR-FLEET-007: Admin audit log and telemetry access
- [x509-parser docs](https://docs.rs/x509-parser/latest/x509_parser/)
