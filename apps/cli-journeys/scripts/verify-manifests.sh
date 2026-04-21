#!/usr/bin/env bash
# Thin stub: forwards to `phenotype-journey verify --manifests-dir ...`.
# The mock Anthropic responder, intents YAML overlay, assertion loop, and
# manifest.verified.json writing are all implemented in Rust.
# Setting ANTHROPIC_API_KEY switches to the live-API backend.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
JOURNEYS_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

PHENOTYPE_JOURNEYS_ROOT="${PHENOTYPE_JOURNEYS_ROOT:-/Users/kooshapari/CodeProjects/Phenotype/repos/phenotype-journeys}"
if command -v phenotype-journey >/dev/null 2>&1; then
    BIN=(phenotype-journey)
else
    BIN=(cargo run --quiet --manifest-path "${PHENOTYPE_JOURNEYS_ROOT}/Cargo.toml" --bin phenotype-journey --)
fi

exec "${BIN[@]}" verify \
    --manifests-dir "${JOURNEYS_ROOT}/manifests" \
    --tapes-dir "${JOURNEYS_ROOT}/tapes" \
    --artefacts "${JOURNEYS_ROOT}" \
    "$@"
