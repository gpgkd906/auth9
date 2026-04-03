#!/usr/bin/env bash
# QA Auto Test: session/04-boundary
# Doc: docs/qa/session/04-boundary.md
# Scenarios: 5
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/../lib/runner.sh"

require_bin jq
require_bin node
require_bin openssl

WEBHOOK_SECRET="${WEBHOOK_SECRET:-dev-webhook-secret-change-in-production}"

_gen_id_token_with_sid() {
  local user_id="$1" email="$2" sid="$3"
  node -e '
const jwt=require("jsonwebtoken"),fs=require("fs");
const now=Math.floor(Date.now()/1000);
const pk=fs.readFileSync(process.argv[1],"utf8");
process.stdout.write(jwt.sign({
  sub:process.argv[2],email:process.argv[3],
  iss:"http://localhost:8080",aud:"auth9",token_type:"identity",
  iat:now,exp:now+3600,sid:process.argv[4]
},pk,{algorithm:"RS256",keyid:"auth9-current"}));
' "$_JWT_PRIVATE_KEY" "$user_id" "$email" "$sid" 2>/dev/null
}

_send_kc_event() {
  local payload="$1"
  local sig
  sig=$(printf '%s' "$payload" | openssl dgst -sha256 -hmac "$WEBHOOK_SECRET" | awk '{print $NF}')
  api_raw POST /api/v1/identity/events \
    -H "Content-Type: application/json" \
    -H "x-keycloak-signature: sha256=$sig" \
    -d "$payload"
}

scenario 1 "Revoke current session returns 400" '
  db_exec "DELETE FROM sessions WHERE user_id IN (SELECT id FROM users WHERE keycloak_id = '\''kc-qa-s04-1'\'');" || true
  db_exec "DELETE FROM users WHERE keycloak_id = '\''kc-qa-s04-1'\'';" || true

  local uid sid
  uid=$(db_query "SELECT LOWER(UUID());")
  sid=$(db_query "SELECT LOWER(UUID());")

  db_exec "INSERT INTO users (id, keycloak_id, email, display_name) VALUES ('\''$uid'\'', '\''kc-qa-s04-1'\'', '\''qa-s04-1@example.com'\'', '\''QA Boundary S1'\'');"
  db_exec "INSERT INTO sessions (id, user_id, device_type, ip_address, location, last_active_at) VALUES ('\''$sid'\'', '\''$uid'\'', '\''desktop'\'', '\''192.168.1.1'\'', '\''Test'\'', NOW());"

  local token
  token=$(_gen_id_token_with_sid "$uid" "qa-s04-1@example.com" "$sid")
  qa_set_token "$token"

  resp=$(api_delete "/api/v1/users/me/sessions/$sid")
  assert_http_status "$(resp_status "$resp")" 400 "revoking current session returns 400"

  assert_db_not_empty \
    "SELECT id FROM sessions WHERE id = '\''$sid'\'' AND revoked_at IS NULL;" \
    "current session remains active"

  db_exec "DELETE FROM sessions WHERE user_id = '\''$uid'\'';" || true
  db_exec "DELETE FROM users WHERE id = '\''$uid'\'';" || true
  qa_set_token ""
'

scenario 2 "Concurrent session limit" '
  db_exec "DELETE FROM sessions WHERE user_id IN (SELECT id FROM users WHERE keycloak_id = '\''kc-qa-s04-2'\'');" || true
  db_exec "DELETE FROM users WHERE keycloak_id = '\''kc-qa-s04-2'\'';" || true

  local uid
  uid=$(db_query "SELECT LOWER(UUID());")
  db_exec "INSERT INTO users (id, keycloak_id, email, display_name) VALUES ('\''$uid'\'', '\''kc-qa-s04-2'\'', '\''qa-s04-2@example.com'\'', '\''QA Boundary S2'\'');"

  for i in $(seq 1 10); do
    local sid
    sid=$(db_query "SELECT LOWER(UUID());")
    db_exec "INSERT INTO sessions (id, user_id, device_type, ip_address, location, last_active_at) VALUES ('\''$sid'\'', '\''$uid'\'', '\''desktop'\'', '\''192.168.1.$i'\'', '\''Loc $i'\'', DATE_ADD(NOW(), INTERVAL $i SECOND));"
  done

  assert_db "SELECT COUNT(*) FROM sessions WHERE user_id = '\''$uid'\'' AND revoked_at IS NULL;" "10" "10 active sessions exist"

  db_exec "DELETE FROM sessions WHERE user_id = '\''$uid'\'';" || true
  db_exec "DELETE FROM users WHERE id = '\''$uid'\'';" || true
