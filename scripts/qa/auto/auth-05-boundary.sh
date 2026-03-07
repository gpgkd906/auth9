#!/usr/bin/env bash
# QA Auto Test: auth/05-boundary
# Doc: docs/qa/auth/05-boundary.md
# Scenarios: 2 (scenario 1 requires parallel browser logins, scenario 2 requires
#              Keycloak OIDC refresh token - only CORS test is fully scriptable)
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/../lib/runner.sh"

scenario 3 "CORS - allowed origin gets correct headers" '
  resp=$(curl -s -o /dev/null -w "%{http_code}" \
    -X OPTIONS "${API_BASE}/api/v1/auth/token" \
    -H "Origin: http://localhost:3000" \
    -H "Access-Control-Request-Method: POST" \
    -H "Access-Control-Request-Headers: content-type")
  assert_match "$resp" "^(200|204)$" "CORS preflight returns 200 or 204"

  headers=$(curl -sI -X OPTIONS "${API_BASE}/api/v1/auth/token" \
    -H "Origin: http://localhost:3000" \
    -H "Access-Control-Request-Method: POST" \
    -H "Access-Control-Request-Headers: content-type" 2>&1)
  assert_contains "$headers" "localhost:3000" "CORS allows localhost:3000"
'

run_all
