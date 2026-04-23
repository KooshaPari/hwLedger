# ADR 0036 — Mobile + WearOS recording backends (Android / iOS / WearOS)

Constrains: FR-JOURNEY-001..006, forward-compat to FR-WEAR-00x (TBD)
Extends: PlayCua ADR-003 (desktop capture stacks), hwLedger ADR-0021 (cross-platform desktop)

Date: 2026-04-22
Status: Accepted (Android + iOS stubs; WearOS deferred)

## Context

hwLedger's journey-record pipeline currently targets Linux + macOS + Windows via
the PlayCua stdio-JSON-RPC sidecar (see PlayCua ADR-003). Product scope now
requires parity on **Android** and **iOS**, plus **forward-compatibility for
WearOS**. PlayCua ADR-003 is desktop-only — mobile OSes expose a different
capture and input surface, and WearOS adds ambient-mode + tiny-viewport
constraints that the desktop Linux/mac/Windows decisions do not cover.

The design goal: **one stdio-JSON-RPC contract across every backend** so that
`hwledger-journey-record --platform <p>` dispatches identically regardless of
whether the sidecar is a PlayCua binary (desktop), a Kotlin APK service
(Android / WearOS), or a WebDriverAgent + XCUITest bridge (iOS).

## Options

### Android capture

| Option | API floor | Audio? | Input driver | Notes |
|---|---|---|---|---|
| `MediaProjection` | API 21 (Android 5.0) | Yes (API 29+ for app audio) | `uiautomator` / `adb shell input` | Canonical. Requires user consent per session. |
| `adb shell screencap -p` | Any | No | `adb shell input` | Fallback, PNG-per-frame, ~2–5 fps; works without app install. |
| `adb shell screenrecord` | API 19 | No (video only) | `adb shell input` | MP4 output, capped at 3 min, no overlay. |
| Root framebuffer (`/dev/graphics/fb0`) | Any | No | any | Rejected — requires root, not portable. |

### Android input

| Option | Reach | Fidelity | Notes |
|---|---|---|---|
| `uiautomator` | API 18+ | Coord + resource-id + text | Best coverage, ships with Android. |
| `adb shell input tap/swipe/text/keyevent` | Any | Coord only | Simple, no semantic targeting. |
| Accessibility service | API 16+ | High | Requires on-device service registration; heavier. |
| Espresso | App-internal | Highest | Only works inside app-under-test process. |

### iOS capture

| Option | OS | Audio? | Driver | Notes |
|---|---|---|---|---|
| `ReplayKit` (`RPScreenRecorder`) | iOS 11+ | Yes | XCUITest via WDA | Canonical for on-device. |
| `AVCaptureSession` + screen input | macOS-host simulator | Yes | XCUITest | Used when the target is a booted simulator. |
| Xcode Instruments screen-recording | Dev-host only | Yes | manual | Rejected — not scriptable for CI. |

### iOS input

| Option | Reach | Notes |
|---|---|---|
| XCUITest + WebDriverAgent (WDA) | Real device + simulator | Canonical; Appium ecosystem. |
| `idb` (Facebook) | Simulator + jailbroken device | Useful fallback for sim-only CI. |
| Rejected: `idevicescreenshot`-only | — | Still-frame only, no input, no video. |

### WearOS capture (forward-compat only)

| Option | Notes |
|---|---|
| `MediaProjection` via paired Android phone bridge | Works but captures phone screen, not watch face. |
| `adb` over Wi-Fi debugging to watch | Same primitives as Android (`screencap`, `input`). |
| Wear 5 screen-sharing APIs | **Not yet stable** — revisit when Wear 5+ surface lands. |

## Decision

### Shipping in this batch — stubbed

- **Android backend**: Kotlin APK sidecar (`hwledger-record-sidecar-android`)
  that speaks the PlayCua stdio-JSON-RPC contract over `adb forward tcp:<host>
  tcp:<device>`. Capture via `MediaProjection`, input via `uiautomator`,
  fallback to `adb shell screencap -p` + `adb shell input` when the APK is not
  installable (e.g., locked-down OEM devices). Stubbed Rust-side; APK ships in
  a follow-up.
