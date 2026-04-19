#!/bin/bash
# Notarize a signed macOS app or DMG for distribution.
# Requires: xcrun (Xcode), Apple Notary credentials
#
# Usage: ./scripts/notarize.sh <app-or-dmg-path>
#
# Environment variables:
#   APPLE_NOTARY_KEY_ID    (required) 10-character Key ID from App Store Connect
#   APPLE_NOTARY_ISSUER_ID (required) UUID of Issuer from App Store Connect
#   APPLE_NOTARY_KEY_PATH  (optional) Path to AuthKey_<KEYID>.p8
#                          Default: ~/.appstoreconnect/private_keys/AuthKey_${APPLE_NOTARY_KEY_ID}.p8
#
# Example:
#   export APPLE_NOTARY_KEY_ID="ABC123DEFG"
#   export APPLE_NOTARY_ISSUER_ID="12345678-1234-1234-1234-123456789012"
#   ./scripts/notarize.sh /path/to/hwLedger-1.0.0.dmg

set -euo pipefail

# === Help ===
if [[ "${1:-}" == "--help" ]] || [[ "${1:-}" == "-h" ]] || [[ $# -eq 0 ]]; then
    cat << 'HELP'
notarize.sh: Notarize a macOS app or DMG with Apple's notary service.

Usage:
  ./scripts/notarize.sh <app-or-dmg-path>

Environment variables:
  APPLE_NOTARY_KEY_ID     (required) 10-character Key ID
  APPLE_NOTARY_ISSUER_ID  (required) UUID Issuer ID
  APPLE_NOTARY_KEY_PATH   (optional) Path to .p8 file
                          Default: ~/.appstoreconnect/private_keys/AuthKey_<KEYID>.p8

Behavior:
  1. Validates credentials and input file
  2. Zips .app bundles (if needed)
  3. Submits to notarytool with --wait
  4. Saves notarization log to apps/macos/build/notarize-<ID>.log
  5. Staples the notarization ticket to the original file
  6. Verifies with spctl

Exit codes:
  0 - Success
  1 - Missing credentials, invalid file, or notarization failed
  2 - Notarization in progress (timeout, check logs later)

Example:
  export APPLE_NOTARY_KEY_ID="ABC123DEFG"
  export APPLE_NOTARY_ISSUER_ID="12345678-1234-1234-1234-123456789012"
  ./scripts/notarize.sh /path/to/hwLedger-1.0.0.dmg

HELP
    exit 0
fi

INPUT_PATH="$1"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
BUILD_LOG_DIR="${PROJECT_ROOT}/apps/macos/build"

# === Validate credentials ===
# Preferred: use the `hwledger` keychain profile stored via
# `xcrun notarytool store-credentials hwledger …`. Falls back to explicit
# env vars if the profile isn't present.
NOTARY_PROFILE="${APPLE_NOTARY_KEYCHAIN_PROFILE:-hwledger}"
USE_PROFILE=0
if xcrun notarytool history --keychain-profile "${NOTARY_PROFILE}" >/dev/null 2>&1; then
    USE_PROFILE=1
fi

KEY_ID="${APPLE_NOTARY_KEY_ID:-}"
ISSUER_ID="${APPLE_NOTARY_ISSUER_ID:-}"

if [ "${USE_PROFILE}" -eq 0 ] && ([ -z "${KEY_ID}" ] || [ -z "${ISSUER_ID}" ]); then
    echo "Error: No notarytool keychain profile '${NOTARY_PROFILE}' found, and APPLE_NOTARY_{KEY_ID,ISSUER_ID} env vars not set."
    echo ""
    echo "Preferred fix — store credentials in keychain (one-time):"
    echo "  xcrun notarytool store-credentials ${NOTARY_PROFILE} \\"
    echo "      --key ~/.appstoreconnect/private_keys/AuthKey_<KEY_ID>.p8 \\"
    echo "      --key-id <KEY_ID> \\"
    echo "      --issuer <ISSUER_UUID>"
    echo ""
    if [ -d ~/.appstoreconnect/private_keys ]; then
        AVAILABLE_KEYS=$(ls ~/.appstoreconnect/private_keys/AuthKey_*.p8 2>/dev/null || true)
        if [ -n "${AVAILABLE_KEYS}" ]; then
            echo "Available .p8 keys found:"
            ls -1 ~/.appstoreconnect/private_keys/AuthKey_*.p8 | sed 's/.*AuthKey_/  /' | sed 's/.p8//'
        fi
    fi
    exit 1
fi

if [ "${USE_PROFILE}" -eq 0 ]; then
    KEY_PATH="${APPLE_NOTARY_KEY_PATH:-${HOME}/.appstoreconnect/private_keys/AuthKey_${KEY_ID}.p8}"
else
    KEY_PATH=""
fi

if [ "${USE_PROFILE}" -eq 0 ] && [ ! -f "${KEY_PATH}" ]; then
    echo "Error: Notary key file not found:"
    echo "  ${KEY_PATH}"
    echo ""
    echo "Create one via:"
    echo "  1. App Store Connect > Users and Access > Keys"
    echo "  2. Generate a new key (App Manager role minimum)"
    echo "  3. Download the .p8 file and save to:"
    echo "  ${HOME}/.appstoreconnect/private_keys/AuthKey_${KEY_ID}.p8"
    echo "  4. chmod 600 on the file"
    exit 1
fi

# === Validate input file ===
if [ ! -e "${INPUT_PATH}" ]; then
    echo "Error: Input file not found: ${INPUT_PATH}"
    exit 1
fi

INPUT_NAME=$(basename "${INPUT_PATH}")
INPUT_EXT="${INPUT_NAME##*.}"

echo "=== hwLedger Notarization ==="
echo "Input:     ${INPUT_PATH}"
echo "Key ID:    ${KEY_ID}"
echo "Issuer:    ${ISSUER_ID}"
echo ""

# === Prepare file for submission ===
SUBMIT_FILE="${INPUT_PATH}"
CLEANUP_ZIP=0

if [ "${INPUT_EXT}" = "app" ]; then
    echo "Detected .app bundle; creating zip for submission..."
    SUBMIT_FILE="${INPUT_PATH}.zip"
    CLEANUP_ZIP=1

    # Use ditto to preserve code signatures and xattrs
    ditto -c -k --keepParent "${INPUT_PATH}" "${SUBMIT_FILE}"

    if [ ! -f "${SUBMIT_FILE}" ]; then
        echo "Error: Failed to create zip"
        exit 1
    fi
    echo "✓ Created: ${SUBMIT_FILE}"
fi

# === Submit to notarytool ===
echo ""
echo "Submitting to Apple Notary Service (may take 1-10 minutes)..."
mkdir -p "${BUILD_LOG_DIR}"

# Capture submission output to extract request UUID
NOTARIZE_OUTPUT=$(mktemp)
trap "rm -f ${NOTARIZE_OUTPUT}" EXIT

if [ "${USE_PROFILE}" -eq 1 ]; then
    xcrun notarytool submit "${SUBMIT_FILE}" \
        --keychain-profile "${NOTARY_PROFILE}" \
        --wait --timeout 1800 \
        2>&1 | tee "${NOTARIZE_OUTPUT}"
else
    xcrun notarytool submit "${SUBMIT_FILE}" \
        --key "${KEY_PATH}" \
        --key-id "${KEY_ID}" \
        --issuer "${ISSUER_ID}" \
        --wait --timeout 1800 \
        2>&1 | tee "${NOTARIZE_OUTPUT}"
fi

# Extract request ID from output
REQUEST_ID=$(grep -i "id:" "${NOTARIZE_OUTPUT}" | head -1 | awk '{print $NF}' || echo "")

if [ -z "${REQUEST_ID}" ]; then
    echo ""
    echo "Error: Failed to extract request ID from notarytool output"
    echo "Full output saved to: ${NOTARIZE_OUTPUT}"
    exit 1
fi

echo ""
echo "✓ Notarization request ID: ${REQUEST_ID}"

# === Fetch and save notarization log ===
echo "Fetching notarization log..."
LOG_PATH="${BUILD_LOG_DIR}/notarize-${REQUEST_ID}.log"

if [ "${USE_PROFILE}" -eq 1 ]; then
    xcrun notarytool log "${REQUEST_ID}" \
        --keychain-profile "${NOTARY_PROFILE}" \
        > "${LOG_PATH}" 2>&1
else
    xcrun notarytool log "${REQUEST_ID}" \
        --key "${KEY_PATH}" \
        --key-id "${KEY_ID}" \
        --issuer "${ISSUER_ID}" \
        > "${LOG_PATH}" 2>&1
fi

echo "✓ Log saved: ${LOG_PATH}"

# Check for issues in log
if grep -iq "error\|invalid\|fail" "${LOG_PATH}"; then
    echo ""
    echo "Warning: Notarization log contains errors or warnings. Review:"
    echo "  ${LOG_PATH}"
fi

# === Staple notarization ticket ===
echo ""
echo "Stapling notarization ticket to original file..."
xcrun stapler staple "${INPUT_PATH}"

if [ $? -ne 0 ]; then
    echo "Error: Failed to staple notarization ticket"
    exit 1
fi

echo "✓ Stapled: ${INPUT_PATH}"

# === Cleanup temp zip ===
if [ $CLEANUP_ZIP -eq 1 ]; then
    rm -f "${SUBMIT_FILE}"
    echo "✓ Cleaned up temporary zip"
fi

# === Verify with spctl ===
echo ""
echo "Verifying with System Integrity Protection (spctl)..."

if [ "${INPUT_EXT}" = "app" ]; then
    spctl -a -t exec -vvv "${INPUT_PATH}" || {
        echo "Error: spctl verification failed"
        exit 1
    }
else
    # DMG
    spctl -a -t install -vvv "${INPUT_PATH}" || {
        echo "Error: spctl verification failed"
        exit 1
    }
fi

echo ""
echo "=== Notarization Complete ==="
echo "File:           ${INPUT_PATH}"
echo "Request ID:     ${REQUEST_ID}"
echo "Log:            ${LOG_PATH}"
echo "Status:         Ready for distribution"
