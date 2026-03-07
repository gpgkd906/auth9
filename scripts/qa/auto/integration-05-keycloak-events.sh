#!/usr/bin/env bash
# QA Auto Test: integration/05-keycloak-events
# Doc: docs/qa/integration/05-keycloak-events.md
# Scenarios: 5
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/../lib/runner.sh"

require_bin jq
require_bin openssl

KC_WEBHOOK_SECRET="${KEYCLOAK_WEBHOOK_SECRET:-dev-webhook-secret-change-in-production}"

_send_kc_event() {
  local body="$1"
  local sig
  sig=$(echo -n "$body" | openssl dgst -sha256 -hmac "$KC_WEBHOOK_SECRET" | awk '{print $NF}')
  api_raw POST /api/v1/keycloak/events \
    -H "Content-Type: application/json" \
    -H "x-keycloak-signature: sha256=$sig" \
    -d "$body"
}

scenario 1 "Receive login success event" '
  TS=$(($(date +%s) * 1000))
  UNIQUE_EMAIL="kc-success-$(date +%s)@example.com"
  BODY="{\"type\":\"LOGIN\",\"realmId\":\"auth9\",\"clientId\":\"auth9-portal\",\"userId\":\"550e8400-e29b-41d4-a716-446655440000\",\"ipAddress\":\"192.168.1.100\",\"time\":${TS},\"details\":{\"username\":\"john\",\"email\":\"${UNIQUE_EMAIL}\",\"authMethod\":\"password\"}}"

  resp=$(_send_kc_event "$BODY")
  assert_http_status "$(resp_status "$resp")" 204 "login event returns 204"

  sleep 1

  EVENT_TYPE=$(db_query "SELECT event_type FROM login_events WHERE email='"'"'${UNIQUE_EMAIL}'"'"' ORDER BY created_at DESC LIMIT 1;" | tr -d '[:space:]')
  assert_eq "$EVENT_TYPE" "success" "login_events has event_type=success"

  IP_ADDR=$(db_query "SELECT ip_address FROM login_events WHERE email='"'"'${UNIQUE_EMAIL}'"'"' ORDER BY created_at DESC LIMIT 1;" | tr -d '[:space:]')
  assert_eq "$IP_ADDR" "192.168.1.100" "ip_address is correct"

  db_exec "DELETE FROM login_events WHERE email='"'"'${UNIQUE_EMAIL}'"'"';" || true
'

scenario 2 "Login failure events trigger security analysis" '
  TS=$(($(date +%s) * 1000))
  UNIQUE_EMAIL="kc-fail-$(date +%s)@example.com"
  TEST_USER_ID="550e8400-e29b-41d4-a716-446655440001"

  for i in $(seq 1 10); do
    EVT_TS=$((TS + i * 1000))
    BODY="{\"type\":\"LOGIN_ERROR\",\"realmId\":\"auth9\",\"userId\":\"${TEST_USER_ID}\",\"ipAddress\":\"192.168.1.200\",\"error\":\"invalid_user_credentials\",\"time\":${EVT_TS},\"details\":{\"username\":\"target-user\",\"email\":\"${UNIQUE_EMAIL}\"}}"
    resp=$(_send_kc_event "$BODY")
    assert_http_status "$(resp_status "$resp")" 204 "failure event ${i} returns 204"
  done

  sleep 2

  FAIL_COUNT=$(db_query "SELECT COUNT(*) FROM login_events WHERE email='"'"'${UNIQUE_EMAIL}'"'"' AND event_type='"'"'failed_password'"'"' AND created_at >= DATE_SUB(NOW(), INTERVAL 5 MINUTE);" | tr -d '[:space:]')
  assert_eq "$FAIL_COUNT" "10" "10 failed_password events recorded"

  ALERT_COUNT=$(db_query "SELECT COUNT(*) FROM security_alerts WHERE user_id='"'"'${TEST_USER_ID}'"'"' AND created_at >= DATE_SUB(NOW(), INTERVAL 5 MINUTE);" | tr -d '[:space:]')
  assert_ne "$ALERT_COUNT" "0" "security alert generated for brute force"

  db_exec "DELETE FROM security_alerts WHERE user_id='"'"'${TEST_USER_ID}'"'"';" || true
  db_exec "DELETE FROM login_events WHERE email='"'"'${UNIQUE_EMAIL}'"'"';" || true
'

scenario 3 "Invalid signature rejected" '
  BODY='"'"'{"type":"LOGIN","time":0}'"'"'

  resp=$(api_raw POST /api/v1/keycloak/events \
    -H "Content-Type: application/json" \
    -H "x-keycloak-signature: sha256=0000000000000000000000000000000000000000000000000000000000000000" \
    -d "$BODY")
  assert_http_status "$(resp_status "$resp")" 401 "invalid signature returns 401"

  resp=$(api_raw POST /api/v1/keycloak/events \
    -H "Content-Type: application/json" \
    -d "$BODY")
  assert_http_status "$(resp_status "$resp")" 401 "missing signature returns 401"
'

scenario 4 "Non-login events are ignored" '
  TS=$(($(date +%s) * 1000))
  IGNORE_EMAIL="kc-ignore-$(date +%s)@example.com"

  for EVT_TYPE in LOGOUT REGISTER REFRESH_TOKEN; do
    BODY="{\"type\":\"${EVT_TYPE}\",\"realmId\":\"auth9\",\"time\":${TS},\"details\":{\"email\":\"${IGNORE_EMAIL}\"}}"
    resp=$(_send_kc_event "$BODY")
    assert_http_status "$(resp_status "$resp")" 204 "${EVT_TYPE} event returns 204"
  done

  ADMIN_BODY="{\"operationType\":\"CREATE\",\"resourceType\":\"USER\",\"realmId\":\"auth9\",\"time\":${TS}}"
  resp=$(_send_kc_event "$ADMIN_BODY")
  assert_http_status "$(resp_status "$resp")" 204 "admin event returns 204"

  sleep 1

  EVT_COUNT=$(db_query "SELECT COUNT(*) FROM login_events WHERE email='"'"'${IGNORE_EMAIL}'"'"' AND created_at >= DATE_SUB(NOW(), INTERVAL 5 MINUTE);" | tr -d '[:space:]')
  assert_eq "$EVT_COUNT" "0" "no login_events for ignored event types"
'

scenario 5 "Social login event processing" '
  TS=$(($(date +%s) * 1000))
  SOCIAL_EMAIL="kc-social-$(date +%s)@gmail.com"
  BODY="{\"type\":\"IDENTITY_PROVIDER_LOGIN\",\"realmId\":\"auth9\",\"userId\":\"550e8400-e29b-41d4-a716-446655440002\",\"ipAddress\":\"10.0.0.1\",\"time\":${TS},\"details\":{\"username\":\"google-user\",\"email\":\"${SOCIAL_EMAIL}\",\"identityProvider\":\"google\"}}"

  resp=$(_send_kc_event "$BODY")
  assert_http_status "$(resp_status "$resp")" 204 "social login event returns 204"

  sleep 1

  EVENT_TYPE=$(db_query "SELECT event_type FROM login_events WHERE email='"'"'${SOCIAL_EMAIL}'"'"' ORDER BY created_at DESC LIMIT 1;" | tr -d '[:space:]')
  assert_eq "$EVENT_TYPE" "social" "login_events has event_type=social"

  db_exec "DELETE FROM login_events WHERE email='"'"'${SOCIAL_EMAIL}'"'"';" || true
'

run_all
