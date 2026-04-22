# Recording Backends

`hwledger-journey-record` is the per-OS capture orchestrator used by the GUI
and web journey harnesses. XCUITest (macOS) and Playwright (web) still drive
UI events; this tool owns only the capture layer.

```bash
hwledger-journey-record \
  --target {app-bundle-id|browser-url|pid} \
  --output <path>.mp4 \
  --duration <sec> \
  --backend {scsk|xvfb|winrdp|auto} \
  [--virtual-cursor] [--headless] [--sandbox]
```

## Feature matrix

| Flag / backend        | `scsk` (macOS)                         | `xvfb` (Linux)             | `winrdp` (Windows)                     |
|-----------------------|----------------------------------------|----------------------------|----------------------------------------|
| Window-scoped capture | Full — `SCContentFilter` on bundle-id  | Stub                       | Stub                                   |
| MP4 muxing            | AVFoundation (`AVAssetWriter`, H.264)  | `ffmpeg -f x11grab`        | `windows-capture` MF encoder           |
| `--virtual-cursor`    | Partial — hook present, overlay TODO   | Stub                       | Stub                                   |
| `--headless`          | Partial — single-display fallback logs | Stub                       | Stub                                   |
| `--sandbox`           | Partial — `sandbox-exec` profile TODO  | `bubblewrap` (planned)     | Windows Sandbox (planned)              |
| Overall status        | **plan-mode** (bails loudly)           | **stub — bails loudly**    | **stub — bails loudly**                |

## macOS (`scsk`)

Wraps the existing `hwledger-gui-recorder` crate, which links against the
Swift static library in
`crates/hwledger-gui-recorder/swift-sck/Sources/SckBridge/SckBridge.swift`.

Install requirements:

* macOS 14+ (ScreenCaptureKit is in the base SDK).
* Grant the recording host binary TCC **Screen Recording** permission on
  first use.
* `swift` toolchain in PATH for the SCK static-lib build (already part of the
  workspace build).

No `ffmpeg` subprocess is spawned; H.264 muxing happens inside AVFoundation.

> **Plan-mode note.** The Swift `SckBridge` static lib is not yet linked into
> the `hwledger-journey-record` binary (no `[[bin]]` target on
> `hwledger-gui-recorder` consumes it yet). Until that wiring lands, the
> `scsk` backend emits the capture plan via `tracing` and then bails with a
> clear error — no silent fallback. The follow-up is tracked in the TODO
> header of `tools/journey-record/src/backends/scsk.rs`.

## Linux (`xvfb`) — stub

Planned pipeline (see `tools/journey-record/src/backends/xvfb.rs` TODO):

1. `Xvfb :<auto-pick> -screen 0 1440x900x24 -ac -nolisten tcp`.
2. Target spawned with `DISPLAY=:<n>`, wrapped by
   `bwrap --unshare-all --die-with-parent --uid 1000 ...` when `--sandbox`.
3. `ffmpeg -f x11grab -video_size 1440x900 -framerate 30 -i :<n> \
             -c:v libx264 -preset ultrafast -pix_fmt yuv420p -y <output>`.
4. Virtual cursor via `xdotool` + ffmpeg `movie`/`overlay` filtergraph.

Install requirements (when implemented): `xvfb`, `ffmpeg`, `bubblewrap`,
`xdotool`. All resolved via `which` with loud failures — no silent
degradation.

## Windows (`winrdp`) — stub

Planned pipeline (see `tools/journey-record/src/backends/winrdp.rs` TODO):

1. Generate a per-run `.wsb` Windows Sandbox config (no host folder mounts,
   own virtual disk).
2. `mstsc /v:localhost` into the sandbox for session isolation when
   `--headless`.
3. Capture via the `windows-capture` Rust crate (`Windows.Graphics.Capture`
   + `GraphicsCaptureItem`) targeting window by PID or title.
4. MP4 muxing via `windows-capture`'s built-in Media Foundation encoder.
5. Virtual cursor via `SendInput` with absolute / virtual-desktop flags.

Install requirements (when implemented): Windows Pro/Enterprise with
Windows Sandbox feature enabled, Rust `windows-capture` crate (vendored via
`Cargo.lock`).

## Harness integration

* `apps/macos/HwLedgerUITests/scripts/run-journeys.sh` → invokes
  `hwledger-journey-record --backend auto --virtual-cursor --headless`
  for the capture layer; XCUITest continues to drive UI events.
* `apps/streamlit/journeys/` → invokes `--backend auto` from Playwright
  fixtures; Playwright drives events, journey-record captures.

## Smoke test

macOS only (Linux/Windows have no runners here):

```bash
cargo test -p hwledger-journey-record --test smoke_macos -- --ignored --nocapture
```

Records 3 s of `com.apple.finder` with `--virtual-cursor`; asserts MP4
exists, size > 50 KB, and (if `ffprobe` is available) duration is in
[2 s, 5 s].
