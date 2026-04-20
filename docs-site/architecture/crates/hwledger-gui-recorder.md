---
title: hwledger-gui-recorder
description: Journey recorder: captures GUI interactions (FFmpeg video + event manifest JSON)...
---

# hwledger-gui-recorder

**Role.** Journey recorder: captures GUI interactions (FFmpeg video + event manifest JSON) for documentation and regression testing.

## Public API surface

| Type | Name | Stability |
|------|------|-----------|
| mod | `error` | stable |
| mod | `ffmpeg` | stable |
| mod | `manifest` | stable |
| mod | `recorder` | stable |
| mod | `sck_bridge` | stable |
| struct | `JourneyRecorder` | stable |
| fn | `new` | stable |

## Dependencies

Top workspace and external dependencies:

| Dependency | Purpose | Workspace |
|------------|---------|-----------|
| `tokio` | Core logic | No |
| `serde` | Core logic | No |
| `serde_json` | Core logic | No |
| `thiserror` | Core logic | No |
| `tracing` | Core logic | No |
| `tracing-subscriber` | Core logic | No |

## Consumers

- - `docs-site`
- `ci-tests`

## Design notes

- Standalone crate: minimal inter-crate dependencies, composable with other inference backends
- Error handling via `thiserror` with custom error types
- Full async/await support via `tokio` where applicable
- All public types implement `Debug` and `Clone`
- Serialization via `serde` for config and wire protocol

## Example usage

```rust
use hwledger_gui_recorder::*;

// Initialize and call main API
```

## Related

- [Source on GitHub](https://github.com/KooshaPari/hwLedger/tree/main/crates/hwledger-gui-recorder)
- [ADR-0001: Rust Core Architecture](/architecture/adrs/0001-rust-core-three-native-guis)
- [ADR-0005: Shared Crate Reuse](/architecture/adrs/0005-shared-crate-reuse)
