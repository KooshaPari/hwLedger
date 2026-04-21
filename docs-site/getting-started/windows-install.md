# Install hwLedger on Windows

The Windows client is a Tauri 2 app backed by the same Rust core as the
SwiftUI macOS client. See
[Windows client strategy (2026-04)](../research/windows-client-strategy-2026-04.md)
for the decision record.

## Install the signed MSI (recommended)

1. Download `hwLedger-<version>-x64.msi` from the latest
   [GitHub release](https://github.com/KooshaPari/hwLedger/releases).
2. Double-click the MSI. SmartScreen should display
   *"hwLedger Contributors"* as the signer (Azure Trusted Signing).
3. Accept the UAC prompt. The installer drops a
   `C:\Program Files\hwLedger\hwLedger.exe` plus an Add/Remove Programs entry.
4. Launch from the Start menu — first run opens the Planner screen.

> [!NOTE]
> If SmartScreen blocks the install ("Unrecognized app"), the certificate has
> not yet propagated. Use the portable `.zip` below until the reputation
> catches up — **do not** click through SmartScreen warnings for unsigned
> installers from arbitrary sources.

## Portable install (unsigned)

For air-gapped machines or CI runners:

1. Grab `hwLedger-<version>-portable.zip` from the same release page.
2. Unzip anywhere. Launch `hwLedger.exe` directly.
3. The app self-contains the WebView2 bootstrapper; it will install WebView2
   on Windows 10 hosts missing it (Windows 11 ships it by default).

## Linux (AppImage / deb)

The same Tauri bundle ships Linux artifacts. Install via:

```bash
sudo apt install ./hwLedger_<version>_amd64.deb
# or
chmod +x hwLedger-<version>.AppImage && ./hwLedger-<version>.AppImage
```

## Build from source

You'll need:

- Rust stable (`rustup install stable`)
- Node 20+ and pnpm 9+
- `cargo-tauri` v2: `cargo install tauri-cli --version '^2'`
- On Linux: `libwebkit2gtk-4.1-dev libssl-dev libgtk-3-dev
  libayatana-appindicator3-dev librsvg2-dev`
- On Windows: MSVC Build Tools 2022, WebView2 runtime

Then from the repo root:

```bash
cd apps/windows/hwledger-tauri
pnpm install
pnpm tauri build
```

Artifacts land in `src-tauri/target/release/bundle/`.

## Accessibility

The UI runs in WebView2 on Windows, which exposes a full UIA tree derived
from the DOM a11y roles. Keyboard nav:

| Key                  | Action                            |
|----------------------|-----------------------------------|
| `Tab` / `Shift+Tab`  | Cycle focus                       |
| `Arrow keys` on tabs | Switch between Planner/Probe/…    |
| `Enter` / `Space`    | Activate focused control          |
| `Esc`                | Close modal dialogs               |

Screen readers (Narrator, NVDA) read each screen's `role="tabpanel"` region
and the stacked-bar chart via its `aria-label` summary. File an issue if a
control is missing its label.
