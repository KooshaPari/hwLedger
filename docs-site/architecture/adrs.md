# Architecture Decision Records

All significant architecture decisions are recorded as ADRs. See the detailed documents below for rationale, alternatives, and trade-offs.

## Current ADRs

| ID | Title | Status | Decided | Impact |
|-----|-------|--------|---------|--------|
| [0001](./adrs/0001-rust-core-three-native-guis.md) | Rust core with three native GUIs (SwiftUI, WinUI 3, Qt 6) | Accepted | 2026-04-18 | FFI approach, platform coverage |
| [0002](./adrs/0002-oMlx-fat-fork.md) | oMlx fat fork for SSD-paged KV cache on macOS | Accepted | 2026-04-18 | Inference backend, performance baseline |
| [0003](./adrs/0003-fleet-wire-axum-not-grpc.md) | Axum + mTLS for fleet wire (not gRPC) | Accepted | 2026-04-18 | Fleet transport, scalability ceiling |
| [0004](./adrs/0004-math-core-dispatch.md) | Math core dispatch per AttentionKind | Accepted | 2026-04-18 | Accuracy, architecture support |
| [0005](./adrs/0005-shared-crate-reuse.md) | Shared crate reuse across Phenotype org | Accepted | 2026-04-18 | Code organization, dependency strategy |
| [0007](./adrs/0007-ffi-raw-c-over-uniffi.md) | Raw C FFI over UniFFI (WinUI/Qt fallback) | Accepted | 2026-04-18 | Windows/Linux FFI approach |

## Decision Process

Each ADR includes:

- **Context**: Problem statement and constraints
- **Decision**: What was chosen and why
- **Rationale**: Trade-offs and alternatives considered
- **Consequences**: Impacts on the system and team

## ADR Index by Category

### Runtime & Inference

- [ADR-0002: oMlx fat fork for SSD-paged KV cache](./adrs/0002-oMlx-fat-fork.md)

### Architecture Patterns

- [ADR-0001: Rust core with three native GUIs](./adrs/0001-rust-core-three-native-guis.md)
- [ADR-0004: Math core dispatch per AttentionKind](./adrs/0004-math-core-dispatch.md)
- [ADR-0005: Shared crate reuse](./adrs/0005-shared-crate-reuse.md)

### Transport & Communication

- [ADR-0003: Axum + mTLS for fleet wire](./adrs/0003-fleet-wire-axum-not-grpc.md)

### FFI & Language Bindings

- [ADR-0007: Raw C FFI over UniFFI for WinUI/Qt](./adrs/0007-ffi-raw-c-over-uniffi.md)

## Future ADRs

Planned decisions for upcoming phases:

- **ADR-0006**: Query language for fleet dispatch (planned)
- **ADR-0008**: Cost model and spot pricing integration (planned)
- **ADR-0009**: Offline-first sync strategy for fleet nodes (planned)
