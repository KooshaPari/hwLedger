# GUI Recording Ecosystem Inventory

**Investigation Date:** 2026-04-18
**Scope:** Screen capture, video recording, keyframe extraction, and demo automation across KooshaPari ecosystem

## Summary

Existing infrastructure for GUI recording spans three primary sources:

1. **hwLedger itself** — placeholder Swift ScreenRecorder + ffmpeg keyframe extraction scripts
2. **KDesktopVirt** — comprehensive ffmpeg pipeline for desktop recording with hardware acceleration
3. **hwledger-release crate** — VHS tape orchestration + ffmpeg keyframe/manifest generation

No blocking duplications; all can be cleanly reused or adapted.

## Detailed Inventory

### 1. hwLedger GUI Recording (Local, macOS)

**Files:**
- `apps/macos/HwLedgerUITests/Sources/Harness/ScreenRecorder.swift` — ScreenCaptureKit placeholder (AVAssetWriter bridge)
- `apps/macos/HwLedgerUITests/scripts/extract-keyframes.sh` — ffmpeg extraction + GIF generation
- `apps/cli-journeys/scripts/extract-keyframes.sh` — identical copy (deduplication candidate)

**What it does:**
- ScreenRecorder.swift: Exposes `startRecording(appIdentifier:)` and `stopRecording()` async methods
  - Uses SCStream + SCStreamConfiguration for 1440×900 H.264 capture
  - Delegates to AVAssetWriter for MP4 output
  - Gracefully handles TCC denial (permission gating)
  - Status: ~80% complete; missing SCStreamDelegate frame delivery handler

- extract-keyframes.sh:
  - Extracts I-frames via `ffmpeg -vf "select='eq(pict_type,I)'"` (true keyframes)
  - Falls back to steady sampling (1 fps) if <3 I-frames found
  - Generates optimized GIF preview at 10 fps with palette-based dithering
  - Output: `keyframe-*.png` + `preview.gif`

**Language:** Swift + Bash
**License:** Implicit (hwLedger repo, likely Apache-2.0)
**Verdict:** **REUSE + COMPLETE** — Extend the placeholder Swift impl; wrap ffmpeg calls in Rust crate

---

### 2. hwledger-release Crate (FFmpeg Orchestration)

**File:** `crates/hwledger-release/src/keyframes.rs`

**What it does:**
- `extract_keyframes(tape_path, output_dir)` — invokes ffmpeg subprocess for I-frame extraction
  - Uses `ffmpeg -i <input> -vf "select=eq(pict_type\,I)" -vsync 0 frame_%04d.png`
  - Wraps via `ReleaseCommand` (timeout, logging, error handling)
  - No GIF generation here; minimal scope

- `generate_manifest(tape_id, keyframes_dir, output_manifest)` — writes JSON manifest stub
  - Current impl: static JSON template (timestamps not yet integrated)

- Also includes `record_tape(tape_path)` in record.rs for VHS orchestration

**Language:** Rust
**License:** hwLedger (Apache-2.0)
**Verdict:** **REUSE + EXTEND** — Import subprocess patterns; enhance manifest with timestamp data

---

### 3. KDesktopVirt Recording Pipeline (Linux/X11)

**Files:**
- `src/recording_pipeline.rs` — enterprise-grade ffmpeg pipeline (3,000+ LOC)
- `src/ffmpeg_pipeline.rs` — detailed encoder config with hardware acceleration
- `src/recording.rs` — low-level recording state machine
- `automation_scripts/*.sh` — demo orchestration with smooth cursor tracking

**What it does:**
- Comprehensive RecordingPipeline struct with:
  - VideoEncoder — hardware acceleration (NVENC, Quick Sync, AMF)
  - AudioProcessor — virtual audio device integration
  - StreamingServer — WebRTC/RTMP streaming
  - QualityController — dynamic bitrate, FPS, resolution control
  - FormatConverter — MP4, WebM, GIF, MKV, FLV support
