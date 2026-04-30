# Research

- `snapwin` proved ScreenCaptureKit can capture the hwLedger window cleanly,
  so the harness moved away from deprecated window capture as the primary path.
- `docs-site/scripts/sync-journey-artefacts.sh` was still syncing from the old
  recordings layout into `public/journeys`; docs consume `public/gui-journeys`.
- `hwledger-journey-render` expects `manifest.verified.json` and staged
  keyframes before Remotion render.
- Local Homebrew `ffmpeg` fails to load `libx265.215.dylib`; macOS `afconvert`
  is the safer AVSpeech conversion path.
- Remotion dependencies are installed, but Chromium launch fails in the nested
  sandbox with `MachPortRendezvousServer` permission denial.
