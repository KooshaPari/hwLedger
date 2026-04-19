# hwLedger — Plan Deepseek
# Intent: Plan with Deepseek at 2K seq, then at 32K to show KV growth under scaling.
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

Output "apps/cli-journeys/recordings/plan-deepseek.gif"
Output "apps/cli-journeys/recordings/plan-deepseek.mp4"

Sleep 1500ms
Type "hwledger plan tests/golden/deepseek-v3.json --seq 2048 --users 2"
Sleep 600ms
Enter
Sleep 2500ms

Type "hwledger plan tests/golden/deepseek-v3.json --seq 32768 --users 2"
Sleep 600ms
Enter
Sleep 2500ms
