#!/bin/bash
# Sync journey artifacts from apps/macos/build/journeys/ to docs-site/public/journeys/

set -e

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
JOURNEY_SRC="${REPO_ROOT}/apps/macos/build/journeys"
JOURNEY_DST="${REPO_ROOT}/docs-site/public/journeys"

mkdir -p "$JOURNEY_DST"

if [ ! -d "$JOURNEY_SRC" ]; then
  echo "Journey source directory does not exist: $JOURNEY_SRC"
  echo "Journeys will be available after running: ./apps/macos/HwLedgerUITests/scripts/run-journeys.sh"
  exit 0
fi

# Copy all journey directories
for journey_dir in "$JOURNEY_SRC"/*; do
  if [ -d "$journey_dir" ]; then
    dirname=$(basename "$journey_dir")
    rm -rf "$JOURNEY_DST/$dirname"
    cp -r "$journey_dir" "$JOURNEY_DST/$dirname"
  fi
done

echo "Synced journeys to $JOURNEY_DST"
