#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TOOL_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

if [[ ! -f "$TOOL_ROOT/package.json" ]]; then
  echo "qa-orchestrator package.json not found: $TOOL_ROOT" >&2
  exit 1
fi

cd "$TOOL_ROOT"
echo "Launching Auth9 QA Orchestrator UI..."
echo "Working directory: $TOOL_ROOT"

if [[ ! -d "$TOOL_ROOT/node_modules" ]]; then
  echo "node_modules not found. Run: cd tools/qa-orchestrator && npm install"
fi

npm run tauri:dev
