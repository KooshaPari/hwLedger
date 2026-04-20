---
title: hwledger-probe
description: "GPU telemetry: abstracts NVIDIA (NVML), AMD (rocm-smi), Apple (metal-rs), Intel,..."
---

# hwledger-probe

**Role.** GPU telemetry: abstracts NVIDIA (NVML), AMD (rocm-smi), Apple (metal-rs), Intel, and system-level (sysinfo) GPU discovery.

## Public API surface

| Type | Name | Stability |
|------|------|-----------|
| mod | `nvidia` | stable |
| mod | `amd` | stable |
| mod | `cache` | stable |
| mod | `metal` | stable |
| mod | `intel` | stable |
| struct | `Device` | stable |
| enum | `ProbeError` | stable |
| trait | `GpuProbe` | stable |

## Dependencies

Top workspace and external dependencies:

| Dependency | Purpose | Workspace |
|------------|---------|-----------|

## Consumers

- - `hwledger-core`
- `hwledger-cli`
- `hwledger-server`

## Design notes

- Standalone crate: minimal inter-crate dependencies, composable with other inference backends
- Error handling via `thiserror` with custom error types
- Full async/await support via `tokio` where applicable
- All public types implement `Debug` and `Clone`
- Serialization via `serde` for config and wire protocol

## Example usage

```rust
use hwledger_probe::*;

// Initialize and call main API
```

## Related

- [Source on GitHub](https://github.com/KooshaPari/hwLedger/tree/main/crates/hwledger-probe)
- [ADR-0001: Rust Core Architecture](/architecture/adrs/0001-rust-core-three-native-guis)
- [ADR-0005: Shared Crate Reuse](/architecture/adrs/0005-shared-crate-reuse)
