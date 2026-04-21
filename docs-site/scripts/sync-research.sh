#!/usr/bin/env bash
# Thin stub: forwards to `phenotype-journey sync --kind research ...`.
set -euo pipefail
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
SRC="${REPO_ROOT}/docs/research"
DST="${REPO_ROOT}/docs-site/research"
[ -d "$SRC" ] || { echo "Source directory $SRC does not exist"; exit 1; }
PHENOTYPE_JOURNEYS_ROOT="${PHENOTYPE_JOURNEYS_ROOT:-/Users/kooshapari/CodeProjects/Phenotype/repos/phenotype-journeys}"
if command -v phenotype-journey >/dev/null 2>&1; then BIN=(phenotype-journey); else BIN=(cargo run --quiet --manifest-path "${PHENOTYPE_JOURNEYS_ROOT}/Cargo.toml" --bin phenotype-journey --); fi
exec "${BIN[@]}" sync --from "$SRC" --to "$DST" --kind research
