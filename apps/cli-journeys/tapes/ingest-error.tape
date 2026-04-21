# hwLedger — Ingest Error
# Intent: Show graceful error handling when a model file doesn't exist.
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

Output "apps/cli-journeys/recordings/ingest-error.gif"
Output "apps/cli-journeys/recordings/ingest-error.mp4"

Sleep 1500ms
Type `hwledger ingest gguf:///tmp/nonexistent.gguf; echo "__EXIT_$?__"`
Sleep 600ms
Enter
Sleep 3500ms
Sleep 2000ms
