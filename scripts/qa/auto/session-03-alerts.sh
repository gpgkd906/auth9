#!/usr/bin/env bash
# QA Auto Test: session/03-alerts
# Doc: docs/qa/session/03-alerts.md
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

scenario 1 "Brute force alert" '
  local uid kc_id email
  uid=$(db_query "SELECT LOWER(UUID());")
  kc_id="kc-qa-s03-1-$(date +%s)"
  email="qa-s03-1-$(date +%s)@example.com"

  db_exec "DELETE FROM login_events WHERE email = '\''$email'\'';" || true
  db_exec "DELETE FROM security_alerts WHERE user_id = '\''$uid'\'';" || true
  db_exec "DELETE FROM users WHERE keycloak_id = '\''$kc_id'\'';" || true
  db_exec "INSERT INTO users (id, keycloak_id, email, display_name) VALUES ('\''$uid'\'', '\''$kc_id'\'', '\''$email'\'', '\''QA Alert S1'\'');"

  for i in $(seq 1 10); do
    local ts
    ts=$(date +%s)000
    local evt="{\"type\":\"LOGIN_ERROR\",\"realmId\":\"auth9\",\"userId\":\"$kc_id\",\"ipAddress\":\"10.200.1.1\",\"error\":\"invalid_user_credentials\",\"time\":$ts,\"details\":{\"username\":\"qa-s03-1\",\"email\":\"$email\",\"credential_type\":\"password\"}}"
    _send_kc_event "$evt" >/dev/null 2>&1
    sleep 0.3
  done

  sleep 2

  local token
  token=$(gen_default_admin_token)
  qa_set_token "$token"

  resp=$(api_get "/api/v1/security/alerts?alert_type=brute_force")
  assert_http_status "$(resp_status "$resp")" 200 "GET alerts with brute_force filter returns 200"
  local body
  body=$(resp_body "$resp")
  assert_json_exists "$body" ".data" "response has data array"

  assert_db_not_empty \
    "SELECT id FROM security_alerts WHERE user_id = '\''$uid'\'' AND alert_type = '\''brute_force'\'';" \
    "brute_force alert created in DB"

  db_exec "DELETE FROM login_events WHERE email = '\''$email'\'';" || true
  db_exec "DELETE FROM security_alerts WHERE user_id = '\''$uid'\'';" || true
  db_exec "DELETE FROM users WHERE id = '\''$uid'\'';" || true
  qa_set_token ""
'

scenario 2 "New device login alert" '
  local uid kc_id email
  uid=$(db_query "SELECT LOWER(UUID());")
  kc_id="kc-qa-s03-2-$(date +%s)"
  email="qa-s03-2-$(date +%s)@example.com"

  db_exec "DELETE FROM login_events WHERE email = '\''$email'\'';" || true
  db_exec "DELETE FROM security_alerts WHERE user_id = '\''$uid'\'';" || true
  db_exec "DELETE FROM users WHERE keycloak_id = '\''$kc_id'\'';" || true
  db_exec "INSERT INTO users (id, keycloak_id, email, display_name) VALUES ('\''$uid'\'', '\''$kc_id'\'', '\''$email'\'', '\''QA Alert S2'\'');"

  local ts_old
  ts_old=$(date +%s)000
  local evt_old="{\"type\":\"LOGIN\",\"realmId\":\"auth9\",\"userId\":\"$kc_id\",\"ipAddress\":\"203.0.113.10\",\"time\":$ts_old,\"details\":{\"username\":\"qa-s03-2\",\"email\":\"$email\"}}"
  _send_kc_event "$evt_old" >/dev/null 2>&1
  sleep 1

  local ts_new
  ts_new=$(date +%s)000
  local evt_new="{\"type\":\"LOGIN\",\"realmId\":\"auth9\",\"userId\":\"$kc_id\",\"ipAddress\":\"198.51.100.5\",\"time\":$ts_new,\"details\":{\"username\":\"qa-s03-2\",\"email\":\"$email\"}}"
  local resp
  resp=$(_send_kc_event "$evt_new")
  assert_http_status "$(resp_status "$resp")" 204 "new device LOGIN event accepted"

  sleep 2

  assert_db_not_empty \
    "SELECT id FROM security_alerts WHERE user_id = '\''$uid'\'' AND alert_type = '\''new_device'\'';" \
    "new_device alert created in DB"

  db_exec "DELETE FROM login_events WHERE email = '\''$email'\'';" || true
  db_exec "DELETE FROM security_alerts WHERE user_id = '\''$uid'\'';" || true
  db_exec "DELETE FROM users WHERE id = '\''$uid'\'';" || true
'

