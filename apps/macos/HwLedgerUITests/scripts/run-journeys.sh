#!/bin/bash
# Shim: control flow moved to tools/run-journeys (Rust). Scripting policy §5 mandates Rust-only glue.
# Kept for backwards compatibility with existing call sites.
set -euo pipefail
exec cargo run --quiet --release -p hwledger-run-journeys --manifest-path "$(git rev-parse --show-toplevel)/Cargo.toml" -- "$@"
