# hwLedger — Plan Help
# Intent: Show the plan command's help text with all available options.
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

Output "apps/cli-journeys/recordings/plan-help.gif"
Output "apps/cli-journeys/recordings/plan-help.mp4"

Sleep 1500ms
Type `hwledger plan --help; echo "__EXIT_$?__"`
Sleep 600ms
Enter
Sleep 3500ms
Sleep 2000ms
