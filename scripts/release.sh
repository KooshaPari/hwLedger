#!/usr/bin/env bash
# Thin wrapper for Rust-based release pipeline
# Delegates to: hwledger-release run <tag>

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

exec cargo run -q -p hwledger-release -- run "$@"
