Set FontSize 14
Set Width 1200
Set Height 600
Set TypingSpeed 40ms
Set Padding 10
Set LoopOffset 100%

Output "apps/cli-journeys/recordings/install-cargo.gif"
Output "apps/cli-journeys/recordings/install-cargo.mp4"

Type "cargo install --path crates/hwledger-cli --root /tmp/hwl-install"
Sleep 500ms
Enter
Sleep 3s
Type "/tmp/hwl-install/bin/hwledger --version"
Sleep 500ms
Enter
Sleep 1s
