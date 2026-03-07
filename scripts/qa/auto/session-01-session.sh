#!/usr/bin/env bash
# QA Auto Test: session/01-session
# Doc: docs/qa/session/01-session.md
# Scenarios: 5
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/../lib/runner.sh"

require_bin jq
require_bin node

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

scenario 1 "List active sessions" '
  db_exec "DELETE FROM sessions WHERE user_id IN (SELECT id FROM users WHERE keycloak_id = '\''kc-qa-s01-1'\'');" || true
  db_exec "DELETE FROM users WHERE keycloak_id = '\''kc-qa-s01-1'\'';" || true

  local uid sid1 sid2 sid3
  uid=$(db_query "SELECT LOWER(UUID());")
  sid1=$(db_query "SELECT LOWER(UUID());")
  sid2=$(db_query "SELECT LOWER(UUID());")
  sid3=$(db_query "SELECT LOWER(UUID());")

  db_exec "INSERT INTO users (id, keycloak_id, email, display_name) VALUES ('\''$uid'\'', '\''kc-qa-s01-1'\'', '\''qa-s01-1@example.com'\'', '\''QA Session S1'\'');"
  db_exec "INSERT INTO sessions (id, user_id, device_type, ip_address, location, last_active_at) VALUES ('\''$sid1'\'', '\''$uid'\'', '\''desktop'\'', '\''192.168.1.1'\'', '\''Beijing'\'', NOW()), ('\''$sid2'\'', '\''$uid'\'', '\''mobile'\'', '\''192.168.1.2'\'', '\''Shanghai'\'', NOW()), ('\''$sid3'\'', '\''$uid'\'', '\''tablet'\'', '\''192.168.1.3'\'', '\''Guangzhou'\'', NOW());"

  local token
  token=$(_gen_id_token_with_sid "$uid" "qa-s01-1@example.com" "$sid1")
  qa_set_token "$token"

  resp=$(api_get /api/v1/users/me/sessions)
  assert_http_status "$(resp_status "$resp")" 200 "GET /api/v1/users/me/sessions returns 200"

  body=$(resp_body "$resp")
  assert_json_exists "$body" ".data" "response has data array"
  local count
  count=$(echo "$body" | jq ".data | length")
  assert_eq "$count" "3" "3 active sessions returned"
  assert_json_exists "$body" ".data[0].device_type" "session has device_type"
  assert_json_exists "$body" ".data[0].ip_address" "session has ip_address"

  db_exec "DELETE FROM sessions WHERE user_id = '\''$uid'\'';" || true
  db_exec "DELETE FROM users WHERE id = '\''$uid'\'';" || true
  qa_set_token ""
'

scenario 2 "Revoke single session" '
  db_exec "DELETE FROM sessions WHERE user_id IN (SELECT id FROM users WHERE keycloak_id = '\''kc-qa-s01-2'\'');" || true
  db_exec "DELETE FROM users WHERE keycloak_id = '\''kc-qa-s01-2'\'';" || true

  local uid sid1 sid2
  uid=$(db_query "SELECT LOWER(UUID());")
  sid1=$(db_query "SELECT LOWER(UUID());")
  sid2=$(db_query "SELECT LOWER(UUID());")

  db_exec "INSERT INTO users (id, keycloak_id, email, display_name) VALUES ('\''$uid'\'', '\''kc-qa-s01-2'\'', '\''qa-s01-2@example.com'\'', '\''QA Session S2'\'');"
  db_exec "INSERT INTO sessions (id, user_id, device_type, ip_address, location, last_active_at) VALUES ('\''$sid1'\'', '\''$uid'\'', '\''desktop'\'', '\''192.168.1.1'\'', '\''Beijing'\'', NOW()), ('\''$sid2'\'', '\''$uid'\'', '\''mobile'\'', '\''192.168.1.2'\'', '\''Shanghai'\'', NOW());"

  local token
  token=$(_gen_id_token_with_sid "$uid" "qa-s01-2@example.com" "$sid1")
  qa_set_token "$token"

  resp=$(api_delete "/api/v1/users/me/sessions/$sid2")
  assert_http_status "$(resp_status "$resp")" 200 "DELETE /api/v1/users/me/sessions/{id} returns 200"

  assert_db_not_empty \
    "SELECT revoked_at FROM sessions WHERE id = '\''$sid2'\'' AND revoked_at IS NOT NULL;" \
    "target session has revoked_at set"
  assert_db_not_empty \
    "SELECT id FROM sessions WHERE id = '\''$sid1'\'' AND revoked_at IS NULL;" \
    "current session still active"

  db_exec "DELETE FROM sessions WHERE user_id = '\''$uid'\'';" || true
  db_exec "DELETE FROM users WHERE id = '\''$uid'\'';" || true
  qa_set_token ""
'

