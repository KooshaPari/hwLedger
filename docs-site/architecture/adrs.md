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
| [0006](./adrs/0006-macos-codesign-notarize-sparkle.md) | macOS Developer ID + Notarization + Sparkle | Accepted | 2026-04-19 | macOS distribution |
| [0007](./adrs/0007-ffi-raw-c-over-uniffi.md) | Raw C FFI over UniFFI (WinUI/Qt fallback) | Accepted | 2026-04-18 | Windows/Linux FFI approach |
| [0008](./adrs/0008-wp21-deferred-pending-apple-dev.md) | WP21 deferred pending Apple Dev enrollment | Accepted | 2026-04-19 | Release sequencing |
| [0009](./adrs/0009-fleet-mtls-admin-cn.md) | Fleet mTLS admin auth via CN extraction | Accepted | 2026-04-19 | Admin endpoint security |
| [0010](./adrs/0010-tts-backend-piper.md) | TTS backend: Piper default, ElevenLabs paid | Accepted | 2026-04-19 | Narration pipeline |
| [0011](./adrs/0011-video-compositing-remotion.md) | Video compositing: Remotion | Accepted | 2026-04-19 | Journey video pipeline |
| [0012](./adrs/0012-terminal-recording-vhs.md) | Terminal recording: VHS | Accepted | 2026-04-19 | CLI capture tooling |
| [0013](./adrs/0013-browser-automation-playwright.md) | Browser automation: Playwright | Accepted | 2026-04-19 | GUI capture tooling |
| [0014](./adrs/0014-ocr-apple-vision-tesseract-claude.md) | OCR: Apple Vision + tesseract + Claude hybrid | Accepted | 2026-04-19 | Attestation grounding |
| [0015](./adrs/0015-vlm-judge-claude.md) | VLM judge: Claude default, local fallback | Accepted | 2026-04-19 | Journey judge + captions |
| [0016](./adrs/0016-ssh-client-russh.md) | SSH client: russh (native Rust) | Accepted | 2026-04-19 | Fleet transport |
| [0017](./adrs/0017-x509-rcgen.md) | X.509 generation: rcgen | Accepted | 2026-04-19 | Cert generation |
| [0018](./adrs/0018-http-server-axum.md) | HTTP server: axum | Accepted | 2026-04-19 | Confirms ADR-0003 |
| [0019](./adrs/0019-db-sqlx-sqlite.md) | Database: sqlx + SQLite | Accepted | 2026-04-19 | Persistence |
| [0020](./adrs/0020-ffi-cbindgen-silgen-name.md) | FFI: cbindgen + `@_silgen_name` | Accepted | 2026-04-19 | Multi-language bindings |
| [0021](./adrs/0021-cross-platform-desktop-stacks.md) | Desktop stacks: SwiftUI + electrobun/Tauri | Accepted | 2026-04-19 | Cross-platform UI |
| [0022](./adrs/0022-windows-native-stack.md) | Windows: WinUI 3 + C#; windows-app-rs deferred | Accepted | 2026-04-19 | Windows path |
| [0023](./adrs/0023-macos-gpu-telemetry.md) | macOS GPU telemetry: IOKit + IOReport | Accepted | 2026-04-19 | Telemetry stack |
| [0024](./adrs/0024-attestation-ed25519.md) | Attestation signing: ed25519 | Accepted | 2026-04-19 | Signature scheme |
| [0025](./adrs/0025-journey-manifest-json-schemars.md) | Journey manifest: JSON + schemars | Accepted | 2026-04-19 | Manifest format |
| [0026](./adrs/0026-web-framework-streamlit.md) | Internal web: Streamlit | Accepted | 2026-04-19 | Ops dashboards |
| [0027](./adrs/0027-charts-plotly.md) | Charts: Plotly (single standard) | Accepted | 2026-04-19 | Visualization |
| [0028](./adrs/0028-cli-parser-clap.md) | CLI parser: clap derive | Accepted | 2026-04-19 | CLI ergonomics |
| [0029](./adrs/0029-python-packaging-uv.md) | Python packaging: uv | Accepted | 2026-04-19 | Python toolchain |
| [0030](./adrs/0030-package-registry-github-packages.md) | Package registry: GitHub Packages | Accepted | 2026-04-19 | Publishing |
| [0031](./adrs/0031-ci-hybrid-local-self-hosted.md) | CI: lefthook + self-hosted runner | Accepted | 2026-04-19 | Verification pipeline |
| [0032](./adrs/0032-keyframe-extraction-ffmpeg.md) | Keyframe extraction: ffmpeg I-frame + 1 fps | Accepted | 2026-04-19 | Video sampling |
| [0033](./adrs/0033-file-watcher-notify.md) | File-watcher: notify (Rust) | Accepted | 2026-04-19 | Hot-reload |

