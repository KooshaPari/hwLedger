#!/usr/bin/env bash
# Thin shim: forwards to the Rust `hwledger-streamlit-journey-runner` binary.
#
# The real boot/health-probe/Playwright/ffmpeg pipeline lives in
# `tools/streamlit-journey-runner/` (Rust). Brief §5 mandates Rust-only glue;
# this script exists only so existing `npm run record` invocations keep working.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
JOURNEYS_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
REPO_ROOT="$(cd "${JOURNEYS_ROOT}/../../.." && pwd)"

BIN="${REPO_ROOT}/target/release/hwledger-streamlit-journey-runner"
if [[ ! -x "${BIN}" ]]; then
    echo "[record-all] building hwledger-streamlit-journey-runner..."
    (cd "${REPO_ROOT}" && cargo build --release -p hwledger-streamlit-journey-runner)
fi

exec "${BIN}" --repo-root "${REPO_ROOT}" "$@"
