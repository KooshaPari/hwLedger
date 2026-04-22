# Prior Recording / Capture Research — Sibling Repo Harvest

**Scope:** Per-OS screen capture, window isolation, input injection, hidden-desktop /
sandbox primitives, and virtual-display research mined from sibling Phenotype-org repos
that never fully shipped. Collected 2026-04-22 in preparation for the PlayCua wrap
decision (see `docs-site/architecture/adrs/0035-playcua-recording-integration.md`).

**Methodology:** `git log --all --oneline` across each sibling repo, filtered on
`capture|window|cursor|record|screen|hidden|virtual|sandbox|sidecar|rdp|xvfb|pipewire|wgc|input|inject|display`,
then per-commit `git show` on anything that looked substantive. Also read ADRs and
RESEARCH.md files. "Mirror" copy lives at
`phenotype-journeys/remotion/borrowed/prior-research-index.md`.

## Salvage taxonomy

- **inherit** — pattern or mechanism that PlayCua should consume directly (wrap or
  copy-with-attribution).
- **reference** — documented trade-off worth re-reading when tuning PlayCua's
  adapters; no copyable code.
- **ignore** — noise, scaffolding, duplicated inspiration.

---

## 1. `dino` (DINOForge) — Windows-first, 788 commits

Dino is a Unity/C# mod platform, *not* useful as a code source for Rust, but it
shipped real Windows capture + input + hidden-desktop code on bare metal and wrote
down the pitfalls. This is the **richest** sibling by a wide margin.

