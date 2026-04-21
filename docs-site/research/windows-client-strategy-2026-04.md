# Windows Client Strategy — hwLedger (2026-04)

**Status**: Research brief (read/write). No code changes.
**Author**: agent (worktree `agent-aebfa7d9`)
**Date**: 2026-04-19
**Companion**: `./cross-platform-desktop-stacks-2026-04.md` (sibling agent — Electron/electrobun/Tauri/Dioxus general survey)
**Upstream decisions**: `../../PLAN.md` §4.1, §12; `../../docs/research/07-ffi-survey.md` (thread 7/8 of the April Haiku research swarm)

> User comment (verbatim): *“Rust-native WinUI via windows-app-rs: too experimental in 2026; small Microsoft team bandwidth. not against this or other experimentals btw.”*
> This brief accepts that framing and narrows the Windows question down to: **what ships a production-quality Windows binary for hwLedger, and what does it cost in agent-time?**

---

## 1. `windows-app-rs` deep dive

### 1.1 Repo state, April 2026

- **Repo**: [`microsoft/windows-app-rs`](https://github.com/microsoft/windows-app-rs) — "Rust for the Windows App SDK".
- **Scope stated by repo**: idiomatic Rust bindings for the Windows App SDK (the umbrella that contains WinUI 3, MRT Core, PushNotifications, WindowsAppRuntime bootstrap, etc.). Sits *on top of* the lower-level [`microsoft/windows-rs`](https://github.com/microsoft/windows-rs) (WinRT / Win32 bindings, which **is** mature and on crates.io as `windows`).
- **Maintainer bandwidth**: Microsoft's own Rust-on-Windows lead has written publicly that "the Windows App SDK and WinUI teams are quite large... the various Rust teams at Microsoft are quite small in comparison, making it harder to plan or predict when certain features will be available" (`windows-rs` issue #3807, "Rust for Windows – October 2025"). That framing carries into 2026: `windows-rs` itself ships monthly, but `windows-app-rs` is a **smaller, less prioritized sibling**.
- **Roadmap state**: roadmap issue (`microsoft/windows-app-rs#6`) still open as a tracking umbrella. Full **XAML component authoring** from Rust is listed as aspirational, not committed.
- **crates.io presence**: *as of 2026-04*, no first-party `windows-app` crate with 1.x guarantees. Consumers build from the git repo or from pre-release crates with unstable APIs. (Compare to `windows = "0.x"` on crates.io, which is production-usable but is Win32/WinRT, not WinUI 3.)
- **Community bindings**: `winio` / `winio-winui3` (docs.rs, last bumped Feb 2026) is a *subset* WinUI 3 wrapper at 0.10.x. Useful proof that bindings are buildable; not a production platform.

### 1.2 What it gives you today

| Layer | Status in `windows-app-rs` (2026-04) |
|---|---|
| WinRT type projection | Inherited from `windows-rs` — **solid** |
| Win32 APIs (HWND, registry, COM) | From `windows-rs` — **solid** |
| WinUI 3 controls instantiation | **Possible but painful** (`Microsoft::UI::Xaml::*` namespaces projected; you write XAML-equivalent trees in Rust) |
| XAML markup files (`.xaml`) | **Not fluent** — no IDL-driven code-gen pipeline; you pay the composition cost by hand |
| XAML hot-reload / Live Visual Tree | **No** (requires VS's managed-language debugger hooks) |
| Component authoring (custom XAML controls) | **Not supported** end-to-end |
| Async/await with `IAsyncOperation<T>` | Partial — futures bridge exists in `windows-rs`, ergonomic gaps remain |
| MSIX packaging | **No Rust-native tooling**; use MSBuild or manual `MakeAppx`/`SignTool` |
| Unpackaged WindowsAppRuntime bootstrap | Working but hand-rolled |

### 1.3 Public production case studies

Assumption (unknown): I could not, within this brief's budget, surface a named commercial Windows product that ships WinUI 3 through `windows-app-rs`. The community examples I found are samples and 0.x crates. **Treat "production case studies" as effectively zero as of 2026-04.**

### 1.4 Risk assessment

- **Bus factor**: low. A handful of Microsoft engineers; no external co-maintainer with commit rights visible on the main branch.
- **Timeline to "production-ready"**: unscheduled. Microsoft has not published a 1.0 target.
- **API churn**: high; every release bumps projected WinRT namespaces.
- **Verdict for hwLedger**: matches the user's instinct — **too experimental to bet the Windows ship on**. Keep on the watch list for a 2027+ re-evaluation.

---

## 2. Alternatives for Windows

### 2.1 Candidate summary table

| Stack | Lang(s) | Bundle (rough) | Cold start | Win code-sign friction | FFI → `hwledger-ffi.dll` | a11y | UI polish ceiling |
|---|---|---|---|---|---|---|---|
| **Tauri 2** | Rust + TS (webview) | 8–25 MB | ~150–400 ms | Azure Key Vault / Azure Trusted Signing via custom `signCommand` (workaround) | **Native** — both halves are Rust; just link `hwledger-core` | UIA via WebView2 (modern, good) | Very high (web) |
| **C# .NET 9 + WinUI 3** (current PLAN.md §4.1 choice) | C# | 20–60 MB (self-contained) / ~5 MB framework-dependent | ~300–800 ms | **First-class** — MSIX, WinGet, Azure Trusted Signing native | `P/Invoke` via `csbindgen`; Native AOT supported | UIA **native** (best of any option) | High (Fluent native) |
| **Uno Platform** (C# XAML → WinAppSDK on Windows) | C# | 40–80 MB | ~400–900 ms | Same as WinUI 3 | Same `P/Invoke`/`csbindgen` | UIA native | High (Fluent on Win; scales to macOS/Linux/Web/iOS) |
| **.NET MAUI** | C# | 50–120 MB | ~500–1200 ms | First-class | `P/Invoke` | UIA native | Medium (desktop is second-class in MAUI) |
| **Dioxus Desktop (wry/WebView)** | Rust | 10–30 MB | ~200–500 ms | Same certificate problem as Tauri; less tooling | Trivial (same process) | UIA via WebView2 | High |
| **Dioxus Native / Blitz (wgpu)** | Rust | 15–40 MB | ~300–700 ms | Same | Trivial | **Weak** (no UIA yet from wgpu canvas) | Unknown — wgpu renderer is pre-production |
| **Flutter Windows** | Dart | 25–60 MB | ~400–900 ms | Cert via SignTool; MSIX via flutter plugin | `dart:ffi` → `cdylib` (mature via `flutter_rust_bridge`) | Windows a11y is the weakest Flutter target | High |
| **Electron** | TS | 90–200 MB | 600–1500 ms | First-class (electron-builder) | `node-ffi` / N-API via `napi-rs` | UIA via Chromium | Very high |
| **electrobun** | TS + Zig/Bun | ~10–15 MB (claim) | TBD | Immature | `bun:ffi` | TBD | Unknown — sibling agent covering |
| **PWA / raw WebView2 shell** | HTML/JS | <5 MB | <200 ms | Only need cert if packaged as MSIX | HTTP to a local Rust sidecar; no direct dylib | Browser defaults | Medium |

Sizes/times are **order-of-magnitude** estimates from published benchmarks; do not quote as spec.

### 2.2 Commentary per candidate

- **Tauri 2**. Production-hardened on Windows; many shipping commercial apps. Azure Trusted Signing is **not first-class** in the `tauri.conf.json` cert block — it requires the custom `signCommand` escape (`"signCommand": "trusted-signing-cli ..."`), or HSM-backed Azure Key Vault. Tauri plugin ecosystem covers auto-update (`tauri-plugin-updater`), WebView2 bundle, MSIX builds. FFI inside Tauri is trivial: your Rust core is already in-process. **Main weakness**: the UI layer is a browser, which conflicts with the user's preference for native platform feel.
- **C# .NET 9 + WinUI 3** (PLAN.md's current pick). Microsoft-blessed, biggest WinUI 3 ecosystem, native Fluent widgets, UIA accessibility is best-in-class, MSIX + WinGet just work, Native AOT reduces cold-start. `csbindgen` emits `P/Invoke` bindings directly from your Rust cdylib — this is battle-tested (`Cysharp/csbindgen` is the canonical tool, widely used in Unity/game tooling). Costs a second language in the stack.
- **Uno Platform**. C# XAML, single codebase → Windows (WinAppSDK), macOS, Linux, Web (WASM), iOS, Android. Uno 6.3/6.4 (late 2025 / early 2026) added .NET 10 support, VS 2026, OpenGL rendering, improved WebView2. FFI identical to WinUI 3 (it *is* C#). Appeals if we want to **consolidate** the mac, Windows, *and* Linux clients into one codebase and retire the SwiftUI app someday. Risk: Uno macOS is less polished than SwiftUI; we'd give up a crown jewel.
- **.NET MAUI**. Microsoft-owned, but desktop (Windows + macOS Catalyst) is the underfunded corner of MAUI; community sentiment through 2025–2026 keeps pointing people at WinUI 3 or Uno for desktop-first work. Skip.
- **Flutter Windows**. `flutter_rust_bridge` is the mature FFI glue. Windows desktop is the *least* battle-tested Flutter surface (a11y lags, MSIX tooling is community-maintained). Reasonable if we also wanted Android/iOS — we don't, per PLAN.md §2 non-goals.
- **Electron**. Known quantity. Biggest bundle, worst cold start, best ecosystem for distribution tooling. Signing is a non-issue. Ship-friendly, polish-friendly, but violates the "native feel" spirit of the multi-client design.
- **electrobun**. Covered by the sibling brief. Flag as "too young" if the sibling confirms.
- **PWA + WebView2 shell**. Cheapest to maintain. But: no direct dylib load, loses offline parity, SmartScreen UX for unsigned MSIX.

---

## 3. hwLedger Windows requirements checklist

From `PLAN.md` §1, §4, §5 and the mac client feature surface, a Windows build must:

- [ ] **R1 — FFI surface**: call `hwledger_plan`, `hwledger_probe_list`, `hwledger_hf_search`, `hwledger_predict` from `hwledger-ffi.dll` (and receive streaming callbacks for probe ticks).
- [ ] **R2 — VRAM visual**: stacked bars + per-layer heatmap. WebGL/Canvas2D or native `Win2D` both acceptable.
- [ ] **R3 — Probe stream**: sustained 1 Hz updates from a background thread without stalling UI.
- [ ] **R4 — Keyboard + a11y**: full keyboard nav; UIA exposes labels/values for every control.
- [ ] **R5 — Code signing**: EV cert or Azure Trusted Signing so SmartScreen doesn't warn first-time users.
- [ ] **R6 — Auto-update**: channel-based updater (Velopack, Squirrel.Windows, or `tauri-plugin-updater`).
- [ ] **R7 — Feature parity with SwiftUI mac client** for the UI surfaces listed in PLAN.md §4 (ledger view, planner, probe stream, HF search).
- [ ] **R8 — Reasonable bundle** (target <40 MB framework-dependent, <100 MB self-contained).
- [ ] **R9 — Crashfree 1 Hz** over multi-hour sessions (hobbyist's laptop monitoring a rented 8×H100).

Rank of each candidate against R1–R9:

| Candidate | R1 | R2 | R3 | R4 | R5 | R6 | R7 | R8 | R9 | Score |
|---|---|---|---|---|---|---|---|---|---|---|
| Tauri 2 | ✅ | ✅ | ✅ | ⚠️ browser a11y | ⚠️ custom `signCommand` | ✅ | ✅ | ✅ | ✅ | **8/9** |
| C# + WinUI 3 (csbindgen) | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ (Velopack) | ✅ | ⚠️ AOT needed for <40 MB | ✅ | **8.5/9** |
| Uno Platform | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ⚠️ (unifies mac too — bigger scope) | ⚠️ | ✅ | **7.5/9** |
| .NET MAUI | ✅ | ✅ | ✅ | ✅ | ✅ | ⚠️ (less mature) | ⚠️ | ❌ | ⚠️ | 6/9 |
| Dioxus Desktop | ✅ | ✅ | ✅ | ⚠️ | ⚠️ | ⚠️ | ✅ | ✅ | ⚠️ (0.7 era — less mileage) | 6.5/9 |
| Flutter Windows | ✅ (via f_r_b) | ✅ | ✅ | ⚠️ | ✅ | ⚠️ | ✅ | ⚠️ | ✅ | 6.5/9 |
| Electron | ✅ (napi-rs) | ✅ | ✅ | ⚠️ | ✅ | ✅ | ✅ | ❌ | ✅ | 7/9 |
| `windows-app-rs` WinUI 3 | ✅ (same process) | ✅ | ✅ | ✅ (via UIA) | ✅ | ❌ (hand-roll) | ⚠️ (slow to build parity) | ✅ | ⚠️ | 6/9 |
| PWA / WebView2 shell | ⚠️ (via sidecar HTTP) | ✅ | ⚠️ (SSE) | ⚠️ | ⚠️ | ⚠️ | ⚠️ | ✅ | ✅ | 5/9 |

---

## 4. Recommendation

**Winner — C# .NET 9 + WinUI 3 + `csbindgen`** (unchanged from PLAN.md §4.1 thread-8 conclusion).

- Native accessibility, native look, MSIX + WinGet + Azure Trusted Signing are first-party.
- `csbindgen` mechanically turns `hwledger-ffi` C headers into C# P/Invoke; the same `hwledger-ffi.dll` the mac and Linux clients load.
- Velopack (or Squirrel.Windows) handles auto-update with MSIX flight channels.
- Native AOT keeps bundle + cold-start reasonable.
- The only cost is a *second* GUI language (C#) in the org — tolerable given Microsoft's investment.

**Runner-up — Tauri 2.** Pick this instead if you prioritize "smallest amount of non-Rust code in the org". Pay the a11y tax (browser UIA is good but not native-perfect) and the Azure Trusted Signing workaround.

**Not recommended — `windows-app-rs`.** The user's concern holds: too experimental in 2026, small MS bandwidth, no public production shipments, no XAML hot-reload, no packaging story. Re-evaluate in 2027 when / if MS publishes a 1.0.

### 4.1 Effort estimate (agent tool-call units)

| Candidate | Scaffolding | FFI bindings | UI parity w/ mac | Signing + updater | Wall-clock (parallel subagents) |
|---|---|---|---|---|---|
| C# + WinUI 3 | 4–6 calls (WinUI template, csbindgen, project structure) | 6–10 calls | 40–80 calls (biggest bucket — 10+ views) | 6–10 calls (MSIX + Velopack + Azure Trusted Signing) | **~60–105 calls; 3–4 subagent waves; 25–40 min wall clock** |
| Tauri 2 | 3–5 calls | 2–4 calls (already Rust) | 30–60 calls (React/Svelte components) | 8–12 calls (Azure Trusted Signing workaround is extra) | **~45–80 calls; 3 waves; 20–30 min** |
| Uno Platform | 5–8 calls | 6–10 calls | 60–100 calls (bigger target — doing mac too) | 6–10 calls | **~80–130 calls; 4–5 waves; 35–55 min** |
| `windows-app-rs` | 10–15 calls (hand-roll bootstrap + XAML composition) | 2–4 calls | 80–140 calls (no XAML markup = handwritten trees) | 15–25 calls (everything hand-rolled) | **~110–180 calls; 5–6 waves; 50–80 min + high risk of rework** |

---

## 5. Decision point framing

- **Ship a Windows binary in < 2 weeks of agent time**: **Tauri 2**. Fastest path; Azure Trusted Signing workaround is one-time pain.
- **Long-term code reuse with the SwiftUI mac client**: neither Tauri nor WinUI 3 gives you this (mac ships SwiftUI, independently). The only candidate that *does* is **Uno Platform**, and only if you commit to retiring SwiftUI — expensive and regressive on mac polish. **Recommend: don't chase this axis.**
- **Minimum maintenance burden (bundle churn, cert renewal, update infra)**: **C# + WinUI 3**. Microsoft owns the pipeline end-to-end — MSIX, WinGet, Azure Trusted Signing, Velopack. Fewest moving parts in a decade-long maintenance view.
- **Staying in-family with the user's "native per-OS" design principle** (SwiftUI on mac, WinUI 3 on Windows, Qt on Linux): **C# + WinUI 3**. This is the existing PLAN.md choice and the brief confirms it.

---

## 6. Assumptions / unknowns called out

1. No hands-on inspection of `microsoft/windows-app-rs` commit cadence in the last 30 days — conclusions are from the October 2025 Microsoft status issue, the still-open roadmap issue, and absence of a 1.0 crate. A quick `git log` pass against the repo before finalizing would sharpen §1.1.
2. No named commercial product is confirmed shipping with `windows-app-rs` as of 2026-04. If one exists (e.g., via the Windows App SDK samples fleet), it does not change the risk verdict.
3. "Azure Trusted Signing via `signCommand`" in Tauri 2 is documented in community posts (Feb 2026) but not inside `tauri.conf.json`'s first-class signing block; this may change in a Tauri 2.x release — re-check at integration time.
4. Bundle-size and cold-start numbers are **orders of magnitude** — not benchmark-grade. Confirm with prototypes before quoting externally.
5. The Uno "consolidate with mac" play assumes SwiftUI retirement; we are **not** recommending that path, but PLAN.md §12 has not explicitly closed it.
6. Dioxus Native (Blitz, wgpu) is pre-production per Dioxus's own team statements in 2025–2026 release notes. Revisit at Dioxus 1.0.
7. Effort estimates assume an agent-driven build loop; human-driven estimates would be 10–20× larger and are out of scope per `~/.claude/CLAUDE.md` "Timescales" policy.

---

## 7. Cross-links

- Sibling agent, cross-platform survey: **`./cross-platform-desktop-stacks-2026-04.md`** (Electron / electrobun / Tauri / Dioxus general comparison across all OSes).
- Upstream plan: **`../../PLAN.md`** §4.1 (component map), §12 (open questions). This brief confirms PLAN.md's `windows-app-rs → defer; C# + csbindgen → adopt` conclusion.
- FFI survey archive: **`../../docs/research/07-ffi-survey.md`**.
- Related: **`../../docs/research/06-gpu-telemetry.md`** (probe stream source) and **`../../ADR.md`** for eventual ADR-0002 "Windows client stack".

---

## Sources (as of 2026-04)

- [microsoft/windows-app-rs](https://github.com/microsoft/windows-app-rs) — Rust for the Windows App SDK
- [microsoft/windows-rs](https://github.com/microsoft/windows-rs) — Rust for Windows (WinRT/Win32)
- [windows-rs issue #3807, "Rust for Windows – October 2025"](https://github.com/microsoft/windows-rs/issues/3807) — Microsoft Rust team bandwidth note
- [windows-app-rs issue #6 — Roadmap](https://github.com/microsoft/windows-app-rs/issues/6)
- [winio / winio-winui3 on crates.io](https://crates.io/crates/winio-winui3)
- [Tauri v2 Windows code signing docs](https://v2.tauri.app/distribute/sign/windows/)
- [tauri-apps/tauri issue #9578 — Azure Trusted Signing feature request](https://github.com/tauri-apps/tauri/issues/9578)
- [Cysharp/csbindgen](https://github.com/Cysharp/csbindgen) — C# FFI generator for Rust
- [Uno Platform 6.4 release](https://platform.uno/blog/uno-platform-6-4/)
- [Uno Platform 5.5 (.NET 9, packaging)](https://platform.uno/blog/5-5/)
- [DioxusLabs/dioxus 0.6 release notes](https://dioxuslabs.com/blog/release-060/)
- [flutter_rust_bridge / rinf, LogRocket overview](https://blog.logrocket.com/using-flutter-rust-bridge-cross-platform-development/)
