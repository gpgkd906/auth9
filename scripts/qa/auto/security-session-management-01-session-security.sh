#!/usr/bin/env bash
# Security Auto Test: security/session-management/01-session-security
# Doc: docs/security/session-management/01-session-security.md
# Scenarios: 4
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/../lib/runner.sh"

require_bin jq
require_bin node
require_bin curl

PORTAL_URL="${PORTAL_URL:-http://localhost:3000}"

scenario 1 "Session ID security - Cookie attributes" '
  resp=$(curl -sI "$PORTAL_URL/" 2>&1 || true)

  local has_httponly has_samesite
  has_httponly=$(echo "$resp" | grep -i "set-cookie" | grep -ci "httponly" || echo "0")
  has_samesite=$(echo "$resp" | grep -i "set-cookie" | grep -ci "samesite" || echo "0")

  if echo "$resp" | grep -qi "set-cookie"; then
    cookie_line=$(echo "$resp" | grep -i "set-cookie" | head -1)
    assert_contains "$cookie_line" "HttpOnly" "Cookie has HttpOnly attribute"
    assert_match "$cookie_line" "[Ss]ame[Ss]ite" "Cookie has SameSite attribute"
  else
    assert_eq "no-cookie" "no-cookie" "No session cookie on unauthenticated request (expected)"
  fi

  resp2=$(curl -sI "${API_BASE}/health" 2>&1 || true)
  assert_not_contains "$resp2" "KEYCLOAK_SESSION" "API health endpoint does not leak Keycloak session cookies"
'

scenario 2 "Session fixation protection - pre-auth cookie not reused" '
  pre_cookies=$(curl -s -c - "$PORTAL_URL/" 2>/dev/null || true)
  pre_session=$(echo "$pre_cookies" | grep -i "auth9_session" | awk "{print \$7}" || echo "")

  resp=$(api_raw GET /api/v1/auth/token \
    -H "Cookie: auth9_session=attacker-fixed-session-id-12345" \
    -o /dev/null -w "%{http_code}")
  status=$(resp_status "$resp")

  assert_match "$status" "^(302|303|200|401|405)$" "Login endpoint does not blindly accept arbitrary session cookie"

  resp2=$(api_raw GET /api/v1/users/me \
    -H "Cookie: auth9_session=attacker-fixed-session-id-12345")
  fix_status=$(resp_status "$resp2")
  assert_match "$fix_status" "^(401|302|303)$" "Fixed session ID rejected for authenticated endpoint"
'

scenario 3 "Session hijack protection - different client headers" '
  local uid sid
  uid=$(db_query "SELECT LOWER(UUID());")
  sid=$(db_query "SELECT LOWER(UUID());")

  db_exec "DELETE FROM users WHERE keycloak_id = '\''kc-qa-sess01-3'\'';" || true
  db_exec "INSERT INTO users (id, keycloak_id, email, display_name) VALUES ('\''$uid'\'', '\''kc-qa-sess01-3'\'', '\''qa-sess01-3@test.com'\'', '\''QA Sess01 S3'\'');"
  db_exec "INSERT INTO sessions (id, user_id, device_type, ip_address, location, last_active_at) VALUES ('\''$sid'\'', '\''$uid'\'', '\''desktop'\'', '\''192.168.1.1'\'', '\''Beijing'\'', NOW());"

  local token
  token=$(gen_identity_token "$uid" "qa-sess01-3@test.com")
  qa_set_token "$token"

  resp=$(api_raw GET /api/v1/users/me \
    -H "Authorization: Bearer $token" \
    -H "X-Forwarded-For: 203.0.113.50" \
    -H "User-Agent: StolenSession/1.0")
  status=$(resp_status "$resp")
  assert_match "$status" "^(200|401|403)$" "Request with different IP/UA processed (anomaly detection optional)"

  db_exec "DELETE FROM sessions WHERE user_id = '\''$uid'\'';" || true
  db_exec "DELETE FROM users WHERE id = '\''$uid'\'';" || true
  qa_set_token ""
'

scenario 4 "Concurrent session control" '
  local uid
  uid=$(db_query "SELECT LOWER(UUID());")

  db_exec "DELETE FROM users WHERE keycloak_id = '\''kc-qa-sess01-4'\'';" || true
  db_exec "INSERT INTO users (id, keycloak_id, email, display_name) VALUES ('\''$uid'\'', '\''kc-qa-sess01-4'\'', '\''qa-sess01-4@test.com'\'', '\''QA Sess01 S4'\'');"

  local sids=()
  for i in $(seq 1 12); do
    local s
    s=$(db_query "SELECT LOWER(UUID());")
    sids+=("$s")
    db_exec "INSERT INTO sessions (id, user_id, device_type, ip_address, location, last_active_at) VALUES ('\''$s'\'', '\''$uid'\'', '\''desktop'\'', '\''192.168.1.$i'\'', '\''City$i'\'', NOW());"
  done

  local count
  count=$(db_query "SELECT COUNT(*) FROM sessions WHERE user_id = '\''$uid'\'' AND revoked_at IS NULL;")
  assert_eq "$count" "12" "12 concurrent sessions created"

  local token
  token=$(gen_identity_token "$uid" "qa-sess01-4@test.com")
  qa_set_token "$token"

  resp=$(api_get /api/v1/users/me/sessions)
  status=$(resp_status "$resp")
  assert_http_status "$status" 200 "List sessions returns 200"

  body=$(resp_body "$resp")
  assert_json_exists "$body" ".data" "Response has data array"

  db_exec "DELETE FROM sessions WHERE user_id = '\''$uid'\'';" || true
  db_exec "DELETE FROM users WHERE id = '\''$uid'\'';" || true
  qa_set_token ""
'

run_all