| SHA | Date | Topic | Salvage |
|-----|------|-------|---------|
| `3d5c025` | 2026-03-28 | **ScreenRecorderLib + WindowRecordingSource (WGC)** — replaces ffmpeg `gdigrab` with true per-window WGC capture. Requires x64 `RuntimeIdentifier`. | **inherit** — confirms PlayCua ADR-003's WGC choice; shows x64-only constraint that PlayCua's Cargo build config needs to pin. |
| `6d1d254` | 2026-03-28 | `fix(capture): replace coordinate-based gdigrab with window-title capture` | **reference** — coordinate-based capture is brittle; always resolve to HWND first. |
| `cca1721` | 2026-03-27 | `use process MainWindowHandle + GetWindowRect for Parsec virtual display` | **reference** — MainWindowHandle-from-PID lookup pattern; maps to PlayCua's `window_title` param, but PID-target is missing (gap). |
| `afaa70d` | 2026-03-25 | **Win32 `CreateDesktop` hidden-desktop launch** — full P/Invoke signatures in PowerShell, `STARTUPINFO.lpDesktop` + `dwFlags=STARTF_USESHOWWINDOW`, `wShowWindow=0`. Renders fully offscreen so user's focus is never stolen. | **inherit** — this is the primitive PlayCua's `sandbox/` referred to but didn't document. Copy the P/Invoke sigs into PlayCua's Windows sandbox adapter. |
| `e5ef720` | 2026-03-30 | `fix _launch_hidden CreateDesktop failure and desktop handle leak` | **inherit** — `CloseDesktop` must be called on the parent's handle AFTER `CreateProcess` returns; leak pattern documented. |
| `1cf771e` | 2026-03-25 | `P/Invoke IntPtr signatures and System.Drawing assembly loading in hidden_desktop_test.ps1` | **reference** — integer-width pitfalls when marshalling Win32; Rust bindings via `windows` crate avoid these, but worth flagging. |
| `051d310` | 2026-03-28 | **Nefarius/MTT Virtual Display Driver** launch — dedicated VDD, Unity `-monitor` flag, NOT Parsec (user's Parsec is a separate device). | **inherit** — virtual-display primitive for "record without using the real display"; complements CreateDesktop. PlayCua has neither today. |
| `45f53ea` | 2026-03-28 | `VDD-based game launch using Parsec Virtual Display` | **reference** — superseded by 051d310 but documents the Unity `-monitor N` flag behaviour. |
| `4e48666` | 2026-03-28 | `revert(mcp): remove Parsec VDD targeting — use dedicated VDD solution` | **reference** — the revert itself encodes the "don't co-opt user-owned VDDs" rule. |
| `739bdd8` | 2026-03-27 | `default game_launch_test to hidden=True + -popupwindow flag` | **reference** — `-popupwindow` removes window chrome from Unity apps; analog for hwLedger Tauri may be `decorations: false`. |
| `f094a46` | 2026-03-25 | **wire `bare-cua-native.exe` into GameCaptureHelper as primary screenshot path** | **inherit** — exact precedent: sibling project already spawned PlayCua (bare-cua) from C# via stdio. Validates the wrap direction. |
| `34b3088` | 2026-03-25 | `complete bare-cua integration, hidden desktop prototype, session docs` | **reference** — session doc showing bare-cua integration end-to-end. |
| `ea85b5e` | 2026-03-25 | **black-frame retry logic** in DXGI screenshot capture | **inherit** — DXGI returns empty frames in the first N iterations; PlayCua ADR-003 doesn't mention this. Add a retry loop to the Windows adapter. |
| `c120931` | 2026-03-25 | `fix BitBlt screen capture` | **reference** — BitBlt fails silently on composited windows (DWM); confirms PlayCua's choice to skip GDI. |
| `6f2c6de` | 2026-03-24 | `second-instance bypass, neural TTS, window capture, ADRs/specs` | **reference** — rolls up ADRs 013/015/017/018. |
| `ADR-013` | 2026-03-24 | **Win32 named-mutex bypass** for single-instance apps (hwLedger Tauri likely has this constraint too on Windows). | **reference** — if we ever need two hwLedger instances running while recording, this doc has the mutex-enumeration recipe. |
| `ADR-015` | native-menu-injector | Win32 menu injection via `AppendMenu`/`TrackPopupMenu` without taking focus. | **reference** — complements `enigo` with window-chrome automation for non-PlayCua paths. |
| `ADR-018` | second-instance-bypass | BepInEx doorstop + `winhttp.dll` proxy to intercept before Unity mutex check. | **ignore** for hwLedger (Unity-specific). |
| `afae04b` | 2026-03-27 | `add game_screenshot*.png to gitignore` | **ignore**. |
| `70b95c1` | 2026-03-26 | `/health endpoint + Windows Task Scheduler auto-start service` | **reference** — if PlayCua daemon ever needs a Windows service wrapper. |

**Dino-specific Windows gold that PlayCua's ADR-003 currently misses:**

1. **DXGI black-frame retry** (`ea85b5e`) — PlayCua's fallback chain should retry N=3
   frames before declaring capture failed.
2. **Hidden Win32 desktop via `CreateDesktop` + `lpDesktop` in `STARTUPINFO`**
   (`afaa70d`, `e5ef720`) — PlayCua's `sandbox/` directory should document and
   expose this as a Windows sandbox strategy; today it's Linux-first.
3. **Nefarius MTT Virtual Display Driver** (`051d310`) — separate from, and
   complementary to, WGC capture: lets you spawn an app onto a display the user
   cannot see. PlayCua ADR-003 discusses capture APIs but not "where to put the
   window being captured."
4. **`-popupwindow` / chromeless launch flag** (`739bdd8`) — application-level hint
   that pairs with hidden-desktop launches.
5. **x64-only runtime pin** (`3d5c025`) — ScreenRecorderLib, and by extension any
   WGC-based capture, requires `-target x86_64-pc-windows-msvc` builds; document
   this in PlayCua's Cargo/README.

## 2. `KDesktopVirt` — 5 commits + 40+ undocumented Python prototypes

A Python + Docker + UI-TARS desktop-virtualisation stub that never shipped a real
session. The git log is thin (post-merge consolidation), but the in-tree files
record legitimate research.

| Item | Topic | Salvage |
|-----|-------|---------|
| `ADR-001-container-virtualization.md` | **Container vs VM-based VDI** for agent-driven desktop sessions — containers 60-80% cheaper, 2-3s cold start vs 30-60s, gVisor/Kata available for security. | **reference** — if hwLedger ever runs journey recordings in CI on a Linux fleet, container-based desktop sessions are the cost-optimal path. Not urgent for macOS/Windows local recording. |
| `ADR-002-ai-automation.md` | **UI-TARS as primary automation engine** with X11/xdotool/wmctrl fallback. Four modes: Normal Scripting, MCP Live Scripting, ACI Agent Interface, Desktop Recording. | **reference** — architectural shape of "four-mode automation" is worth noting if hwLedger journey-record grows into agent-authored journeys. |
| `ADR-003-mcp-interface.md` | MCP server wrapping KDesktopVirt commands. | **ignore** — already covered by PlayCua's openrpc contract. |
| `kvirtualstage-legacy/CAPTURE_EXECUTION_REPORT.md` | **Post-mortem of a recording system that produced zero artefacts** because Docker wasn't running. | **reference** — canonical example of why hwLedger must *fail loudly* if the PlayCua binary is missing (no silent "skipped capture" paths). |
| `desktop_usage_recorder.py` | Python recorder using a fictional `ComprehensiveAutomationPlatform.recording_engine.start_desktop_recording` backend. | **ignore** — the backend never existed. |
| `934c7c7` | 2026-04-04 | `chore: merge KVirtualStage - desktop automation consolidation` | **reference** — KDesktopVirt absorbed KVirtualStage; the legacy dir is the only survivor of KVS. |

## 3. `KVirtualStage` (GitHub remote only — 19 commits)

Cloned shallowly to `/tmp/KVirtualStage-mine`. All research was merged into
KDesktopVirt; the commit messages below provide provenance.

| SHA | Date | Topic | Salvage |
|-----|------|-------|---------|
| `0212465` | initial | `🎭 Initial release of KVirtualStage - Playwright for Desktop Automation` | **reference** — "Playwright for Desktop" framing; the name KVirtualStage came from this aspiration. |
| `050a603` | initial | `🚀 Initial KVirtualStage Implementation` | **ignore** — stub. |
| `93ddcc8` | evolution plan | `Add Comprehensive KVirtualStage Evolution Plan` | **reference** — roadmap doc only. |
| `787ae91` | real ACI | `Implement Real Agent-Computer Interface with Virtual Desktop Automation` | **ignore** — never executed against a real desktop per CAPTURE_EXECUTION_REPORT.md. |
| `3a1a182` | mcp | `Fix MCP tools display and demonstrate actual working functionality` | **ignore**. |

**Net KVS contribution:** the *name* and the aspiration — "Playwright for Desktop." No
salvageable code. The useful consolidation lives inside KDesktopVirt.

## 4. `KaskMan` (3 commits)

| SHA | Topic | Salvage |
|-----|-------|---------|
| `169587b` | `docs: add README/SPEC/PLAN` | **ignore** — idealistic OpenClaw predecessor; no capture code. |
| `806eaf7` | `ci(legacy-enforcement): add legacy tooling anti-pattern gate` | **ignore**. |
| `72ae463` | `chore(deps-dev): bump @typescript-eslint/eslint-plugin` | **ignore**. |

**Verdict:** confirmed as archived / reference-only per MEMORY.md.

## 5. `agslag-docs` (6 commits)

Archived router/orchestrator docs for a multi-agent platform. Unrelated to
capture; mined for completeness.

| SHA | Topic | Salvage |
|-----|-------|---------|
| `ADR-001-router-orchestrator.md` | Centralised agent router; NATS-backed. | **ignore** (not capture). |
| `ADR-002-mcp-protocol.md` | MCP wire format. | **ignore** — PlayCua uses JSON-RPC 2.0 already. |
| `ADR-003-multi-agent-communication.md` | NATS vs Kafka vs gRPC streaming. | **ignore**. |
| `eeaaf8b` / `da1866e` / `aceb1f5` | Initial README/SPEC/PLAN imports. | **ignore**. |
| `ebe54f0` | `ci(legacy-enforcement): legacy tooling anti-pattern gate (WARN mode)` | **ignore**. |
| `079cc7d` | `chore: add AgilePlus scaffolding` | **ignore**. |

**Verdict:** no capture research. Listed here so the audit trail is exhaustive.

---

## Aggregate commit count mined

- `dino`: 20 commits directly relevant (of 788 total)
- `KDesktopVirt`: 5 commits (of 5) + 3 ADRs + 1 post-mortem
- `KVirtualStage`: 5 commits (of 19) via shallow clone
- `KaskMan`: 3 commits (all scanned; 0 salvageable)
- `agslag-docs`: 6 commits + 3 ADRs (all scanned; 0 salvageable)

**Total: 42 sibling-repo items reviewed; 27 meet the "relevant" bar; 10 flagged
`inherit` for PlayCua upstream issues.**

## Top-5 salvageable snippets

1. **Hidden Win32 desktop (`CreateDesktop` + `STARTUPINFO.lpDesktop`)** — dino
   `afaa70d`. Sandbox primitive PlayCua lacks; full P/Invoke reproduced in the
   commit. Provenance: `repos/dino@afaa70d:src/Tools/DinoforgeMcp/dinoforge_mcp/server.py`.
2. **DXGI black-frame retry loop** — dino `ea85b5e`. PlayCua ADR-003 doesn't
   mention the empty-frame warmup; Windows adapter should retry ≥3 frames.
3. **Nefarius MTT virtual display driver** — dino `051d310`. Separate from
   capture: gives you a display to aim the app at. Complements WGC.
4. **WindowRecordingSource (WGC video, not just stills)** — dino `3d5c025`.
   PlayCua's `start_recording` stub should use WGC on Windows (currently unclear
   which adapter drives video-mode).
