# DAG WBS

1. [done] Rehydrate GUI screenshots through the native harness.
2. [done] Normalize GUI sync into `public/gui-journeys`.
3. [done] Add media coverage audit and wire it into docs build.
4. [done] Fill missing CLI keyframe coverage for `hf-search-deepseek`.
5. [done] Harden AVSpeech/TTS conversion away from broken `ffmpeg`.
6. [done] Re-run Remotion rich renders with Edge TTS for CLI, GUI, and Streamlit journeys.
7. [done] Fix local macOS debug bundling so `--no-codesign` produces an ad-hoc
   signed bundle after `install_name_tool` changes the executable.
8. [blocked] Re-run GUI journeys from a non-sandboxed Terminal session and replace
   any `passed=false` GUI manifests with fresh passing captures.
9. [next] Tighten `audit-media-coverage.mjs` from warning to failure for referenced
   GUI manifests that still report `passed=false` after the fresh captures land.
