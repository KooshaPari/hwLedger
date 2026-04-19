Set FontSize 14
Set Width 1200
Set Height 600
Set TypingSpeed 40ms
Set Padding 10
Set LoopOffset 100%

Output "apps/cli-journeys/recordings/fleet-audit.gif"
Output "apps/cli-journeys/recordings/fleet-audit.mp4"

Type "# Audit fleet node health and VRAM across 10 devices"
Sleep 300ms
Enter
Sleep 300ms
Type "hwledger fleet audit \\"
Sleep 300ms
Enter
Sleep 200ms
Type "  --server http://localhost:8080 \\"
Sleep 300ms
Enter
Sleep 200ms
Type "  --limit 10 \\"
Sleep 300ms
Enter
Sleep 200ms
Type "  --verify"
Sleep 500ms
Enter
Sleep 2s
Type "# Audit complete: 10 nodes verified, all healthy"
Sleep 300ms
Enter
Sleep 500ms
