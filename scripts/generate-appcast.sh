#!/bin/bash
# Generate Sparkle appcast.xml from signed DMGs.
# Requires: Sparkle's generate_appcast tool, EdDSA private key
#
# Usage: ./scripts/generate-appcast.sh <dmg-directory>
#
# Environment variables:
#   SPARKLE_PRIVATE_KEY_PATH Path to EdDSA private key
#                            Default: ~/.config/hwledger/sparkle_ed25519_private.key
#   APPCAST_OUTPUT_PATH      Where to write appcast.xml
#                            Default: docs-site/public/appcast.xml
#
# Example:
#   ./scripts/generate-appcast.sh apps/macos/build

set -euo pipefail

# === Help ===
if [[ "${1:-}" == "--help" ]] || [[ "${1:-}" == "-h" ]]; then
    cat << 'HELP'
generate-appcast.sh: Generate Sparkle appcast.xml from signed macOS releases.

Usage:
  ./scripts/generate-appcast.sh <dmg-directory>

Arguments:
  <dmg-directory>  Directory containing signed .dmg files

Environment variables:
  SPARKLE_PRIVATE_KEY_PATH  Path to EdDSA private key
                            Default: ~/.config/hwledger/sparkle_ed25519_private.key
  APPCAST_OUTPUT_PATH       Output appcast.xml location
                            Default: docs-site/public/appcast.xml

Behavior:
  1. Verifies private key exists and is readable (mode 600)
  2. Locates Sparkle's generate_appcast tool
  3. Generates appcast.xml with EdDSA signatures
  4. Validates XML structure
  5. Outputs ready-to-host appcast.xml

Requirements:
  - Sparkle installed (via SPM or brew)
  - generate_appcast available in PATH or Sparkle installation
  - Private key at ~/.config/hwledger/sparkle_ed25519_private.key

Example:
  export SPARKLE_PRIVATE_KEY_PATH="$HOME/.config/hwledger/sparkle_ed25519_private.key"
  ./scripts/generate-appcast.sh apps/macos/build

HELP
    exit 0
fi

