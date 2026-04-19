# WP21: Apple Developer Secrets Setup

Configure GitHub Actions secrets and local credentials for hwLedger macOS release automation (codesigning, notarization, Sparkle updates).

## Status

- Codesigning: READY (Developer ID Application cert installed locally)
- Notarization: WAITING (App Store Connect credentials required)
- Sparkle: WAITING (EdDSA keypair generation required)
- GitHub Release workflow: READY (release.yml deployed)

## Deliverables Installed Locally

### 1. Developer ID Application Certificate

**Status:** Already installed in user keychain

- **Team ID:** `GCT2BN8WLL`
- **Bundle ID:** `com.kooshapari.hwLedger`
- **Certificate SHA1:** `F1EB6297426AA6642983DB46C073CAB4CF8BB935`
- **CN:** Developer ID Application: Koosha Paridehpour (GCT2BN8WLL)
- **Location:** `/Users/kooshapari/Library/Keychains/login.keychain-db`

To verify locally:
```bash
security find-identity -v | grep "Developer ID Application"
# Expected output:
# ABC... "Developer ID Application: Koosha Paridehpour (GCT2BN8WLL)"
```

### 2. Codesigning Infrastructure

All scripts installed and ready:

- `apps/macos/HwLedger/entitlements.plist` — Hardened runtime entitlements
- `apps/macos/HwLedgerUITests/scripts/bundle-app.sh --codesign` — Signs app bundle
- `scripts/build-dmg.sh` — Signs DMG installer
- `.github/workflows/release.yml` — CI/CD workflow (requires cert secret)

### 3. Build and Bundle Scripts

All scripts support local testing without CI:

```bash
# Test local build + codesign + bundle
./apps/macos/HwLedgerUITests/scripts/bundle-app.sh --codesign release
# Outputs: apps/macos/build/HwLedger.app (signed)

# Test DMG creation and codesigning
./scripts/build-dmg.sh \
  --app apps/macos/build/HwLedger.app \
  --out apps/macos/build/hwLedger-1.0.0.dmg
# Outputs: apps/macos/build/hwLedger-1.0.0.dmg (signed)

# Verify signatures locally
codesign -v --verbose=2 apps/macos/build/HwLedger.app
spctl -a -t exec -vv apps/macos/build/HwLedger.app
```

## Still Required: App Store Connect Credentials

To enable notarization in CI, obtain three items from App Store Connect:

### Step 1: Create API Key in App Store Connect

