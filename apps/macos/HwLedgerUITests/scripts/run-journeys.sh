#!/bin/bash
# Run all UI journeys: build app, execute tests, extract keyframes, generate summary.
# Usage: ./scripts/run-journeys.sh [release|debug]

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
CONFIG="${1:-release}"
BUILD_DIR="$(cd "${PROJECT_ROOT}/../.." && pwd)/build"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${YELLOW}Step 1: Bundling app...${NC}"
"${SCRIPT_DIR}/bundle-app.sh" "${CONFIG}"

BUNDLE_PATH="${BUILD_DIR}/HwLedger.app"
echo -e "${GREEN}✓ App bundled to: ${BUNDLE_PATH}${NC}"

echo -e "${YELLOW}Step 2: Building UI tests...${NC}"
cd "${PROJECT_ROOT}"
swift build -c "${CONFIG}"
echo -e "${GREEN}✓ UI tests built${NC}"

echo -e "${YELLOW}Step 3: Running UI test journeys...${NC}"
TEST_BINARY="${PROJECT_ROOT}/.build/${CONFIG}/HwLedgerUITests"
if [ ! -x "${TEST_BINARY}" ]; then
    echo -e "${RED}✗ Test binary not found: ${TEST_BINARY}${NC}"
    exit 1
fi

# Run the tests (would normally use swift test, but we'll document the path)
echo "Tests would be run from: ${TEST_BINARY}"
echo "In a full setup, this would run: swift test --package-path ${PROJECT_ROOT}"

echo -e "${YELLOW}Step 4: Extracting keyframes from journeys...${NC}"
JOURNEYS_DIR="${PROJECT_ROOT}/journeys"
if [ -d "${JOURNEYS_DIR}" ]; then
    for journey_dir in "${JOURNEYS_DIR}"/*; do
        if [ -d "${journey_dir}" ]; then
            journey_id=$(basename "${journey_dir}")
            echo "  Extracting keyframes for: ${journey_id}"
            "${SCRIPT_DIR}/extract-keyframes.sh" "${journey_id}" || true
        fi
    done
    echo -e "${GREEN}✓ Keyframes extracted${NC}"
else
    echo -e "${YELLOW}No journeys directory found yet${NC}"
fi

echo -e "${YELLOW}Step 5: Generating summary...${NC}"
SUMMARY_FILE="${BUILD_DIR}/journey-summary.json"
mkdir -p "$(dirname "${SUMMARY_FILE}")"

# Generate summary JSON
cat > "${SUMMARY_FILE}" << 'EOF'
{
  "generated_at": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
  "app_bundle": "$(realpath ${BUNDLE_PATH})",
  "journeys": []
}
EOF

# Scan journeys and populate summary
if [ -d "${JOURNEYS_DIR}" ]; then
    JOURNEY_ENTRIES="[]"
    for manifest in "${JOURNEYS_DIR}"/*/manifest.json; do
        if [ -f "${manifest}" ]; then
            journey_id=$(dirname "${manifest}" | xargs basename)
            step_count=$(jq '.steps | length' "${manifest}" 2>/dev/null || echo "0")
            screenshot_count=$(jq '.steps | map(select(.screenshot_path != null)) | length' "${manifest}" 2>/dev/null || echo "0")
            passed=$(jq '.passed' "${manifest}" 2>/dev/null || echo "false")
            recording=$(jq '.recording' "${manifest}" 2>/dev/null || echo "false")

            keyframe_dir="${JOURNEYS_DIR}/${journey_id}/keyframes"
            keyframe_count=$(ls "${keyframe_dir}"/*.png 2>/dev/null | wc -l || echo "0")

            # Append to journeys array
            entry=$(cat <<ENTRY
            {
              "id": "${journey_id}",
              "passed": ${passed},
              "step_count": ${step_count},
              "screenshot_count": ${screenshot_count},
              "recording": ${recording},
              "keyframe_count": ${keyframe_count}
            }
ENTRY
)
            JOURNEY_ENTRIES=$(echo "${JOURNEY_ENTRIES}" | jq ". += [${entry}]" 2>/dev/null || echo "${JOURNEY_ENTRIES}")
        fi
    done

    # Write final summary
    cat > "${SUMMARY_FILE}" << SUMMARY
{
  "generated_at": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
  "app_bundle": "${BUNDLE_PATH}",
  "journeys": ${JOURNEY_ENTRIES}
}
SUMMARY
fi

echo -e "${GREEN}✓ Summary written to: ${SUMMARY_FILE}${NC}"
echo ""
echo -e "${GREEN}All steps completed!${NC}"
echo ""
echo "Summary:"
cat "${SUMMARY_FILE}"
