#!/usr/bin/env bash
# Scripting policy: thin wrapper; real logic in tools/cli-journey-record/src/main.rs.
set -euo pipefail
exec cargo run --quiet --manifest-path "$(dirname "${BASH_SOURCE[0]}")/../../../Cargo.toml" --bin hwledger-cli-journey-record -- "$@"
