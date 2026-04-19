#!/bin/bash
set -euo pipefail

# Hot-reload development script for macOS SwiftUI app
# Watches Sources/**/*.swift and rebuilds + relaunches on change
# Target cycle time: <3s

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
APP_DIR="$(dirname "$SCRIPT_DIR")"
REPO_ROOT="$(cd "$APP_DIR/../../.." && pwd)"

echo "hwLedger macOS Hot-Reload Dev"
echo "Watching $APP_DIR/Sources for changes..."

# Check for fswatch
if ! command -v fswatch &> /dev/null; then
    echo "ERROR: fswatch not found. Install it:"
    echo "  brew install fswatch"
    exit 1
fi

# Check if app is already running
check_running() {
    pgrep -f "HwLedger" > /dev/null 2>&1 || return 1
}

build_and_run() {
    local start_time=$(date +%s%3N)

    # Kill existing instance
    if check_running; then
        echo "Stopping existing HwLedger..."
        pkill -f "HwLedger" || true
        sleep 0.5
    fi

    # Build
    echo "[$(date +'%H:%M:%S')] Building..."
    if ! swift build > /tmp/hwledger-build.log 2>&1; then
        echo "Build failed:"
        tail -20 /tmp/hwledger-build.log
        return 1
    fi

    # Run
    echo "[$(date +'%H:%M:%S')] Launching HwLedger..."
    cd "$APP_DIR"
    swift run HwLedgerApp > /dev/null 2>&1 &

    local end_time=$(date +%s%3N)
    local elapsed=$((end_time - start_time))
    echo "✓ Cycle complete in ${elapsed}ms"
}

# Initial build
build_and_run

# Watch for changes
fswatch -r "$APP_DIR/Sources" --event Updated | while read -r file; do
    echo ""
    build_and_run
done
