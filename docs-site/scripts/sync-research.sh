#!/usr/bin/env bash
# Thin stub: forwards to `phenotype-journey sync --kind research ...`.
set -euo pipefail
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
SRC="${REPO_ROOT}/docs/research"
DST="${REPO_ROOT}/docs-site/research"
[ -d "$SRC" ] || { echo "Source directory $SRC does not exist"; exit 1; }
PHENOTYPE_JOURNEYS_ROOT="${PHENOTYPE_JOURNEYS_ROOT:-/Users/kooshapari/CodeProjects/Phenotype/repos/phenotype-journeys}"
if command -v phenotype-journey >/dev/null 2>&1; then BIN=(phenotype-journey); else BIN=(cargo run --quiet --manifest-path "${PHENOTYPE_JOURNEYS_ROOT}/Cargo.toml" --bin phenotype-journey --); fi
"${BIN[@]}" sync --from "$SRC" --to "$DST" --kind research

# phenotype-journey sync is flat (non-recursive); mirror subdirectories manually.
if [ -d "${SRC}/imports-2026-04" ]; then
  mkdir -p "${DST}/imports-2026-04"
  cp "${SRC}/imports-2026-04"/*.md "${DST}/imports-2026-04/" 2>/dev/null || true
fi
