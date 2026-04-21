#!/usr/bin/env bash
# Boot the hwLedger Streamlit app on port 8511.
#
# Preferred path: the Rust dev harness (`hwledger-streamlit-dev`) which also
# hot-restarts Streamlit when the FFI dylib is rebuilt.
# Fallback: plain `uv run streamlit run app.py` when the Rust binary isn't
# built (e.g. bare checkout / CI smoke test).

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"
PORT="${HWLEDGER_STREAMLIT_PORT:-8511}"

cd "${SCRIPT_DIR}"

if [[ -x "${REPO_ROOT}/target/release/hwledger-streamlit-dev" ]]; then
    echo "→ using Rust dev harness with FFI hot-reload"
    exec "${REPO_ROOT}/target/release/hwledger-streamlit-dev"
fi

if [[ -x "${REPO_ROOT}/target/debug/hwledger-streamlit-dev" ]]; then
    echo "→ using Rust dev harness (debug) with FFI hot-reload"
    exec "${REPO_ROOT}/target/debug/hwledger-streamlit-dev"
fi

if ! command -v uv >/dev/null 2>&1; then
    echo "ERROR: neither hwledger-streamlit-dev nor uv are available." >&2
    echo "  install uv (brew install uv) or build the harness:" >&2
    echo "  cargo build --release -p hwledger-devtools" >&2
    exit 1
fi

echo "→ Rust harness not built; falling back to plain streamlit on :${PORT}"
uv sync
exec uv run streamlit run app.py --server.port "${PORT}" --server.headless true
