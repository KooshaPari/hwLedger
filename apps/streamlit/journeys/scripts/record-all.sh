#!/usr/bin/env bash
# Record all Streamlit Playwright journeys.
#
# Boots the Streamlit app on a free port, waits for it to respond, runs Playwright,
# shuts the app down, then converts Playwright's video.webm artefacts into
# recording.mp4 + recording.gif beside the manifest.json for each journey, and
# copies manifests into apps/streamlit/journeys/manifests/<slug>/manifest.json.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
JOURNEYS_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
APP_ROOT="$(cd "${JOURNEYS_ROOT}/.." && pwd)"
RECORDINGS_DIR="${JOURNEYS_ROOT}/recordings"
MANIFESTS_DIR="${JOURNEYS_ROOT}/manifests"
PW_OUTPUT_DIR="${JOURNEYS_ROOT}/playwright-output"

PORT="${STREAMLIT_PORT:-8599}"
STREAMLIT_URL="http://127.0.0.1:${PORT}"
HEALTH_URL="${STREAMLIT_URL}/_stcore/health"

mkdir -p "${RECORDINGS_DIR}" "${MANIFESTS_DIR}" "${PW_OUTPUT_DIR}"

for tool in ffmpeg npx; do
  if ! command -v "${tool}" >/dev/null 2>&1; then
    echo "ERROR: '${tool}' is required on PATH." >&2
    exit 2
  fi
done

cleanup() {
  if [[ -n "${STREAMLIT_PID:-}" ]] && kill -0 "${STREAMLIT_PID}" 2>/dev/null; then
    echo "[record-all] stopping streamlit pid=${STREAMLIT_PID}"
    kill "${STREAMLIT_PID}" 2>/dev/null || true
    wait "${STREAMLIT_PID}" 2>/dev/null || true
  fi
}
trap cleanup EXIT

echo "[record-all] booting streamlit on ${STREAMLIT_URL}"
pushd "${APP_ROOT}" >/dev/null
if command -v uv >/dev/null 2>&1; then
  uv run streamlit run app.py \
    --server.port "${PORT}" \
    --server.headless true \
    --browser.gatherUsageStats false \
    --server.runOnSave false \
    >"${JOURNEYS_ROOT}/.streamlit.log" 2>&1 &
else
  streamlit run app.py \
    --server.port "${PORT}" \
    --server.headless true \
    --browser.gatherUsageStats false \
    --server.runOnSave false \
    >"${JOURNEYS_ROOT}/.streamlit.log" 2>&1 &
fi
STREAMLIT_PID=$!
popd >/dev/null

echo "[record-all] waiting for ${HEALTH_URL}"
for i in $(seq 1 60); do
  if curl -sSf "${HEALTH_URL}" >/dev/null 2>&1; then
    echo "[record-all] streamlit healthy after ${i}s"
    break
  fi
  if ! kill -0 "${STREAMLIT_PID}" 2>/dev/null; then
    echo "[record-all] streamlit exited early — see .streamlit.log" >&2
    tail -n 40 "${JOURNEYS_ROOT}/.streamlit.log" >&2 || true
    exit 3
  fi
  sleep 1
done

if ! curl -sSf "${HEALTH_URL}" >/dev/null 2>&1; then
  echo "[record-all] streamlit failed to become healthy" >&2
  exit 4
fi

echo "[record-all] installing playwright browsers (chromium) if needed"
pushd "${JOURNEYS_ROOT}" >/dev/null
if [[ ! -d node_modules ]]; then
  if command -v bun >/dev/null 2>&1; then
    bun install
  else
    npm install
  fi
fi
npx --yes playwright install chromium >/dev/null

echo "[record-all] running playwright"
STREAMLIT_URL="${STREAMLIT_URL}" HEADLESS="${HEADLESS:-0}" \
  npx --yes playwright test --config=playwright.config.ts

popd >/dev/null

echo "[record-all] converting videos"
# Playwright stores per-test videos under playwright-output/<spec>-<name>/video.webm.
# We match each video to its recording/<slug>/ dir by finding the manifest that lives
# next to the newest frame-001.png and using its id.
shopt -s nullglob
for manifest in "${RECORDINGS_DIR}"/*/manifest.json; do
  slug="$(basename "$(dirname "${manifest}")")"
  # Find the matching playwright video by looking inside playwright-output.
  # Playwright names the folder after the spec + test title; we just pick the
  # most recently modified webm that belongs to a folder whose name contains
  # the slug substring (e.g. "streamlit-planner").
  keyword="${slug#streamlit-}"
  video=""
  # newest-first scan
  while IFS= read -r candidate; do
    dir="$(basename "$(dirname "${candidate}")")"
    case "${dir}" in
      *"${keyword}"*) video="${candidate}"; break ;;
    esac
  done < <(ls -t "${PW_OUTPUT_DIR}"/*/video.webm 2>/dev/null || true)

  target_mp4="$(dirname "${manifest}")/${slug}.mp4"
  target_gif="$(dirname "${manifest}")/${slug}.gif"
  if [[ -n "${video}" && -f "${video}" ]]; then
    echo "[record-all] ${slug}: ${video} -> ${target_mp4} + .gif"
    ffmpeg -y -loglevel error -i "${video}" -c:v libx264 -pix_fmt yuv420p -movflags +faststart "${target_mp4}"
    ffmpeg -y -loglevel error -i "${video}" -vf "fps=10,scale=800:-1:flags=lanczos" "${target_gif}"
  else
    echo "[record-all] ${slug}: no playwright video found; skipping mp4/gif conversion"
  fi

  # Copy manifest into manifests/<slug>/manifest.json so verify-manifests.sh can find it.
  mkdir -p "${MANIFESTS_DIR}/${slug}"
  cp "${manifest}" "${MANIFESTS_DIR}/${slug}/manifest.json"
done

echo "[record-all] done"