'

scenario 3 "Social login event recording" '
  local uid kc_id email ts
  uid=$(db_query "SELECT LOWER(UUID());")
  kc_id="kc-qa-s04-3-$(date +%s)"
  email="qa-s04-3-$(date +%s)@example.com"
  ts=$(date +%s)000

  db_exec "DELETE FROM login_events WHERE email = '\''$email'\'';" || true
  db_exec "DELETE FROM users WHERE keycloak_id = '\''$kc_id'\'';" || true
  db_exec "INSERT INTO users (id, keycloak_id, email, display_name) VALUES ('\''$uid'\'', '\''$kc_id'\'', '\''$email'\'', '\''QA Boundary S3'\'');"

  local evt="{\"type\":\"IDENTITY_PROVIDER_LOGIN\",\"realmId\":\"auth9\",\"userId\":\"$kc_id\",\"ipAddress\":\"203.0.113.50\",\"time\":$ts,\"details\":{\"username\":\"qa-s04-3\",\"email\":\"$email\",\"identity_provider\":\"google\"}}"
  local resp
  resp=$(_send_kc_event "$evt")
  assert_http_status "$(resp_status "$resp")" 204 "social login event accepted"

  sleep 1

  assert_db_not_empty \
    "SELECT id FROM login_events WHERE email = '\''$email'\'' AND event_type = '\''social'\'';" \
    "social login event recorded in DB"

  db_exec "DELETE FROM login_events WHERE email = '\''$email'\'';" || true
  db_exec "DELETE FROM security_alerts WHERE user_id = '\''$uid'\'';" || true
  db_exec "DELETE FROM users WHERE id = '\''$uid'\'';" || true
'

scenario 4 "Suspicious IP alert (password spray)" '
  local ts_base
  ts_base=$(date +%s)

  db_exec "DELETE FROM security_alerts WHERE alert_type = '\''suspicious_ip'\'' AND details LIKE '\''%10.99.99.99%'\'';" || true

  for i in $(seq 1 6); do
    local spray_uid="spray-user-$ts_base-$i"
    local spray_email="spray-target-$ts_base-$i@example.com"
    local ts
    ts=$(date +%s)000
    local evt="{\"type\":\"LOGIN_ERROR\",\"realmId\":\"auth9\",\"userId\":\"$spray_uid\",\"error\":\"invalid_user_credentials\",\"ipAddress\":\"10.99.99.99\",\"time\":$ts,\"details\":{\"username\":\"$spray_email\",\"email\":\"$spray_email\"}}"
    _send_kc_event "$evt" >/dev/null 2>&1
    sleep 0.5
  done

  sleep 2

  assert_db_not_empty \
    "SELECT id FROM security_alerts WHERE alert_type = '\''suspicious_ip'\'' ORDER BY created_at DESC LIMIT 1;" \
    "suspicious_ip alert created after password spray"

  local severity
  severity=$(db_query "SELECT severity FROM security_alerts WHERE alert_type = '\''suspicious_ip'\'' ORDER BY created_at DESC LIMIT 1;")
  assert_eq "$severity" "critical" "suspicious_ip alert severity is critical"

  for i in $(seq 1 6); do
    db_exec "DELETE FROM login_events WHERE email = '\''spray-target-$ts_base-$i@example.com'\'';" || true
  done
  db_exec "DELETE FROM security_alerts WHERE alert_type = '\''suspicious_ip'\'' AND details LIKE '\''%10.99.99.99%'\'';" || true
'

scenario 5 "Login events API response time" '
  local token
  token=$(gen_default_admin_token)
  qa_set_token "$token"

  local start_time end_time elapsed
  start_time=$(date +%s)
  resp=$(api_get "/api/v1/analytics/login-events?page=1&per_page=50")
  end_time=$(date +%s)
  elapsed=$((end_time - start_time))

  assert_http_status "$(resp_status "$resp")" 200 "GET login-events returns 200"
  assert_json_exists "$(resp_body "$resp")" ".data" "response has data"
  assert_json_exists "$(resp_body "$resp")" ".pagination" "response has pagination"

  if [[ "$elapsed" -le 3 ]]; then
    assert_eq "fast" "fast" "response time within 3 seconds"
  else
    assert_eq "$elapsed" "<=3" "response time exceeds 3 seconds"
  fi

  qa_set_token ""
'

run_all
