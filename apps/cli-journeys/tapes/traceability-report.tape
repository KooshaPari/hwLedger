# hwLedger — Traceability Report
# Intent: Generate a markdown traceability report with coverage data and inspect the output.
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

Output "apps/cli-journeys/recordings/traceability-report.gif"
Output "apps/cli-journeys/recordings/traceability-report.mp4"

Sleep 1500ms
Type "cargo run -q -p hwledger-traceability -- --markdown-out /tmp/trace.md --verbose"
Sleep 600ms
Enter
Sleep 4000ms

Type "head -40 /tmp/trace.md"
Sleep 600ms
Enter
Sleep 3000ms

Type "grep -c '^| FR-' /tmp/trace.md"
Sleep 600ms
Enter
Sleep 2000ms