## Decision Process

Each ADR includes:

- **Context**: Problem statement and constraints
- **Options**: Table of alternatives with pros/cons
- **Decision**: What was chosen
- **Rationale**: Why
- **Consequences**: Trade-offs inherited
- **Revisit when**: Triggers to re-open the decision

## ADR Index by Category

### Architecture Patterns
- [ADR-0001](./adrs/0001-rust-core-three-native-guis.md), [ADR-0004](./adrs/0004-math-core-dispatch.md), [ADR-0005](./adrs/0005-shared-crate-reuse.md)

### FFI & Language Bindings
- [ADR-0007](./adrs/0007-ffi-raw-c-over-uniffi.md), [ADR-0020](./adrs/0020-ffi-cbindgen-silgen-name.md)

### UI & Desktop Stacks
- [ADR-0021](./adrs/0021-cross-platform-desktop-stacks.md), [ADR-0022](./adrs/0022-windows-native-stack.md)

### Transport & Communication
- [ADR-0003](./adrs/0003-fleet-wire-axum-not-grpc.md), [ADR-0016](./adrs/0016-ssh-client-russh.md), [ADR-0018](./adrs/0018-http-server-axum.md)

### Security & Attestation
- [ADR-0009](./adrs/0009-fleet-mtls-admin-cn.md), [ADR-0017](./adrs/0017-x509-rcgen.md), [ADR-0024](./adrs/0024-attestation-ed25519.md)

### Data & Persistence
- [ADR-0019](./adrs/0019-db-sqlx-sqlite.md), [ADR-0025](./adrs/0025-journey-manifest-json-schemars.md)

### Runtime & Inference
- [ADR-0002](./adrs/0002-oMlx-fat-fork.md), [ADR-0023](./adrs/0023-macos-gpu-telemetry.md)

### Journey capture & media
- [ADR-0010](./adrs/0010-tts-backend-piper.md), [ADR-0011](./adrs/0011-video-compositing-remotion.md), [ADR-0012](./adrs/0012-terminal-recording-vhs.md), [ADR-0013](./adrs/0013-browser-automation-playwright.md), [ADR-0014](./adrs/0014-ocr-apple-vision-tesseract-claude.md), [ADR-0015](./adrs/0015-vlm-judge-claude.md), [ADR-0032](./adrs/0032-keyframe-extraction-ffmpeg.md)

### Tooling, Ops & Distribution
- [ADR-0006](./adrs/0006-macos-codesign-notarize-sparkle.md), [ADR-0008](./adrs/0008-wp21-deferred-pending-apple-dev.md), [ADR-0026](./adrs/0026-web-framework-streamlit.md), [ADR-0027](./adrs/0027-charts-plotly.md), [ADR-0028](./adrs/0028-cli-parser-clap.md), [ADR-0029](./adrs/0029-python-packaging-uv.md), [ADR-0030](./adrs/0030-package-registry-github-packages.md), [ADR-0031](./adrs/0031-ci-hybrid-local-self-hosted.md), [ADR-0033](./adrs/0033-file-watcher-notify.md)
