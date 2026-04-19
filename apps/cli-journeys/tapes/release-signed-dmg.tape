Set FontSize 14
Set Width 1200
Set Height 700
Set TypingSpeed 40ms
Set Padding 10
Set LoopOffset 100%

Output "apps/cli-journeys/recordings/release-signed-dmg.gif"
Output "apps/cli-journeys/recordings/release-signed-dmg.mp4"

Type "# Create a signed macOS release (DMG with code signature + notarization)"
Sleep 300ms
Enter
Sleep 300ms
Type "scripts/release.sh v0.1.0-alpha --dmg --sign --notarize"
Sleep 500ms
Enter
Sleep 2s
Type "# Release pipeline:"
Sleep 300ms
Enter
Sleep 300ms
Type "#   1. Build and sign hwLedger.app (Developer ID)"
Sleep 300ms
Enter
Sleep 300ms
Type "#   2. Create DMG archive with codesign and spctl verify"
Sleep 300ms
Enter
Sleep 300ms
Type "#   3. Submit to Apple notarization service"
Sleep 300ms
Enter
Sleep 300ms
Type "#   4. Staple ticket to DMG"
Sleep 300ms
Enter
Sleep 1s