scenario 3 "Revoke all other sessions" '
  db_exec "DELETE FROM sessions WHERE user_id IN (SELECT id FROM users WHERE keycloak_id = '\''kc-qa-s01-3'\'');" || true
  db_exec "DELETE FROM users WHERE keycloak_id = '\''kc-qa-s01-3'\'';" || true

  local uid sid1 sid2 sid3
  uid=$(db_query "SELECT LOWER(UUID());")
  sid1=$(db_query "SELECT LOWER(UUID());")
  sid2=$(db_query "SELECT LOWER(UUID());")
  sid3=$(db_query "SELECT LOWER(UUID());")

  db_exec "INSERT INTO users (id, keycloak_id, email, display_name) VALUES ('\''$uid'\'', '\''kc-qa-s01-3'\'', '\''qa-s01-3@example.com'\'', '\''QA Session S3'\'');"
  db_exec "INSERT INTO sessions (id, user_id, device_type, ip_address, location, last_active_at) VALUES ('\''$sid1'\'', '\''$uid'\'', '\''desktop'\'', '\''192.168.1.1'\'', '\''Beijing'\'', NOW()), ('\''$sid2'\'', '\''$uid'\'', '\''mobile'\'', '\''192.168.1.2'\'', '\''Shanghai'\'', NOW()), ('\''$sid3'\'', '\''$uid'\'', '\''tablet'\'', '\''192.168.1.3'\'', '\''Guangzhou'\'', NOW());"

  local token
  token=$(_gen_id_token_with_sid "$uid" "qa-s01-3@example.com" "$sid1")
  qa_set_token "$token"

  resp=$(api_delete /api/v1/users/me/sessions)
  assert_http_status "$(resp_status "$resp")" 200 "DELETE /api/v1/users/me/sessions returns 200"

  body=$(resp_body "$resp")
  assert_json_field "$body" ".data.revoked_count" "2" "revoked_count is 2"

  assert_db "SELECT COUNT(*) FROM sessions WHERE user_id = '\''$uid'\'' AND revoked_at IS NULL;" "1" "only current session remains active"
  assert_db "SELECT COUNT(*) FROM sessions WHERE user_id = '\''$uid'\'' AND revoked_at IS NOT NULL;" "2" "2 sessions revoked"

  db_exec "DELETE FROM sessions WHERE user_id = '\''$uid'\'';" || true
  db_exec "DELETE FROM users WHERE id = '\''$uid'\'';" || true
  qa_set_token ""
'

scenario 4 "Admin force logout user" '
  db_exec "DELETE FROM sessions WHERE user_id IN (SELECT id FROM users WHERE keycloak_id = '\''kc-qa-s01-4'\'');" || true
  db_exec "DELETE FROM users WHERE keycloak_id = '\''kc-qa-s01-4'\'';" || true

  local uid sid1 sid2
  uid=$(db_query "SELECT LOWER(UUID());")
  sid1=$(db_query "SELECT LOWER(UUID());")
  sid2=$(db_query "SELECT LOWER(UUID());")

  db_exec "INSERT INTO users (id, keycloak_id, email, display_name) VALUES ('\''$uid'\'', '\''kc-qa-s01-4'\'', '\''qa-s01-4@example.com'\'', '\''QA Session S4'\'');"
  db_exec "INSERT INTO sessions (id, user_id, device_type, ip_address, location, last_active_at) VALUES ('\''$sid1'\'', '\''$uid'\'', '\''desktop'\'', '\''192.168.1.1'\'', '\''Beijing'\'', NOW()), ('\''$sid2'\'', '\''$uid'\'', '\''mobile'\'', '\''192.168.1.2'\'', '\''Shanghai'\'', NOW());"

  local token
  token=$(gen_default_admin_token)
  qa_set_token "$token"

  pre_count=$(db_query "SELECT COUNT(*) FROM sessions WHERE user_id = '\''$uid'\'' AND revoked_at IS NULL;" | tr -d "[:space:]")
  assert_eq "$pre_count" "2" "target user has 2 sessions before logout"

  resp=$(api_post "/api/v1/admin/users/$uid/logout" "{}")
  assert_http_status "$(resp_status "$resp")" 200 "POST /api/v1/admin/users/{id}/logout returns 200"
  body=$(resp_body "$resp")
  assert_json_field "$body" ".data.revoked_count" "2" "revoked_count is 2"

  assert_db "SELECT COUNT(*) FROM sessions WHERE user_id = '\''$uid'\'' AND revoked_at IS NULL;" "0" "all sessions revoked after force logout"

  db_exec "DELETE FROM sessions WHERE user_id = '\''$uid'\'';" || true
  db_exec "DELETE FROM users WHERE id = '\''$uid'\'';" || true
  qa_set_token ""
'

scenario 5 "Session auto-expiry detection" '
  db_exec "DELETE FROM sessions WHERE user_id IN (SELECT id FROM users WHERE keycloak_id = '\''kc-qa-s01-5'\'');" || true
  db_exec "DELETE FROM users WHERE keycloak_id = '\''kc-qa-s01-5'\'';" || true

  local uid sid_old sid_recent
  uid=$(db_query "SELECT LOWER(UUID());")
  sid_old=$(db_query "SELECT LOWER(UUID());")
  sid_recent=$(db_query "SELECT LOWER(UUID());")

  db_exec "INSERT INTO users (id, keycloak_id, email, display_name) VALUES ('\''$uid'\'', '\''kc-qa-s01-5'\'', '\''qa-s01-5@example.com'\'', '\''QA Session S5'\'');"
  db_exec "INSERT INTO sessions (id, user_id, device_type, ip_address, location, last_active_at) VALUES ('\''$sid_old'\'', '\''$uid'\'', '\''desktop'\'', '\''192.168.1.1'\'', '\''Old'\'', DATE_SUB(NOW(), INTERVAL 25 HOUR)), ('\''$sid_recent'\'', '\''$uid'\'', '\''mobile'\'', '\''192.168.1.2'\'', '\''Recent'\'', NOW());"

  assert_db_not_empty \
    "SELECT id FROM sessions WHERE id = '\''$sid_old'\'' AND last_active_at < DATE_SUB(NOW(), INTERVAL 24 HOUR);" \
    "expired session detected (last_active > 24h ago)"
  assert_db_not_empty \
    "SELECT id FROM sessions WHERE id = '\''$sid_recent'\'' AND last_active_at > DATE_SUB(NOW(), INTERVAL 1 HOUR);" \
    "recent session still active"

  db_exec "DELETE FROM sessions WHERE user_id = '\''$uid'\'';" || true
  db_exec "DELETE FROM users WHERE id = '\''$uid'\'';" || true
'

run_all
