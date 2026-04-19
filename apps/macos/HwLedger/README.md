# hwLedger macOS Application

SwiftUI-native macOS app for the hwLedger capacity planner.

## Prerequisites

- macOS 14+ (Sonoma or later)
- Swift 5.10+
- The Swift Package wrapper (`../hwledger-swift/`) and its XCFramework already built

## Building the XCFramework

Before building this app, ensure the XCFramework is built from the repository root:

```bash
./scripts/build-xcframework.sh --release
```

This creates `apps/macos/xcframework/HwLedgerCore.xcframework`.

## Running the App

From this directory:

```bash
swift run HwLedgerApp
```

This compiles and launches the app. The first run takes longer due to linking.

## Testing

```bash
swift test
```

Tests include:
- Core version validation (FR-UI-001)
- Device detection (FR-UI-002)
- Gauge color threshold logic (FR-UI-001)
- Stacked bar proportion computation (FR-UI-001)

## Architecture

### Project Structure

```
HwLedger/
├── Package.swift                 # Swift Package manifest (executable target)
├── Sources/HwLedgerApp/
│   ├── HwLedgerApp.swift        # @main entry point
│   ├── ContentView.swift        # NavigationSplitView + sidebar
│   ├── AppState.swift           # @Observable root state
│   ├── Screens/                 # Six screen stubs
│   │   ├── LibraryScreen.swift  # Model library (WP18)
│   │   ├── PlannerScreen.swift  # Hero screen with sliders (WP18)
│   │   ├── FleetScreen.swift    # Device grid + telemetry (WP19)
│   │   ├── RunScreen.swift      # Inference runner (WP19)
│   │   ├── LedgerScreen.swift   # Audit log (WP20)
│   │   └── SettingsScreen.swift # Configuration
│   └── Components/              # Reusable UI components
│       ├── Gauge.swift          # Green/yellow/red threshold gauge
│       └── StackedBar.swift     # Memory breakdown stacked bar
└── Tests/
    └── HwLedgerAppTests/
        └── AppStateTests.swift
```

### Navigation Model

- **Root**: `NavigationSplitView` with a sidebar listing six screens
- **State**: `@Observable AppState` injected via `.environment()`
- **Selection**: Default to `.planner` screen
- **Icons**: System SF Symbols for each screen

### Screens

1. **Library** — Model grid (local GGUF, MLX, Ollama, HF) with search/filter. Stub ready for WP18.
2. **Planner** (hero) — Four sliders (seq_len, users, batch, quant) + live StackedBar breakdown. Calls `HwLedger.plan()` on each slider update with a test DeepSeek-V3 config.
3. **Fleet** — Device list from `HwLedger.detectDevices()` with live refresh button. Telemetry details (WP19).
4. **Run** — Inference launcher (token streaming, predicted vs actual memory). Stub ready for WP19.
5. **Ledger** — Audit log timeline (event-sourced hash chain). Stub ready for WP20.
6. **Settings** — Read-only display of core version + placeholders for HF token, Tailscale, SSH identities.

### Components

#### Gauge.swift

Horizontal bar with live percentage fill. Colors by threshold:
- Green: `value <= greenThreshold` (default 0.6)
- Yellow: `greenThreshold < value <= yellowThreshold` (default 0.85)
- Red: `value > yellowThreshold`

Public static function `colorForValue(_:green:yellow:)` is testable without SwiftUI rendering.

#### StackedBar.swift

Horizontal stacked bar for memory breakdown (Weights | KV | Runtime | Prefill | Free). Each segment shows label + value in MB below the bar. Pure proportional computation; all test-covered.

## Dependency Graph

```
HwLedgerApp
├── ContentView               (NavigationSplitView + sidebar)
├── AppState                  (Observable state holder)
│   └── HwLedger.plan()       (C FFI via Swift Package)
│   └── HwLedger.detectDevices()
│   └── HwLedger.coreVersion()
├── Screens/
│   ├── LibraryScreen
│   ├── PlannerScreen         (uses Gauge, StackedBar, plan results)
│   ├── FleetScreen           (displays appState.devices)
│   ├── RunScreen
│   ├── LedgerScreen
│   └── SettingsScreen        (displays appState.coreVersion)
└── Components/
    ├── Gauge                 (pure SwiftUI, testable color logic)
    └── StackedBar            (pure SwiftUI, testable proportions)
```

## Lines of Code

- **App Entry**: 26 LOC (HwLedgerApp.swift)
- **Navigation**: 38 LOC (ContentView.swift)
- **State Management**: 37 LOC (AppState.swift)
- **Screens**: 213 LOC (6 screens, mostly stubs)
  - PlannerScreen: 140 LOC (sliders + plan result rendering)
  - FleetScreen: 42 LOC (device list)
  - SettingsScreen: 45 LOC (version + placeholders)
  - Others: 26 LOC each (stubs)
- **Components**: 115 LOC (Gauge + StackedBar + helpers)
- **Tests**: 62 LOC (version, devices, gauge, stacked bar)
- **Total**: 491 LOC

## Known Limitations (WP18/WP19)

- Library screen: no model grid UI (placeholder text)
- Planner: uses static test config (DeepSeek-V3). Config picker deferred to WP18.
- Run screen: no inference launcher UI
- Ledger screen: no audit log display
- Fleet screen: no live telemetry sampling (HwLedger.sample() ready; UI comes WP19)

## Next Steps (WP18-WP20)

**WP18 (Planner + Library hero UX):**
- Add model picker (file browser + HF Hub search)
- Replace static config with selected model
- Add per-layer heatmap to Planner
- Export planner snapshot as vLLM/llama.cpp/MLX flags

**WP19 (Fleet + Run telemetry):**
- Live device telemetry grid (HwLedger.sample() polling loop)
- Inference launcher UI (prompt input, token stream)
- Predicted vs actual memory panel on Run screen

**WP20 (Ledger + polish):**
- Event log display (hash-chain traversal UI)
- Codesigning + notarization
- DMG + Sparkle auto-update integration

## Building for Release

Not in scope for WP17. WP21+ includes:
- Codesigning with Apple Developer certificate
- Notarization via Apple Gatekeeper
- DMG distribution with Sparkle framework

## Troubleshooting

### "error: can't find module 'HwLedger'"

The Swift Package dependency is not linked. Ensure `../hwledger-swift/Package.swift` exists and the path is correct.

### "error: can't find binary 'HwLedgerCore'"

The XCFramework was not built. From the repo root:

```bash
./scripts/build-xcframework.sh --release
```

### Build hangs on first run

Swift Package Manager is resolving dependencies and compiling. This is normal and takes 30-90s on first build.

## License

Apache 2.0 — same as hwLedger core.
