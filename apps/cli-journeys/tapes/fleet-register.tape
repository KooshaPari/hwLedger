Set FontSize 14
Set Width 1200
Set Height 600
Set TypingSpeed 40ms
Set Padding 10
Set LoopOffset 100%

Output "apps/cli-journeys/recordings/fleet-register.gif"
Output "apps/cli-journeys/recordings/fleet-register.mp4"

Type "# Register this device as a fleet node"
Sleep 300ms
Enter
Sleep 300ms
Type "hwledger fleet register \\"
Sleep 300ms
Enter
Sleep 200ms
Type "  --server http://localhost:8080 \\"
Sleep 300ms
Enter
Sleep 200ms
Type "  --token dev-token-abc123 \\"
Sleep 300ms
Enter
Sleep 200ms
Type "  --hostname laptop-01 \\"
Sleep 300ms
Enter
Sleep 200ms
Type "  --arch x86_64 --os darwin"
Sleep 500ms
Enter
Sleep 1.5s
Type "# Registration complete; device joined fleet"
Sleep 300ms
Enter
Sleep 500ms
