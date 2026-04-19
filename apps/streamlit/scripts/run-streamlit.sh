#!/bin/bash
set -euo pipefail

# hwLedger Streamlit launcher
# Prerequisites: uv (https://docs.astral.sh/uv/)

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
APP_DIR="$(dirname "$SCRIPT_DIR")"

# Check for uv
if ! command -v uv &> /dev/null; then
    echo "ERROR: uv not found. Install it first:"
    echo "  brew install uv"
    exit 1
fi

# Sync dependencies and run
echo "Syncing dependencies..."
cd "$APP_DIR"
uv sync

echo "Launching Streamlit on port 8501..."
uv run streamlit run app.py --server.port 8501
