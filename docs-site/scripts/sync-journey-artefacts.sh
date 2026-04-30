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
DIST_SRC="${REPO_ROOT}/docs-site/.vitepress/dist/gui-journeys"
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

has_gui_media() {
  [ -d "$1" ] && find "$1" -type f \( \
    -name '*.png' -o \
    -name '*.jpg' -o \
    -name '*.jpeg' -o \
    -name '*.gif' -o \
    -name '*.mp4' \
  \) -print -quit | grep -q .
}

has_verified_gui_media() {
  [ -d "$1" ] && find "$1" -type f \( \
    -name 'manifest.verified.json' -o \
    -name '*.rich.mp4' \
  \) -print -quit | grep -q .
}

if ! has_gui_media "$SRC" && has_verified_gui_media "$DST"; then
  echo "GUI journey source has manifests but no media; preserving verified docs assets in $DST"
  node "${REPO_ROOT}/docs-site/scripts/normalize-gui-journeys.mjs" "${REPO_ROOT}/docs-site"
  exit 0
fi

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
for manifest in "$DST"/*/manifest.json; do
  [ -f "$manifest" ] || continue
  id="$(basename "$(dirname "$manifest")")"
  target="$DST/$id/$id.rich.mp4"
  if [ ! -s "$target" ]; then
    if [ -s "$RECORDINGS_SRC/$id/recording.rich.mp4" ]; then
      cp "$RECORDINGS_SRC/$id/recording.rich.mp4" "$target"
    elif [ -s "$DIST_SRC/$id/$id.rich.mp4" ]; then
      cp "$DIST_SRC/$id/$id.rich.mp4" "$target"
    fi
  fi
  silent="$DST/$id/$id.silent.mp4"
  if [ ! -s "$silent" ] && [ -s "$DIST_SRC/$id/$id.silent.mp4" ]; then
    cp "$DIST_SRC/$id/$id.silent.mp4" "$silent"
  fi
done
node "${REPO_ROOT}/docs-site/scripts/normalize-gui-journeys.mjs" "${REPO_ROOT}/docs-site"
