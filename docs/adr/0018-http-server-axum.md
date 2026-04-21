# ADR 0018 — HTTP server framework: axum

Constrains: FR-FLEET-001..007, ADR-0003

Date: 2026-04-19
Status: Accepted

## Context

ADR-0003 decided on Axum + JSON/HTTPS + mTLS over gRPC for the fleet wire. This ADR records the framework choice itself — why axum beats actix-web, warp, rocket, and others — since that decision was made inline in ADR-0003 and deserves its own record.

## Options

| Option | Async model | Extractor ergonomics | tower compat | mTLS integration | Perf | Maintainer |
|---|---|---|---|---|---|---|
| axum 0.7 | tokio | Excellent (type-level extractors) | Native | via rustls + tower | High | tokio-rs |
| actix-web 4 | actix | Good (FromRequest) | No (own middleware) | via rustls | Highest | Nikolay Kim |
| warp 0.3 | tokio | Filters (functional) | Partial | via rustls | High | Stalled |
| rocket 0.5 | tokio | Macros | No | Limited | Medium | Sergio Benitez |
| salvo 0.70 | tokio | Middleware trait | Partial | Yes | High | Chinese community |
| poem 3 | tokio | Endpoint trait | Partial | Yes | High | Sun Rui |

## Decision

**axum 0.7** for `hwledger-server`, `hwledger-docs-mcp-server`, and any future HTTP endpoints in the workspace.

## Rationale

- Native `tower` integration means we get retries, timeouts, rate limits, and tracing spans for free via the crates we already use elsewhere (`tower-http`, `tracing`).
- Type-level extractors compose cleanly with the cert-extraction pattern in ADR-0009 (`AdminCertExtension`).
- Owned by the tokio team — first-class async primitive alignment; no risk of runtime mismatch.
- Benchmarks put axum within 10% of actix-web for our workload (JSON in/out); not the bottleneck.
- actix-web's own middleware stack diverges from tower, which would fragment the workspace.

## Consequences

- Locked to tokio runtime. No async-std fallback. Accepted: the whole workspace is tokio.
- Type errors on extractor misconfiguration can be intimidating; mitigated by canonical handler signatures in code review.

## Revisit when

- A Rust HTTP framework demonstrably outperforms axum by >2× on our workload.
- tower maintainership changes substantially.

## References

- axum: https://github.com/tokio-rs/axum
- ADR-0003 (fleet wire), ADR-0009 (mTLS admin CN).
