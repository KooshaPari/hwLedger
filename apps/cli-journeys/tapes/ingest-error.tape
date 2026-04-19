Set FontSize 16
Set Width 1200
Set Height 700
Set TypingSpeed 40ms
Set Padding 10
Set LoopOffset 100%

Output "apps/cli-journeys/recordings/ingest-error.gif"
Output "apps/cli-journeys/recordings/ingest-error.mp4"

Type "hwledger ingest gguf:///tmp/nonexistent.gguf"
Sleep 500ms
Enter
Sleep 1.5s
