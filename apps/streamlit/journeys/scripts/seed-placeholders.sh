#!/usr/bin/env bash
# Generate committed placeholder journey artefacts (PNG keyframes + MP4 + GIF)
# when the Streamlit app cannot yet be booted on a given machine. These files
# make docs-site buildable end-to-end before a real recording pass runs and are
# overwritten by record-all.sh once the FFI library is in place.
#
# Idempotent: re-running regenerates the placeholders from the embedded manifest
# seed data below.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
JOURNEYS_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
RECORDINGS_DIR="${JOURNEYS_ROOT}/recordings"
MANIFESTS_DIR="${JOURNEYS_ROOT}/manifests"

command -v ffmpeg >/dev/null 2>&1 || { echo "ffmpeg required" >&2; exit 2; }
command -v python3 >/dev/null 2>&1 || { echo "python3 required" >&2; exit 2; }

python3 "${SCRIPT_DIR}/seed_placeholders.py" \
  --recordings-dir "${RECORDINGS_DIR}" \
  --manifests-dir "${MANIFESTS_DIR}"
