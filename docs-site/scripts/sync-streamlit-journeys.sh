#!/usr/bin/env bash
# Sync Streamlit journey artefacts from apps/streamlit/journeys into
# docs-site/public/streamlit-journeys/ so <JourneyViewer> can load the
# manifests + keyframes at runtime. Mirrors sync-journey-artefacts.sh.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
SRC_ROOT="${REPO_ROOT}/apps/streamlit/journeys"
DST_ROOT="${REPO_ROOT}/docs-site/public/streamlit-journeys"

mkdir -p "${DST_ROOT}/manifests" "${DST_ROOT}/recordings"

if [[ ! -d "${SRC_ROOT}" ]]; then
  echo "No Streamlit journey source at ${SRC_ROOT}; skipping." >&2
  exit 0
fi

# Manifests (both manifest.json and manifest.verified.json).
if [[ -d "${SRC_ROOT}/manifests" ]]; then
  for slug_dir in "${SRC_ROOT}/manifests"/*/; do
    [[ -d "${slug_dir}" ]] || continue
    slug="$(basename "${slug_dir}")"
    mkdir -p "${DST_ROOT}/manifests/${slug}"
    for f in "${slug_dir}"*.json; do
      [[ -f "${f}" ]] || continue
      cp "${f}" "${DST_ROOT}/manifests/${slug}/"
    done
  done
fi

# Recordings (frames + .mp4 + .gif + per-slug manifest.json).
if [[ -d "${SRC_ROOT}/recordings" ]]; then
  for slug_dir in "${SRC_ROOT}/recordings"/*/; do
    [[ -d "${slug_dir}" ]] || continue
    slug="$(basename "${slug_dir}")"
    rm -rf "${DST_ROOT}/recordings/${slug}"
    cp -r "${slug_dir}" "${DST_ROOT}/recordings/${slug}"
  done
fi

echo "Synced Streamlit journeys to ${DST_ROOT}"
