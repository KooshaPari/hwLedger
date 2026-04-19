# hwLedger — Probe Watch
# Intent: Watch probe metrics update in real time with 1-second refresh intervals.
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

Output "apps/cli-journeys/recordings/probe-watch.gif"
Output "apps/cli-journeys/recordings/probe-watch.mp4"

Sleep 1500ms
Type "hwledger probe watch --interval 1s --json"
Sleep 600ms
Enter
Sleep 4500ms
Ctrl+C
Sleep 1000ms
