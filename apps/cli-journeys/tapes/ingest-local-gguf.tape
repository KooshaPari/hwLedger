# hwLedger — Ingest Local GGUF
# Intent: Ingest a local GGUF model file and output JSON metadata.
#
# Styling — keep identical across all tapes for visual consistency.

Set Shell "bash"
Set Theme "Catppuccin Mocha"
Set FontFamily "JetBrains Mono"
Set FontSize 20
Set Width 1280
Set Height 720
Set Padding 40
Set TypingSpeed 75ms
Set LoopOffset 100%

Output "apps/cli-journeys/recordings/ingest-local-gguf.gif"
Output "apps/cli-journeys/recordings/ingest-local-gguf.mp4"

Sleep 1500ms
Type `hwledger ingest gguf://tests/golden/fixtures/tiny.gguf --json; echo "__EXIT_$?__"`
Sleep 600ms
Enter
Sleep 3500ms
Sleep 2000ms
