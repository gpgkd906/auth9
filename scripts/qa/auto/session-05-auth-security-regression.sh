#!/usr/bin/env bash
# QA Auto Test: session/05-auth-security-regression
# Doc: docs/qa/session/05-auth-security-regression.md
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

scenario 1 "Normal user calling admin force-logout should be rejected" '
  db_exec "DELETE FROM sessions WHERE user_id IN (SELECT id FROM users WHERE keycloak_id = '\''kc-qa-s05-1'\'');" || true
  db_exec "DELETE FROM sessions WHERE user_id IN (SELECT id FROM users WHERE keycloak_id = '\''kc-qa-s05-1-target'\'');" || true
  db_exec "DELETE FROM users WHERE keycloak_id IN ('\''kc-qa-s05-1'\'', '\''kc-qa-s05-1-target'\'');" || true

  local normal_uid target_uid normal_sid target_sid
  normal_uid=$(db_query "SELECT LOWER(UUID());")
  target_uid=$(db_query "SELECT LOWER(UUID());")
  normal_sid=$(db_query "SELECT LOWER(UUID());")
  target_sid=$(db_query "SELECT LOWER(UUID());")

  db_exec "INSERT INTO users (id, keycloak_id, email, display_name) VALUES ('\''$normal_uid'\'', '\''kc-qa-s05-1'\'', '\''qa-normal-user@example.com'\'', '\''QA Normal User'\'');"
  db_exec "INSERT INTO users (id, keycloak_id, email, display_name) VALUES ('\''$target_uid'\'', '\''kc-qa-s05-1-target'\'', '\''qa-target-user@example.com'\'', '\''QA Target User'\'');"
  db_exec "INSERT INTO sessions (id, user_id, device_type, ip_address, location, last_active_at) VALUES ('\''$target_sid'\'', '\''$target_uid'\'', '\''desktop'\'', '\''192.168.1.1'\'', '\''Test'\'', NOW());"

  local pre_count
  pre_count=$(db_query "SELECT COUNT(*) FROM sessions WHERE user_id = '\''$target_uid'\'' AND revoked_at IS NULL;")

  local normal_token
  normal_token=$(_gen_id_token_with_sid "$normal_uid" "qa-normal-user@example.com" "$normal_sid")
  qa_set_token "$normal_token"

  resp=$(api_post "/api/v1/admin/users/$target_uid/logout" "{}")
  local status
  status=$(resp_status "$resp")
  assert_match "$status" "^(401|403)$" "normal user denied admin force-logout"

  local post_count
  post_count=$(db_query "SELECT COUNT(*) FROM sessions WHERE user_id = '\''$target_uid'\'' AND revoked_at IS NULL;")
  assert_eq "$post_count" "$pre_count" "target user sessions unchanged"

  db_exec "DELETE FROM sessions WHERE user_id IN ('\''$normal_uid'\'', '\''$target_uid'\'');" || true
  db_exec "DELETE FROM users WHERE id IN ('\''$normal_uid'\'', '\''$target_uid'\'');" || true
  qa_set_token ""
'

scenario 2 "Token blacklisted after force-logout" '
  db_exec "DELETE FROM sessions WHERE user_id IN (SELECT id FROM users WHERE keycloak_id = '\''kc-qa-s05-2'\'');" || true
  db_exec "DELETE FROM users WHERE keycloak_id = '\''kc-qa-s05-2'\'';" || true

  local uid sid
  uid=$(db_query "SELECT LOWER(UUID());")
  sid=$(db_query "SELECT LOWER(UUID());")

  db_exec "INSERT INTO users (id, keycloak_id, email, display_name) VALUES ('\''$uid'\'', '\''kc-qa-s05-2'\'', '\''qa-s05-2@example.com'\'', '\''QA Blacklist Test'\'');"
  db_exec "INSERT INTO sessions (id, user_id, device_type, ip_address, location, last_active_at) VALUES ('\''$sid'\'', '\''$uid'\'', '\''desktop'\'', '\''192.168.1.1'\'', '\''Test'\'', NOW());"

  local user_token
  user_token=$(_gen_id_token_with_sid "$uid" "qa-s05-2@example.com" "$sid")

  qa_set_token "$user_token"
  resp=$(api_get /api/v1/auth/userinfo)
  local pre_status
  pre_status=$(resp_status "$resp")
  assert_eq "$pre_status" "200" "identity token works before force-logout"

  local admin_token
  admin_token=$(gen_default_admin_token)
  qa_set_token "$admin_token"
  resp=$(api_post "/api/v1/admin/users/$uid/logout" "{}")
  assert_http_status "$(resp_status "$resp")" 200 "admin force-logout succeeds"

  qa_set_token "$user_token"
  resp=$(api_get /api/v1/auth/userinfo)
  local post_status
  post_status=$(resp_status "$resp")
  assert_eq "$post_status" "401" "identity token rejected after force-logout"

  db_exec "DELETE FROM sessions WHERE user_id = '\''$uid'\'';" || true
  db_exec "DELETE FROM users WHERE id = '\''$uid'\'';" || true
  qa_set_token ""
