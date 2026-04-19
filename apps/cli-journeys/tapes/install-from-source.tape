Set FontSize 14
Set Width 1200
Set Height 600
Set TypingSpeed 40ms
Set Padding 10
Set LoopOffset 100%

Output "apps/cli-journeys/recordings/install-from-source.gif"
Output "apps/cli-journeys/recordings/install-from-source.mp4"

Type "# Clone hwLedger"
Sleep 300ms
Enter
Sleep 300ms
Type "git clone https://github.com/KooshaPari/hwLedger.git"
Sleep 500ms
Enter
Sleep 1s
Type "cd hwLedger"
Sleep 300ms
Enter
Sleep 300ms
Type "# Build the CLI binary"
Sleep 300ms
Enter
Sleep 300ms
Type "cargo build --release -p hwledger-cli"
Sleep 500ms
Enter
Sleep 3s
Type "# Verify installation"
Sleep 300ms
Enter
Sleep 300ms
Type "target/release/hwledger-cli --version"
Sleep 500ms
Enter
Sleep 1s
