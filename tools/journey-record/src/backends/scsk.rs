//! macOS `ScreenCaptureKit` backend.
//!
//! Window-scoped per-bundle-id capture via Apple's `ScreenCaptureKit` and
//! `AVAssetWriter`. The Swift implementation already lives in
//! `crates/hwledger-gui-recorder/swift-sck/Sources/SckBridge/SckBridge.swift`
//! — it does `SCShareableContent.current` → filter by bundle-id →
//! `SCContentFilter` → `SCStream` @ 1440×900 → H.264 MP4 via
//! `AVAssetWriter` (no ffmpeg subprocess).
//!
//! This backend drives that bridge, plus:
//!
//! * `--virtual-cursor` — synthetic cursor sprite composited into the stream
//!   via a `CoreGraphics` overlay pass in `StreamDelegate`; the user's real
//!   OS cursor is never captured.
//! * `--headless` — target app moved to an off-screen `NSScreen` / virtual
//!   display (falls back with a clear warning on single-display hosts).
//! * `--sandbox` — target spawned under `sandbox-exec -p <profile>` with a
//!   generated `.sb` profile scoped to the journey temp dir + the app's
//!   container.
//!
//! ## Current wiring status
//!
//! The Swift static library exists and the Rust FFI shim (`sck_bridge.rs` in
//! `hwledger-gui-recorder`) is in place, but the workspace has no linker
//! step that produces a binary with the Swift symbols resolved — the
//! `[[bin]]` target on `hwledger-gui-recorder` is currently commented out
//! and its `build.rs` only runs `cbindgen`, not `swift build`. Until that
//! link path is finalized, this backend runs in "plan" mode: it validates
//! the request, emits a structured log describing the capture plan, and
//! bails with a clear, actionable error that names the missing wiring step.
//! This preserves the "fail loudly" contract (see global "Optionality and
//! Failure Behavior" policy).
//!
//! On non-macOS hosts it additionally refuses to run.
//!
//! TODO(journey-record): wire the Swift static lib so this backend can flip
//! from "plan" mode to "record" mode. Work:
//!
//! 1. Add `[[bin]]` target to `hwledger-gui-recorder` (or to this crate)
//!    that links `libSckBridge.a` from `swift build -c release`.
//! 2. Extend `build.rs` to invoke `swift build --package-path swift-sck`
//!    and emit `cargo:rustc-link-search` + `cargo:rustc-link-lib=static=SckBridge`.
//! 3. Flip `PLAN_ONLY` below to `false` (or remove) and un-guard the
//!    `hwledger-gui-recorder` path-dep in `Cargo.toml`.

use anyhow::{bail, Result};

use crate::RecordRequest;

const PLAN_ONLY: bool = true;

pub async fn run(req: &RecordRequest) -> Result<()> {
    #[cfg(not(target_os = "macos"))]
    {
        let _ = req;
        bail!(
            "scsk backend selected on non-macOS host; use --backend xvfb or --backend winrdp, or rebuild on macOS"
        );
    }

    #[cfg(target_os = "macos")]
    {
        emit_plan(req);

        if PLAN_ONLY {
            bail!(
                "scsk backend is in plan mode — Swift SckBridge static lib not yet linked into \
                 `hwledger-journey-record`. See tools/journey-record/src/backends/scsk.rs TODO \
                 header (3-step wiring). The capture plan was logged above; no MP4 was written."
            );
        }

        // NOTE: unreachable while PLAN_ONLY=true, but kept to pin the API
        // shape for the follow-up wiring work.
        #[allow(unreachable_code)]
        {
            return drive_sck_bridge(req).await;
        }
    }
}

#[cfg(target_os = "macos")]
fn emit_plan(req: &RecordRequest) {
    tracing::info!(
        backend = "scsk",
        target = %req.target,
        output = %req.output.display(),
        width = req.width,
        height = req.height,
        fps = req.fps,
        duration_s = req.duration.map(|d| d.as_secs()).unwrap_or(0),
        virtual_cursor = req.virtual_cursor,
        headless = req.headless,
        sandbox = req.sandbox,
        "scsk capture plan"
    );

    if req.sandbox {
        tracing::info!("plan: generate sandbox-exec .sb profile scoped to journey temp dir + app container");
    }
    if req.headless {
        tracing::info!("plan: route target to off-screen NSScreen virtual display; fall back to primary with warning if single-display");
    }
    if req.virtual_cursor {
        tracing::info!("plan: enable CoreGraphics overlay pass in SckBridge StreamDelegate; real cursor stays on user's desktop");
    }
}

#[cfg(target_os = "macos")]
async fn drive_sck_bridge(_req: &RecordRequest) -> Result<()> {
    // When the Swift static lib is linked, this function will:
    //   1. hwledger_sck_check_permission() — fail loudly on TCC denial
    //   2. optional sandbox + virtual-display setup
    //   3. hwledger_sck_start_recording(bundle_id, path, w, h, fps)
    //   4. optional virtual-cursor overlay activation
    //   5. sleep(duration) OR wait for Ctrl+C
    //   6. hwledger_sck_stop_recording()
    // See crates/hwledger-gui-recorder/src/sck_bridge.rs for the FFI signatures.
    bail!("drive_sck_bridge is not wired — see PLAN_ONLY TODO in tools/journey-record/src/backends/scsk.rs");
}
