#!/usr/bin/env bash
# QA Auto Test: integration/11-keycloak26-event-stream
# Doc: docs/qa/integration/11-keycloak26-event-stream.md
# Scenarios: 4
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

scenario 1 "Keycloak 26 basic health and compatibility" '
  kc_status=$(curl -s -o /dev/null -w "%{http_code}" "http://localhost:8081/health/ready" 2>/dev/null || echo "000")
  assert_eq "$kc_status" "200" "Keycloak /health/ready returns 200"

  kc_logs=$(docker logs auth9-keycloak 2>&1 | tail -50 || echo "")
  assert_not_contains "$kc_logs" "legacy-logout-redirect-uri" "no legacy param errors"

  kc_version=$(curl -s "http://localhost:8081/realms/master/.well-known/openid-configuration" \
    | jq -r '"'"'.issuer // ""'"'"' 2>/dev/null || echo "")
  assert_ne "$kc_version" "" "Keycloak OIDC discovery is accessible"
'

scenario 2 "Webhook event push and processing" '
  TS=$(($(date +%s) * 1000))
  UNIQUE_EMAIL="kc26-evt-$(date +%s)@example.com"
  BODY="{\"type\":\"LOGIN_ERROR\",\"realmId\":\"auth9\",\"clientId\":\"auth9-portal\",\"userId\":\"550e8400-e29b-41d4-a716-446655440010\",\"ipAddress\":\"192.168.1.100\",\"error\":\"invalid_user_credentials\",\"time\":${TS},\"details\":{\"username\":\"testuser\",\"email\":\"${UNIQUE_EMAIL}\"}}"

  resp=$(_send_kc_event "$BODY")
  assert_http_status "$(resp_status "$resp")" 204 "webhook event returns 204"

  sleep 2

  EVENT_TYPE=$(db_query "SELECT event_type FROM login_events WHERE email='"'"'${UNIQUE_EMAIL}'"'"' ORDER BY created_at DESC LIMIT 1;" | tr -d '[:space:]')
  assert_eq "$EVENT_TYPE" "failed_password" "login_events has event_type=failed_password"

  db_exec "DELETE FROM login_events WHERE email='"'"'${UNIQUE_EMAIL}'"'"';" || true
'

scenario 3 "Duplicate event deduplication" '
  TS=$(($(date +%s) * 1000))
  UNIQUE_EMAIL="kc26-dedup-$(date +%s)@example.com"
  EVT_ID="evt-dedup-$(date +%s)"

  BODY="{\"id\":\"${EVT_ID}\",\"type\":\"LOGIN_ERROR\",\"realmId\":\"auth9\",\"clientId\":\"auth9-portal\",\"userId\":\"550e8400-e29b-41d4-a716-446655440011\",\"ipAddress\":\"198.51.100.10\",\"error\":\"invalid_user_credentials\",\"time\":${TS},\"details\":{\"username\":\"target\",\"email\":\"${UNIQUE_EMAIL}\"}}"

  resp1=$(_send_kc_event "$BODY")
  assert_http_status "$(resp_status "$resp1")" 204 "first event accepted"

  sleep 1

  resp2=$(_send_kc_event "$BODY")
  assert_http_status "$(resp_status "$resp2")" 204 "duplicate event accepted (idempotent)"

  sleep 1

  EVT_COUNT=$(db_query "SELECT COUNT(*) FROM login_events WHERE email='"'"'${UNIQUE_EMAIL}'"'"' AND event_type='"'"'failed_password'"'"' AND created_at >= DATE_SUB(NOW(), INTERVAL 5 MINUTE);" | tr -d '[:space:]')
  assert_eq "$EVT_COUNT" "1" "only 1 event recorded (duplicate deduplicated)"

  db_exec "DELETE FROM login_events WHERE email='"'"'${UNIQUE_EMAIL}'"'"';" || true
'

scenario 4 "Expired event rejection (time window anti-replay)" '
  OLD_EMAIL="kc26-old-$(date +%s)@example.com"
  EVT_ID="evt-old-$(date +%s)"

  BODY="{\"id\":\"${EVT_ID}\",\"type\":\"LOGIN\",\"realmId\":\"auth9\",\"clientId\":\"auth9-portal\",\"userId\":\"550e8400-e29b-41d4-a716-446655440012\",\"ipAddress\":\"203.0.113.11\",\"time\":1600000000000,\"details\":{\"username\":\"old\",\"email\":\"${OLD_EMAIL}\"}}"

  resp=$(_send_kc_event "$BODY")
  status=$(resp_status "$resp")
  assert_match "$status" "^(204|400)$" "old event returns 204 (ignored) or 400 (rejected)"

  sleep 1

  EVT_COUNT=$(db_query "SELECT COUNT(*) FROM login_events WHERE email='"'"'${OLD_EMAIL}'"'"' AND created_at >= DATE_SUB(NOW(), INTERVAL 1 HOUR);" | tr -d '[:space:]')
  assert_eq "$EVT_COUNT" "0" "expired event not written to login_events"
'

run_all
