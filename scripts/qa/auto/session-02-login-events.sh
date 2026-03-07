#!/usr/bin/env bash
# QA Auto Test: session/02-login-events
# Doc: docs/qa/session/02-login-events.md
# Scenarios: 5
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/../lib/runner.sh"

require_bin jq
require_bin openssl

WEBHOOK_SECRET="${WEBHOOK_SECRET:-dev-webhook-secret-change-in-production}"

_send_kc_event() {
  local payload="$1"
  local sig
  sig=$(printf '%s' "$payload" | openssl dgst -sha256 -hmac "$WEBHOOK_SECRET" | awk '{print $NF}')
  api_raw POST /api/v1/keycloak/events \
    -H "Content-Type: application/json" \
    -H "x-keycloak-signature: sha256=$sig" \
    -d "$payload"
}

scenario 1 "Login success event recording" '
  local uid kc_id email ts
  uid=$(db_query "SELECT LOWER(UUID());")
  kc_id="kc-qa-s02-1-$(date +%s)"
  email="qa-s02-1-$(date +%s)@example.com"
  ts=$(date +%s)000

  db_exec "DELETE FROM login_events WHERE email = '\''$email'\'';" || true
  db_exec "DELETE FROM users WHERE keycloak_id = '\''$kc_id'\'';" || true
  db_exec "INSERT INTO users (id, keycloak_id, email, display_name) VALUES ('\''$uid'\'', '\''$kc_id'\'', '\''$email'\'', '\''QA Login S1'\'');"

  local evt_body="{\"type\":\"LOGIN\",\"realmId\":\"auth9\",\"userId\":\"$kc_id\",\"ipAddress\":\"203.0.113.1\",\"time\":$ts,\"details\":{\"username\":\"qa-s02-1\",\"email\":\"$email\"}}"
  local resp
  resp=$(_send_kc_event "$evt_body")
  assert_http_status "$(resp_status "$resp")" 204 "Keycloak LOGIN event accepted"

  sleep 1

  local token
  token=$(gen_default_admin_token)
  qa_set_token "$token"

  resp=$(api_get "/api/v1/analytics/login-events?email=$email")
  assert_http_status "$(resp_status "$resp")" 200 "GET login-events returns 200"
  local body
  body=$(resp_body "$resp")
  local count
  count=$(echo "$body" | jq ".data | length")
  assert_match "$count" "^[1-9]" "at least 1 login event recorded"
  assert_json_field "$body" ".data[0].event_type" "success" "event_type is success"

  db_exec "DELETE FROM login_events WHERE email = '\''$email'\'';" || true
  db_exec "DELETE FROM users WHERE id = '\''$uid'\'';" || true
  qa_set_token ""
'

scenario 2 "Login failed event recording" '
  local uid kc_id email ts
  uid=$(db_query "SELECT LOWER(UUID());")
  kc_id="kc-qa-s02-2-$(date +%s)"
  email="qa-s02-2-$(date +%s)@example.com"
  ts=$(date +%s)000

  db_exec "DELETE FROM login_events WHERE email = '\''$email'\'';" || true
  db_exec "DELETE FROM users WHERE keycloak_id = '\''$kc_id'\'';" || true
  db_exec "INSERT INTO users (id, keycloak_id, email, display_name) VALUES ('\''$uid'\'', '\''$kc_id'\'', '\''$email'\'', '\''QA Login S2'\'');"

  local evt_body="{\"type\":\"LOGIN_ERROR\",\"realmId\":\"auth9\",\"userId\":\"$kc_id\",\"ipAddress\":\"192.168.1.100\",\"error\":\"invalid_user_credentials\",\"time\":$ts,\"details\":{\"username\":\"qa-s02-2\",\"email\":\"$email\",\"credential_type\":\"password\"}}"
  local resp
  resp=$(_send_kc_event "$evt_body")
  assert_http_status "$(resp_status "$resp")" 204 "Keycloak LOGIN_ERROR event accepted"

  sleep 1

  local token
  token=$(gen_default_admin_token)
  qa_set_token "$token"

  resp=$(api_get "/api/v1/analytics/login-events?email=$email")
  assert_http_status "$(resp_status "$resp")" 200 "GET login-events returns 200"
  local body
  body=$(resp_body "$resp")
  assert_json_field "$body" ".data[0].event_type" "failed_password" "event_type is failed_password"

  db_exec "DELETE FROM login_events WHERE email = '\''$email'\'';" || true
  db_exec "DELETE FROM users WHERE id = '\''$uid'\'';" || true
  qa_set_token ""
'

