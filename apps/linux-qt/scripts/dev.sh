#!/bin/bash
set -euo pipefail

# Hot-reload development for Linux Qt 6 + C++
#
# Prerequisites:
#   - Qt 6 SDK installed
#   - cargo watch installed: cargo install cargo-watch
#
# Hot-reload strategy:
#   - cargo-watch monitors source changes
#   - CMake rebuilds the project
#   - Binary is relaunched
#
# Usage: ./apps/linux-qt/scripts/dev.sh
#
# Note: This is a stub. The Qt client is not yet implemented.
#       Once scaffolded, use the pattern below.

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
APP_DIR="$(dirname "$SCRIPT_DIR")"

echo "hwLedger Linux Qt 6 Hot-Reload Development"
echo "Note: The Qt 6 client is not yet scaffolded."
echo ""
echo "To implement hot reload once ready:"
echo ""
echo "1. Install dependencies:"
echo "   cargo install cargo-watch"
echo ""
echo "2. Use cargo-watch for rebuilds:"
echo "   cargo watch -x build -c -C $APP_DIR"
echo ""
echo "3. Or use cmake with file watchers:"
echo "   cmake --build build --target all --watch"
echo ""
echo "4. Optionally combine with Qt Live Preview:"
echo "   slint-live-preview main.slint"
echo ""
echo "See apps/linux-qt/ for project structure."
