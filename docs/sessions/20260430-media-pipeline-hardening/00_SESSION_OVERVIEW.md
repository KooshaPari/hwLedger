# Session Overview

Goal: harden the docs media pipeline so GUI, CLI, and Streamlit journey pages
have validated keyframes, verified manifests, and rich-video coverage checks.

Outcome:
- GUI harness screenshots now attach to explicit step indexes and auto-capture
  missing step keyframes.
- GUI docs sync now targets `docs-site/public/gui-journeys`, preserves generated
  rich media, and normalizes `frame_NNN.png` keyframes.
- Docs build now runs a media coverage audit.
- macOS AVSpeech voiceover conversion no longer requires the broken Homebrew
  `ffmpeg` path.
- Unsigned local macOS debug bundles now get an ad-hoc signature after
  `install_name_tool` mutates the executable, avoiding invalid-signature
  bundles for local journey work.

Remaining blocker: LaunchServices in the current sandbox still refuses the local
debug bundle with `kLSNoExecutableErr`, even though the bundle plist, executable,
and ad-hoc signature validate. GUI journey regeneration should be rerun from a
non-sandboxed Terminal session with Accessibility and Screen Recording grants.
