#!/bin/bash
set -euo pipefail

# Hot-reload development for Linux Slint UI
#
# Prerequisites:
#   - Slint SDK installed
#   - cargo-watch installed: cargo install cargo-watch
#
# Hot-reload strategy:
#   - slint-live-preview watches .slint files
#   - cargo watch monitors Rust code
#   - Rebuild triggered on any change
#
# Usage: ./apps/linux-slint/scripts/dev.sh
#
# Note: This is a stub. The Slint client is not yet implemented.
#       Once scaffolded, use the pattern below.

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
APP_DIR="$(dirname "$SCRIPT_DIR")"

echo "hwLedger Linux Slint Hot-Reload Development"
echo "Note: The Slint client is not yet scaffolded."
echo ""
echo "To implement hot reload once ready:"
echo ""
echo "1. Install dependencies:"
echo "   cargo install cargo-watch"
echo "   cargo install slint-lsp"
echo ""
echo "2. Use slint-live-preview for instant UI updates:"
echo "   slint-live-preview $APP_DIR/ui/main.slint"
echo ""
echo "3. In parallel, run cargo-watch for Rust code:"
echo "   cargo watch -x 'build --release' -C $APP_DIR"
echo ""
echo "4. Or combine into a single watch command:"
echo "   cargo watch -x 'build --release -p hwledger-slint'"
echo ""
echo "See apps/linux-slint/ for project structure."
