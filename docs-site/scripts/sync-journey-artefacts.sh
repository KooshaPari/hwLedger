#!/usr/bin/env bash
# Thin stub: forwards to `phenotype-journey sync --kind gui-journeys ...`.
# Source-of-truth is apps/macos/HwLedgerUITests/recordings/<slug>/recording.rich.mp4
# (mirrors the CLI/Streamlit layout). Legacy apps/macos/build/journeys is
# still honoured if the recordings/ tree is empty.
set -euo pipefail
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
SRC="${REPO_ROOT}/apps/macos/HwLedgerUITests/recordings"
LEGACY_SRC="${REPO_ROOT}/apps/macos/build/journeys"
DST="${REPO_ROOT}/docs-site/public/journeys"
if [ ! -d "$SRC" ] || [ -z "$(ls -A "$SRC" 2>/dev/null)" ]; then
  SRC="$LEGACY_SRC"
fi
[ -d "$SRC" ] || { echo "Journey source directory does not exist: $SRC"; exit 0; }
PHENOTYPE_JOURNEYS_ROOT="${PHENOTYPE_JOURNEYS_ROOT:-/Users/kooshapari/CodeProjects/Phenotype/repos/phenotype-journeys}"
if command -v phenotype-journey >/dev/null 2>&1; then BIN=(phenotype-journey); else BIN=(cargo run --quiet --manifest-path "${PHENOTYPE_JOURNEYS_ROOT}/Cargo.toml" --bin phenotype-journey --); fi
exec "${BIN[@]}" sync --from "$SRC" --to "$DST" --kind gui-journeys
