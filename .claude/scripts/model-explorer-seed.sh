#!/usr/bin/env bash
# .claude/scripts/model-explorer-seed.sh
#
# Wraps `cargo run -p hwledger-cli -- seed build --size=2000` from the
# model-explorer Rust workspace (`apps/model-explorer/rust/`), forwarding
# `HF_TOKEN` from env, `.env`, or `hf auth token`. Extra args are passed
# through after the default seed-build invocation.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WORKSPACE_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)/apps/model-explorer/rust"
[[ -f "$WORKSPACE_DIR/Cargo.toml" ]] || { echo "workspace not found: $WORKSPACE_DIR" >&2; exit 1; }

if [[ -z "${HF_TOKEN:-}" && -f "$WORKSPACE_DIR/../../../.env" ]]; then
    HF_TOKEN="$(grep -E '^HF_TOKEN=' "$WORKSPACE_DIR/../../../.env" | head -n1 | cut -d= -f2- | tr -d '"' || true)"
    export HF_TOKEN
fi
if [[ -z "${HF_TOKEN:-}" ]] && command -v hf >/dev/null 2>&1; then
    HF_TOKEN="$(hf auth token 2>/dev/null || true)"
    export HF_TOKEN
fi
[[ -n "${HF_TOKEN:-}" ]] && echo "model-explorer-seed: HF_TOKEN resolved (len=${#HF_TOKEN})" >&2 \
                        || echo "model-explorer-seed: HF_TOKEN not set; running unauthenticated" >&2

cd "$WORKSPACE_DIR"
exec cargo run -p hwledger-cli -- seed build --size=2000 "$@"
