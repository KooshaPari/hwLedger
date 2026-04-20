#!/bin/bash
# Record all VHS tapes for CLI journeys.
# Produces recordings/.gif + recordings/.mp4 and summary.json

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
JOURNEYS_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
REPO_ROOT="$(cd "${JOURNEYS_ROOT}/../.." && pwd)"
TAPES_DIR="${JOURNEYS_ROOT}/tapes"
RECORDINGS_DIR="${JOURNEYS_ROOT}/recordings"

# Tapes use relative output paths like apps/cli-journeys/recordings/...
# VHS resolves those against its cwd, so always run from the repo root.
cd "${REPO_ROOT}"

# Ensure VHS is available
if ! command -v vhs &> /dev/null; then
    echo "Error: vhs not found. Install with: brew install vhs"
    exit 1
fi

# Ensure CLI binary exists. Upstream crate produces hwledger-cli; tapes call hwledger.
CLI_BIN="${REPO_ROOT}/target/release/hwledger"
CLI_SRC="${REPO_ROOT}/target/release/hwledger-cli"
if [ ! -f "${CLI_SRC}" ]; then
    echo "Error: hwledger-cli binary not found at ${CLI_SRC}"
    echo "Run: cargo build --release -p hwledger-cli"
    exit 1
fi
# Keep tape-friendly `hwledger` symlink in sync with the freshly built binary.
if [ ! -f "${CLI_BIN}" ] || [ "${CLI_SRC}" -nt "${CLI_BIN}" ]; then
    ln -sf "hwledger-cli" "${CLI_BIN}"
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

# Warn about tapes missing the canonical `__EXIT_$?__` sentinel. The
# phenotype-journey assert layer uses this sentinel to gate the last keyframe
# against the expected exit code when the intents YAML declares
# `expected_exit`. See phenotype-journeys/README.md for the canonical pattern.
MISSING_SENTINEL=()
for tape_file in "${TAPES_DIR}"/*.tape; do
    [ -f "${tape_file}" ] || continue
    if ! grep -q "__EXIT_" "${tape_file}"; then
        MISSING_SENTINEL+=("$(basename "${tape_file}")")
    fi
done
if [ ${#MISSING_SENTINEL[@]} -gt 0 ]; then
    echo "warn: these tapes do not emit an __EXIT_\$?__ sentinel — expected_exit assertions will not gate them:"
    for n in "${MISSING_SENTINEL[@]}"; do echo "  - ${n}"; done
    echo "Add a trailing line like: Type \"<cmd>; echo __EXIT_\\\$?__\" ; Enter"
fi

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
