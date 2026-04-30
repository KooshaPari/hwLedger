# Implementation Strategy

- Keep raw XCUITest outputs in `apps/macos/HwLedgerUITests/journeys`.
- Sync into the docs public tree with generated media preservation so rich MP4s
  are not deleted by raw artifact refreshes.
- Normalize GUI keyframes deterministically as `keyframes/frame_NNN.png`.
- Keep the media audit in `docs-site/scripts` and execute it during
  `bun run build`.
- Prefer macOS-native audio conversion for AVSpeech fallback.
