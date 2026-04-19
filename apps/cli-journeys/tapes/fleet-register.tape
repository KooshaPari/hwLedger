Set FontSize 14
Set Width 1200
Set Height 600
Set TypingSpeed 40ms
Set Padding 10
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

Type "hwledger fleet register \\"
Sleep 300ms
Enter
Sleep 200ms
Type "  --server http://localhost:8080 \\"
Sleep 300ms
Enter
Sleep 200ms
Type "  --token dev-bootstrap-token \\"
Sleep 300ms
Enter
Sleep 200ms
Type "  --hostname demo-laptop"
Sleep 500ms
Enter
Sleep 2s

Hide
Type "kill %1; rm -f /tmp/hwl-fleet.db"
Sleep 300ms
Enter
Sleep 500ms
Show