1. Visit [App Store Connect](https://appstoreconnect.apple.com/) → Users and Access → Keys
2. Under "App Manager" section, click "Generate API Key"
3. Record the three values immediately (cannot be retrieved later):
   - **Key ID** — 10 alphanumeric characters (e.g., `ABC123DEFG`)
   - **Issuer ID** — UUID format (e.g., `12345678-1234-1234-1234-123456789012`)
   - **API Key (p8 file)** — Download the `.p8` file

### Step 2: Save Credentials Locally

Store credentials securely (never commit):

```bash
# Create secure directory
mkdir -p ~/.appstoreconnect/private_keys
chmod 700 ~/.appstoreconnect

# Save the .p8 file
# Rename it to match the Key ID: AuthKey_<KEY_ID>.p8
# Example: AuthKey_ABC123DEFG.p8
chmod 600 ~/.appstoreconnect/private_keys/AuthKey_*.p8

# Verify it's readable
cat ~/.appstoreconnect/private_keys/AuthKey_ABC123DEFG.p8
```

### Step 3: Add GitHub Actions Secrets

Set three secrets in the GitHub repository:

1. Go to https://github.com/KooshaPari/hwLedger/settings/secrets/actions
2. Create/update the following secrets:

| Secret Name | Value | Source |
|-------------|-------|--------|
| `APPLE_NOTARY_KEY_ID` | 10-char Key ID from Step 1 | App Store Connect |
| `APPLE_NOTARY_ISSUER_ID` | UUID Issuer ID from Step 1 | App Store Connect |
| `APPLE_NOTARY_KEY_BASE64` | Base64-encoded `.p8` file | See below |

**Encode the .p8 file for the secret:**

```bash
# Replace ABC123DEFG with your actual Key ID
cat ~/.appstoreconnect/private_keys/AuthKey_ABC123DEFG.p8 | base64 | pbcopy

# Then paste into the GitHub secret
```

### Step 4: Verify Notarization Works Locally

Test the notarization script with your credentials:

```bash
# Export credentials
export APPLE_NOTARY_KEY_ID="ABC123DEFG"
export APPLE_NOTARY_ISSUER_ID="12345678-1234-1234-1234-123456789012"
export APPLE_NOTARY_KEY_PATH="$HOME/.appstoreconnect/private_keys/AuthKey_ABC123DEFG.p8"

# Test notarize script
./scripts/notarize.sh apps/macos/build/hwLedger-1.0.0.dmg

# Expected output:
# ✓ Notarization request ID: <UUID>
# ✓ Log saved: apps/macos/build/notarize-<UUID>.log
# ✓ Stapled: apps/macos/build/hwLedger-1.0.0.dmg
# ✓ Verification with spctl passed
```

## Still Required: Sparkle EdDSA Keypair

To enable automatic in-app updates, generate Sparkle's EdDSA keypair:

### Step 1: Install Sparkle (if not already)

```bash
brew install sparkle
# or via SPM: .package(url: "https://github.com/sparkle-project/Sparkle", from: "2.6.0")
```

### Step 2: Generate EdDSA Keypair

```bash
# Create config directory
mkdir -p ~/.config/hwledger
chmod 700 ~/.config/hwledger

# Generate keys
sparkle/bin/generate_keys ~/.config/hwledger

# Expected output:
# Generated Ed25519 key pair.
# Public key: <base64-string-without-quotes>
# Private key saved to: ~/.config/hwledger/sparkle_ed25519_private.key
```

The command generates two files:
- `sparkle_ed25519_private.key` — KEEP SECURE, back up to 1Password/Bitwarden immediately
- `sparkle_ed25519_public.key` — Use in app's Info.plist

### Step 3: Secure the Private Key

```bash
chmod 600 ~/.config/hwledger/sparkle_ed25519_private.key

# IMPORTANT: Back up the private key to 1Password or Bitwarden
# If lost, all previously-released versions become un-updatable
# Secure backup: only you should have a copy
```

### Step 4: Add Public Key to App

1. Read the public key:
```bash
cat ~/.config/hwledger/sparkle_ed25519_public.key
# Output: <base64-string-without-quotes>
```

2. Add to GitHub Actions secret `SPARKLE_PUBLIC_KEY`:
   - Go to https://github.com/KooshaPari/hwLedger/settings/secrets/actions
   - Create secret: `SPARKLE_PUBLIC_KEY` with the base64 string

3. The `bundle-app.sh` script will read `SPARKLE_PUBLIC_KEY` env var and add it to Info.plist:
```bash
export SPARKLE_PUBLIC_KEY="<base64-string>"
./apps/macos/HwLedgerUITests/scripts/bundle-app.sh --codesign release
```

### Step 5: Add Sparkle Private Key to GitHub Actions

For appcast generation, add the private key as a GitHub Actions secret:

```bash
# Encode the private key
cat ~/.config/hwledger/sparkle_ed25519_private.key | base64 | pbcopy

# Create GitHub secret: SPARKLE_PRIVATE_KEY_BASE64
# https://github.com/KooshaPari/hwLedger/settings/secrets/actions
```

The `release.yml` workflow will decode it and use it to sign the appcast.

## GitHub Actions Secrets Checklist

Before tagging a release, verify all secrets are set:

```bash
gh secret list --repo KooshaPari/hwLedger
```

Expected secrets:

| Secret | Required for | Status |
|--------|--------------|--------|
| `APPLE_DEVELOPER_ID_CERT_P12` | Codesigning in CI | |
| `APPLE_DEVELOPER_ID_CERT_PASSWORD` | Codesigning in CI | |
| `APPLE_NOTARY_KEY_ID` | Notarization | PENDING |
| `APPLE_NOTARY_ISSUER_ID` | Notarization | PENDING |
| `APPLE_NOTARY_KEY_BASE64` | Notarization | PENDING |
| `SPARKLE_PUBLIC_KEY` | In-app update feed | PENDING |
| `SPARKLE_PRIVATE_KEY_BASE64` | Appcast generation | PENDING |

## Workflow Behavior After Setup

### When All Secrets Are Configured

```bash
git tag v1.0.0
git push --tags
```

CI/CD workflow will:
1. Build XCFramework (Rust) — 2-3 min
2. Build and bundle app — 1-2 min
3. Codesign app bundle — 30 sec
4. Create DMG — 1 min
5. Notarize DMG — 5-15 min (waiting for Apple)
6. Generate appcast.xml — 1 min
7. Upload to GitHub Release — 1 min
8. Commit appcast to `docs-site/public/` — triggers GitHub Pages redeploy

**Total time:** ~15-25 min

**Output:**
- GitHub Release tagged `v1.0.0` with DMG attached
- `docs-site/public/appcast.xml` updated (visible at https://kooshapari.github.io/hwLedger/appcast.xml)
- In-app updater will detect new version automatically

### If Notarization Secrets Missing

Workflow continues with codesigned (but not notarized) DMG:

```
✓ Codesigning complete
⚠ Notarization credentials not configured
ℹ DMG is safe for local distribution, but macOS will warn on first launch
ℹ To notarize: configure APPLE_NOTARY_KEY_ID, APPLE_NOTARY_ISSUER_ID, APPLE_NOTARY_KEY_BASE64
```

**Local notarization workaround:**

```bash
# After release is tagged, notarize manually
export APPLE_NOTARY_KEY_ID="ABC123DEFG"
export APPLE_NOTARY_ISSUER_ID="12345678-1234-1234-1234-123456789012"
export APPLE_NOTARY_KEY_PATH="$HOME/.appstoreconnect/private_keys/AuthKey_ABC123DEFG.p8"

./scripts/notarize.sh ~/Downloads/hwLedger-1.0.0.dmg

# Then upload the notarized DMG:
gh release upload v1.0.0 ~/Downloads/hwLedger-1.0.0.dmg --clobber
```

## Local Testing Without CI

All scripts work locally before pushing to CI:

```bash
# Build and sign locally
./scripts/build-xcframework.sh --release
./apps/macos/HwLedgerUITests/scripts/bundle-app.sh --codesign release
./scripts/build-dmg.sh \
  --app apps/macos/build/HwLedger.app \
  --out apps/macos/build/hwLedger-local.dmg

# Verify signatures
codesign -v --verbose=2 apps/macos/build/HwLedger.app
codesign -v --verbose=2 apps/macos/build/hwLedger-local.dmg

# Notarize (if credentials available)
export APPLE_NOTARY_KEY_ID="ABC123DEFG"
export APPLE_NOTARY_ISSUER_ID="12345678-1234-1234-1234-123456789012"
export APPLE_NOTARY_KEY_PATH="$HOME/.appstoreconnect/private_keys/AuthKey_ABC123DEFG.p8"
./scripts/notarize.sh apps/macos/build/hwLedger-local.dmg

# Generate appcast (if Sparkle key available)
./scripts/generate-appcast.sh apps/macos/build
cat docs-site/public/appcast.xml | head -20
```

## Troubleshooting

### Codesigning fails with "identity not found"

```
Error: identity "Developer ID Application: ..." not found
```

Fix: Developer ID cert not installed locally. Import it:

```bash
# If you have the .p12 file
security import ~/path/to/cert.p12 -k ~/Library/Keychains/login.keychain-db -P <password>

# Verify import
security find-identity -v | grep "Developer ID"
```

### Notarization fails with "invalid credentials"

```
Error: Invalid credentials (401)
```

Fix: Check Key ID format and secret encoding:

```bash
# Verify Key ID is 10 chars
echo "ABC123DEFG" | wc -c  # Should print 11 (10 chars + newline)

# Verify Issuer ID is UUID format
echo "12345678-1234-1234-1234-123456789012" | grep -E '^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$'

# Test .p8 file is valid (should start with -----BEGIN PRIVATE KEY-----)
head -1 ~/.appstoreconnect/private_keys/AuthKey_ABC123DEFG.p8
```

### DMG won't open on downloaded machine

```
"hwLedger-1.0.0.dmg" can't be opened because Apple cannot check it for malicious software
```

This happens for codesigned but **not notarized** DMGs. Fix:

1. Notarize using credentials: see **Step 4** above
2. Or locally allow via: `sudo spctl --master-disable` (not recommended)
3. Or bypass one-time: Right-click → Open → Open (first time only)

### Sparkle update check doesn't work

Verify Info.plist keys are set:

```bash
defaults read apps/macos/build/HwLedger.app/Contents/Info SUFeedURL
defaults read apps/macos/build/HwLedger.app/Contents/Info SUPublicEDKey
```

Both should return values. If not, regenerate bundle:

```bash
export SPARKLE_PUBLIC_KEY="<base64-string-from-public-key-file>"
export SPARKLE_FEED_URL="https://kooshapari.github.io/hwLedger/appcast.xml"
./apps/macos/HwLedgerUITests/scripts/bundle-app.sh --codesign release
```

## Manual Appcast Generation

If CI fails to generate appcast, do it locally:

```bash
# Ensure private key is available
ls -l ~/.config/hwledger/sparkle_ed25519_private.key
# Should show: 600 permissions

# Install generate_appcast if missing
brew install sparkle

# Generate appcast from DMG directory
./scripts/generate-appcast.sh apps/macos/build

# Verify output
cat docs-site/public/appcast.xml | head -30

# Commit and push to trigger GitHub Pages
cd docs-site
git add public/appcast.xml
git commit -m "docs: update Sparkle appcast"
git push

# Verify it's live
curl https://kooshapari.github.io/hwLedger/appcast.xml | head -10
```

## References

- [Apple Notary API](https://developer.apple.com/documentation/notaryapi)
- [Sparkle Documentation](https://sparkle-project.org/)
- [Code Signing Guide (Apple)](https://developer.apple.com/library/archive/documentation/Security/Conceptual/CodeSigningGuide/Introduction/Introduction.html)
- [hwLedger WP21 Plan](../../PLAN.md#wp21)
