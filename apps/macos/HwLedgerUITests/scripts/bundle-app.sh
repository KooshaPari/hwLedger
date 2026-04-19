#!/bin/bash
# Bundle the HwLedger executable into a macOS .app bundle with codesigning.
# Usage: ./scripts/bundle-app.sh [--codesign] [release|debug]
# Options:
#   --codesign   Enable codesigning and notarization checks (default: on)
#   --no-codesign Disable codesigning
# Arguments:
#   release|debug Build configuration (default: release)
# Example:
#   ./scripts/bundle-app.sh --codesign release
#   ./scripts/bundle-app.sh --no-codesign debug

set -euo pipefail

# === Help ===
if [[ "${1:-}" == "--help" ]] || [[ "${1:-}" == "-h" ]]; then
    cat << 'HELP'
bundle-app.sh: Create and codesign a macOS app bundle for hwLedger.

Usage:
  ./scripts/bundle-app.sh [OPTIONS] [CONFIG]

Options:
  --codesign      Enable codesigning (default)
  --no-codesign   Disable codesigning
  --help          Show this help

Arguments:
  CONFIG          Build configuration: release or debug (default: release)

Environment variables:
  BUNDLE_ID          CFBundleIdentifier (default: com.kooshapari.hwLedger)
  VERSION            CFBundleShortVersionString (read from git describe)
  CODESIGN_IDENTITY  Signing identity (default: Developer ID Application)
  SPARKLE_PUBLIC_KEY Sparkle EdDSA public key (optional, for SUPublicEDKey)
  SPARKLE_FEED_URL   Sparkle update feed URL (optional, for SUFeedURL)

Example:
  BUNDLE_ID=com.example.app ./scripts/bundle-app.sh --codesign release

HELP
    exit 0
fi

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
HWLEDGER_SRC="${PROJECT_ROOT}/../HwLedger"
BUILD_DIR="${PROJECT_ROOT}/../../build"
BUNDLE_DIR="${BUILD_DIR}/HwLedger.app"

# === Parse options ===
CODESIGN=1
CONFIG="release"

while [[ $# -gt 0 ]]; do
    case "$1" in
        --codesign)
            CODESIGN=1
            shift
            ;;
        --no-codesign)
            CODESIGN=0
            shift
            ;;
        release|debug)
            CONFIG="$1"
            shift
            ;;
        --help|-h)
            # Already handled above
            exit 0
            ;;
        *)
            echo "Error: unknown option '$1'"
            exit 1
            ;;
    esac
done

# === Configuration ===
BUNDLE_ID="${BUNDLE_ID:-com.kooshapari.hwLedger}"
TEAM_ID="GCT2BN8WLL"
CODESIGN_IDENTITY="Developer ID Application: Koosha Paridehpour (${TEAM_ID})"

# Version from git describe, fallback to 0.1.0
VERSION=$(cd "${PROJECT_ROOT}/../../.." && git describe --tags --always 2>/dev/null || echo "0.1.0")
VERSION="${VERSION#v}"  # Strip leading 'v' if present

# Git short SHA for CFBundleVersion
BUILD_VERSION=$(cd "${PROJECT_ROOT}/../../.." && git rev-parse --short HEAD 2>/dev/null || echo "1")

# Sparkle keys (optional, read from env)
SPARKLE_PUBLIC_KEY="${SPARKLE_PUBLIC_KEY:-}"
SPARKLE_FEED_URL="${SPARKLE_FEED_URL:-}"

echo "=== hwLedger Bundle Script ==="
echo "Configuration:  ${CONFIG}"
echo "Bundle ID:      ${BUNDLE_ID}"
echo "Version:        ${VERSION}"
echo "Build version:  ${BUILD_VERSION}"
echo "Codesign:       $([ $CODESIGN -eq 1 ] && echo 'enabled' || echo 'disabled')"
if [ -n "${SPARKLE_FEED_URL}" ]; then
    echo "Sparkle feed:   ${SPARKLE_FEED_URL}"
fi
echo ""

echo "Building HwLedger (${CONFIG})..."
cd "${HWLEDGER_SRC}"
swift build -c "${CONFIG}"

# Determine executable path
if [ "${CONFIG}" = "release" ]; then
    EXEC_PATH="${HWLEDGER_SRC}/.build/release/HwLedgerApp"
