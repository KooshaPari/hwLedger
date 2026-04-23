# tools/vlm-judge — DEPRECATED (renamed)

This directory was renamed to [`tools/frame-describer/`](../frame-describer/)
on 2026-04-22 per [ADR-0038](../../docs-site/architecture/adrs/0038-frame-describer-two-stage.md).

Why the rename: the component does *frame description* (blind per-step
captioning) and only scores agreement as a side effect — "judge" bakes in
obsolete single-VLM-as-ground-truth framing. See ADR-0038 §Decision.

## Migration

| Old | New |
|-----|-----|
| `cargo run -p hwledger-vlm-judge` | `cargo run -p hwledger-frame-describer` |
| `tools/vlm-judge/src/main.rs` | `tools/frame-describer/src/main.rs` |
| binary `hwledger-vlm-judge` | binary `hwledger-frame-describer` |

## Back-compat alias

The `vlm-judge` script in this directory is a thin shim that `exec`s the new
binary, so existing scripts/CI that call `./tools/vlm-judge/vlm-judge` keep
working. This shim will be removed after all known consumers migrate
(tracked in the ADR-0038 deprecation note).