'

scenario 3 "OIDC callback should not leak tokens in redirect URL" '
  resp=$(api_raw GET "/api/v1/auth/callback?code=fake-code&state=fake-state")
  local status
  status=$(resp_status "$resp")
  local body
  body=$(resp_body "$resp")

  local headers
  headers=$(curl -sI "${API_BASE}/api/v1/auth/callback?code=fake-code&state=fake-state" 2>&1)

  assert_not_contains "$headers" "access_token=" "redirect does not contain access_token"
  assert_not_contains "$headers" "id_token=" "redirect does not contain id_token"
  assert_not_contains "$body" "access_token=" "response body does not contain access_token"
  assert_not_contains "$body" "id_token=" "response body does not contain id_token"
'

scenario 4 "Spoofed x-tenant-id cannot bypass rate limiting" '
  local got_429="no"
  for i in $(seq 1 30); do
    resp=$(api_raw GET /api/v1/tenants -H "x-tenant-id: spoof-$i")
    local status
    status=$(resp_status "$resp")
    if [[ "$status" == "429" ]]; then
      got_429="yes"
      break
    fi
  done

  if [[ "$got_429" == "yes" ]]; then
    assert_eq "yes" "yes" "rate limiter triggered despite x-tenant-id rotation"
  else
    assert_eq "no-429" "429-expected" "rate limiter not triggered (threshold may be > 30)"
  fi
'

scenario 5 "Dynamic path rate limit key should collapse to template" '
  local token
  token=$(gen_default_admin_token)
  qa_set_token "$token"

  for i in $(seq 1 20); do
    local fake_id
    fake_id=$(printf "00000000-0000-0000-0000-%012d" "$i")
    api_get "/api/v1/users/$fake_id" >/dev/null 2>&1 || true
  done

  if command -v redis-cli &>/dev/null; then
    local key_count
    key_count=$(redis-cli --raw KEYS "auth9:ratelimit:*:GET:/api/v1/users/*" 2>/dev/null | wc -l | tr -d " ")
    if [[ -n "$key_count" && "$key_count" -lt 5 ]]; then
      assert_eq "collapsed" "collapsed" "rate limit keys collapsed to template path"
    else
      local docker_count
      docker_count=$(docker exec auth9-redis redis-cli --raw KEYS "auth9:ratelimit:*:GET:/api/v1/users/*" 2>/dev/null | wc -l | tr -d " ")
      if [[ -n "$docker_count" && "$docker_count" -lt 5 ]]; then
        assert_eq "collapsed" "collapsed" "rate limit keys collapsed to template path (docker)"
      else
        assert_eq "${docker_count:-unknown}" "<=4" "rate limit keys may not be collapsed"
      fi
    fi
  else
    local docker_count
    docker_count=$(docker exec auth9-redis redis-cli --raw KEYS "auth9:ratelimit:*:GET:/api/v1/users/*" 2>/dev/null | wc -l | tr -d " ")
    if [[ -n "$docker_count" && "$docker_count" -lt 5 ]]; then
      assert_eq "collapsed" "collapsed" "rate limit keys collapsed to template path"
    else
      assert_eq "${docker_count:-unknown}" "<=4" "rate limit keys may not be collapsed"
    fi
  fi

  qa_set_token ""
'

run_all
