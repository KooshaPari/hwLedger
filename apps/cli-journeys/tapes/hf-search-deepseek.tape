# hwLedger — Hugging Face search for DeepSeek-V3
# Intent: Search the HF Hub anonymously, then one-shot plan for DeepSeek-V3.

Set Shell "bash"
Set Theme "Catppuccin Mocha"
Set FontFamily "JetBrains Mono"
Set FontSize 20
Set Width 1280
Set Height 720
Set Padding 40
Set TypingSpeed 75ms
Set LoopOffset 100%

Output "apps/cli-journeys/recordings/hf-search-deepseek.gif"
Output "apps/cli-journeys/recordings/hf-search-deepseek.mp4"

Sleep 1200ms
Type `hwledger search query "deepseek v3" --limit 5`
Sleep 300ms
Enter
Sleep 3500ms

Type `hwledger search plan deepseek-ai/DeepSeek-V3 --seq 8192; echo "__EXIT_$?__"`
Sleep 300ms
Enter
Sleep 5000ms
