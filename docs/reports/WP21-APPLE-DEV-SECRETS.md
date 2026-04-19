# WP21 — Apple Developer + Sparkle — LOCAL release pipeline

**Release pipeline runs locally via Lefthook, not in CI.** macOS-latest runners are billed at ~10× Linux minutes and the KooshaPari account has no budget for that after the free tier. Local signing is also strictly better for key material safety — the `.p8` and Developer ID cert never leave the dev machine.

## What's wired

| Item | Location | Purpose |
|---|---|---|
| Developer ID Application cert | login keychain | codesign via `security find-identity` |
| `AuthKey_<KEY_ID>.p8` | `~/.appstoreconnect/private_keys/` | notarytool via App Store Connect API |
| Sparkle Ed25519 private key | `~/.config/hwledger/sparkle_ed25519_private.key` (chmod 600) | sign appcast.xml |
| Sparkle Ed25519 public key | `OIZuw+nbKJZkyDQ/QFUWyEOdHXC2UEWka/4UUdMGeMg=` | committed to Info.plist as `SUPublicEDKey` |
| `lefthook.yml` + `scripts/release.sh` | repo root | orchestrate build → sign → notarize → appcast |

All 7 GitHub Actions secrets have been **removed** — they're not needed any more.

## Credentials

- **Team ID**: `GCT2BN8WLL`
- **Bundle ID**: `com.kooshapari.hwLedger`
- **Notary Issuer ID**: `d2e8211c-db4c-4733-9e1e-1db18e244ba3`
- **Notary Key ID**: _pending new `.p8`_ (the 2025-03-30 key at `AuthKey_2GU85D9582.p8` is revoked or orphaned — `notarytool store-credentials` rejects it)

## Outstanding blocker

Generate a fresh App Store Connect API key:

1. https://appstoreconnect.apple.com/access/integrations/api → **Team Keys** → **+** → name it `hwledger-notarize` with access role "Developer" (minimum for notarytool).
2. Download the `.p8` **once** (the only chance).
3. Drop into `~/Downloads/AuthKey_<NEW_KEY_ID>.p8`.
4. Run:
   ```bash
   mv ~/Downloads/AuthKey_*.p8 ~/.appstoreconnect/private_keys/
   chmod 600 ~/.appstoreconnect/private_keys/AuthKey_*.p8
   xcrun notarytool store-credentials hwledger \
       --key ~/.appstoreconnect/private_keys/AuthKey_<NEW_KEY_ID>.p8 \
       --key-id <NEW_KEY_ID> \
       --issuer d2e8211c-db4c-4733-9e1e-1db18e244ba3
   ```
   The last command validates the key against Apple's API and stores it as a keychain profile named `hwledger`. If it prints `Profile stored`, you're good.

## Release flow (after new `.p8` lands)

```bash
export APPLE_NOTARY_KEY_ID=<NEW_KEY_ID>
export APPLE_NOTARY_ISSUER_ID=d2e8211c-db4c-4733-9e1e-1db18e244ba3

git tag v0.1.0-alpha
git push --tags
```

The `pre-push` Lefthook hook detects the `v*` tag and runs `scripts/release.sh v0.1.0-alpha`. That does:
1. `cargo build --release -p hwledger-ffi` for arm64
2. `cbindgen` + `xcodebuild -create-xcframework`
3. `swift build` + bundle into `.app`
4. `codesign` with Developer ID, `--options runtime --timestamp`
5. `hdiutil create` the DMG + `codesign`
6. `xcrun notarytool submit --wait` (~5–15 min on Apple's side)
7. `xcrun stapler staple`
8. Sparkle `generate_appcast` signed with the Ed25519 private key
9. Writes signed `.dmg` and `appcast.xml` into `apps/macos/build/`

After the script succeeds:

```bash
gh release create v0.1.0-alpha \
  apps/macos/build/hwLedger-0.1.0-alpha.dmg \
  --notes "first signed release" \
  --repo KooshaPari/hwLedger

git add docs-site/public/appcast.xml
git commit -m 'chore(release): appcast for v0.1.0-alpha'
git push
```

GH Pages redeploys → users' Sparkle pings find the new version → auto-update kicks in.

## Lefthook other jobs

- **pre-commit**: rustfmt on staged .rs files, `.p8/.p12/.pem/.key` file-add guard, trufflehog secret scan.
- **pre-push**: per-crate fmt check, workspace clippy `-D warnings`, full test suite, Swift build (if XCFramework present), release-tag detection.

Bypass hooks (emergency only): `git commit --no-verify` or `git push --no-verify`. Don't.

## Why no Mac App Store

Developer ID distribution only — GitHub Releases serves the DMG, Sparkle handles updates. No App Store review delays, no 30% cut, and the user keeps single-binary control. See ADR-0002 (oMlx fork) for the equivalent opinionated stance on inference runtime.
