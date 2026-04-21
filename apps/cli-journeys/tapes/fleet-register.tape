# hwLedger — Fleet Register
# Intent: Register a new agent with the fleet, then verify it appears in fleet status.
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

Output "apps/cli-journeys/recordings/fleet-register.gif"
Output "apps/cli-journeys/recordings/fleet-register.mp4"

Hide
Type "hwledger-server --port 8080 --db /tmp/hwl-fleet.db &"
Sleep 500ms
Enter
Sleep 2s
Type "sleep 1"
Sleep 300ms
Enter
Sleep 1s
Show

Sleep 1500ms
Type "hwledger fleet register \\"
Sleep 600ms
Enter
Sleep 600ms
Type "  --server http://localhost:8080 \\"
Sleep 600ms
Enter
Sleep 600ms
Type "  --token dev-bootstrap-token \\"
Sleep 600ms
Enter
Sleep 600ms
Type "  --hostname demo-laptop"
Sleep 600ms
Enter
Sleep 2500ms

Type `hwledger fleet status --server http://localhost:8080 --token dev-bootstrap-token; echo "__EXIT_$?__"`
Sleep 600ms
Enter
Sleep 3500ms
Sleep 2000ms

Hide
Type "kill %1; rm -f /tmp/hwl-fleet.db"
Sleep 300ms
Enter
Sleep 500ms
Show
