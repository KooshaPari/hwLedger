# ADR 0024 — Attestation signing: ed25519

Constrains: FR-JOURNEY-007, FR-FLEET-011

Date: 2026-04-19
Status: Accepted

## Context

Every journey render and every fleet audit event is signed. Signatures must be fast enough to not bottleneck per-frame attestation (ADR-0015), small enough to embed in JSON manifests (ADR-0025), and future-proof against obvious pitfalls (small-subgroup attacks, malleability, signer-forgery).

## Options

| Option | Sig size | Verify cost | Library | PQC-safe | Keygen determinism |
|---|---|---|---|---|---|
| ed25519-dalek 2.1 | 64 B | ~40 µs | `ed25519-dalek` | No | Deterministic |
| ECDSA P-256 | 64–72 B (DER) | ~60 µs | `p256` (RustCrypto) | No | Non-deterministic (needs RNG) |
| RSA-4096 | 512 B | ~150 µs | `rsa` (RustCrypto) | No | N/A |
| minisign (Ed25519 prehash) | 104 B | ~40 µs | `minisign-verify` | No | Deterministic |
| GPG / OpenPGP | >500 B | Slow | `sequoia` | No | Deterministic (if Ed25519) |
| ML-DSA / Dilithium | ~2.5 KB | ~80 µs | `pqcrypto-dilithium` | Yes | Deterministic |
| SLH-DSA / SPHINCS+ | ~8 KB | Slow | `pqcrypto-sphincsplus` | Yes | Deterministic |

## Decision

Use **ed25519-dalek 2.1** for all attestation signing (journey manifests, fleet audit events, admin cert signing in ADR-0017).

## Rationale

- Smallest + fastest signature in the non-PQ ecosystem, with RustCrypto-maintained implementation that's been audited.
- Deterministic signing eliminates RNG failure modes in CI containers.
- 64-byte signatures embed cleanly in JSON as base64 (88 chars) without ballooning manifest size.
- Minisign offers the same crypto with more envelope; we keep ed25519-dalek because we do our own envelope (ADR-0025).

## Consequences

- No post-quantum protection. Accepted through ~2030 per NIST PQC migration guidance (PQC is required for long-lived archival signatures, not MVP audit logs). Revisit trigger below.
- Key rotation requires a re-signing sweep over the audit log. Mitigated by a monthly rotation window with both old + new signers valid.

## Revisit when

- ML-DSA (FIPS-204) has stable Rust implementations in RustCrypto or BoringSSL, with ≥1-year of production use elsewhere.
- Any journey artifact becomes long-lived (>5 yr retention) — PQC becomes mandatory sooner for those.
- ed25519-dalek is deprecated (unlikely; RustCrypto is stable).

## References

- ed25519-dalek: https://github.com/dalek-cryptography/curve25519-dalek/tree/main/ed25519-dalek
- NIST PQC: https://csrc.nist.gov/projects/post-quantum-cryptography
- ADR-0017 (rcgen), ADR-0025 (manifest).
