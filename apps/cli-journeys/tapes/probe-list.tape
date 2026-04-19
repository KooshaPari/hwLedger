Set FontSize 16
Set Width 1200
Set Height 700
Set TypingSpeed 40ms
Set Padding 10
Set LoopOffset 100%

Output "apps/cli-journeys/recordings/probe-list.gif"
Output "apps/cli-journeys/recordings/probe-list.mp4"

Type "hwledger probe list --json"
Sleep 500ms
Enter
Sleep 1s
