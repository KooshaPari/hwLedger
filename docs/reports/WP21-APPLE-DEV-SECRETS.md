# WP21 — Apple Developer + Sparkle secrets setup

Current status of the release pipeline: **4 of 4 credential groups configured in GitHub Secrets; 1 open diagnostic (notary key validation)**.

## Secrets already configured in GitHub Actions

| Secret | Source | Set at |
|---|---|---|
| `APPLE_NOTARY_ISSUER_ID` | App Store Connect → Users & Access → Integrations → Team Keys | 2026-04-19 |
| `APPLE_NOTARY_KEY_ID` | App Store Connect, derived from `AuthKey_<KEYID>.p8` filename | 2026-04-19 |
| `APPLE_NOTARY_KEY_BASE64` | `base64 < ~/.appstoreconnect/private_keys/AuthKey_2GU85D9582.p8` | 2026-04-19 |
| `SPARKLE_PUBLIC_KEY` | generated 2026-04-19, stored in repo + committed Info.plist placeholder | 2026-04-19 |
| `SPARKLE_PRIVATE_KEY_BASE64` | local generate, base64-encoded | 2026-04-19 |
| `APPLE_DEVELOPER_ID_CERT_P12` | `security export -t identities -f pkcs12 -k login.keychain-db` | 2026-04-19 |
| `APPLE_DEVELOPER_ID_CERT_PASSWORD` | random 32-byte base64 generated at export time | 2026-04-19 |

Values — **do not paste private secrets into PRs**:

- **Team ID**: `GCT2BN8WLL`
- **Bundle ID**: `com.kooshapari.hwLedger`
- **Notary Key ID**: `2GU85D9582`
- **Notary Issuer ID**: `d2e8211c-db4c-4733-9e1e-1db18e244ba3`
- **Sparkle public key**: `OIZuw+nbKJZkyDQ/QFUWyEOdHXC2UEWka/4UUdMGeMg=`
- **Sparkle private key**: `~/.config/hwledger/sparkle_ed25519_private.key` (chmod 600). **Back this up to 1Password / Bitwarden immediately.**

## Still needs attention

### 1. `.p8` notarization key validation is failing — verify it isn't revoked

`xcrun notarytool store-credentials` validation failed against the key dated 2025-03-30 — it may have been revoked. Verify at https://appstoreconnect.apple.com/access/integrations/api . If the row shows "Revoked", regenerate:

1. Click + → generate new key named `hwledger-notarize`.
2. Download the `.p8` immediately (only chance).
3. Drop into `~/Downloads/AuthKey_<NEWID>.p8`.
4. `mv ~/Downloads/AuthKey_*.p8 ~/.appstoreconnect/private_keys/ && chmod 600 ~/.appstoreconnect/private_keys/AuthKey_*.p8`.
5. Tell me the new Key ID and I'll update the 3 GitHub secrets (`APPLE_NOTARY_KEY_ID` + `APPLE_NOTARY_KEY_BASE64` + re-run `xcrun notarytool store-credentials hwledger`).

### 2. (Optional) store the Apple creds as a notarytool keychain profile for local use

```bash
xcrun notarytool store-credentials hwledger \
    --key ~/.appstoreconnect/private_keys/AuthKey_<KEY_ID>.p8 \
    --key-id <KEY_ID> \
    --issuer d2e8211c-db4c-4733-9e1e-1db18e244ba3
```

## First release flow

```bash
git tag v0.1.0-alpha
git push --tags
```

CI will: build XCFramework → bundle + codesign → create + sign DMG → notarize via notarytool → generate appcast.xml signed with Sparkle private key → upload DMG + appcast to GitHub Release → commit appcast to `docs-site/public/` (triggers docs redeploy → users' Sparkle pings find v0.1.0-alpha).

## Graceful degradation

If `APPLE_NOTARY_*` secrets are missing, the release workflow uploads a **codesigned-but-not-notarized** DMG with a warning. Users will see a Gatekeeper prompt on first launch.

If `SPARKLE_PRIVATE_KEY_BASE64` is missing, no appcast.xml is generated — Sparkle will detect no updates but also can't be fooled into installing anything.
