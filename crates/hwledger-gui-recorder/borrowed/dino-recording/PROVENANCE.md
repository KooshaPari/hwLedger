# dino-recording — Borrow Provenance

**Source repo investigated:** `github.com/KooshaPari/dino` (local mirror at `/Users/kooshapari/CodeProjects/Phenotype/repos/dino`, HEAD `6dab955`).

## Finding: Nothing to borrow

`KooshaPari/dino` resolves to **DINOForge**, a Unity/.NET mod platform for the game
_Diplomacy is Not an Option_ — not a screen-capture or window-recording project. It
shares only a name with the hypothetical recording-stack project referenced in the
task brief.

Searched across `src/`, `manifests/`, `packs/`, and the MCP server for any of:

- `ScreenCaptureKit`, `SCStream`, `SCShareableContent`, `SCContentFilter`
- `Windows.Graphics.Capture`, `GraphicsCaptureItem`
- `Xvfb`, `x11grab`, `ffmpeg -f avfoundation`
- Virtual cursor / cursor multiplexing patterns
- Headless virtual-display orchestration (Sidecar, `bubblewrap`, Windows Sandbox)
- App sandboxing (`sandbox-exec`, `bwrap`)

Only hits were:

| File | Purpose | Borrowable? |
|------|---------|-------------|
| `src/Tools/Cli/Commands/ScreenshotCommand.cs` | Single-shot Unity framebuffer dump via the game's own scripting API | No — Unity-internal |
| `src/Tools/McpServer/Tools/GameAnalyzeScreenTool.cs` | MCP handler that forwards screenshots from the game | No — MCP glue |
| `src/Tools/McpServer/Tools/GameUIAutomationTool.cs` | Unity UI automation over the game's internal input queue | No — Unity-specific |

None of these touch OS-level capture APIs (SCK, GraphicsCapture, x11grab). They
operate _inside_ the Unity process via the game's reverse-engineered scripting
bridge.

## Conclusion

Per-OS capture patterns (SCSK window-targeted capture, virtual cursor, Xvfb
sandboxing, `windows-capture` crate usage, `bubblewrap`/`sandbox-exec`/Windows
Sandbox isolation) were implemented from scratch here, with references to the
canonical Apple / freedesktop / Microsoft documentation rather than dino.

The existing `hwledger-gui-recorder` crate (Swift SCK bridge in `swift-sck/Sources/SckBridge/SckBridge.swift`)
pre-dates this investigation and already implements the SCK window-filter pattern
the brief hypothesised dino would have; it is our canonical macOS backend.

No files imported. No commit SHA pinned — nothing was lifted.
