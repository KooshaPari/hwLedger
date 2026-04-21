# hwLedger — Traceability Strict
# Intent: Show strict mode enforcement: passing with complete traces, then failing on missing trace comment.
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

Output "apps/cli-journeys/recordings/traceability-strict.gif"
Output "apps/cli-journeys/recordings/traceability-strict.mp4"

Sleep 1500ms
Type "hwledger-traceability"
Sleep 600ms
Enter
Sleep 3000ms

Type `set -o pipefail; hwledger-traceability --strict 2>&1 | tail -6; echo "__EXIT_$?__"`
Sleep 300ms
Enter
Sleep 3500ms
Sleep 2000ms
