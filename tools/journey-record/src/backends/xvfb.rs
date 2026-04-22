//! Linux `xvfb` backend — **stub**.
//!
//! Planned pipeline (TODO, tracked under G-recording-backends):
//!
//! 1. `Xvfb :99 -screen 0 1440x900x24 -ac -nolisten tcp &` (pick display
//!    number dynamically to avoid collisions with the user's real X server).
//! 2. Spawn target under `DISPLAY=:99`, optionally wrapped by
//!    `bubblewrap --unshare-all --die-with-parent --uid 1000 --ro-bind / / ...`
//!    when `--sandbox` is set, so the app cannot see the user's home or
//!    network namespace.
//! 3. `ffmpeg -f x11grab -video_size 1440x900 -framerate 30 -i :99 \
//!             -c:v libx264 -preset ultrafast -pix_fmt yuv420p -y <output.mp4>`.
//! 4. Virtual cursor via `xdotool mousemove --sync <x> <y>` driven by the
//!    Playwright harness, overlaid through ffmpeg's `movie` + `overlay`
//!    filtergraph so the real user cursor (on `:0`) is never captured.
//! 5. Tear down Xvfb + any bubblewrap-spawned children on drop.
//!
//! All external binaries (`Xvfb`, `ffmpeg`, `bwrap`, `xdotool`) must be
//! resolved via `which::which` with explicit, loud failures — no silent
//! degradation (see global "Optionality and Failure Behavior" policy).

use anyhow::{bail, Result};

use crate::RecordRequest;

pub async fn run(req: &RecordRequest) -> Result<()> {
    let _ = req;
    bail!(
        "xvfb backend is a stub — Linux capture pipeline (Xvfb + ffmpeg x11grab + bwrap + xdotool) not yet implemented. See tools/journey-record/src/backends/xvfb.rs TODO header and docs-site/reference/recording-backends.md for the planned pipeline."
    );
}
