# hwLedger — Fleet Audit
# Intent: Audit the fleet with a 3-agent limit to see agent metadata and status.
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

Output "apps/cli-journeys/recordings/fleet-audit.gif"
Output "apps/cli-journeys/recordings/fleet-audit.mp4"

Hide
Type "hwledger-server --port 8080 --db /tmp/hwl-audit.db &"
Sleep 500ms
Enter
Sleep 2s
Type "sleep 1"
Sleep 300ms
Enter
Sleep 1s
Show

Sleep 1500ms
Type "hwledger fleet audit \\"
Sleep 600ms
Enter
Sleep 600ms
Type "  --server http://localhost:8080 \\"
Sleep 600ms
Enter
Sleep 600ms
Type "  --limit 3"
Sleep 600ms
Enter
Sleep 2500ms

Hide
Type "kill %1; rm -f /tmp/hwl-audit.db"
Sleep 300ms
Enter
Sleep 500ms
Show
