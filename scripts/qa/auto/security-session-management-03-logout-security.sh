#!/usr/bin/env bash
# Security Auto Test: security/session-management/03-logout-security
# Doc: docs/security/session-management/03-logout-security.md
# Scenarios: 4
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

scenario 1 "Complete logout invalidates tokens" '
  local uid sid
  uid=$(db_query "SELECT LOWER(UUID());")
  sid="sid-logout-$(echo $uid | cut -c1-8)"

  db_exec "DELETE FROM users WHERE keycloak_id = '\''kc-qa-lo03-1'\'';" || true
  db_exec "INSERT INTO users (id, keycloak_id, email, display_name) VALUES ('\''$uid'\'', '\''kc-qa-lo03-1'\'', '\''qa-lo03-1@test.com'\'', '\''QA Logout S1'\'');"
  db_exec "INSERT INTO sessions (id, user_id, device_type, ip_address, location, last_active_at) VALUES ('\''$sid'\'', '\''$uid'\'', '\''desktop'\'', '\''192.168.1.1'\'', '\''Test'\'', NOW());"

  local token
  token=$(_gen_id_token_with_sid "$uid" "qa-lo03-1@test.com" "$sid")

  qa_set_token "$token"
  resp=$(api_get /api/v1/users/me)
  assert_http_status "$(resp_status "$resp")" 200 "Token valid before logout"

  resp=$(api_post /api/v1/auth/logout "{}")
  logout_status=$(resp_status "$resp")
  assert_match "$logout_status" "^(200|302|307)$" "Logout returns success or redirect"

  qa_set_token "$token"
  resp=$(api_get /api/v1/users/me)
  assert_match "$(resp_status "$resp")" "^(401|429)$" "Token rejected after logout"

  db_exec "DELETE FROM sessions WHERE user_id = '\''$uid'\'';" || true
  db_exec "DELETE FROM users WHERE id = '\''$uid'\'';" || true
  qa_set_token ""
'

skip_scenario 2 "OIDC single logout (SLO)" "Requires multiple OIDC client apps running simultaneously"

scenario 3 "Admin force logout mechanism" '
  local uid sid1 sid2
  uid=$(db_query "SELECT LOWER(UUID());")
  sid1=$(db_query "SELECT LOWER(UUID());")
  sid2=$(db_query "SELECT LOWER(UUID());")

  db_exec "DELETE FROM users WHERE keycloak_id = '\''kc-qa-lo03-3'\'';" || true
  db_exec "INSERT INTO users (id, keycloak_id, email, display_name) VALUES ('\''$uid'\'', '\''kc-qa-lo03-3'\'', '\''qa-lo03-3@test.com'\'', '\''QA Logout S3'\'');"
  db_exec "INSERT INTO sessions (id, user_id, device_type, ip_address, location, last_active_at) VALUES ('\''$sid1'\'', '\''$uid'\'', '\''desktop'\'', '\''192.168.1.1'\'', '\''Beijing'\'', NOW()), ('\''$sid2'\'', '\''$uid'\'', '\''mobile'\'', '\''192.168.1.2'\'', '\''Shanghai'\'', NOW());"

  local admin_token
  admin_token=$(gen_default_admin_token)
  qa_set_token "$admin_token"

  resp=$(api_post "/api/v1/admin/users/$uid/logout" "{}")
  assert_http_status "$(resp_status "$resp")" 200 "Admin force logout returns 200"

  body=$(resp_body "$resp")
  assert_json_exists "$body" ".data.revoked_count" "Response contains revoked_count"

  assert_db "SELECT COUNT(*) FROM sessions WHERE user_id = '\''$uid'\'' AND revoked_at IS NULL;" "0" "All sessions revoked after admin force logout"

  db_exec "DELETE FROM sessions WHERE user_id = '\''$uid'\'';" || true
  db_exec "DELETE FROM users WHERE id = '\''$uid'\'';" || true
  qa_set_token ""
'

scenario 4 "Cache-control headers on authenticated endpoints" '
  local uid
  uid=$(db_query "SELECT LOWER(UUID());")

  db_exec "DELETE FROM users WHERE keycloak_id = '\''kc-qa-lo03-4'\'';" || true
  db_exec "INSERT INTO users (id, keycloak_id, email, display_name) VALUES ('\''$uid'\'', '\''kc-qa-lo03-4'\'', '\''qa-lo03-4@test.com'\'', '\''QA Logout S4'\'');"

  local token
  token=$(gen_identity_token "$uid" "qa-lo03-4@test.com")

  headers=$(curl -sI -H "Authorization: Bearer $token" "${API_BASE}/api/v1/users/me" 2>&1 || true)

  assert_not_contains "$headers" "public" "Cache-Control does not include public for authenticated endpoint"

  db_exec "DELETE FROM users WHERE id = '\''$uid'\'';" || true
'

run_all
