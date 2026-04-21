#!/usr/bin/env bash
# Thin stub: forwards to `phenotype-journey record --tapes-dir ...`.
# The real pipeline lives in Rust (phenotype-journeys/bin/phenotype-journey).
# Stub kept so existing callers and docs that reference the path keep working;
# new code should invoke the binary directly (see docs/engineering/scripting-policy.md).
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
JOURNEYS_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
REPO_ROOT="$(cd "${JOURNEYS_ROOT}/../.." && pwd)"

# Ensure the hwledger-cli → hwledger tape alias exists (tapes call `hwledger`).
CLI_SRC="${REPO_ROOT}/target/release/hwledger-cli"
CLI_BIN="${REPO_ROOT}/target/release/hwledger"
if [ ! -f "${CLI_SRC}" ]; then
    echo "Error: hwledger-cli binary not found at ${CLI_SRC}" >&2
    echo "Run: cargo build --release -p hwledger-cli" >&2
    exit 1
fi
if [ ! -f "${CLI_BIN}" ] || [ "${CLI_SRC}" -nt "${CLI_BIN}" ]; then
    ln -sf "hwledger-cli" "${CLI_BIN}"
fi

PHENOTYPE_JOURNEYS_ROOT="${PHENOTYPE_JOURNEYS_ROOT:-/Users/kooshapari/CodeProjects/Phenotype/repos/phenotype-journeys}"
if command -v phenotype-journey >/dev/null 2>&1; then
    BIN=(phenotype-journey)
else
    BIN=(cargo run --quiet --manifest-path "${PHENOTYPE_JOURNEYS_ROOT}/Cargo.toml" --bin phenotype-journey --)
fi

exec "${BIN[@]}" record \
    --tapes-dir "${JOURNEYS_ROOT}/tapes" \
    --recordings-dir "${JOURNEYS_ROOT}/recordings" \
    --cwd "${REPO_ROOT}" \
    --path-prepend "${REPO_ROOT}/target/release" \
    --summary-path "${JOURNEYS_ROOT}/record-summary.json" \
    "$@"
