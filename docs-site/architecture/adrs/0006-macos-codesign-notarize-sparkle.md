# ADR-0006: macOS v1 Distribution — Developer ID + Notarization + Sparkle

**Status:** Accepted (WP21)

**Date:** 2026-04-18

## Context

hwLedger v1 MVP targets macOS first for native desktop distribution. The app must be distributed directly from GitHub Releases (not via Mac App Store) with:

1. **Secure delivery**: Code signing + notarization to prevent macOS Gatekeeper warnings
2. **Automatic updates**: In-app updater for seamless new version discovery
3. **Non-intrusive UX**: Silent updates with opt-in checks ("Check for Updates…" menu)

## Decision

Use the following macOS distribution strategy for v1 and beyond:

### 1. Codesigning

**Approach:** Developer ID Application certificate (not Mac App Store).

- **Identity:** Developer ID Application: Koosha Paridehpour (GCT2BN8WLL)
- **Scope:** Sign app bundle, DMG installer, and all nested binaries
- **Hardened runtime:** Enable hardened-runtime entitlements (CS.disable-library-validation only, no sandbox)
- **Timestamp:** Always use Apple's timestamp servers for signature longevity

**Rationale:**
- Developer ID allows distribution outside Mac App Store sandbox restrictions
- App needs network (fleet communication), subprocess spawning (MLX sidecar), and file I/O
- Hardened runtime protects against runtime exploitation without app store overhead
- Timestamped signatures remain valid even after certificate expiration

### 2. Notarization

**Approach:** Apple Notary API (xcrun notarytool) with EdDSA key.

- **Credentials:** App Store Connect Issuer ID + Key ID (not legacy staple-mode)
- **Files:** DMG and .app bundles submitted before distribution
- **Wait semantics:** Synchronous wait (--wait) in CI to block on results
- **Failure handling:** Logs saved to `apps/macos/build/notarize-<ID>.log` for diagnostics

**Rationale:**
- Notarization is required for Big Sur+ Gatekeeper trust on M1/M2/M3 Macs
- EdDSA credentials are time-bound, more secure than certificate-based
- Notary API (not legacy stapler) recommended by Apple as of 2023
- Synchronous waits in CI allow roll-forward/roll-back on failure

### 3. Updates

**Approach:** Sparkle 2.6+ with EdDSA signatures (not DSA).

- **Feed:** Self-hosted appcast.xml at https://kooshapari.github.io/hwLedger/appcast.xml (via GitHub Pages)
- **Signing:** appcast signed with EdDSA private key, app verifies with public key in Info.plist
- **Trigger:** User clicks "Check for Updates…" in app menu (not silent background checks initially)

**Rationale:**
- Sparkle is the de-facto standard for macOS app updates (used by Transmit, Sequel Pro, etc.)
- EdDSA is more secure than legacy DSA, future-proof
- GitHub Pages hosting is free, reliable, and tied to existing release workflow
- Manual update checks are safer for MVP; silent updates can be added post-v1

### 4. Distribution

**Approach:** GitHub Releases with notarized DMG.

- **Artifact:** Single `hwLedger-<VERSION>.dmg` uploaded to release
- **Rollout:** Users download from GitHub Release, double-click DMG, drag to Applications
- **No app store:** No direct integration with Mac App Store (reduces review friction, full control over pricing/licensing model)

**Rationale:**
- GitHub Releases provides free, redundant CDN delivery
- DMG is the standard distribution container for macOS (familiar UX)
- Simplifies release workflow (no app store submission delays)
- Clear pricing control: open-source under Apache 2.0 (free) if needed

## Implementation

### Scripts Delivered (WP21)

| Script | Purpose |
|--------|---------|
| `apps/macos/HwLedger/entitlements.plist` | Hardened runtime entitlements (no sandbox) |
| `apps/macos/HwLedgerUITests/scripts/bundle-app.sh --codesign` | Build, bundle, and codesign app |
| `scripts/build-dmg.sh` | Create and codesign DMG using create-dmg (brew) or hdiutil fallback |
| `scripts/notarize.sh` | Submit to Apple Notary API, wait, staple, verify |
| `scripts/generate-appcast.sh` | Generate Sparkle appcast.xml with EdDSA signatures |
| `.github/workflows/release.yml` | CI/CD: build → codesign → DMG → notarize → appcast → release |

### Local Development

All scripts work locally for testing (no CI required):

