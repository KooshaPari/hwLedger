#!/usr/bin/env bash
# Thin stub: forwards to `phenotype-journey extract-keyframes --recordings-dir ...`.
# The real extractor (I-frame + 1fps fallback + stale-frame cleanup) lives in Rust.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
JOURNEYS_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

PHENOTYPE_JOURNEYS_ROOT="${PHENOTYPE_JOURNEYS_ROOT:-/Users/kooshapari/CodeProjects/Phenotype/repos/phenotype-journeys}"
if command -v phenotype-journey >/dev/null 2>&1; then
    BIN=(phenotype-journey)
else
    BIN=(cargo run --quiet --manifest-path "${PHENOTYPE_JOURNEYS_ROOT}/Cargo.toml" --bin phenotype-journey --)
fi

ARGS=(--recordings-dir "${JOURNEYS_ROOT}/recordings" --keyframes-dir "${JOURNEYS_ROOT}/keyframes")
if [ $# -ge 1 ]; then
    ARGS+=(--tape "$1")
fi
exec "${BIN[@]}" extract-keyframes "${ARGS[@]}"
