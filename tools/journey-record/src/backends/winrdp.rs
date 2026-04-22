//! Windows `winrdp` backend — **stub**.
//!
//! Planned pipeline (TODO, tracked under G-recording-backends):
//!
//! 1. When `--headless` is set, spawn an isolated RDP session to `localhost`
//!    via `mstsc /v:localhost /span` inside a Windows Sandbox container
//!    (`.wsb` config generated per-run with its own virtual disk + no host
//!    folder mounts). The user keeps driving their real desktop on session 0.
//! 2. Inside the session, use the `windows-capture` Rust crate (which wraps
//!    `Windows.Graphics.Capture` + `GraphicsCaptureItem` for per-window
//!    capture) to grab frames from the target window selected by PID or
//!    window title.
//! 3. Mux frames to MP4 via `windows-capture`'s built-in `MF` encoder (no
//!    ffmpeg subprocess needed).
//! 4. Virtual cursor via `SendInput` with `MOUSEEVENTF_ABSOLUTE |
//!    MOUSEEVENTF_VIRTUALDESK` targeting the sandbox'd desktop; real user
//!    cursor is on session 0 and never captured.
//! 5. When `--sandbox` is set without `--headless`, run the target under a
//!    Windows Sandbox container but capture from the parent session via
//!    RDP screen-scraping.
//!
//! All external integrations (Windows Sandbox, RDP client, `windows-capture`
//! crate) must fail loudly when unavailable (see global "Optionality and
//! Failure Behavior" policy).

use anyhow::{bail, Result};

use crate::RecordRequest;

pub async fn run(req: &RecordRequest) -> Result<()> {
    let _ = req;
    bail!(
        "winrdp backend is a stub — Windows capture pipeline (Windows.Graphics.Capture via windows-capture crate + Windows Sandbox + RDP isolation) not yet implemented. See tools/journey-record/src/backends/winrdp.rs TODO header and docs-site/reference/recording-backends.md for the planned pipeline."
    );
}
