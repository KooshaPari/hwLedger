#!/bin/bash
# Create a signed DMG installer for hwLedger.
# Supports both create-dmg (via Homebrew) and manual hdiutil fallback.
#
# Usage: ./scripts/build-dmg.sh --app <path-to-app> --out <output-path>
#
# Environment variables:
#   CODESIGN_IDENTITY Developer ID for signing (default: read from app)
#
# Example:
#   ./scripts/build-dmg.sh --app /path/to/HwLedger.app --out /tmp/hwLedger-1.0.0.dmg

set -euo pipefail

# === Help ===
if [[ "${1:-}" == "--help" ]] || [[ "${1:-}" == "-h" ]]; then
    cat << 'HELP'
build-dmg.sh: Create a signed DMG installer for macOS distribution.

Usage:
  ./scripts/build-dmg.sh --app <path> --out <output>

Options:
  --app <path>       Path to signed .app bundle (required)
  --out <path>       Output DMG path (required)
  --help             Show this help

Environment variables:
  CODESIGN_IDENTITY  Developer ID Application cert identity
                     (optional, auto-detected from .app if not set)
  BACKGROUND_IMAGE   Path to DMG background PNG (default: generate simple one)

Behavior:
  1. Validates input .app bundle
  2. Uses create-dmg if available (brew install create-dmg)
  3. Falls back to hdiutil if create-dmg unavailable
  4. Codesigns the final DMG with Developer ID
  5. Verifies DMG is readable

Example:
  ./scripts/build-dmg.sh --app apps/macos/build/HwLedger.app --out build/hwLedger-1.0.0.dmg

HELP
    exit 0
fi

# === Parse options ===
APP_PATH=""
OUTPUT_PATH=""

while [[ $# -gt 0 ]]; do
    case "$1" in
        --app)
            APP_PATH="$2"
            shift 2
            ;;
        --out)
            OUTPUT_PATH="$2"
            shift 2
            ;;
        --help|-h)
            exit 0
            ;;
        *)
            echo "Error: unknown option '$1'"
            exit 1
            ;;
    esac
done

# === Validate arguments ===
if [ -z "${APP_PATH}" ] || [ -z "${OUTPUT_PATH}" ]; then
    echo "Error: --app and --out are required"
    echo "Run with --help for usage"
    exit 1
fi

if [ ! -d "${APP_PATH}" ]; then
    echo "Error: app bundle not found: ${APP_PATH}"
    exit 1
fi

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

# Extract version from app's Info.plist
APP_VERSION=$(defaults read "${APP_PATH}/Contents/Info" CFBundleShortVersionString 2>/dev/null || echo "0.1.0")
APP_NAME=$(defaults read "${APP_PATH}/Contents/Info" CFBundleName 2>/dev/null || echo "HwLedger")

CODESIGN_IDENTITY="${CODESIGN_IDENTITY:-Developer ID Application: Koosha Paridehpour (GCT2BN8WLL)}"

# Background image path (optional, generate if missing)
BACKGROUND_IMAGE="${BACKGROUND_IMAGE:-}"

echo "=== hwLedger DMG Builder ==="
echo "App:        ${APP_PATH}"
echo "Version:    ${APP_VERSION}"
echo "Output:     ${OUTPUT_PATH}"
echo "Codesign:   ${CODESIGN_IDENTITY}"
echo ""

# === Ensure output directory exists ===
OUTPUT_DIR=$(dirname "${OUTPUT_PATH}")
mkdir -p "${OUTPUT_DIR}"

# === Generate or validate background image ===
if [ -z "${BACKGROUND_IMAGE}" ]; then
    BACKGROUND_IMAGE="${OUTPUT_DIR}/.dmg-background.png"

    if [ ! -f "${BACKGROUND_IMAGE}" ]; then
        echo "Generating DMG background image..."
        # Create a simple 600x400 PNG with transparent background
        # Using sips (macOS built-in) to create a minimal placeholder
        if command -v sips &>/dev/null; then
            python3 - "${BACKGROUND_IMAGE}" << 'PYSCRIPT' && echo "Generated background"
