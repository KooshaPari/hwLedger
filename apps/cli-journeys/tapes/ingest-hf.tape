Set FontSize 14
Set Width 1200
Set Height 600
Set TypingSpeed 40ms
Set Padding 10
Set LoopOffset 100%

Output "apps/cli-journeys/recordings/ingest-hf.gif"
Output "apps/cli-journeys/recordings/ingest-hf.mp4"

Type "# Ingest model metadata from Hugging Face hub"
Sleep 300ms
Enter
Sleep 300ms
Type "hwledger ingest hf://Qwen/Qwen2-7B --json"
Sleep 500ms
Enter
Sleep 2s
Type "# Model metadata cached locally for fast planning"
Sleep 300ms
Enter
Sleep 500ms