- **iOS backend**: XCUITest + WebDriverAgent runner wrapped by a thin Swift
  CLI (`hwledger-record-sidecar-ios`) that speaks the same stdio-JSON-RPC
  contract. Capture via `ReplayKit` on real devices, `AVCaptureSession` screen
  input on booted simulators. Shipping plan: tethered simulator for CI, real
  device for final renders. Stubbed Rust-side; Swift runner ships in a
  follow-up.

### Deferred — WearOS

Document the Android `MediaProjection` + `adb` path as the most likely
starting point once Wear 5+ exposes desktop-adjacent screen APIs. Revisit
when:

1. Wear 5 (or later) publishes a stable screen-capture API beyond the paired-
   phone proxy, **or**
2. A shipping product requirement forces the issue and a paired-phone bridge
   is accepted as interim.

### CLI surface

`hwledger-journey-record` accepts:

```
--platform {auto|linux|macos|windows|android|ios|wearos}
```

`auto` detects from `std::env::consts::OS` (desktop) and probes `adb devices`
+ `xcrun simctl list devices booted` for mobile. `wearos` is accepted but
returns an explicit "deferred — see ADR-0036" error until the WearOS backend
ships.

## Rationale

- **Contract parity**: stdio-JSON-RPC was chosen for PlayCua precisely
  because it is language-neutral. Keeping every backend behind the same
  contract means `hwledger-journey-record` does not grow a per-platform
  matrix of dispatch logic.
- **Native sidecars in native languages**: Android → Kotlin (required for
  `MediaProjection` + `uiautomator`), iOS → Swift (required for `ReplayKit`
  + `XCUITest`). Rust stays on the orchestrator side where the scripting
  policy places it.
- **Fallback-first on Android**: OEM and locked-down devices routinely block
  third-party APK install but allow `adb`. Shipping both paths means the CLI
  never hard-fails on a supported device class.
- **WearOS deferral** is honest about the state of the ecosystem — Wear 4 and
  earlier have no equivalent of `MediaProjection`, and the paired-phone
  proxy captures the wrong surface.

## Consequences

- Two new sidecar projects enter the build matrix: `hwledger-record-sidecar-android`
  (Kotlin / Gradle) and `hwledger-record-sidecar-ios` (Swift / xcodebuild).
  Both are optional build targets gated by platform-specific toolchain
  availability — hwLedger core builds without either.
- CI: Android sidecar can be built on any Linux runner with the Android SDK;
  iOS sidecar requires a macOS runner with Xcode (already present for
  SwiftUI path — ADR-0021).
- The `hwledger-journey-record` binary grows a `--platform` enum but stays
  a ≤200-LOC pre-flight + dispatch wrapper; no capture logic leaks in.
- WearOS remains an explicit TODO in docs and a loud runtime error — not a
  silent no-op (per global "fail clearly, not silently" policy).

## Revisit when

- Wear 5+ screen-capture API lands (trigger WearOS implementation).
- Android 15+ changes `MediaProjection` consent UX (current design assumes
  per-session consent prompt).
- iOS WebDriverAgent becomes unmaintained — evaluate `idb` as replacement.
- Appium 3.x drops XCUITest support (unlikely; track upstream).

## References

- PlayCua ADR-003 (desktop capture stacks) — companion record.
- hwLedger ADR-0021 (cross-platform desktop stacks) — UI layer, complements
  capture layer here.
- Android MediaProjection: https://developer.android.com/reference/android/media/projection/MediaProjection
- Android uiautomator: https://developer.android.com/training/testing/ui-automator
- iOS ReplayKit: https://developer.apple.com/documentation/replaykit
- WebDriverAgent: https://github.com/appium/WebDriverAgent
- Appium: https://appium.io
- Prior-research index (gh-remote mining): `vendor/phenotype-journeys/remotion/borrowed/prior-research-index.md`
