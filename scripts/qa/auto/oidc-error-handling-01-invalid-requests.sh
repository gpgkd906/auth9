#!/usr/bin/env bash
# QA Auto Test: oidc-error-handling-01
# Doc: docs/oidc/error-handling/01-invalid-requests.md
# Scenarios: 5
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/../lib/runner.sh"

require_bin jq

scenario 1 "Authorize without client_id returns error" '
  body=$(curl -s -w '\''\n%{http_code}'\'' "${API_BASE}/api/v1/auth/authorize?response_type=code&redirect_uri=https://example.com/callback&scope=openid&state=test123")
  status=$(echo "$body" | tail -1)
  body=$(echo "$body" | sed '\''$d'\'')
  if [ "$status" != "400" ] && [ "$status" != "302" ]; then
    assert_eq "$status" "400" "Authorize without client_id returns 400 or 302"
  fi
'

scenario 2 "Authorize with invalid client_id returns error" '
  body=$(curl -s -w '\''\n%{http_code}'\'' "${API_BASE}/api/v1/auth/authorize?client_id=nonexistent-client&response_type=code&redirect_uri=https://example.com/callback&scope=openid&state=test123")
  status=$(echo "$body" | tail -1)
  body=$(echo "$body" | sed '\''$d'\'')
  if [ "$status" != "400" ] && [ "$status" != "302" ]; then
    assert_eq "$status" "400" "Authorize with invalid client_id returns 400 or 302"
  fi
'

scenario 3 "Authorize with unregistered redirect_uri returns error" '
  body=$(curl -s -w '\''\n%{http_code}'\'' "${API_BASE}/api/v1/auth/authorize?client_id=test&response_type=code&redirect_uri=https://evil.example.com/callback&scope=openid&state=test123")
  status=$(echo "$body" | tail -1)
  body=$(echo "$body" | sed '\''$d'\'')
  assert_http_status "$status" 400 "Authorize with unregistered redirect_uri returns 400"
'

scenario 4 "Token request with unsupported grant_type returns error" '
  body=$(curl -s -w '\''\n%{http_code}'\'' -X POST "${API_BASE}/api/v1/auth/token" \
    -H "Content-Type: application/x-www-form-urlencoded" \
    -d "grant_type=device_code&client_id=test")
  status=$(echo "$body" | tail -1)
  body=$(echo "$body" | sed '\''$d'\'')
  assert_http_status "$status" 400 "Token with unsupported grant_type returns 400"
'

scenario 5 "Authorize without openid scope" '
  body=$(curl -s -w '\''\n%{http_code}'\'' "${API_BASE}/api/v1/auth/authorize?client_id=test&response_type=code&redirect_uri=https://example.com/callback&scope=profile&state=test123")
  status=$(echo "$body" | tail -1)
  body=$(echo "$body" | sed '\''$d'\'')
  if [ "$status" = "200" ]; then
    assert_eq "$status" "not 200" "Authorize without openid scope should not return 200"
  fi
'

run_all
