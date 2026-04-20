---
title: hwledger-verify
description: "Cryptographic verification: validates inference results, ledger hash chains, and..."
---

# hwledger-verify

**Role.** Cryptographic verification: validates inference results, ledger hash chains, and audit trail integrity.

## Public API surface

| Type | Name | Stability |
|------|------|-----------|
| mod | `cache` | stable |
| mod | `client` | stable |
| struct | `VerifierConfig` | stable |
| fn | `with_api_key` | stable |
| fn | `with_describe_model` | stable |
| fn | `with_judge_model` | stable |
| fn | `with_base_url` | stable |
| fn | `with_cache_disabled` | stable |

## Dependencies

Top workspace and external dependencies:

| Dependency | Purpose | Workspace |
|------------|---------|-----------|
| `base64` | Core logic | No |
| `clap` | Core logic | No |

## Consumers

- - `hwledger-core`
- `hwledger-server`

## Design notes

- Standalone crate: minimal inter-crate dependencies, composable with other inference backends
- Error handling via `thiserror` with custom error types
- Full async/await support via `tokio` where applicable
- All public types implement `Debug` and `Clone`
- Serialization via `serde` for config and wire protocol

## Example usage

```rust
use hwledger_verify::*;

// Initialize and call main API
```

## Related

- [Source on GitHub](https://github.com/KooshaPari/hwLedger/tree/main/crates/hwledger-verify)
- [ADR-0001: Rust Core Architecture](/architecture/adrs/0001-rust-core-three-native-guis)
- [ADR-0005: Shared Crate Reuse](/architecture/adrs/0005-shared-crate-reuse)
