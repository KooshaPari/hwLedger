#!/usr/bin/env bash
# Thin stub: forwards to `phenotype-journey sync --kind streamlit-journeys ...`.
set -euo pipefail
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
SRC="${REPO_ROOT}/apps/streamlit/journeys"
DST="${REPO_ROOT}/docs-site/public/streamlit-journeys"
[ -d "$SRC" ] || { echo "No Streamlit journey source at $SRC; skipping."; exit 0; }
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
    -path '*/audio/*' \
  \) -print0)
fi
"${BIN[@]}" sync --from "$SRC" --to "$DST" --kind streamlit-journeys
if [ -d "$GENERATED_BACKUP" ]; then
  while IFS= read -r -d '' generated; do
    rel="${generated#"$GENERATED_BACKUP"/}"
    target="$DST/$rel"
    if [ -s "$target" ]; then
      continue
    fi
    mkdir -p "$(dirname "$target")"
    cp "$generated" "$target"
  done < <(find "$GENERATED_BACKUP" -type f -print0)
fi
