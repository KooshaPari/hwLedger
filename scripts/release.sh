#!/usr/bin/env bash
# Local release pipeline — runs on the developer machine, not CI.
# Produces: signed+notarized DMG, signed appcast.xml, ready to upload to GH Release.
#
# Usage: ./scripts/release.sh v0.1.0-alpha
#
# Reads credentials from local env / keychain — no network calls to CI secrets.
# Required env:
#   APPLE_NOTARY_KEY_ID      — App Store Connect Key ID (10-char)
#   APPLE_NOTARY_ISSUER_ID   — UUID
#   APPLE_NOTARY_KEY_PATH    — default ~/.appstoreconnect/private_keys/AuthKey_<KEY_ID>.p8
#   SPARKLE_PRIVATE_KEY_PATH — default ~/.config/hwledger/sparkle_ed25519_private.key
#   DEVELOPER_ID_SIGNER      — default "Developer ID Application: Koosha Paridehpour (GCT2BN8WLL)"
#
# After success:
#   gh release create <tag> apps/macos/build/*.dmg --notes-from-tag

set -euo pipefail

TAG="${1:-}"
if [[ -z "$TAG" ]]; then
  echo "usage: $0 <tag>  (e.g. v0.1.0-alpha)" >&2
  exit 1
fi
if ! [[ "$TAG" =~ ^v[0-9] ]]; then
  echo "tag must start with v<digit>: $TAG" >&2
  exit 1
fi

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

# Credentials: prefer `notarytool store-credentials hwledger` keychain profile.
# Falls back to explicit APPLE_NOTARY_{KEY_ID,ISSUER_ID} env vars.
NOTARY_PROFILE="${APPLE_NOTARY_KEYCHAIN_PROFILE:-hwledger}"
if xcrun notarytool history --keychain-profile "$NOTARY_PROFILE" >/dev/null 2>&1; then
  echo "[creds] using notarytool keychain profile: $NOTARY_PROFILE"
else
  : "${APPLE_NOTARY_KEY_ID:?set APPLE_NOTARY_KEY_ID or store a keychain profile via: xcrun notarytool store-credentials ${NOTARY_PROFILE} --key … --key-id … --issuer …}"
  : "${APPLE_NOTARY_ISSUER_ID:?APPLE_NOTARY_ISSUER_ID env var required}"
  APPLE_NOTARY_KEY_PATH="${APPLE_NOTARY_KEY_PATH:-$HOME/.appstoreconnect/private_keys/AuthKey_${APPLE_NOTARY_KEY_ID}.p8}"
  if [[ ! -f "$APPLE_NOTARY_KEY_PATH" ]]; then
    echo "missing .p8: $APPLE_NOTARY_KEY_PATH" >&2; exit 1
  fi
fi
SPARKLE_PRIVATE_KEY_PATH="${SPARKLE_PRIVATE_KEY_PATH:-$HOME/.config/hwledger/sparkle_ed25519_private.key}"
DEVELOPER_ID_SIGNER="${DEVELOPER_ID_SIGNER:-Developer ID Application: Koosha Paridehpour (GCT2BN8WLL)}"
if [[ ! -f "$SPARKLE_PRIVATE_KEY_PATH" ]]; then
  echo "missing Sparkle private key: $SPARKLE_PRIVATE_KEY_PATH" >&2
  echo "generate once: python3 -c 'from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PrivateKey; from cryptography.hazmat.primitives import serialization; import base64; sk=Ed25519PrivateKey.generate(); print(base64.b64encode(sk.private_bytes(encoding=serialization.Encoding.Raw, format=serialization.PrivateFormat.Raw, encryption_algorithm=serialization.NoEncryption())).decode())' > \"$SPARKLE_PRIVATE_KEY_PATH\" && chmod 600 \"$SPARKLE_PRIVATE_KEY_PATH\"" >&2
  exit 1
fi
if ! security find-identity -v -p codesigning 2>&1 | grep -q "$DEVELOPER_ID_SIGNER"; then
  echo "Developer ID signer not found in keychain: $DEVELOPER_ID_SIGNER" >&2
  exit 1
fi

echo "=== hwLedger local release pipeline ==="
echo "Tag:            $TAG"
echo "Signer:         $DEVELOPER_ID_SIGNER"
echo "Notary:         keychain-profile=$NOTARY_PROFILE${APPLE_NOTARY_KEY_ID:+ / key-id=$APPLE_NOTARY_KEY_ID}"
echo

# 1. Build XCFramework (arm64-only by default; set HWLEDGER_UNIVERSAL=1 for fat).
echo "[1/6] Building XCFramework"
./scripts/build-xcframework.sh --release

# 2. Bundle + codesign .app.
echo "[2/6] Bundling + codesigning .app"
./apps/macos/HwLedgerUITests/scripts/bundle-app.sh --codesign

APP_PATH="$REPO_ROOT/apps/build/HwLedger.app"
if [[ ! -d "$APP_PATH" ]]; then
  echo "bundle missing: $APP_PATH" >&2
  exit 1
fi

# 3. Build + sign DMG.
echo "[3/6] Building + signing DMG"
DMG_PATH="$REPO_ROOT/apps/build/hwLedger-${TAG#v}.dmg"
./scripts/build-dmg.sh --app "$APP_PATH" --out "$DMG_PATH"

# 4. Notarize + staple. notarize.sh auto-detects the `hwledger` keychain
# profile set by `notarytool store-credentials`; env-var fallback works too.
echo "[4/6] Submitting to Apple notary (may take 5-15 min)"
./scripts/notarize.sh "$DMG_PATH"

# 5. Generate signed appcast.
echo "[5/6] Generating signed appcast"
python3 "$REPO_ROOT/scripts/sign-appcast.py" \
  "$DMG_PATH" \
  "${TAG#v}" \
  "$SPARKLE_PRIVATE_KEY_PATH" \
  "$REPO_ROOT/docs-site/public/appcast.xml"

# 6. (appcast already written to docsite by sign-appcast.py)
echo "[6/6] Appcast published to docsite"

echo
echo "=== Release pipeline complete ==="
echo "DMG:     $DMG_PATH"
echo "Appcast: $REPO_ROOT/docs-site/public/appcast.xml"
echo
echo "Next:"
echo "  git add docs-site/public/appcast.xml"
echo "  git commit -m 'chore(release): appcast for $TAG'"
echo "  git push && git push --tags"
echo "  gh release create $TAG \"$DMG_PATH\" --notes-from-tag --repo KooshaPari/hwLedger"