5. **`x86_64-pc-windows-msvc` x64-only runtime pin** — dino `3d5c025`. PlayCua's
   WGC path must not be compiled for x86.

## Cross-Project Reuse Opportunities

- **PlayCua upstream (source of truth)**: should consume items 1-5 above as ADR
  addenda to its own ADR-003. hwLedger will file the gaps as PlayCua issues;
  hwLedger itself does not fork.
- **phenotype-journeys**: the recording flow in phenotype-journeys currently
  terminal-only (VHS); this index documents the GUI-capture stack that future
  `journey-record --target window://` work will plug into.
- **`hwledger-gui-recorder` crate**: the existing macOS SCK sidecar should be
  refactored into a thin shim over `playcua screenshot`/`playcua start_recording`
  once ADR 0035 lands, reducing per-OS surface area maintained inside hwLedger.

---

## 6. KooshaPari orchestration stack (PlayCua → NVMS → PhenoCompose → BytePort)

Layer map (top-down consumer → bottom-up primitive):

```
BytePort       ← LLM-generated portfolio UX over `odin.nvms` IaC    [phase-4-defer]
PhenoCompose   ← process-compose + NVMS backend                     [phase-3]
NanoVMS        ← MicroVM (Firecracker) / WASM (wasmtime) / OCI     [phase-2]
PlayCua        ← capture + input JSON-RPC primitive                 [inherit-now]
```