else
    EXEC_PATH="${HWLEDGER_SRC}/.build/debug/HwLedgerApp"
fi

if [ ! -f "${EXEC_PATH}" ]; then
    echo "Error: executable not found at ${EXEC_PATH}"
    exit 1
fi

# Create bundle structure
echo "Creating app bundle at ${BUNDLE_DIR}..."
rm -rf "${BUNDLE_DIR}"
mkdir -p "${BUNDLE_DIR}/Contents/MacOS"
mkdir -p "${BUNDLE_DIR}/Contents/Resources"

# Copy executable
cp "${EXEC_PATH}" "${BUNDLE_DIR}/Contents/MacOS/HwLedger"
chmod +x "${BUNDLE_DIR}/Contents/MacOS/HwLedger"

# Build Info.plist content dynamically
echo "Generating Info.plist..."

# Start with base plist structure
PLIST_DICT='
    <key>CFBundleDevelopmentRegion</key>
    <string>en</string>
    <key>CFBundleExecutable</key>
    <string>HwLedger</string>
    <key>CFBundleIdentifier</key>
    <string>'${BUNDLE_ID}'</string>
    <key>CFBundleInfoDictionaryVersion</key>
    <string>6.0</string>
    <key>CFBundleName</key>
    <string>HwLedger</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleShortVersionString</key>
    <string>'${VERSION}'</string>
    <key>CFBundleVersion</key>
    <string>'${BUILD_VERSION}'</string>
    <key>LSApplicationCategoryType</key>
    <string>public.app-category.utilities</string>
    <key>LSMinimumSystemVersion</key>
    <string>14.0</string>
    <key>LSUIElement</key>
    <false/>
    <key>NSHighResolutionCapable</key>
    <true/>
    <key>NSPrincipalClass</key>
    <string>NSApplication</string>
    <key>NSRequiresIPhoneOS</key>
    <false/>'

# Add Sparkle keys if provided
if [ -n "${SPARKLE_FEED_URL}" ]; then
    PLIST_DICT="${PLIST_DICT}"'
    <key>SUFeedURL</key>
    <string>'${SPARKLE_FEED_URL}'</string>'
fi

if [ -n "${SPARKLE_PUBLIC_KEY}" ]; then
    PLIST_DICT="${PLIST_DICT}"'
    <key>SUPublicEDKey</key>
    <string>'${SPARKLE_PUBLIC_KEY}'</string>'
fi

# Write Info.plist
cat > "${BUNDLE_DIR}/Contents/Info.plist" << EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
${PLIST_DICT}
</dict>
</plist>
EOF

echo "✓ Info.plist created"

# === Codesign if enabled ===
if [ $CODESIGN -eq 1 ]; then
    echo "Codesigning app bundle..."

    if ! command -v codesign &>/dev/null; then
        echo "Error: codesign not found. Ensure Xcode is installed."
        exit 1
    fi

    codesign --sign "${CODESIGN_IDENTITY}" \
        --options runtime \
        --timestamp \
        --entitlements "${PROJECT_ROOT}/../HwLedger/entitlements.plist" \
        --deep "${BUNDLE_DIR}"

    if [ $? -ne 0 ]; then
        echo "Error: codesigning failed"
        exit 1
    fi

    echo "✓ App signed with ${CODESIGN_IDENTITY}"

    # Verify signature
    echo "Verifying signature..."
    codesign --verify --strict --verbose=2 "${BUNDLE_DIR}"
    if [ $? -ne 0 ]; then
        echo "Error: signature verification failed"
        exit 1
    fi
    echo "✓ Signature verified"

    # System policy verification (may require notarization for full success)
    echo "Running System Integrity Protection checks..."
    if command -v spctl &>/dev/null; then
        spctl -a -t exec -vv "${BUNDLE_DIR}" || {
            echo "Note: spctl check failed (expected before notarization)"
        }
    fi
else
    echo "Codesigning disabled (--no-codesign)"
fi

echo ""
echo "=== Bundle Complete ==="
echo "Location:   ${BUNDLE_DIR}"
echo "Executable: ${BUNDLE_DIR}/Contents/MacOS/HwLedger"
echo "Ready for: packaging into DMG, notarization, or direct execution"
