# hwLedger — Install Cargo
# Intent: Install hwledger from source with cargo, then verify version and help.
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

Output "apps/cli-journeys/recordings/install-cargo.gif"
Output "apps/cli-journeys/recordings/install-cargo.mp4"

Sleep 1500ms
Type "cargo install --path crates/hwledger-cli --root /tmp/hwl-install"
Sleep 600ms
Enter
Sleep 3500ms

Type "/tmp/hwl-install/bin/hwledger --version"
Sleep 600ms
Enter
Sleep 1500ms

Type "/tmp/hwl-install/bin/hwledger --help"
Sleep 600ms
Enter
Sleep 2500ms
