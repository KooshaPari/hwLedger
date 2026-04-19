#!/usr/bin/env bash
# hwLedger XCFramework build script
# Builds universal static library (arm64 + x86_64) + Swift header + XCFramework
#
# Usage: ./scripts/build-xcframework.sh [--release | --debug]
# Default: --release
#
# Requires:
#   - macOS (Darwin)
#   - rustup with aarch64-apple-darwin + x86_64-apple-darwin targets installed
#   - cbindgen installed (cargo install cbindgen)
#   - Xcode 16+ (xcodebuild)

set -euo pipefail

# === Configuration ===
PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CARGO_MANIFEST="${PROJECT_ROOT}/crates/hwledger-ffi/Cargo.toml"
CBINDGEN_CONFIG="${PROJECT_ROOT}/crates/hwledger-ffi/cbindgen.toml"
BUILD_DIR="${PROJECT_ROOT}/apps/macos/xcframework/build"
UNIVERSAL_DIR="${BUILD_DIR}/universal"
INCLUDE_DIR="${BUILD_DIR}/include"
XCFRAMEWORK_OUTPUT="${PROJECT_ROOT}/apps/macos/xcframework/HwLedgerCore.xcframework"

# === Arguments ===
BUILD_MODE="${1:-release}"
case "${BUILD_MODE}" in
  --release|release)
    BUILD_MODE="release"
    CARGO_FLAGS="--release"
    ;;
  --debug|debug)
    BUILD_MODE="debug"
    CARGO_FLAGS=""
    ;;
  *)
    echo "Error: unknown build mode '${BUILD_MODE}'. Use --release or --debug."
    exit 1
    ;;
esac

# === Checks ===
if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "Error: build-xcframework.sh only runs on macOS."
  echo "Detected: $(uname -s)"
  exit 1
fi

if ! command -v cargo &>/dev/null; then
  echo "Error: cargo not found. Install Rust via rustup."
  exit 1
fi

if ! command -v cbindgen &>/dev/null; then
  echo "Error: cbindgen not found. Install via: cargo install cbindgen"
  exit 1
fi

if ! command -v xcodebuild &>/dev/null; then
  echo "Error: xcodebuild not found. Install Xcode 16+."
  exit 1
fi

echo "=== hwLedger XCFramework Builder ==="
echo "Mode: ${BUILD_MODE}"
echo "Project root: ${PROJECT_ROOT}"
echo ""

# === Clean output directories ===
echo "[1/6] Cleaning output directories..."
rm -rf "${BUILD_DIR}"
mkdir -p "${UNIVERSAL_DIR}" "${INCLUDE_DIR}"

# === Build arm64 ===
echo "[2/6] Building hwledger-ffi for aarch64-apple-darwin..."
cargo build ${CARGO_FLAGS} \
  --target aarch64-apple-darwin \
  -p hwledger-ffi \
  --manifest-path "${CARGO_MANIFEST}"

ARM64_LIB="${PROJECT_ROOT}/target/aarch64-apple-darwin/${BUILD_MODE}/libhwledger_ffi.a"
if [[ ! -f "${ARM64_LIB}" ]]; then
  echo "Error: ARM64 static library not found at ${ARM64_LIB}"
  exit 1
fi
echo "✓ ARM64 library: ${ARM64_LIB}"

# === Build x86_64 ===
echo "[3/6] Building hwledger-ffi for x86_64-apple-darwin..."
cargo build ${CARGO_FLAGS} \
  --target x86_64-apple-darwin \
  -p hwledger-ffi \
  --manifest-path "${CARGO_MANIFEST}"

X86_64_LIB="${PROJECT_ROOT}/target/x86_64-apple-darwin/${BUILD_MODE}/libhwledger_ffi.a"
if [[ ! -f "${X86_64_LIB}" ]]; then
  echo "Error: x86_64 static library not found at ${X86_64_LIB}"
  exit 1
fi
echo "✓ x86_64 library: ${X86_64_LIB}"

# === Create universal binary ===
echo "[4/6] Creating universal static library with lipo..."
lipo -create \
  "${ARM64_LIB}" \
  "${X86_64_LIB}" \
  -output "${UNIVERSAL_DIR}/libhwledger_ffi.a"

if [[ ! -f "${UNIVERSAL_DIR}/libhwledger_ffi.a" ]]; then
  echo "Error: failed to create universal library"
  exit 1
fi

LIPO_INFO=$(lipo -info "${UNIVERSAL_DIR}/libhwledger_ffi.a")
echo "✓ Universal library created:"
echo "  ${LIPO_INFO}"

# === Generate C header ===
echo "[5/6] Generating C header with cbindgen..."
cbindgen --config "${CBINDGEN_CONFIG}" \
  --crate hwledger-ffi \
  --output "${INCLUDE_DIR}/hwledger.h"

if [[ ! -f "${INCLUDE_DIR}/hwledger.h" ]]; then
  echo "Error: cbindgen failed to generate header"
  exit 1
fi
echo "✓ Header generated: ${INCLUDE_DIR}/hwledger.h"
echo "  First 10 lines:"
head -10 "${INCLUDE_DIR}/hwledger.h" | sed 's/^/    /'

# === Create module map ===
echo "[6/6] Creating module.modulemap..."
cat > "${INCLUDE_DIR}/module.modulemap" << 'EOF'
module HwLedgerCore {
  header "hwledger.h"
  export *
}
EOF
echo "✓ Module map created"

# === Build XCFramework ===
echo "[7/7] Building XCFramework..."
rm -rf "${XCFRAMEWORK_OUTPUT}"

xcodebuild -create-xcframework \
  -library "${UNIVERSAL_DIR}/libhwledger_ffi.a" \
  -headers "${INCLUDE_DIR}" \
  -output "${XCFRAMEWORK_OUTPUT}"

if [[ ! -d "${XCFRAMEWORK_OUTPUT}" ]]; then
  echo "Error: xcodebuild failed to create XCFramework"
  exit 1
fi

echo "✓ XCFramework created:"
echo "  ${XCFRAMEWORK_OUTPUT}"
echo ""
echo "=== Build Complete ==="
echo ""
echo "Location: ${XCFRAMEWORK_OUTPUT}"
echo "Size: $(du -sh "${XCFRAMEWORK_OUTPUT}" | awk '{print $1}')"
echo ""
echo "To use in Xcode:"
echo "  1. Drag HwLedgerCore.xcframework into your project"
echo "  2. Add HwLedger Swift package via File > Add Packages"
echo "  3. import HwLedger in your Swift code"
