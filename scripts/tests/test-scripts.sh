#!/bin/bash
# Lightweight shell tests for hwLedger build scripts.
# Tests error handling and help output without requiring actual builds.
#
# Usage: ./scripts/tests/test-scripts.sh
#
# Requirements: bash 4+, no external deps for basic tests

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Test counters
TESTS_PASSED=0
TESTS_FAILED=0

# === Test Utilities ===

assert_script_exists() {
    local script="$1"
    if [ ! -f "${script}" ]; then
        echo -e "${RED}FAIL${NC} Script not found: ${script}"
        ((TESTS_FAILED++))
        return 1
    fi
    echo -e "${GREEN}PASS${NC} Script exists: ${script}"
    ((TESTS_PASSED++))
    return 0
}

assert_script_executable() {
    local script="$1"
    if [ ! -x "${script}" ]; then
        echo -e "${RED}FAIL${NC} Script not executable: ${script}"
        ((TESTS_FAILED++))
        return 1
    fi
    echo -e "${GREEN}PASS${NC} Script executable: ${script}"
    ((TESTS_PASSED++))
    return 0
}

assert_help_works() {
    local script="$1"
    local output
    output=$("${script}" --help 2>&1 || true)
    if ! echo "${output}" | grep -q "Usage\|usage\|Options"; then
        echo -e "${RED}FAIL${NC} Help output missing for: ${script}"
        echo "Got: ${output}"
        ((TESTS_FAILED++))
        return 1
    fi
    echo -e "${GREEN}PASS${NC} Help works: ${script}"
    ((TESTS_PASSED++))
    return 0
}

assert_env_check() {
    local script="$1"
    local missing_env="$2"
    local output
    # Unset the environment variable and expect error
    output=$(env -i "${script}" 2>&1 || true)
    if ! echo "${output}" | grep -q "Error\|Missing\|required"; then
        echo -e "${RED}FAIL${NC} Missing env check for: ${script}"
        echo "Expected error output but got: ${output}"
        ((TESTS_FAILED++))
        return 1
    fi
    echo -e "${GREEN}PASS${NC} Env check works: ${script}"
    ((TESTS_PASSED++))
    return 0
}

assert_file_exists() {
    local file="$1"
    if [ ! -f "${file}" ]; then
        echo -e "${RED}FAIL${NC} File not found: ${file}"
        ((TESTS_FAILED++))
        return 1
    fi
    echo -e "${GREEN}PASS${NC} File exists: ${file}"
    ((TESTS_PASSED++))
    return 0
}

# === Test Suites ===

echo "=== hwLedger Build Script Tests ==="
echo ""

# Test 1: Script files exist and are executable
echo "Test Suite: File Structure"
assert_script_exists "${REPO_ROOT}/apps/macos/HwLedgerUITests/scripts/bundle-app.sh"
assert_script_executable "${REPO_ROOT}/apps/macos/HwLedgerUITests/scripts/bundle-app.sh"

assert_script_exists "${REPO_ROOT}/scripts/notarize.sh"
assert_script_executable "${REPO_ROOT}/scripts/notarize.sh"

assert_script_exists "${REPO_ROOT}/scripts/build-dmg.sh"
assert_script_executable "${REPO_ROOT}/scripts/build-dmg.sh"

assert_script_exists "${REPO_ROOT}/scripts/generate-appcast.sh"
assert_script_executable "${REPO_ROOT}/scripts/generate-appcast.sh"

echo ""

# Test 2: Help output
echo "Test Suite: Help Output"
assert_help_works "${REPO_ROOT}/apps/macos/HwLedgerUITests/scripts/bundle-app.sh"
assert_help_works "${REPO_ROOT}/scripts/notarize.sh"
assert_help_works "${REPO_ROOT}/scripts/build-dmg.sh"
assert_help_works "${REPO_ROOT}/scripts/generate-appcast.sh"

echo ""

