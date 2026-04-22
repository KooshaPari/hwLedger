# ADR 0035 — PlayCua as the per-OS recording / capture / input-injection primitive

- **Status:** Accepted
- **Date:** 2026-04-22
- **Deciders:** Koosha Pari, hwLedger infra agents
- **Supersedes (partial):** Implicit direction taken by agent dispatch `a3773560`
  (per-OS GUI recorder built from scratch inside hwLedger using ScreenCaptureKit
  on macOS, `xvfb`/PipeWire on Linux, WinRDP on Windows). That direction did not
  land tracked backends (`scsk/`, `xvfb/`, `winrdp/` were never committed); this
  ADR cancels it.
- **Related:** PlayCua ADR-001 (Hexagonal), ADR-002 (stdio JSON-RPC), ADR-003
  (Cross-Platform Capture). hwLedger ADR 0010 (TTS), 0011 (Remotion
  compositing), 0023 (macOS GPU telemetry).
- **Research index:** [`docs/research/prior-recording-research-index.md`](../../../docs/research/prior-recording-research-index.md)

## Context

hwLedger journey recording needs three orthogonal per-OS primitives:

1. **Window-scoped screen capture** (PNG stills + MP4 video) that does not grab
   the entire display.
2. **Synthetic mouse-cursor sprite** that moves/clicks in a scripted pattern
   during the recording *without* disturbing the user's real cursor (the
   "cursor-mux" requirement).
3. **Hidden / sandboxed execution surface** so recordings can happen on an
   offscreen or virtual display while the user continues to work.

PlayCua (sibling repo `/Users/kooshapari/CodeProjects/Phenotype/repos/PlayCua`)
already solves (1) and (2) end-to-end for screenshots and input injection:

- **Hexagonal Rust workspace** (PlayCua ADR-001) with platform-specific capture
  adapters tiered behind `xcap` as the cross-platform fallback (PlayCua
  ADR-003). Target latency <50ms.
- **`enigo`**-based input injection for keyboard + mouse at `input.move`,
  `input.click`, `input.key`, `input.type`, `input.scroll`. Synthesized events
  are independent of the user's real cursor — this IS the cursor-mux primitive.
- **stdio JSON-RPC 2.0** over newline-delimited JSON (PlayCua ADR-002), i.e.
  identical framing to LSP. Zero port management, zero socket cleanup, clean
  process-death semantics.
- A `sandbox/` subtree (Linux-first today) hinting at future execution-surface
  isolation.

(3) is partially built in PlayCua but **not surfaced in JSON-RPC** yet, and
**video-mode recording is entirely absent** (`start_recording` /
`stop_recording` are not in `contracts/openrpc.json` or
`native/src/ipc/dispatcher.rs`). A survey of sibling repos (see research
index) surfaces concrete Windows-first solutions for both gaps, notably from
`dino` (`3d5c025` — WGC `WindowRecordingSource` video), `afaa70d` and `e5ef720`
— Win32 `CreateDesktop`-based hidden-desktop launch with documented
P/Invoke pitfalls), and `051d310` (Nefarius MTT Virtual Display Driver).

## Decision

**Wrap PlayCua. Don't re-implement per-OS capture inside hwLedger.**

Concretely:

1. Ship `tools/journey-record/` — a Rust stdio JSON-RPC client (landed in commit
   `d176788`). It spawns PlayCua's native binary (located via `PLAYCUA_BIN`,
   `~/.cache/hwledger/bin/playcua`, or a `cargo run` fallback against a sibling
   PlayCua checkout) and drives recording + cursor-mux over stdio.
2. Consume PlayCua as an **external binary**, not as a Cargo dependency. PlayCua
   remains the canonical source of truth; hwLedger does not fork.
3. File upstream issues on PlayCua for the gaps required to light up hwLedger's
   journey-record flow (enumerated in §Open Upstream Gaps below). hwLedger
   tracks a pinned PlayCua version in an out-of-tree lockfile / release notes —
   not in the Cargo workspace — so that recording reproducibility is auditable
   without coupling the Rust dependency graph.
4. The existing `crates/hwledger-gui-recorder` macOS ScreenCaptureKit shim
   stays for now (it is already wired into docs-site capture pipelines) and
   will be **refactored into a thin adapter over PlayCua** once the upstream
   video-mode methods ship. No new per-OS backends land inside hwLedger.

## Alternatives considered

### A. Build per-OS from scratch (what `a3773560` was heading toward)

Rejected. hwLedger is not a screen-capture project. Maintaining three full
adapter sets (macOS SCK, Linux PipeWire/XShm, Windows WGC), plus sandbox +
virtual-display drivers, would dominate hwLedger's maintenance envelope and
duplicate PlayCua's existing hexagonal architecture. The research-index survey
confirms every pattern we'd need was already prototyped (dino, KDesktopVirt,
KVirtualStage) without surviving past proof-of-concept.

### B. Fork PlayCua

Rejected. PlayCua is active and single-maintainer; a fork fragments the ADR-003
capture-adapter vetting that upstream has already done. Forking is only the
right call if PlayCua stalls or refuses features we need — see `Revisit`
below.

### C. Bypass PlayCua and call `xcap` / `enigo` / `sandbox` crates directly from
hwLedger

Rejected. Loses PlayCua's hexagonal plugin system, the tiered fallback chain
(WGC → xcap on Windows; CGWindowListCreateImage → xcap on macOS), the vetted
adapter-selection logic, and the sandbox primitive. Also re-introduces the
per-OS maintenance burden without the upstream insulation that a binary
boundary provides.

