#!/bin/bash
# Lint documentation with vale and markdownlint

set -e

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

echo "Running Vale (style guide)..."
if command -v vale &> /dev/null; then
  vale .
else
  echo "Vale not installed. Install with: brew install vale"
  exit 1
fi

echo ""
echo "Running markdownlint (formatting)..."
if command -v markdownlint-cli2 &> /dev/null; then
  markdownlint-cli2 "**/*.md" "#node_modules" || exit 1
else
  echo "markdownlint-cli2 not installed. Install with: npm install -g markdownlint-cli2"
  exit 1
fi

echo ""
echo "All lint checks passed!"