scenario 3 "MFA failed event recording" '
  local uid kc_id email ts
  uid=$(db_query "SELECT LOWER(UUID());")
  kc_id="kc-qa-s02-3-$(date +%s)"
  email="qa-s02-3-$(date +%s)@example.com"
  ts=$(date +%s)000

  db_exec "DELETE FROM login_events WHERE email = '\''$email'\'';" || true
  db_exec "DELETE FROM users WHERE keycloak_id = '\''$kc_id'\'';" || true
  db_exec "INSERT INTO users (id, keycloak_id, email, display_name) VALUES ('\''$uid'\'', '\''$kc_id'\'', '\''$email'\'', '\''QA Login S3'\'');"

  local evt_body="{\"type\":\"LOGIN_ERROR\",\"realmId\":\"auth9\",\"userId\":\"$kc_id\",\"ipAddress\":\"192.168.1.100\",\"error\":\"invalid_user_credentials\",\"time\":$ts,\"details\":{\"username\":\"qa-s02-3\",\"email\":\"$email\",\"credential_type\":\"otp\"}}"
  local resp
  resp=$(_send_kc_event "$evt_body")
  assert_http_status "$(resp_status "$resp")" 204 "Keycloak MFA error event accepted"

  sleep 1

  local token
  token=$(gen_default_admin_token)
  qa_set_token "$token"

  resp=$(api_get "/api/v1/analytics/login-events?email=$email")
  assert_http_status "$(resp_status "$resp")" 200 "GET login-events returns 200"
  local body
  body=$(resp_body "$resp")
  assert_json_field "$body" ".data[0].event_type" "failed_mfa" "event_type is failed_mfa"

  db_exec "DELETE FROM login_events WHERE email = '\''$email'\'';" || true
  db_exec "DELETE FROM users WHERE id = '\''$uid'\'';" || true
  qa_set_token ""
'

scenario 4 "Account lockout after repeated failures" '
  local uid kc_id email
  uid=$(db_query "SELECT LOWER(UUID());")
  kc_id="kc-qa-s02-4-$(date +%s)"
  email="qa-s02-4-$(date +%s)@example.com"

  db_exec "DELETE FROM login_events WHERE email = '\''$email'\'';" || true
  db_exec "DELETE FROM security_alerts WHERE user_id = '\''$uid'\'';" || true
  db_exec "DELETE FROM users WHERE keycloak_id = '\''$kc_id'\'';" || true
  db_exec "INSERT INTO users (id, keycloak_id, email, display_name) VALUES ('\''$uid'\'', '\''$kc_id'\'', '\''$email'\'', '\''QA Login S4'\'');"

  for i in $(seq 1 6); do
    local ts
    ts=$(date +%s)000
    local evt_body="{\"type\":\"LOGIN_ERROR\",\"realmId\":\"auth9\",\"userId\":\"$kc_id\",\"ipAddress\":\"192.168.1.100\",\"error\":\"invalid_user_credentials\",\"time\":$ts,\"details\":{\"username\":\"qa-s02-4\",\"email\":\"$email\",\"credential_type\":\"password\"}}"
    _send_kc_event "$evt_body" >/dev/null 2>&1
    sleep 0.3
  done

  sleep 2

  local token
  token=$(gen_default_admin_token)
  qa_set_token "$token"

  resp=$(api_get "/api/v1/analytics/login-events?email=$email")
  assert_http_status "$(resp_status "$resp")" 200 "GET login-events returns 200"
  local body
  body=$(resp_body "$resp")
  local all_types
  all_types=$(echo "$body" | jq -r "[.data[].event_type] | join(\",\")")
  assert_contains "$all_types" "locked" "lockout event recorded after repeated failures"

  db_exec "DELETE FROM login_events WHERE email = '\''$email'\'';" || true
  db_exec "DELETE FROM security_alerts WHERE user_id = '\''$uid'\'';" || true
  db_exec "DELETE FROM users WHERE id = '\''$uid'\'';" || true
  qa_set_token ""
'

scenario 5 "Login analytics statistics" '
  local token
  token=$(gen_default_admin_token)
  qa_set_token "$token"

  resp=$(api_get "/api/v1/analytics/login-stats?days=7")
  assert_http_status "$(resp_status "$resp")" 200 "GET /api/v1/analytics/login-stats returns 200"
  local body
  body=$(resp_body "$resp")
  assert_json_exists "$body" ".data" "response has data"

  resp=$(api_get "/api/v1/analytics/login-stats?period=daily")
  assert_http_status "$(resp_status "$resp")" 200 "GET login-stats period=daily returns 200"

  resp=$(api_get "/api/v1/analytics/login-stats?period=monthly")
  assert_http_status "$(resp_status "$resp")" 200 "GET login-stats period=monthly returns 200"

  resp=$(api_get "/api/v1/analytics/daily-trend?days=7")
  assert_http_status "$(resp_status "$resp")" 200 "GET daily-trend returns 200"
  body=$(resp_body "$resp")
  assert_json_exists "$body" ".data" "daily-trend response has data"

  qa_set_token ""
'

run_all
