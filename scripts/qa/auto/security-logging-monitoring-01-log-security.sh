#!/usr/bin/env bash
# Security Auto Test: security/logging-monitoring/01-log-security
# Doc: docs/security/logging-monitoring/01-log-security.md
# Scenarios: 5
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/../lib/runner.sh"

require_bin jq
require_bin node

scenario 1 "Log injection - CRLF in user input" '
  local injected_email="admin%0a[INFO] Login successful for admin from 127.0.0.1"

  resp=$(api_raw POST /api/v1/auth/forgot-password \
    -H "Content-Type: application/json" \
    -d "{\"email\":\"$injected_email\"}")
  status=$(resp_status "$resp")
  assert_match "$status" "^(400|422|404|200)$" "CRLF injection in email handled gracefully"

  resp2=$(api_raw POST /api/v1/auth/forgot-password \
    -H "Content-Type: application/json" \
    -d "{\"email\":\"test\\\\n[CRITICAL] System compromised\"}")
  status2=$(resp_status "$resp2")
  assert_match "$status2" "^(400|422|404|200)$" "Newline injection in email handled gracefully"
'

scenario 2 "Audit log completeness and immutability" '
  local admin_token
  admin_token=$(gen_default_admin_token)
  qa_set_token "$admin_token"

  resp=$(api_get "/api/v1/audit-logs?limit=5")
  status=$(resp_status "$resp")
  assert_http_status "$status" 200 "Audit logs accessible by admin"

  body=$(resp_body "$resp")
  assert_json_exists "$body" ".data" "Audit log response has data array"

  resp=$(api_raw DELETE /api/v1/audit-logs/00000000-0000-0000-0000-000000000001 \
    -H "Authorization: Bearer $admin_token")
  del_status=$(resp_status "$resp")
  assert_match "$del_status" "^(404|405|403)$" "DELETE audit log endpoint returns 404/405/403"

  resp=$(api_raw PUT /api/v1/audit-logs/00000000-0000-0000-0000-000000000001 \
    -H "Authorization: Bearer $admin_token" \
    -H "Content-Type: application/json" \
    -d "{\"action\":\"modified\"}")
  put_status=$(resp_status "$resp")
  assert_match "$put_status" "^(404|405|403)$" "PUT audit log endpoint returns 404/405/403"

  qa_set_token ""
'

scenario 3 "Sensitive data not leaked in error responses" '
  resp=$(api_raw POST /api/v1/tenants \
    -H "Content-Type: application/json" \
    -d "{invalid json}")
  body=$(resp_body "$resp")
  assert_not_contains "$body" "stack" "Malformed JSON error does not contain stack trace"
  assert_not_contains "$body" "src/" "Malformed JSON error does not contain source paths"
  assert_not_contains "$body" "DATABASE_URL" "Error does not contain DATABASE_URL"

  resp2=$(api_raw POST "/api/v1/auth/token" \
    -H "Content-Type: application/json" \
    -d "{\"grant_type\":\"client_credentials\",\"client_id\":\"invalid\",\"client_secret\":\"invalid-secret\"}")
  body2=$(resp_body "$resp2")
  assert_not_contains "$body2" "invalid-secret" "Error does not echo back client_secret"
  assert_not_contains "$body2" "password" "Error does not contain password field"
'

scenario 4 "Security alert system - brute force detection via webhook" '
  local admin_token
  admin_token=$(gen_default_admin_token)

  local SECRET="dev-webhook-secret-change-in-production"  # pragma: allowlist secret
  local fake_user_id="550e8400-e29b-41d4-a716-446655440099"

  for i in $(seq 1 6); do
    local ts
    ts=$(date +%s)
    local body="{\"type\":\"LOGIN_ERROR\",\"realmId\":\"auth9\",\"clientId\":\"auth9-portal\",\"userId\":\"$fake_user_id\",\"ipAddress\":\"10.99.99.1\",\"error\":\"invalid_user_credentials\",\"time\":${ts}000,\"details\":{\"username\":\"brute-test@test.com\",\"email\":\"brute-test@test.com\"}}"
    local sig
    sig=$(printf "%s" "$body" | openssl dgst -sha256 -hmac "$SECRET" 2>/dev/null | awk "{print \$NF}")
    curl -s -X POST "${API_BASE}/api/v1/keycloak/events" \
      -H "Content-Type: application/json" \
      -H "x-keycloak-signature: sha256=$sig" \
      -d "$body" >/dev/null 2>&1 || true
    sleep 0.5
  done

  sleep 2
  qa_set_token "$admin_token"
  resp=$(api_get "/api/v1/security/alerts?limit=10")
  status=$(resp_status "$resp")
  assert_http_status "$status" 200 "Security alerts endpoint accessible"

  body=$(resp_body "$resp")
  assert_json_exists "$body" ".data" "Security alerts response has data"
  qa_set_token ""
'

scenario 5 "Error responses do not leak internals" '
  resp=$(api_raw GET /api/v1/nonexistent-endpoint-xyz)
  status=$(resp_status "$resp")
  body=$(resp_body "$resp")
  assert_match "$status" "^(404|405)$" "Nonexistent endpoint returns 404/405"
  assert_not_contains "$body" "at /src" "404 does not leak source file paths"
  assert_not_contains "$body" "panic" "404 does not contain panic info"

  resp2=$(api_raw GET /api/v1/tenants)
  status2=$(resp_status "$resp2")
  body2=$(resp_body "$resp2")
  assert_http_status "$status2" 401 "Unauthenticated request returns 401"
  assert_json_exists "$body2" ".error" "Error response has error field"

  resp3=$(api_raw GET "/api/v1/tenants/not-a-valid-uuid" \
    -H "Authorization: Bearer $(gen_default_admin_token)")
  body3=$(resp_body "$resp3")
  assert_not_contains "$body3" "SQL" "Invalid UUID does not leak SQL info"
  assert_not_contains "$body3" "database" "Invalid UUID does not leak database info"
'

run_all