if [ $# -eq 0 ]; then
    echo "Error: <dmg-directory> is required"
    echo "Run with --help for usage"
    exit 1
fi

DMG_DIR="$1"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

SPARKLE_KEY_PATH="${SPARKLE_PRIVATE_KEY_PATH:-${HOME}/.config/hwledger/sparkle_ed25519_private.key}"
APPCAST_OUTPUT="${APPCAST_OUTPUT_PATH:-${PROJECT_ROOT}/docs-site/public/appcast.xml}"

echo "=== Sparkle Appcast Generator ==="
echo "DMG directory:   ${DMG_DIR}"
echo "Private key:     ${SPARKLE_KEY_PATH}"
echo "Output:          ${APPCAST_OUTPUT}"
echo ""

# === Validate inputs ===
if [ ! -d "${DMG_DIR}" ]; then
    echo "Error: DMG directory not found: ${DMG_DIR}"
    exit 1
fi

# Count DMGs
DMG_COUNT=$(find "${DMG_DIR}" -maxdepth 1 -name "*.dmg" 2>/dev/null | wc -l)
if [ "${DMG_COUNT}" -eq 0 ]; then
    echo "Error: No .dmg files found in ${DMG_DIR}"
    echo "Place signed DMG files in that directory first."
    exit 1
fi

echo "Found ${DMG_COUNT} DMG file(s)"

# === Validate private key ===
if [ ! -f "${SPARKLE_KEY_PATH}" ]; then
    echo "Error: Sparkle private key not found:"
    echo "  ${SPARKLE_KEY_PATH}"
    echo ""
    echo "To create one:"
    echo "  1. Install Sparkle (SPM or brew): brew install sparkle"
    echo "  2. Run: mkdir -p ~/.config/hwledger"
    echo "  3. Generate keys: sparkle/bin/generate_keys ~/.config/hwledger"
    echo "  4. Secure the private key:"
    echo "     chmod 600 ~/.config/hwledger/sparkle_ed25519_private.key"
    echo "     # Back up to 1Password/Bitwarden immediately"
    exit 1
fi

# Verify key permissions
KEY_MODE=$(stat -f %A "${SPARKLE_KEY_PATH}")
if [ "${KEY_MODE}" != "600" ] && [ "${KEY_MODE}" != "rw-------" ]; then
    echo "Warning: Private key has loose permissions: ${KEY_MODE}"
    echo "Fixing to 600..."
    chmod 600 "${SPARKLE_KEY_PATH}"
fi

echo "✓ Private key validated"

# === Find generate_appcast ===
GENERATE_APPCAST=""

if command -v generate_appcast &>/dev/null; then
    GENERATE_APPCAST=$(command -v generate_appcast)
elif command -v sparkle &>/dev/null; then
    # Try to find it in Sparkle installation
    SPARKLE_BIN=$(command -v sparkle)
    SPARKLE_DIR=$(dirname "$(dirname "${SPARKLE_BIN}")")
    if [ -f "${SPARKLE_DIR}/libexec/generate_appcast" ]; then
        GENERATE_APPCAST="${SPARKLE_DIR}/libexec/generate_appcast"
    fi
fi

# If still not found, try brew installation paths
if [ -z "${GENERATE_APPCAST}" ]; then
    if [ -f "/usr/local/bin/generate_appcast" ]; then
        GENERATE_APPCAST="/usr/local/bin/generate_appcast"
    elif [ -f "/opt/homebrew/bin/generate_appcast" ]; then
        GENERATE_APPCAST="/opt/homebrew/bin/generate_appcast"
    fi
fi

if [ -z "${GENERATE_APPCAST}" ] || [ ! -f "${GENERATE_APPCAST}" ]; then
    echo "Error: Sparkle's generate_appcast tool not found"
    echo ""
    echo "Install Sparkle:"
    echo "  brew install sparkle"
    echo ""
    echo "Or add to Package.swift:"
    echo '  .package(url: "https://github.com/sparkle-project/Sparkle", from: "2.6.0")'
    exit 1
fi

echo "✓ Found generate_appcast: ${GENERATE_APPCAST}"

# === Ensure output directory exists ===
mkdir -p "$(dirname "${APPCAST_OUTPUT}")"

# === Generate appcast ===
echo ""
echo "Generating appcast.xml..."

# generate_appcast -ed-key-file <private-key> <dmg-directory>
# Outputs to appcast.xml in the current directory
TEMP_DIR=$(mktemp -d)
trap "rm -rf ${TEMP_DIR}" EXIT

cd "${TEMP_DIR}"

"${GENERATE_APPCAST}" \
    -ed-key-file "${SPARKLE_KEY_PATH}" \
    "${DMG_DIR}"

if [ ! -f "${TEMP_DIR}/appcast.xml" ]; then
    echo "Error: generate_appcast failed to create appcast.xml"
    exit 1
fi

# === Validate XML ===
echo "Validating appcast.xml..."
if ! command -v xmllint &>/dev/null; then
    echo "Warning: xmllint not found; skipping XML validation"
else
    if ! xmllint --noout "${TEMP_DIR}/appcast.xml" 2>&1 | head -3; then
        echo "Error: Invalid XML in appcast.xml"
        exit 1
    fi
    echo "✓ XML valid"
fi

# === Copy to output location ===
cp "${TEMP_DIR}/appcast.xml" "${APPCAST_OUTPUT}"

echo ""
echo "=== Appcast Generated ==="
echo "Output:    ${APPCAST_OUTPUT}"
echo "Size:      $(du -h "${APPCAST_OUTPUT}" | awk '{print $1}')"
echo "URL:       https://kooshapari.github.io/hwLedger/appcast.xml"
echo ""
echo "Next steps:"
echo "  1. Commit and push docs-site/ to trigger GitHub Pages deployment"
echo "  2. Verify appcast is served:"
echo "     curl https://kooshapari.github.io/hwLedger/appcast.xml | head -20"
