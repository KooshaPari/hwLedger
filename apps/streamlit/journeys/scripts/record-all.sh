#!/bin/bash
# Shim: control flow moved to tools/streamlit-recorder (Rust). Scripting policy §5 mandates Rust-only glue.
# Kept for backwards compatibility with existing call sites (CI, docs).
set -euo pipefail
exec cargo run --quiet --release -p hwledger-streamlit-recorder --manifest-path "$(git rev-parse --show-toplevel)/Cargo.toml" -- "$@"
