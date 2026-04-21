# hwLedger — First Plan
# Intent: Run your first plan with colored output showing token distribution for 4 users.
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

Output "apps/cli-journeys/recordings/first-plan.gif"
Output "apps/cli-journeys/recordings/first-plan.mp4"

Sleep 1500ms
Type `hwledger plan tests/golden/llama3-70b.json --seq 4096 --users 4; echo "__EXIT_$?__"`
Sleep 600ms
Enter
Sleep 4000ms
Sleep 2000ms