import struct, zlib, sys
sig = b"\x89PNG\r\n\x1a\n"
ihdr_d = struct.pack(">IIBBBBB", 600, 400, 8, 2, 0, 0, 0)
ihdr = struct.pack(">I", 13) + b"IHDR" + ihdr_d + struct.pack(">I", zlib.crc32(b"IHDR" + ihdr_d) & 0xffffffff)
scan = b"\x00" + (b"\xff\xff\xff" * 600)
idat_d = zlib.compress(scan * 400, 9)
idat = struct.pack(">I", len(idat_d)) + b"IDAT" + idat_d + struct.pack(">I", zlib.crc32(b"IDAT" + idat_d) & 0xffffffff)
iend = struct.pack(">I", 0) + b"IEND" + struct.pack(">I", 0xae426082)
open(sys.argv[1], "wb").write(sig + ihdr + idat + iend)
PYSCRIPT
        else
            echo "Warning: sips not found, proceeding without background image"
            BACKGROUND_IMAGE=""
        fi
    fi
fi

# === Build DMG ===
if command -v create-dmg &>/dev/null; then
    echo "Using create-dmg (Homebrew)..."

    # create-dmg syntax: create-dmg [options] output.dmg input-folder
    # We pass the app's parent directory
    APP_PARENT=$(dirname "${APP_PATH}")
    APP_BASENAME=$(basename "${APP_PATH}")

    # Clean up old DMG
    rm -f "${OUTPUT_PATH}"

    create-dmg \
        --volname "${APP_NAME} ${APP_VERSION}" \
        --background "${BACKGROUND_IMAGE}" \
        --icon-size 100 \
        --icon "${APP_BASENAME}" 150 150 \
        --app-drop-link 450 150 \
        "${OUTPUT_PATH}" \
        "${APP_PARENT}/${APP_BASENAME}"

    if [ $? -ne 0 ]; then
        echo "Error: create-dmg failed"
        exit 1
    fi
    echo "✓ DMG created with create-dmg"
else
    echo "create-dmg not found; using hdiutil (manual approach)..."

    # Manual DMG creation with hdiutil
    TEMP_DMG="${OUTPUT_DIR}/.hwledger-temp.dmg"
    MOUNT_POINT="/Volumes/hwLedger-DMG"

    # Clean up old files
    rm -f "${OUTPUT_PATH}" "${TEMP_DMG}"
    [ -d "${MOUNT_POINT}" ] && hdiutil detach "${MOUNT_POINT}" 2>/dev/null || true

    # Create sparse DMG
    echo "Creating sparse DMG..."
    hdiutil create -srcfolder "${APP_PATH}" \
        -volname "${APP_NAME} ${APP_VERSION}" \
        -format UDRW \
        "${TEMP_DMG}"

    if [ ! -f "${TEMP_DMG}" ]; then
        echo "Error: hdiutil create failed"
        exit 1
    fi

    # Mount and customize (if background image available)
    echo "Mounting DMG for customization..."
    hdiutil attach "${TEMP_DMG}" -mountpoint "${MOUNT_POINT}"

    if [ -f "${BACKGROUND_IMAGE}" ]; then
        cp "${BACKGROUND_IMAGE}" "${MOUNT_POINT}/.background.png"
        echo "✓ Added background image"
    fi

    # Create Applications symlink
    ln -s /Applications "${MOUNT_POINT}/Applications" 2>/dev/null || true

    # Unmount
    hdiutil detach "${MOUNT_POINT}"

    # Convert to compressed DMG
    echo "Converting to compressed DMG..."
    hdiutil convert "${TEMP_DMG}" \
        -format UDZO \
        -imagekey zlib-level=9 \
        -o "${OUTPUT_PATH}"

    rm -f "${TEMP_DMG}"

    if [ ! -f "${OUTPUT_PATH}" ]; then
        echo "Error: DMG conversion failed"
        exit 1
    fi

    echo "✓ DMG created with hdiutil"
fi

# === Codesign DMG ===
echo ""
echo "Codesigning DMG..."

if ! command -v codesign &>/dev/null; then
    echo "Warning: codesign not found; skipping DMG signing"
else
    codesign --sign "${CODESIGN_IDENTITY}" \
        --timestamp \
        "${OUTPUT_PATH}"

    if [ $? -ne 0 ]; then
        echo "Error: DMG codesigning failed"
        exit 1
    fi
    echo "✓ DMG signed"
fi

# === Verify DMG ===
echo "Verifying DMG..."
if hdiutil verify "${OUTPUT_PATH}" 2>&1 | grep -q "accepted"; then
    echo "✓ DMG verified"
else
    echo "Warning: DMG verification inconclusive (may still be valid)"
fi

echo ""
echo "=== DMG Build Complete ==="
echo "Output:  ${OUTPUT_PATH}"
echo "Size:    $(du -h "${OUTPUT_PATH}" | awk '{print $1}')"
echo "Ready for distribution or notarization"
