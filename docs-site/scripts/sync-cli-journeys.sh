#!/bin/bash
# Sync CLI journey artefacts from apps/cli-journeys/ to docs-site/public/cli-journeys/.
# Publishes recordings, keyframes, and BOTH manifest.json + manifest.verified.json.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
SRC="${REPO_ROOT}/apps/cli-journeys"
DST="${REPO_ROOT}/docs-site/public/cli-journeys"

if [ ! -d "$SRC" ]; then
  echo "No CLI journeys source at $SRC — skipping."
  exit 0
fi

mkdir -p "$DST/recordings" "$DST/keyframes" "$DST/manifests"

# Recordings (mp4 + gif)
if [ -d "$SRC/recordings" ]; then
  cp -R "$SRC/recordings/." "$DST/recordings/"
fi

# Keyframes per journey
if [ -d "$SRC/keyframes" ]; then
  for j in "$SRC/keyframes"/*; do
    [ -d "$j" ] || continue
    name=$(basename "$j")
    rm -rf "$DST/keyframes/$name"
    cp -R "$j" "$DST/keyframes/$name"
  done
fi

# Manifests (both manifest.json and manifest.verified.json)
if [ -d "$SRC/manifests" ]; then
  for j in "$SRC/manifests"/*; do
    [ -d "$j" ] || continue
    name=$(basename "$j")
    mkdir -p "$DST/manifests/$name"
    for f in manifest.json manifest.verified.json; do
      if [ -f "$j/$f" ]; then
        cp "$j/$f" "$DST/manifests/$name/$f"
      fi
    done
  done
fi

count=$(find "$DST/manifests" -name manifest.verified.json | wc -l | tr -d ' ')
echo "Synced CLI journeys to $DST ($count verified manifests)"
