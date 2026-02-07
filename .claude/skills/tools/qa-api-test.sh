#!/usr/bin/env bash
# Auth9 QA API testing helper.
# Wraps curl with automatic admin token injection.
#
# Usage:
#   qa-api-test.sh GET  /api/v1/tenants
#   qa-api-test.sh POST /api/v1/users '{"email":"test@example.com","password":"Pass123!"}'
#   qa-api-test.sh PUT  /api/v1/tenants/{id}/password-policy '{"min_length":12}'
#
# Environment:
#   AUTH9_URL  - Base URL (default: http://localhost:8080)
#   AUTH9_TOKEN - Pre-generated token (skips gen_token.js if set)

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"

AUTH9_URL="${AUTH9_URL:-http://localhost:8080}"

# Generate or reuse token
if [ -z "${AUTH9_TOKEN:-}" ]; then
  AUTH9_TOKEN="$(node "$SCRIPT_DIR/gen_token.js" 2>/dev/null)"
fi

METHOD="${1:?Usage: qa-api-test.sh METHOD PATH [JSON_BODY]}"
PATH_PART="${2:?Usage: qa-api-test.sh METHOD PATH [JSON_BODY]}"
BODY="${3:-}"

if [ -n "$BODY" ]; then
  # Force IPv4 to avoid occasional IPv6 localhost connection issues in sandboxed envs.
  curl -4 -s -X "$METHOD" "${AUTH9_URL}${PATH_PART}" \
    -H "Authorization: Bearer $AUTH9_TOKEN" \
    -H "Content-Type: application/json" \
    -d "$BODY"
else
  # Force IPv4 to avoid occasional IPv6 localhost connection issues in sandboxed envs.
  curl -4 -s -X "$METHOD" "${AUTH9_URL}${PATH_PART}" \
    -H "Authorization: Bearer $AUTH9_TOKEN"
fi