### D. Use a different computer-use framework (trycua/cua, AutoKit, etc.)

Rejected. PlayCua is our own sibling, already in the Phenotype-org reuse
target list, and already exposes the exact stdio contract dino's `bare-cua`
integration (`f094a46`) validated end-to-end.

## Consequences

### Positive

- hwLedger's per-OS capture surface collapses to a single spawn-and-roundtrip
  call path. No direct `#[cfg(target_os = "...")]` capture code in hwLedger.
- PlayCua's ADR-003 fallback chain (WGC → xcap; CG → xcap) applies transparently.
- stdio framing means zero network attack surface for the recorder.
- The cursor-mux requirement is met by PlayCua's `input.move` + `input.click`;
  `enigo` synthesizes events independent of the user's real pointer.
- Sandbox primitive is a single upstream flip away once PlayCua exposes it over
  RPC.

### Negative / trade-offs

- **PlayCua binary distribution** — hwLedger builds must either find a system
  PlayCua, a cached release, or fall back to `cargo run` in the sibling repo.
  The `locate()` method enforces loud failure when `PLAYCUA_BIN` is set to a
  bogus path.
- **Version pinning lives out of tree** — a PlayCua version mismatch will
  surface as JSON-RPC `-32601 Method not found` at runtime. Mitigation: `ping`
  on `Session::spawn` returns PlayCua's `CARGO_PKG_VERSION`; we will surface it
  in recording manifests.
- **Video-mode recording is an upstream gap** — see §Open Upstream Gaps.
- **Integration test currently `#[ignore]`** — lights up only on macOS with a
  PlayCua release that implements `start_recording`.

### Open Upstream Gaps (to be filed on PlayCua)

1. **`start_recording` / `stop_recording`** (video mode). Windows: WGC
   `WindowRecordingSource` per `dino@3d5c025`. macOS: AVAssetWriter piped from
   SCK frames per hwLedger's existing `crates/hwledger-gui-recorder`. Linux:
   PipeWire screen-share + GStreamer `videoconvert ! x264enc ! mp4mux`.
2. **DXGI black-frame retry loop** on the Windows screenshot path
   (`dino@ea85b5e`). PlayCua's ADR-003 does not mention this; the first N
   DXGI frames can be empty and must be retried before declaring failure.
3. **Hidden Win32 desktop sandbox strategy** (`dino@afaa70d`, `e5ef720`). The
   `sandbox/` directory is Linux-first today. Copy the `CreateDesktop` +
   `STARTUPINFO.lpDesktop` P/Invoke pattern (the commit has the full signature
   including the `CloseDesktop` leak fix).
4. **Nefarius MTT Virtual Display Driver** adapter (`dino@051d310`) — separate
   from capture; gives a display to point the target app at so the real display
   stays free for the user.
5. **PID- and bundle-id-scoped window lookup**. Today `screenshot` takes only
   `window_title`. `dino@cca1721` demonstrates `MainWindowHandle +
   GetWindowRect` from a PID on Windows; macOS would use
   `CGWindowListCopyWindowInfo` filtered by `kCGWindowOwnerPID`.
6. **x64-only runtime pin for Windows** (`dino@3d5c025`). Any WGC-based path
   requires `x86_64-pc-windows-msvc`; document in PlayCua's Cargo config and
   README.
7. **`-popupwindow` / chromeless launch hint** (`dino@739bdd8`) for
   `process.launch`, to pair with hidden-desktop launches.

### Revisit

This ADR is revisited if **any** of:

- PlayCua stalls for > 90 days without a release touching the gaps above.
- hwLedger needs a recording feature PlayCua's maintainers decline to merge.
- Container-based desktop sessions (KDesktopVirt ADR-001) become hwLedger CI's
  primary execution surface for journey recordings, in which case we may need
  to host PlayCua inside that session rather than wrap it from the host.

## Cancellation of a3773560's direction

No `scsk/`, `xvfb/`, or `winrdp/` backends landed under agent dispatch
`a3773560` (verified via `git log --oneline` on this branch). The existing
`tools/cli-journey-record` is a terminal-tape pre-flight for VHS and is
orthogonal to window capture; it is retained unchanged. The new
`tools/journey-record` absorbs the intended per-OS recorder scope.

## Tests

- **Rust unit (lib):** `rpc_roundtrip_ping_and_record` exercises a full JSON-RPC
  roundtrip (ping → start_recording → input.move → stop_recording) against an
  in-memory mock transport. Additional tests cover target parsing, cursor-track
  parsing, and PlayCua-binary resolution fail-loud behaviour. All 4 tests pass:
  `cargo test -p hwledger-journey-record --lib`.
- **Rust integration (`#[ignore]`):** `tests/integration_finder_record.rs`
  spawns real PlayCua, records a Finder window for 3 seconds on macOS, and
  asserts the MP4 exists and is non-empty. Currently ignored because PlayCua's
  `start_recording` method is the upstream gap described above; the test will
  un-gate when PlayCua ships that method.

## References

- Research index: `docs/research/prior-recording-research-index.md`.
- PlayCua ADR-001, ADR-002, ADR-003: `../../../../PlayCua/ADR-00{1,2,3}-*.md`
  (sibling repo).
- Dino commits: `3d5c025`, `afaa70d`, `e5ef720`, `ea85b5e`, `cca1721`,
  `051d310`, `f094a46`, `739bdd8`.
- Prior ledger commits: `d176788` (tool landing), `5cc04e7` (research index).
