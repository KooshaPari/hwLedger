# Known Issues

- Homebrew `ffmpeg` is broken locally because it references a missing
  `libx265.215.dylib`.
- LaunchServices refuses the freshly bundled local macOS app in this sandbox:
  `kLSNoExecutableErr: The executable is missing`. Direct bundle inspection shows
  `CFBundleExecutable=HwLedger`, `Contents/MacOS/HwLedger` exists, and ad-hoc
  signing validates, so the next run should happen from a non-sandboxed Terminal.
- Several existing GUI verified manifests still report `passed=false`; they have
  keyframes and rich videos, but must be regenerated from passing harness runs
  before the media audit can make `passed=false` a hard failure.
