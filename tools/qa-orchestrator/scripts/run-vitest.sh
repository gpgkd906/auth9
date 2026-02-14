#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TOOL_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
VITEST_BIN="$TOOL_ROOT/node_modules/.bin/vitest"

if [[ ! -x "$VITEST_BIN" ]]; then
  echo "vitest not found in tools/qa-orchestrator/node_modules" >&2
  echo "Install test dependencies first:" >&2
  echo "  cd tools/qa-orchestrator && npm install -D vitest @vitest/coverage-v8" >&2
  exit 1
fi

cd "$TOOL_ROOT"
exec "$VITEST_BIN" "$@"