# Test 3: Entitlements and Info.plist
echo "Test Suite: Config Files"
assert_file_exists "${REPO_ROOT}/apps/macos/HwLedger/entitlements.plist"

echo ""

# Test 4: Environment variable checks
echo "Test Suite: Environment Validation"

# Test notarize.sh with missing env
output=$("${REPO_ROOT}/scripts/notarize.sh" 2>&1 || true)
if echo "${output}" | grep -q "Error.*Missing\|environment\|required"; then
    echo -e "${GREEN}PASS${NC} notarize.sh validates missing credentials"
    ((TESTS_PASSED++))
else
    echo -e "${RED}FAIL${NC} notarize.sh should validate missing credentials"
    ((TESTS_FAILED++))
fi

echo ""

# Test 5: DMG script argument validation
echo "Test Suite: Argument Validation"

output=$(${REPO_ROOT}/scripts/build-dmg.sh 2>&1 || true)
if echo "${output}" | grep -q "Error.*required\|--app.*--out"; then
    echo -e "${GREEN}PASS${NC} build-dmg.sh validates required args"
    ((TESTS_PASSED++))
else
    echo -e "${RED}FAIL${NC} build-dmg.sh should validate required arguments"
    ((TESTS_FAILED++))
fi

echo ""

# Test 6: appcast.sh directory validation
echo "Test Suite: Input Validation"

output=$(${REPO_ROOT}/scripts/generate-appcast.sh /nonexistent/path 2>&1 || true)
if echo "${output}" | grep -q "Error\|not found"; then
    echo -e "${GREEN}PASS${NC} generate-appcast.sh validates input path"
    ((TESTS_PASSED++))
else
    echo -e "${RED}FAIL${NC} generate-appcast.sh should validate input path"
    ((TESTS_FAILED++))
fi

echo ""

# Test 7: Package.swift has Sparkle dependency
echo "Test Suite: Sparkle Integration"

if grep -q "sparkle-project/Sparkle" "${REPO_ROOT}/apps/macos/HwLedger/Package.swift"; then
    echo -e "${GREEN}PASS${NC} Sparkle dependency added to Package.swift"
    ((TESTS_PASSED++))
else
    echo -e "${RED}FAIL${NC} Sparkle dependency missing from Package.swift"
    ((TESTS_FAILED++))
fi

if grep -q "Sparkle" "${REPO_ROOT}/apps/macos/HwLedger/Sources/HwLedgerApp/HwLedgerApp.swift"; then
    echo -e "${GREEN}PASS${NC} Sparkle imported in HwLedgerApp.swift"
    ((TESTS_PASSED++))
else
    echo -e "${RED}FAIL${NC} Sparkle import missing from HwLedgerApp.swift"
    ((TESTS_FAILED++))
fi

echo ""

# Test 8: GitHub Actions workflow
echo "Test Suite: CI/CD Workflow"
assert_file_exists "${REPO_ROOT}/.github/workflows/release.yml"

if grep -q "macos-latest" "${REPO_ROOT}/.github/workflows/release.yml"; then
    echo -e "${GREEN}PASS${NC} release.yml uses macos-latest runner"
    ((TESTS_PASSED++))
else
    echo -e "${RED}FAIL${NC} release.yml should use macos-latest"
    ((TESTS_FAILED++))
fi

echo ""

# Test 9: Documentation
echo "Test Suite: Documentation"
assert_file_exists "${REPO_ROOT}/docs/reports/WP21-APPLE-DEV-SECRETS.md"

echo ""

# === Summary ===
echo "=== Test Summary ==="
echo -e "${GREEN}Passed:${NC} ${TESTS_PASSED}"
echo -e "${RED}Failed:${NC} ${TESTS_FAILED}"
echo ""

if [ ${TESTS_FAILED} -eq 0 ]; then
    echo -e "${GREEN}All tests passed!${NC}"
    exit 0
else
    echo -e "${RED}Some tests failed.${NC}"
    exit 1
fi
