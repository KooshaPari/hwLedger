#!/bin/bash
# Sync research briefs from docs/research/ to docs-site/research/

set -e

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
RESEARCH_SRC="${REPO_ROOT}/docs/research"
RESEARCH_DST="${REPO_ROOT}/docs-site/research"

if [ ! -d "$RESEARCH_SRC" ]; then
  echo "Source directory $RESEARCH_SRC does not exist"
  exit 1
fi

mkdir -p "$RESEARCH_DST"

# Copy all research markdown files
for research_file in "$RESEARCH_SRC"/*.md; do
  if [ -f "$research_file" ]; then
    filename=$(basename "$research_file")
    cp "$research_file" "$RESEARCH_DST/$filename"
  fi
done

echo "Synced $(ls -1 $RESEARCH_SRC/*.md 2>/dev/null | wc -l) research briefs to $RESEARCH_DST"
