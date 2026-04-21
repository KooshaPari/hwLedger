#!/bin/bash
# Shim: control flow moved to tools/bundle-app (Rust). Scripting policy §5 mandates Rust-only glue.
# Kept for backwards compatibility with existing call sites (xcodebuild pre-actions, docs).
set -euo pipefail
exec cargo run --quiet --release -p hwledger-bundle-app --manifest-path "$(git rev-parse --show-toplevel)/Cargo.toml" -- "$@"
