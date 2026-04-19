#!/bin/bash
# Sync ADR files from docs/adr/ to docs-site/architecture/adrs/

set -e

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
ADR_SRC="${REPO_ROOT}/docs/adr"
ADR_DST="${REPO_ROOT}/docs-site/architecture/adrs"

if [ ! -d "$ADR_SRC" ]; then
  echo "Source directory $ADR_SRC does not exist"
  exit 1
fi

mkdir -p "$ADR_DST"

# Copy all ADR markdown files
for adr_file in "$ADR_SRC"/*.md; do
  if [ -f "$adr_file" ]; then
    filename=$(basename "$adr_file")
    cp "$adr_file" "$ADR_DST/$filename"
  fi
done

echo "Synced $(ls -1 $ADR_SRC/*.md 2>/dev/null | wc -l) ADRs to $ADR_DST"
