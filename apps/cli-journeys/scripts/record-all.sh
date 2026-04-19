#!/bin/bash
# Record all VHS tapes for CLI journeys.
# Produces recordings/.gif + recordings/.mp4 and summary.json

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
JOURNEYS_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
REPO_ROOT="$(cd "${JOURNEYS_ROOT}/../.." && pwd)"
TAPES_DIR="${JOURNEYS_ROOT}/tapes"
RECORDINGS_DIR="${JOURNEYS_ROOT}/recordings"

# Ensure VHS is available
if ! command -v vhs &> /dev/null; then
    echo "Error: vhs not found. Install with: brew install vhs"
    exit 1
fi

# Ensure CLI binary exists
CLI_BIN="${REPO_ROOT}/target/release/hwledger"
if [ ! -f "${CLI_BIN}" ]; then
    echo "Error: hwledger binary not found at ${CLI_BIN}"
    echo "Run: cargo build --release -p hwledger-cli"
    exit 1
fi

mkdir -p "${RECORDINGS_DIR}"

# Summary JSON
SUMMARY_FILE="${JOURNEYS_ROOT}/record-summary.json"
echo "{" > "${SUMMARY_FILE}"
echo '  "timestamp": "'$(date -u +%Y-%m-%dT%H:%M:%SZ)'\",' >> "${SUMMARY_FILE}"
echo '  "vhs_version": "'$(vhs --version | awk '{print $NF}')'\",' >> "${SUMMARY_FILE}"
echo '  "cli_binary": "'${CLI_BIN}'\",' >> "${SUMMARY_FILE}"
echo '  "tapes": [' >> "${SUMMARY_FILE}"

# Prepend CLI binary to PATH
export PATH="${REPO_ROOT}/target/release:${PATH}"

# Process each tape
TAPE_COUNT=0
PASSED_COUNT=0

for tape_file in "${TAPES_DIR}"/*.tape; do
    if [ ! -f "${tape_file}" ]; then
        continue
    fi

    tape_name=$(basename "${tape_file}" .tape)
    echo "Recording: ${tape_name}..."

    # Run VHS and capture exit code
    if vhs "${tape_file}" 2>&1; then
        status="passed"
        ((PASSED_COUNT++))
        exit_code=0
    else
        status="failed"
        exit_code=$?
    fi

    # Get output file sizes if they exist
    gif_file="${RECORDINGS_DIR}/${tape_name}.gif"
    mp4_file="${RECORDINGS_DIR}/${tape_name}.mp4"
    gif_size=0
    mp4_size=0

    if [ -f "${gif_file}" ]; then
        gif_size=$(stat -f%z "${gif_file}" 2>/dev/null || stat -c%s "${gif_file}" 2>/dev/null || echo 0)
    fi
    if [ -f "${mp4_file}" ]; then
        mp4_size=$(stat -f%z "${mp4_file}" 2>/dev/null || stat -c%s "${mp4_file}" 2>/dev/null || echo 0)
    fi

    # Append to summary
    if [ ${TAPE_COUNT} -gt 0 ]; then
        echo "    }," >> "${SUMMARY_FILE}"
    fi

    cat >> "${SUMMARY_FILE}" <<EOF
    {
      "name": "${tape_name}",
      "status": "${status}",
      "exit_code": ${exit_code},
      "gif_path": "${gif_file}",
      "gif_size_bytes": ${gif_size},
      "mp4_path": "${mp4_file}",
      "mp4_size_bytes": ${mp4_size}
EOF

    ((TAPE_COUNT++))
done

# Close summary JSON
echo "    }" >> "${SUMMARY_FILE}"
echo "  ]," >> "${SUMMARY_FILE}"
echo "  \"total_tapes\": ${TAPE_COUNT}," >> "${SUMMARY_FILE}"
echo "  \"passed\": ${PASSED_COUNT}," >> "${SUMMARY_FILE}"
echo "  \"failed\": $((TAPE_COUNT - PASSED_COUNT))" >> "${SUMMARY_FILE}"
echo "}" >> "${SUMMARY_FILE}"

echo ""
echo "Recording complete!"
echo "Summary: ${PASSED_COUNT}/${TAPE_COUNT} tapes passed"
echo "Details: ${SUMMARY_FILE}"
