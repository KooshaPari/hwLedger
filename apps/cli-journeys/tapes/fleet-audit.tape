Set FontSize 14
Set Width 1200
Set Height 600
Set TypingSpeed 40ms
Set Padding 10
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

Type "hwledger fleet audit \\"
Sleep 300ms
Enter
Sleep 200ms
Type "  --server http://localhost:8080 \\"
Sleep 300ms
Enter
Sleep 200ms
Type "  --limit 3"
Sleep 500ms
Enter
Sleep 2s

Hide
Type "kill %1; rm -f /tmp/hwl-audit.db"
Sleep 300ms
Enter
Sleep 500ms
Show
