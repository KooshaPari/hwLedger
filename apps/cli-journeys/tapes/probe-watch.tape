Set FontSize 16
Set Width 1200
Set Height 700
Set TypingSpeed 40ms
Set Padding 10
Set LoopOffset 100%

Output "apps/cli-journeys/recordings/probe-watch.gif"
Output "apps/cli-journeys/recordings/probe-watch.mp4"

Type "hwledger probe watch --interval 1s --json"
Sleep 500ms
Enter
Sleep 3s
Ctrl+C
Sleep 500ms
