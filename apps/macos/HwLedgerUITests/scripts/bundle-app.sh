#!/bin/bash
# Bundle the HwLedger executable into a macOS .app bundle
# Usage: ./scripts/bundle-app.sh [release|debug]

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
HWLEDGER_SRC="${PROJECT_ROOT}/../HwLedger"
BUILD_DIR="${PROJECT_ROOT}/../../build"
BUNDLE_DIR="${BUILD_DIR}/HwLedger.app"
CONFIG="${1:-release}"

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

# Create Info.plist
cat > "${BUNDLE_DIR}/Contents/Info.plist" << 'EOF'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleDevelopmentRegion</key>
    <string>en</string>
    <key>CFBundleExecutable</key>
    <string>HwLedger</string>
    <key>CFBundleIdentifier</key>
    <string>com.kooshapari.hwLedger</string>
    <key>CFBundleInfoDictionaryVersion</key>
    <string>6.0</string>
    <key>CFBundleName</key>
    <string>HwLedger</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleShortVersionString</key>
    <string>0.1.0</string>
    <key>CFBundleVersion</key>
    <string>1</string>
    <key>LSApplicationCategoryType</key>
    <string>public.app-category.utilities</string>
    <key>LSUIElement</key>
    <false/>
    <key>NSHighResolutionCapable</key>
    <true/>
    <key>NSPrincipalClass</key>
    <string>NSApplication</string>
    <key>NSRequiresIPhoneOS</key>
    <false/>
</dict>
</plist>
EOF

echo "Bundle created: ${BUNDLE_DIR}"
echo "Executable: ${BUNDLE_DIR}/Contents/MacOS/HwLedger"
