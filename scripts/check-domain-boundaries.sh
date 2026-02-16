#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CORE_DIR="$REPO_ROOT/auth9-core"

if ! command -v rg >/dev/null 2>&1; then
  echo "[boundary-check] rg is required" >&2
  exit 2
fi

echo "[boundary-check] checking server route assembly..."
# server/mod.rs should assemble domain routers, not directly bind api handlers.
if rg -n "\.route\(.*api::" "$CORE_DIR/src/server/mod.rs" | rg -v "metrics::metrics_handler" >/dev/null; then
  echo "[boundary-check] FAIL: direct api:: route registration found in src/server/mod.rs" >&2
  rg -n "\.route\(.*api::" "$CORE_DIR/src/server/mod.rs" | rg -v "metrics::metrics_handler" >&2
  exit 1
fi

echo "[boundary-check] checking domain route modules for concrete repo coupling..."
# Route modules should not depend on concrete repository implementations.
if rg -n "RepositoryImpl|crate::repository::" "$CORE_DIR/src/domains" --glob "*/routes.rs" >/dev/null; then
  echo "[boundary-check] FAIL: domain route modules reference repository impl/details" >&2
  rg -n "RepositoryImpl|crate::repository::" "$CORE_DIR/src/domains" --glob "*/routes.rs" >&2
  exit 1
fi

echo "[boundary-check] checking legacy router count..."
if rg -n "build_full_router" "$CORE_DIR/src/server/mod.rs" >/dev/null; then
  echo "[boundary-check] OK: build_full_router exists and composes domains"
fi

echo "[boundary-check] PASS"