- Supports 60fps, 1080p/4K encoding
- Real-time monitoring and quality adaptation

**Language:** Rust
**License:** KDesktopVirt repo (presumed open)
**Verdict:** **REFERENCE ONLY** — Overkill for macOS single-window recording; contains valuable patterns for multi-format output

---

### 4. Bash Keyframe Extraction Scripts

**Files:**
- `hwLedger/apps/macos/HwLedgerUITests/scripts/extract-keyframes.sh`
- `hwLedger/apps/cli-journeys/scripts/extract-keyframes.sh` (duplicate)
- Various in KDesktopVirt for demo automation

**What it does:**
- Simple, proven ffmpeg invocations
- Graceful fallback (I-frames → steady sampling)
- GIF generation with palette optimization

**Verdict:** **CODIFY IN RUST** — wrap as subprocess calls in hwledger-gui-recorder crate

---

## Reuse Decision Matrix

| Component | Source | Verdict | Action |
|-----------|--------|---------|--------|
| ScreenCaptureKit bridge | hwLedger ScreenRecorder.swift | Reuse + complete | Finish SCStreamDelegate implementation |
| I-frame extraction | hwledger-release/keyframes.rs | Reuse + extend | Import subprocess pattern; enhance manifest |
| GIF generation | extract-keyframes.sh | Codify in Rust | Wrap ffmpeg subprocess with filter args |
| Quality config | KDesktopVirt recording_pipeline.rs | Reference | Adapt resolution/fps/codec patterns (not needed for MVP) |
| Manifest schema | hwledger-release + custom | Create | Journey manifest with timestamps, frame metadata |
| Permission handling | hwLedger ScreenRecorder.swift | Reuse | CGPreflightScreenCaptureAccess check |

## Architecture Recommendation

**New crate: `hwledger-gui-recorder`**

```
crates/hwledger-gui-recorder/
├── Cargo.toml (Rust FFI + subprocess deps)
├── src/
│   ├── lib.rs              (pub api: start_recording, stop_recording)
│   ├── sck_bridge.rs       (FFI glue to Swift static lib)
│   ├── ffmpeg.rs           (subprocess wrapper: keyframes, gif, convert)
│   ├── manifest.rs         (journey manifest serialization)
│   └── error.rs            (RecorderError enum)
├── swift-sck/
│   ├── Package.swift       (Swift Package for ScreenCaptureKit)
│   └── Sources/SckBridge/
│       ├── SckBridge.swift (@_cdecl exports for FFI)
│       └── Recorder.swift  (SCStream + AVAssetWriter impl)
└── tests/
    ├── integration_test.rs (E2E: record → extract → verify)
```

**No duplication:** Dedup `extract-keyframes.sh` files → single Rust impl.

**Swift ScreenRecorder status:** Already 80% complete; finish SCStreamDelegate frame delivery.

## Open Questions

1. Do we need hardware acceleration? (No — H.264 software codec sufficient for 1440×900 @ 30fps)
2. Multiple output formats? (No — MP4 + GIF sufficient for embedding)
3. Real-time metrics / quality adaptation? (No — fixed 1440×900, 30fps)
4. Audio capture? (No — GUI journeys are visual-only)

## Notarization / Entitlements

ScreenCaptureKit requires:
- macOS 14+ (same as hwLedger app)
- `com.apple.security.device.camera` — false (not used)
- **No explicit entitlement required; system prompts for Screen Recording permission on first use**
- Programmatic check: `CGPreflightScreenCaptureAccess()` returns `true` if permission granted

## Follow-ups Post-MVP

1. Integrate with XCUITest AppDriver harness (wire manifest timestamps to UI events)
2. Embed `RecordingEmbed` Vue component in docs-site for preview
3. Parallel recording for multiple journeys (use hwledger-release pattern: Arc<Semaphore>)
4. Hardware acceleration for 4K recording (future, if needed)
