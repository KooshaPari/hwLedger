Set FontSize 14
Set Width 1200
Set Height 600
Set TypingSpeed 40ms
Set Padding 10
Set LoopOffset 100%

Output "apps/cli-journeys/recordings/ingest-local-gguf.gif"
Output "apps/cli-journeys/recordings/ingest-local-gguf.mp4"

Type "hwledger ingest gguf://tests/golden/fixtures/tiny.gguf --json"
Sleep 500ms
Enter
Sleep 1.5s
