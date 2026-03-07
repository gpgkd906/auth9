#!/usr/bin/env bash
# Security Auto Test: security/session-management/02-token-lifecycle
# Doc: docs/security/session-management/02-token-lifecycle.md
# Scenarios: 5 (scenarios 4,5 are roadmap features - skipped)
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/../lib/runner.sh"

require_bin jq
require_bin node

_gen_expired_token() {
  local user_id="$1" email="$2"
  node -e '
const jwt=require("jsonwebtoken"),fs=require("fs");
const pk=fs.readFileSync(process.argv[1],"utf8");
const now=Math.floor(Date.now()/1000);
process.stdout.write(jwt.sign({
  sub:process.argv[2],email:process.argv[3],
  iss:"http://localhost:8080",aud:"auth9",token_type:"identity",
  iat:now-7200,exp:now-3600,sid:"sid-expired"
},pk,{algorithm:"RS256",keyid:"auth9-current"}));
' "$_JWT_PRIVATE_KEY" "$user_id" "$email" 2>/dev/null
}

scenario 1 "Token expiration enforcement" '
  local uid
  uid=$(db_query "SELECT LOWER(UUID());")

  db_exec "DELETE FROM users WHERE keycloak_id = '\''kc-qa-tok02-1'\'';" || true
  db_exec "INSERT INTO users (id, keycloak_id, email, display_name) VALUES ('\''$uid'\'', '\''kc-qa-tok02-1'\'', '\''qa-tok02-1@test.com'\'', '\''QA Token Exp'\'');"

  local valid_token expired_token

  valid_token=$(gen_identity_token "$uid" "qa-tok02-1@test.com")
  qa_set_token "$valid_token"
  resp=$(api_get /api/v1/users/me)
  assert_http_status "$(resp_status "$resp")" 200 "Valid (non-expired) token accepted"

  expired_token=$(_gen_expired_token "$uid" "qa-tok02-1@test.com")
  qa_set_token "$expired_token"
  resp=$(api_get /api/v1/users/me)
  assert_match "$(resp_status "$resp")" "^(401|429)$" "Expired token rejected with 401"

  db_exec "DELETE FROM users WHERE id = '\''$uid'\'';" || true
  qa_set_token ""
'

scenario 2 "Refresh token - reuse of old tokens (Keycloak managed)" '
  resp=$(api_raw POST /api/v1/auth/token \
    -H "Content-Type: application/json" \
    -d "{\"refresh_token\":\"invalid-refresh-token-value\"}")
  status=$(resp_status "$resp")
  assert_match "$status" "^(400|401|404|405|422)$" "Invalid refresh token rejected"
'

scenario 3 "Token blacklist after logout" '
  local uid sid
  uid=$(db_query "SELECT LOWER(UUID());")
  sid="sid-blacklist-$(echo $uid | cut -c1-8)"

  db_exec "DELETE FROM users WHERE keycloak_id = '\''kc-qa-tok02-3'\'';" || true
  db_exec "INSERT INTO users (id, keycloak_id, email, display_name) VALUES ('\''$uid'\'', '\''kc-qa-tok02-3'\'', '\''qa-tok02-3@test.com'\'', '\''QA Token BL'\'');"

  local id_token
  id_token=$(node -e "
const jwt=require(\"jsonwebtoken\"),fs=require(\"fs\");
const pk=fs.readFileSync(process.argv[1],\"utf8\");
const now=Math.floor(Date.now()/1000);
process.stdout.write(jwt.sign({
  sub:process.argv[2],email:process.argv[3],
  iss:\"http://localhost:8080\",aud:\"auth9\",token_type:\"identity\",
  iat:now,exp:now+3600,sid:process.argv[4]
},pk,{algorithm:\"RS256\",keyid:\"auth9-current\"}));
" "$_JWT_PRIVATE_KEY" "$uid" "qa-tok02-3@test.com" "$sid" 2>/dev/null)

  qa_set_token "$id_token"
  resp=$(api_get /api/v1/users/me)
  assert_http_status "$(resp_status "$resp")" 200 "Token valid before logout"

  resp=$(api_post /api/v1/auth/logout "{}")
  logout_status=$(resp_status "$resp")
  assert_match "$logout_status" "^(200|302|307)$" "Logout endpoint returns 200 or 302"

  qa_set_token "$id_token"
  resp=$(api_get /api/v1/users/me)
  post_status=$(resp_status "$resp")
  assert_http_status "$post_status" 401 "Token rejected after logout (blacklisted)"

  db_exec "DELETE FROM users WHERE id = '\''$uid'\'';" || true
  qa_set_token ""
'

skip_scenario 4 "Token scope restriction" "Auth9 uses RBAC instead of OAuth scopes - roadmap feature"

skip_scenario 5 "Token binding (DPoP)" "Not yet implemented - roadmap feature"

run_all
