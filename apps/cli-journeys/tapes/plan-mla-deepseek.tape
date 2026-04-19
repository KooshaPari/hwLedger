# hwLedger — Plan MLA Deepseek
# Intent: Show MLA classification + KV sequence invariance across 2K, 32K, 128K sequences.
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

Output "apps/cli-journeys/recordings/plan-mla-deepseek.gif"
Output "apps/cli-journeys/recordings/plan-mla-deepseek.mp4"

Sleep 1500ms
Type "hwledger plan tests/golden/deepseek-v3.json --seq 2048 --mla"
Sleep 600ms
Enter
Sleep 2500ms

Type "hwledger plan tests/golden/deepseek-v3.json --seq 32768 --mla"
Sleep 600ms
Enter
Sleep 2500ms

Type "hwledger plan tests/golden/deepseek-v3.json --seq 131072 --mla"
Sleep 600ms
Enter
Sleep 2500ms
