Set FontSize 14
Set Width 1200
Set Height 600
Set TypingSpeed 40ms
Set Padding 10
Set LoopOffset 100%

Output "apps/cli-journeys/recordings/ingest-ollama.gif"
Output "apps/cli-journeys/recordings/ingest-ollama.mp4"

Type "# Ingest model from local Ollama server"
Sleep 300ms
Enter
Sleep 300ms
Type "hwledger ingest ollama://llama3:latest --json"
Sleep 500ms
Enter
Sleep 1.5s
Type "# Successfully queried Ollama; model info cached"
Sleep 300ms
Enter
Sleep 500ms
