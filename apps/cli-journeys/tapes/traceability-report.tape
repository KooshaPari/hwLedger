Set FontSize 13
Set Width 1200
Set Height 700
Set TypingSpeed 40ms
Set Padding 10
Set LoopOffset 100%

Output "apps/cli-journeys/recordings/traceability-report.gif"
Output "apps/cli-journeys/recordings/traceability-report.mp4"

Type "# Generate full traceability report (spec -> test -> code)"
Sleep 300ms
Enter
Sleep 300ms
Type "cargo run -q -p hwledger-traceability -- \\"
Sleep 300ms
Enter
Sleep 200ms
Type "  --markdown-out /tmp/trace.md \\"
Sleep 300ms
Enter
Sleep 200ms
Type "  --include-coverage"
Sleep 500ms
Enter
Sleep 2s
Type "# Export and view top of report"
Sleep 300ms
Enter
Sleep 300ms
Type "head -50 /tmp/trace.md"
Sleep 500ms
Enter
Sleep 1s
