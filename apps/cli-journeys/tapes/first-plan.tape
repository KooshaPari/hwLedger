Set FontSize 16
Set Width 1200
Set Height 700
Set TypingSpeed 40ms
Set Padding 10
Set LoopOffset 100%

Output "apps/cli-journeys/recordings/first-plan.gif"
Output "apps/cli-journeys/recordings/first-plan.mp4"

Type "# Plan memory for Llama2-70B at batch size 4"
Sleep 300ms
Enter
Sleep 300ms
Type "hwledger plan tests/golden/llama3-70b.json --seq 4096 --users 4 --color"
Sleep 500ms
Enter
Sleep 2s
