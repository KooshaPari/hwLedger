# hwLedger — Plan via HF resolver
# Intent: Demonstrate the model resolver dispatching on (a) bare HF repo id,
# (b) full HF URL, and (c) `gold:` fixture shortcut — all feeding the planner.
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

Output "apps/cli-journeys/recordings/plan-hf-resolve.gif"
Output "apps/cli-journeys/recordings/plan-hf-resolve.mp4"

Sleep 1500ms
Type "hwledger plan deepseek-ai/DeepSeek-V3 --seq 4K"
Sleep 600ms
Enter
Sleep 4500ms

Type "hwledger plan https://huggingface.co/mistralai/Mistral-7B-v0.1 --seq 4K"
Sleep 600ms
Enter
Sleep 4500ms

Type `hwledger plan gold:deepseek-v3 --seq 32K; echo "__EXIT_$?__"`
Sleep 600ms
Enter
Sleep 4500ms
Sleep 2000ms
