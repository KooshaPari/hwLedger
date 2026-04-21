# ADR 0022 — Windows native stack: WinUI 3 / C# first, `windows-app-rs` deferred

Constrains: FR-UI-001

Date: 2026-04-19
Status: Accepted

## Context

ADR-0001 put WinUI 3 + C# as the Windows frontend. In the interim a research brief (`docs/research/windows-client-strategy-2026-04.md` — pending import) evaluated `windows-app-rs` (pure Rust WinUI 3 via projections) as a route to eliminate C#. This ADR locks in the C# path for v1 and records the re-evaluation criteria.

## Options

| Stack | Native feel | FFI to Rust core | Ecosystem maturity | Build complexity | MSIX packaging |
|---|---|---|---|---|---|
| WinUI 3 + C# .NET 9 | Excellent | `csbindgen` → raw C ABI | Mature | Medium | First-class |
| WinUI 3 + windows-app-rs | Excellent (if works) | Direct (same crate tree) | Immature (<0.3 in 2026-04) | High | Manual |
| WPF | Legacy Win 10 feel | P/Invoke | Mature (stagnant) | Low | OK |
| WinForms | 90s look | P/Invoke | Mature (stagnant) | Trivial | OK |
| Tauri/electrobun on Win | WebView2 polish | Native Rust | Mature (Tauri) | Low | Add-on |
| Avalonia | Cross-platform (OK feel) | via C# interop | Mature | Low | First-class |

## Decision

v1 Windows build is **WinUI 3 hosted in C# .NET 9** (Native AOT compatible), consuming `hwledger-ffi` via `csbindgen`. Packaging through MSIX + Velopack auto-update + WinGet manifest. electrobun/Tauri is the lightweight alternative per ADR-0021.

**Deferred**: `windows-app-rs` + Rust-native WinUI is parked until the project reaches ≥0.5 with >5 shipping consumers and Microsoft's Windows App SDK team publishes a stable projection schedule.

## Rationale

- C# + WinUI 3 is the path Microsoft documents and ships. Stack Overflow + docs coverage is orders of magnitude deeper than windows-app-rs.
- `csbindgen` (ADR-0020) closes the C#/Rust gap with ~zero runtime overhead (source-gen P/Invoke, AOT-friendly).
- Deferring windows-app-rs avoids being an involuntary early adopter on a 2-person community project.

## Consequences

- One C# sub-project adds a .NET SDK dep to the Win build. Acceptable; does not cross-contaminate other platforms.
- If the windows-app-rs story matures, we can migrate without breaking MSIX packaging (Windows App SDK API is the same).

## Revisit when

- `windows-app-rs` reaches ≥0.5, has ≥5 shipping consumers, or Microsoft publicly adopts it.
- Microsoft drops or stalls WinUI 3 investment.

## References

- Research brief (pending import): `docs/research/windows-client-strategy-2026-04.md`
- `windows-app-rs`: https://github.com/rust-windowing/windows-app-rs
- ADR-0001, ADR-0020, ADR-0021.
