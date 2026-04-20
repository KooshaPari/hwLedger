#!/usr/bin/env bash
# Thin stub: forwards to `phenotype-journey verify --manifests-dir ...`.
# Built-in mock replaces the old Python mock server; the Rust pipeline writes
# manifest.verified.json in the same shape as the CLI pipeline.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
JOURNEYS_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

PHENOTYPE_JOURNEYS_ROOT="${PHENOTYPE_JOURNEYS_ROOT:-/Users/kooshapari/CodeProjects/Phenotype/repos/phenotype-journeys}"
if command -v phenotype-journey >/dev/null 2>&1; then BIN=(phenotype-journey); else BIN=(cargo run --quiet --manifest-path "${PHENOTYPE_JOURNEYS_ROOT}/Cargo.toml" --bin phenotype-journey --); fi
exec "${BIN[@]}" verify \
    --manifests-dir "${JOURNEYS_ROOT}/manifests" \
    --tapes-dir "${JOURNEYS_ROOT}/tapes" \
    --artefacts "${JOURNEYS_ROOT}" \
    "$@"
