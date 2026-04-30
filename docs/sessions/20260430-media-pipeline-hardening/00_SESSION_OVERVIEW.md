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

Remaining blocker: Remotion Chromium cannot launch inside the nested sandbox, so
full rich/TTS regeneration was validated to the point of invocation but not
completed for the unreferenced `what-if-gui` capture.