scenario 3 "Impossible travel alert" '
  local uid kc_id email
  uid=$(db_query "SELECT LOWER(UUID());")
  kc_id="kc-qa-s03-3-$(date +%s)"
  email="qa-s03-3-$(date +%s)@example.com"

  db_exec "DELETE FROM login_events WHERE email = '\''$email'\'';" || true
  db_exec "DELETE FROM security_alerts WHERE user_id = '\''$uid'\'';" || true
  db_exec "DELETE FROM users WHERE keycloak_id = '\''$kc_id'\'';" || true
  db_exec "INSERT INTO users (id, keycloak_id, email, display_name) VALUES ('\''$uid'\'', '\''$kc_id'\'', '\''$email'\'', '\''QA Alert S3'\'');"

  local ts1
  ts1=$(date +%s)000
  local evt1="{\"type\":\"LOGIN\",\"realmId\":\"auth9\",\"userId\":\"$kc_id\",\"ipAddress\":\"203.0.113.10\",\"time\":$ts1,\"details\":{\"username\":\"qa-s03-3\",\"email\":\"$email\"}}"
  _send_kc_event "$evt1" >/dev/null 2>&1
  sleep 1

  local ts2
  ts2=$(date +%s)000
  local evt2="{\"type\":\"LOGIN\",\"realmId\":\"auth9\",\"userId\":\"$kc_id\",\"ipAddress\":\"198.51.100.99\",\"time\":$ts2,\"details\":{\"username\":\"qa-s03-3\",\"email\":\"$email\"}}"
  local resp
  resp=$(_send_kc_event "$evt2")
  assert_http_status "$(resp_status "$resp")" 204 "different-location LOGIN event accepted"

  sleep 2

  assert_db_not_empty \
    "SELECT id FROM security_alerts WHERE user_id = '\''$uid'\'' AND alert_type = '\''impossible_travel'\'';" \
    "impossible_travel alert created in DB"

  db_exec "DELETE FROM login_events WHERE email = '\''$email'\'';" || true
  db_exec "DELETE FROM security_alerts WHERE user_id = '\''$uid'\'';" || true
  db_exec "DELETE FROM users WHERE id = '\''$uid'\'';" || true
'

scenario 4 "Resolve security alert" '
  local uid
  uid=$(db_query "SELECT LOWER(UUID());")
  local alert_id
  alert_id=$(db_query "SELECT LOWER(UUID());")

  db_exec "DELETE FROM security_alerts WHERE id = '\''$alert_id'\'';" || true
  db_exec "DELETE FROM users WHERE keycloak_id = '\''kc-qa-s03-4'\'';" || true
  db_exec "INSERT INTO users (id, keycloak_id, email, display_name) VALUES ('\''$uid'\'', '\''kc-qa-s03-4'\'', '\''qa-s03-4@example.com'\'', '\''QA Alert S4'\'');"
  db_exec "INSERT INTO security_alerts (id, user_id, alert_type, severity, details, created_at) VALUES ('\''$alert_id'\'', '\''$uid'\'', '\''brute_force'\'', '\''high'\'', '\''{ \"attempts\": 10 }'\'', NOW());"

  local token
  token=$(gen_default_admin_token)
  qa_set_token "$token"

  resp=$(api_post "/api/v1/security/alerts/$alert_id/resolve" "{}")
  assert_http_status "$(resp_status "$resp")" 200 "POST /api/v1/security/alerts/{id}/resolve returns 200"
  local body
  body=$(resp_body "$resp")
  assert_json_exists "$body" ".data.resolved_at" "resolved_at is set"
  assert_json_exists "$body" ".data.resolved_by" "resolved_by is set"

  assert_db_not_empty \
    "SELECT resolved_at FROM security_alerts WHERE id = '\''$alert_id'\'' AND resolved_at IS NOT NULL;" \
    "alert resolved_at set in DB"

  db_exec "DELETE FROM security_alerts WHERE id = '\''$alert_id'\'';" || true
  db_exec "DELETE FROM users WHERE id = '\''$uid'\'';" || true
  qa_set_token ""
'

scenario 5 "Security alert filtering" '
  local uid a1 a2 a3
  uid=$(db_query "SELECT LOWER(UUID());")
  a1=$(db_query "SELECT LOWER(UUID());")
  a2=$(db_query "SELECT LOWER(UUID());")
  a3=$(db_query "SELECT LOWER(UUID());")

  db_exec "DELETE FROM security_alerts WHERE user_id = '\''$uid'\'';" || true
  db_exec "DELETE FROM users WHERE keycloak_id = '\''kc-qa-s03-5'\'';" || true
  db_exec "INSERT INTO users (id, keycloak_id, email, display_name) VALUES ('\''$uid'\'', '\''kc-qa-s03-5'\'', '\''qa-s03-5@example.com'\'', '\''QA Alert S5'\'');"
  db_exec "INSERT INTO security_alerts (id, user_id, alert_type, severity, details, created_at) VALUES ('\''$a1'\'', '\''$uid'\'', '\''brute_force'\'', '\''high'\'', '\''{ \"test\": true }'\'', NOW()), ('\''$a2'\'', '\''$uid'\'', '\''new_device'\'', '\''medium'\'', '\''{ \"test\": true }'\'', NOW()), ('\''$a3'\'', '\''$uid'\'', '\''impossible_travel'\'', '\''high'\'', '\''{ \"test\": true }'\'', NOW());"

  local token
  token=$(gen_default_admin_token)
  qa_set_token "$token"

  resp=$(api_get "/api/v1/security/alerts?severity=high")
  assert_http_status "$(resp_status "$resp")" 200 "GET alerts severity=high returns 200"
  local body
  body=$(resp_body "$resp")
  local count
  count=$(echo "$body" | jq ".data | length")
  assert_match "$count" "^[1-9]" "high severity filter returns results"

  resp=$(api_get "/api/v1/security/alerts?alert_type=new_device")
  assert_http_status "$(resp_status "$resp")" 200 "GET alerts alert_type=new_device returns 200"

  resp=$(api_get "/api/v1/security/alerts?unresolved_only=true")
  assert_http_status "$(resp_status "$resp")" 200 "GET alerts unresolved_only returns 200"
  body=$(resp_body "$resp")
  count=$(echo "$body" | jq ".data | length")
  assert_match "$count" "^[1-9]" "unresolved filter returns results"

  db_exec "DELETE FROM security_alerts WHERE user_id = '\''$uid'\'';" || true
  db_exec "DELETE FROM users WHERE id = '\''$uid'\'';" || true
  qa_set_token ""
'

run_all
