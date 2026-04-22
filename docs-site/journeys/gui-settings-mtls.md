# GUI Journey: Settings mTLS Admin Cert

This page documents the **settings-gui-mtls** journey, which exercises the mTLS admin flow in Settings — generating an admin client cert and copying its PEM to clipboard.

## Overview

**Journey ID:** `settings-gui-mtls`
**Status:** Implemented (placeholder artefacts — real recording pending on user Mac)
**Last Updated:** 2026-04-19

## Keyframe walkthrough

<Shot src="/gui-journeys/settings-gui-mtls/keyframes/frame_002.png"
      caption="Settings screen visible — System section in view"
      size="small" align="right" />

<Shot src="/gui-journeys/settings-gui-mtls/keyframes/frame_004.png"
      caption="Cursor clicks 'Generate Admin Cert'"
      size="small" align="left" />

<Shot src="/gui-journeys/settings-gui-mtls/keyframes/frame_005.png"
      caption="Cert block populates — PEM text visible"
      size="small" align="right" />

## What you'll see

- App launches on Planner; cursor moves down the sidebar to **Settings**.
- Settings opens on the System section; user scrolls down past Fleet Server + Logging until **mTLS Admin** is in view.
- "Generate Admin Cert" shows a spinner and a status line `issuing cert, CN=admin@local ...`.
- PEM text area populates with `-----BEGIN CERTIFICATE-----` block and a SHA256 thumbprint row.
- User clicks **Copy PEM**; button inverts briefly and a bottom-center toast reads `Copied admin cert to clipboard`.
- Footer updates: `Last issued: just now — valid 90d`.

<JourneyViewer manifest="/gui-journeys/settings-gui-mtls/manifest.verified.json" />

## What to watch for

- **Cert issuance feedback** — the button's spinner should disappear and the PEM area should populate in a single state flip; no intermediate "empty" PEM.
- **Clipboard action** — the toast only appears after the clipboard write actually succeeds. If the user revokes Pasteboard access the toast must switch to an error variant (not silently succeed).
- **Validity footer** — reflects the issuance policy (`FR-MTLS-003` / WP22 mTLS admin).

## Reproduce

```bash
cd apps/macos/HwLedgerUITests
./scripts/bundle-app.sh --no-codesign debug

swift test --filter SettingsMTLSJourneyTests/testSettingsGUIMTLS

cd ../../..
bash docs-site/scripts/sync-journey-artefacts.sh
```

## Source

- Test: [`apps/macos/HwLedgerUITests/Tests/SettingsMTLSJourneyTests.swift`](https://github.com/KooshaPari/hwLedger/blob/main/apps/macos/HwLedgerUITests/Tests/SettingsMTLSJourneyTests.swift)
- Manifest: [`docs-site/public/gui-journeys/settings-gui-mtls/manifest.json`](/gui-journeys/settings-gui-mtls/manifest.json)
- Verified manifest: [`docs-site/public/gui-journeys/settings-gui-mtls/manifest.verified.json`](/gui-journeys/settings-gui-mtls/manifest.verified.json)
- Recording: [`recording.mp4`](/gui-journeys/settings-gui-mtls/recording.mp4) · [`preview.gif`](/gui-journeys/settings-gui-mtls/preview.gif)
