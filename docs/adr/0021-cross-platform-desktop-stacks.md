# ADR 0021 — Cross-platform desktop stacks: SwiftUI + electrobun/Tauri split

Constrains: FR-UI-001..004, refines ADR-0001

Date: 2026-04-19
Status: Accepted

## Context

ADR-0001 set the policy of three native frontends. Since then the workspace produced a research brief (`docs/research/cross-platform-desktop-stacks-2026-04.md` — pending import) comparing Tauri 2, Electron, electrobun, Dioxus Desktop, Uno Platform, .NET MAUI, Flutter Desktop, and PWA. This ADR consolidates that brief into a binding decision record.

## Options

| Stack | macOS polish | Win polish | Linux polish | Bundle size | Rust FFI | Primary language |
|---|---|---|---|---|---|---|
| SwiftUI (mac only) | Best-in-class | N/A | N/A | Tiny | Via XCFramework | Swift |
| Tauri 2 | Good (WKWebView) | Good (WebView2) | Good (WebKitGTK) | ~15 MB | Native | Rust + TS |
| electrobun | Good (WKWebView) | Good (WebView2) | Good (WebKitGTK) | ~10 MB | FFI (Bun) | Bun + TS |
| Electron | OK | OK | OK | ~120 MB | FFI (Node) | Node + TS |
| Dioxus Desktop | OK (Wry) | OK (Wry) | OK (Wry) | ~25 MB | Native | Rust |
| Uno Platform | OK | Native WinUI | Via Skia | ~80 MB | via C# interop | C# |
| .NET MAUI | OK | Native WinUI | No | ~70 MB | via C# interop | C# |
| Flutter Desktop | OK | OK | OK | ~40 MB | via dart:ffi | Dart |
| PWA | Browser chrome | Browser chrome | Browser chrome | 0 | No | N/A |

## Decision

- **macOS**: SwiftUI + XCFramework-wrapped Rust core. No webview anywhere on mac.
- **Windows + Linux**: electrobun (Bun + WKWebView/WebView2/WebKitGTK) preferred; Tauri 2 as the fallback if electrobun stalls or a consumer hits a blocker. Both share the same TS UI codebase.
- **No Electron.** Excluded on bundle size, perf, and precedent (LM Studio, Signal, 1Password all fled Electron).

## Rationale

- SwiftUI is the single best choice for macOS polish. Matches Path C of the research brief.
- Windows native polish needs WebView2 or WinUI 3. WinUI 3 ships in ADR-0001 C# path; electrobun gives us a lighter alternative that shares code with Linux.
- electrobun is ~10 MB vs Tauri 2's ~15 MB and uses Bun as the runtime, aligning with our Bun-preferred scripting policy. Tauri is the safety valve if electrobun proves unready.
- One TS UI codebase across Win + Linux halves UI maintenance vs three full native toolkits.

## Consequences

- Two UI stacks on mac vs PC. Mac code is SwiftUI; everything else is TS inside electrobun/Tauri. Rust core (ADR-0020) is shared.
- electrobun is young (<1.0 as of 2026-04); we pin to a known-good commit and watch the fallback Tauri path.
- Research brief (`docs/research/cross-platform-desktop-stacks-2026-04.md`) tracks live comparisons and should be imported into the repo.

## Revisit when

- electrobun reaches 1.0 or is abandoned.
- Tauri 2 adds native WebView2 upgrades that close its bundle gap.
- Dioxus Desktop reaches feature parity (would collapse to one stack: Rust on every target).

## References

- Research brief (pending import): `docs/research/cross-platform-desktop-stacks-2026-04.md`
- ADR-0001, ADR-0020.