hwLedger's `journey-record` lives at the PlayCua layer today (Phase 1, ADR 0035).
ADR 0037 documents how `--isolate {host|container|microvm|wasm}` dispatches into
NVMS in Phase 2, PhenoCompose orchestrates multi-service journeys in Phase 3,
and BytePort publishes them as portfolio artifacts in Phase 4.

| Item | Source | Topic | Salvage |
|------|--------|-------|---------|
| PlayCua `ADR-003` per-OS capture paths | `KooshaPari/PlayCua@ADR-003` | stdio JSON-RPC + xcap/enigo + WGC/SCK/X11 adapters | **inherit-now** — consumed by hwLedger ADR 0035; covered exhaustively in sections 1-5 above. |
| NanoVMS 3-tier isolation | `KooshaPari/HexaKit` (living code) / `KooshaPari/nanovms` (deprecated standalone) | MicroVM (Firecracker) / WASM (wasmtime) / OCI container runtimes behind one API; Cilium networking; multi-platform (linux/amd64, linux/arm64, darwin shim). | **phase-2** — consumed by ADR 0037's `--isolate {container\|microvm\|wasm}` port. Container tier lands first (covers Streamlit + CLI); MicroVM is the Firecracker revisit trigger; WASM tier is experimental. |
| NanoVMS `@trace VM-NNN` invariants | deprecated nanovms README | requirements-table format (`VM-001` .. `VM-NNN`) — every adapter method must cite a VM-NNN invariant. | **phase-2** — the `@trace VM-NNN` annotation pattern mirrors hwLedger's FR-SHARED-NNN convention; reuse verbatim in the isolate port's rustdoc. |
| PhenoCompose `journey.phenocompose.yml` | `KooshaPari/PhenoCompose` (Go wrapper over process-compose + NVMS backend) | one YAML file spins up N NVMS sandboxes (one per service) with shared virtual network; PlayCua attaches to the `playcua-recorder` service's virtual display. | **phase-3** — design-docs stub at `apps/journey-phenocompose/example.yml` documents the surface; runtime wiring deferred until NVMS container tier lands. |
| BytePort `odin.nvms` IaC + LLM-UX | `KooshaPari/BytePort` README | single-file IaC (NAME/DESCRIPTION/SERVICES with PATH/PORT/ENV); GitHub repo → AWS deploy via Spin; ChatGPT-generated portfolio cards embed a live instance. | **phase-4-defer** — hwLedger will expose a `PublishAdapter` trait with a `ByteportAdapter` stub (`tools/journey-record/src/publish/byteport.rs`) so Phase 4 is a single-file impl swap. Not on the critical path. |

### Verdicts by phase

- **Phase 1 (inherit-now)**: PlayCua wrap (ADR 0035). `--isolate host` default. User's real desktop captured directly; documented limitation that cursor/notifications can leak into recordings.
- **Phase 2 (NVMS container tier)**: `--isolate container` dispatches PlayCua inside an NVMS OCI container running Xvfb/Wayland. Unblocks clean CLI + Streamlit recordings without polluting the user's main desktop. Trigger: NVMS container-tier GA in HexaKit.
- **Phase 3 (PhenoCompose multi-service)**: `journey.phenocompose.yml` drives N NVMS services (hwledger-server + streamlit + playcua-recorder). Trigger: PhenoCompose 0.1 release; Phase 2 container tier in production.
- **Phase 4 (BytePort publish, deferred)**: one-command portfolio publish from `.hwledger/journeys/*.verified.json` → BytePort card. Trigger: explicit user ask; not on the hwLedger critical path.

**Revisit triggers:** NVMS loses its Firecracker backend (Phase 2 WASM-only fallback), PlayCua stalls (swap the primitive layer), PhenoCompose deprecates process-compose (swap orchestrator).
