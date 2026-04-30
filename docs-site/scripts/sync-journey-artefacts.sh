#!/usr/bin/env bash
# Thin stub: forwards to `phenotype-journey sync --kind gui-journeys ...`.
# Source-of-truth is apps/macos/HwLedgerUITests/journeys/<slug>/, where the
# XCUITest harness writes keyframes, cursor tracks, recording.mp4, and
# manifest.json. Legacy recordings/build trees are still honoured as fallback
# inputs for older captures, but docs consume the canonical public/gui-journeys
# layout directly.
set -euo pipefail
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
SRC="${REPO_ROOT}/apps/macos/HwLedgerUITests/journeys"
RECORDINGS_SRC="${REPO_ROOT}/apps/macos/HwLedgerUITests/recordings"
LEGACY_SRC="${REPO_ROOT}/apps/macos/build/journeys"
DST="${REPO_ROOT}/docs-site/public/gui-journeys"
if [ ! -d "$SRC" ] || [ -z "$(ls -A "$SRC" 2>/dev/null)" ]; then
  SRC="$RECORDINGS_SRC"
fi
if [ ! -d "$SRC" ] || [ -z "$(ls -A "$SRC" 2>/dev/null)" ]; then
  SRC="$LEGACY_SRC"
fi
[ -d "$SRC" ] || { echo "Journey source directory does not exist: $SRC"; exit 0; }
PHENOTYPE_JOURNEYS_ROOT="${PHENOTYPE_JOURNEYS_ROOT:-/Users/kooshapari/CodeProjects/Phenotype/repos/phenotype-journeys}"
if command -v phenotype-journey >/dev/null 2>&1; then BIN=(phenotype-journey); else BIN=(cargo run --quiet --manifest-path "${PHENOTYPE_JOURNEYS_ROOT}/Cargo.toml" --bin phenotype-journey --); fi

GENERATED_BACKUP="$(mktemp -d)"
trap 'rm -rf "$GENERATED_BACKUP"' EXIT
if [ -d "$DST" ]; then
  while IFS= read -r -d '' generated; do
    rel="${generated#"$DST"/}"
    mkdir -p "$GENERATED_BACKUP/$(dirname "$rel")"
    cp "$generated" "$GENERATED_BACKUP/$rel"
	  done < <(find "$DST" -type f \( \
	    -name 'manifest.verified.json' -o \
	    -name '*.rich.mp4' -o \
	    -name '*.silent.mp4' -o \
	    -path '*/audio/*' -o \
	    -name 'preview.gif' \
	  \) -print0)
fi

"${BIN[@]}" sync --from "$SRC" --to "$DST" --kind gui-journeys
node "${REPO_ROOT}/docs-site/scripts/normalize-gui-journeys.mjs" "${REPO_ROOT}/docs-site"
if [ -d "$GENERATED_BACKUP" ]; then
  while IFS= read -r -d '' generated; do
    rel="${generated#"$GENERATED_BACKUP"/}"
    mkdir -p "$DST/$(dirname "$rel")"
    cp "$generated" "$DST/$rel"
	  done < <(find "$GENERATED_BACKUP" -type f -print0)
fi