```bash
# Local codesign + bundle
./apps/macos/HwLedgerUITests/scripts/bundle-app.sh --codesign release

# Local DMG creation + codesign
./scripts/build-dmg.sh --app apps/macos/build/HwLedger.app --out build/hwLedger-1.0.0.dmg

# Local notarization (with credentials)
export APPLE_NOTARY_KEY_ID="ABC123DEFG"
export APPLE_NOTARY_ISSUER_ID="12345678-..."
export APPLE_NOTARY_KEY_PATH=~/.appstoreconnect/private_keys/AuthKey_ABC123DEFG.p8
./scripts/notarize.sh build/hwLedger-1.0.0.dmg

# Local appcast generation
./scripts/generate-appcast.sh apps/macos/build
```

### Sparkle Integration

- **Package.swift:** Added `.package(url: "https://github.com/sparkle-project/Sparkle", from: "2.6.0")`
- **HwLedgerApp.swift:** Wired `SPUStandardUpdaterController`, added "Check for Updates…" menu item
- **Info.plist keys:**
  - `SUFeedURL` — appcast.xml location (set by `bundle-app.sh`)
  - `SUPublicEDKey` — EdDSA public key for appcast verification (set by `bundle-app.sh` if `SPARKLE_PUBLIC_KEY` env var provided)

## Prerequisites

### Already Installed

- Developer ID Application certificate (in login keychain)
- Rust FFI + Swift bindings

### Required for Notarization (from user)

- App Store Connect Issuer ID (UUID)
- Key ID (10-char alphanumeric)
- `.p8` API key file (saved to `~/.appstoreconnect/private_keys/`)

### Required for Sparkle (user must generate)

- EdDSA keypair (one-time via `sparkle/bin/generate_keys`)
- Private key saved to `~/.config/hwledger/sparkle_ed25519_private.key` (chmod 600, backed up securely)

See [docs/reports/WP21-APPLE-DEV-SECRETS.md](../../docs/reports/WP21-APPLE-DEV-SECRETS.md) for step-by-step setup.

## Workflow: Tag to Release

### User pushes tag
```bash
git tag v1.0.0
git push --tags
```

### CI/CD runs (15-25 min):

1. Checkout (1 min)
2. Import Developer ID cert from GitHub secret (30 sec)
3. Build XCFramework (2-3 min)
4. Bundle + codesign app (2 min)
5. Create + codesign DMG (1 min)
6. Notarize DMG (5-15 min, waiting on Apple)
7. Generate appcast.xml (1 min)
8. Upload to GitHub Release (1 min)

### Outputs:

- GitHub Release `v1.0.0` with `hwLedger-1.0.0.dmg` (notarized)
- `docs-site/public/appcast.xml` (auto-committed, triggers GitHub Pages redeploy)

### Users get:

- DMG from GitHub Release, double-click to install
- In-app "Check for Updates…" discovers new versions from appcast.xml
- Seamless future updates via Sparkle

## Tradeoffs

### Chosen: Developer ID + Notarization (not App Store)

**Pros:**
- Full control over pricing, licensing, update cadence
- No app store review process or sandboxing restrictions
- Direct GitHub distribution, familiar to open-source users

**Cons:**
- Users must trust developer's certificate (but notarization mitigates)
- First-launch warning if user has not enabled security policy
- Manual distribution (not discoverable in App Store)

### Chosen: GitHub Pages appcast (not dedicated server)

**Pros:**
- Free, reliable CDN
- Integrated with GitHub Actions
- No infrastructure to maintain

**Cons:**
- Static XML only (no analytics, versioning database)
- GitHub outage = update checks fail (acceptable for MVP)

### Chosen: EdDSA (not DSA or RSA)

**Pros:**
- Modern, post-quantum resistant curve
- Smaller keys, faster verification
- Sparkle v2.6+ native support

**Cons:**
- Legacy RSA-signed appcasts incompatible (but v1 only, fresh start)

## Future Enhancements

- **Silent updates:** Remove user prompt, auto-update on background thread (post-v1)
- **Delta updates:** Sparkle can deliver only changed bytes (reduces bandwidth)
- **Analytics:** Log appcast fetches to measure adoption (privacy-preserving)
- **Staged rollout:** Use GitHub Releases pre-release flag to limit initial audience
- **Windows/Linux:** Velopack (MSIX), AppImage/Flatpak (deferred post-v1)

## References

- [WP21 Plan](../../PLAN.md#wp21)
- [WP21 Setup Guide](../../docs/reports/WP21-APPLE-DEV-SECRETS.md)
- [Sparkle 2.6 Docs](https://sparkle-project.org/)
- [Apple Notary API](https://developer.apple.com/documentation/notaryapi/)
- [Code Signing Guide (Apple)](https://developer.apple.com/library/archive/documentation/Security/Conceptual/CodeSigningGuide/)
