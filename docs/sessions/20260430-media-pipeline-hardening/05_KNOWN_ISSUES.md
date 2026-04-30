# Known Issues

- Remotion Chromium render is blocked in the current nested sandbox:
  `bootstrap_check_in org.chromium.Chromium.MachPortRendezvousServer:
  Permission denied`.
- Homebrew `ffmpeg` is broken locally because it references a missing
  `libx265.215.dylib`.
- `what-if-gui` remains an unreferenced GUI capture with missing rich MP4 and is
  reported as a media audit warning rather than a docs build failure.
