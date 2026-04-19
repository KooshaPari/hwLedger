Set FontSize 16
Set Width 1200
Set Height 700
Set TypingSpeed 40ms
Set Padding 10
Set LoopOffset 100%

Output "apps/cli-journeys/recordings/plan-deepseek.gif"
Output "apps/cli-journeys/recordings/plan-deepseek.mp4"

Type "hwledger plan tests/golden/deepseek-v3.json --seq 2048 --users 2"
Sleep 500ms
Enter
Sleep 2s
