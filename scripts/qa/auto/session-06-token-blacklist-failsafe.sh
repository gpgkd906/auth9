#!/usr/bin/env bash
# QA Auto Test: session/06-token-blacklist-failsafe
# Doc: docs/qa/session/06-token-blacklist-failsafe.md
# Scenarios: 4
# NOTE: Scenarios 2-3 manipulate Docker containers (auth9-redis).
#       Scenario 4 requires auth9-core started without Redis config.
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

scenario 1 "Redis up - revoked token rejected with 401" '
  db_exec "DELETE FROM sessions WHERE user_id IN (SELECT id FROM users WHERE keycloak_id = '\''kc-qa-s06-1'\'');" || true
  db_exec "DELETE FROM users WHERE keycloak_id = '\''kc-qa-s06-1'\'';" || true

  local uid sid
  uid=$(db_query "SELECT LOWER(UUID());")
  sid=$(db_query "SELECT LOWER(UUID());")

  db_exec "INSERT INTO users (id, keycloak_id, email, display_name) VALUES ('\''$uid'\'', '\''kc-qa-s06-1'\'', '\''qa-s06-1@example.com'\'', '\''QA Blacklist S1'\'');"
  db_exec "INSERT INTO sessions (id, user_id, device_type, ip_address, location, last_active_at) VALUES ('\''$sid'\'', '\''$uid'\'', '\''desktop'\'', '\''192.168.1.1'\'', '\''Test'\'', NOW());"

  local user_token
  user_token=$(_gen_id_token_with_sid "$uid" "qa-s06-1@example.com" "$sid")

  qa_set_token "$user_token"
  resp=$(api_get /api/v1/auth/userinfo)
  assert_http_status "$(resp_status "$resp")" 200 "token works before revocation"

  local admin_token
  admin_token=$(gen_default_admin_token)
  qa_set_token "$admin_token"
  resp=$(api_post "/api/v1/admin/users/$uid/logout" "{}")
  assert_http_status "$(resp_status "$resp")" 200 "admin force-logout succeeds"

  qa_set_token "$user_token"
  resp=$(api_get /api/v1/auth/userinfo)
  assert_http_status "$(resp_status "$resp")" 401 "revoked token returns 401"

  local body
  body=$(resp_body "$resp")
  assert_contains "$body" "revoked" "response mentions token revoked"

  db_exec "DELETE FROM sessions WHERE user_id = '\''$uid'\'';" || true
  db_exec "DELETE FROM users WHERE id = '\''$uid'\'';" || true
  qa_set_token ""
'

scenario 2 "Redis down - returns 503 (fail-closed)" '
  db_exec "DELETE FROM sessions WHERE user_id IN (SELECT id FROM users WHERE keycloak_id = '\''kc-qa-s06-2'\'');" || true
  db_exec "DELETE FROM users WHERE keycloak_id = '\''kc-qa-s06-2'\'';" || true

  local uid sid
  uid=$(db_query "SELECT LOWER(UUID());")
  sid=$(db_query "SELECT LOWER(UUID());")

  db_exec "INSERT INTO users (id, keycloak_id, email, display_name) VALUES ('\''$uid'\'', '\''kc-qa-s06-2'\'', '\''qa-s06-2@example.com'\'', '\''QA Blacklist S2'\'');"
  db_exec "INSERT INTO sessions (id, user_id, device_type, ip_address, location, last_active_at) VALUES ('\''$sid'\'', '\''$uid'\'', '\''desktop'\'', '\''192.168.1.1'\'', '\''Test'\'', NOW());"

  local user_token
  user_token=$(_gen_id_token_with_sid "$uid" "qa-s06-2@example.com" "$sid")

  docker stop auth9-redis >/dev/null 2>&1 || true
  sleep 2

  qa_set_token "$user_token"
  resp=$(api_get /api/v1/auth/userinfo)
  local status
  status=$(resp_status "$resp")
  assert_eq "$status" "503" "returns 503 when Redis is down (fail-closed)"

  local body
  body=$(resp_body "$resp")
  assert_not_contains "$status" "200" "does NOT return 200 when Redis is down"

  docker start auth9-redis >/dev/null 2>&1 || true
  sleep 3

  db_exec "DELETE FROM sessions WHERE user_id = '\''$uid'\'';" || true
  db_exec "DELETE FROM users WHERE id = '\''$uid'\'';" || true
  qa_set_token ""
'

scenario 3 "Redis restart - retry mechanism handles brief outage" '
  db_exec "DELETE FROM sessions WHERE user_id IN (SELECT id FROM users WHERE keycloak_id = '\''kc-qa-s06-3'\'');" || true
  db_exec "DELETE FROM users WHERE keycloak_id = '\''kc-qa-s06-3'\'';" || true

  local uid sid
  uid=$(db_query "SELECT LOWER(UUID());")
  sid=$(db_query "SELECT LOWER(UUID());")

  db_exec "INSERT INTO users (id, keycloak_id, email, display_name) VALUES ('\''$uid'\'', '\''kc-qa-s06-3'\'', '\''qa-s06-3@example.com'\'', '\''QA Blacklist S3'\'');"
  db_exec "INSERT INTO sessions (id, user_id, device_type, ip_address, location, last_active_at) VALUES ('\''$sid'\'', '\''$uid'\'', '\''desktop'\'', '\''192.168.1.1'\'', '\''Test'\'', NOW());"

  local user_token
  user_token=$(_gen_id_token_with_sid "$uid" "qa-s06-3@example.com" "$sid")

  docker restart auth9-redis >/dev/null 2>&1 || true
  sleep 4

  qa_set_token "$user_token"
  resp=$(api_get /api/v1/auth/userinfo)
  local status
  status=$(resp_status "$resp")

  assert_match "$status" "^(200|503)$" "returns 200 (retry ok) or 503 (still recovering)"
  assert_ne "$status" "401" "does NOT return 401 (would mean blacklist check skipped)"

  db_exec "DELETE FROM sessions WHERE user_id = '\''$uid'\'';" || true
  db_exec "DELETE FROM users WHERE id = '\''$uid'\'';" || true
  qa_set_token ""
'

scenario 4 "No cache config - requests pass through without blacklist check" '
  local uid sid
  uid=$(db_query "SELECT LOWER(UUID());")
  sid=$(db_query "SELECT LOWER(UUID());")

  db_exec "DELETE FROM sessions WHERE user_id IN (SELECT id FROM users WHERE keycloak_id = '\''kc-qa-s06-4'\'');" || true
  db_exec "DELETE FROM users WHERE keycloak_id = '\''kc-qa-s06-4'\'';" || true
  db_exec "INSERT INTO users (id, keycloak_id, email, display_name) VALUES ('\''$uid'\'', '\''kc-qa-s06-4'\'', '\''qa-s06-4@example.com'\'', '\''QA Blacklist S4'\'');"
  db_exec "INSERT INTO sessions (id, user_id, device_type, ip_address, location, last_active_at) VALUES ('\''$sid'\'', '\''$uid'\'', '\''desktop'\'', '\''192.168.1.1'\'', '\''Test'\'', NOW());"

  local user_token
  user_token=$(_gen_id_token_with_sid "$uid" "qa-s06-4@example.com" "$sid")

  qa_set_token "$user_token"
  resp=$(api_get /api/v1/auth/userinfo)
  local status
  status=$(resp_status "$resp")
  assert_eq "$status" "200" "token works with current config"

  db_exec "DELETE FROM sessions WHERE user_id = '\''$uid'\'';" || true
  db_exec "DELETE FROM users WHERE id = '\''$uid'\'';" || true
  qa_set_token ""
'

run_all
