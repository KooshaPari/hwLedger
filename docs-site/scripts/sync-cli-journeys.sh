#!/usr/bin/env bash
# Thin stub: forwards to `phenotype-journey sync --kind cli-journeys ...`.
set -euo pipefail
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
SRC="${REPO_ROOT}/apps/cli-journeys"
DST="${REPO_ROOT}/docs-site/public/cli-journeys"
[ -d "$SRC" ] || { echo "No CLI journeys source at $SRC — skipping."; exit 0; }
PHENOTYPE_JOURNEYS_ROOT="${PHENOTYPE_JOURNEYS_ROOT:-/Users/kooshapari/CodeProjects/Phenotype/repos/phenotype-journeys}"
if command -v phenotype-journey >/dev/null 2>&1; then BIN=(phenotype-journey); else BIN=(cargo run --quiet --manifest-path "${PHENOTYPE_JOURNEYS_ROOT}/Cargo.toml" --bin phenotype-journey --); fi
exec "${BIN[@]}" sync --from "$SRC" --to "$DST" --kind cli-journeys
