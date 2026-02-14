#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TOOL_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
BINARY="$TOOL_ROOT/src-tauri/target/release/qa-orchestrator"

if [[ -x "$BINARY" ]]; then
  exec "$BINARY" --cli "$@"
fi

cd "$TOOL_ROOT/src-tauri"
exec cargo run --release -- --cli "$@"
