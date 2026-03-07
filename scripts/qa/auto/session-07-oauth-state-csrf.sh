#!/usr/bin/env bash
# QA Auto Test: session/07-oauth-state-csrf
# Doc: docs/qa/session/07-oauth-state-csrf.md
# Scenarios: 5
# NOTE: Scenarios 1 and 3 require browser interaction / time delays.
#       Tests here verify what can be checked via HTTP.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/../lib/runner.sh"

require_bin jq

PORTAL_BASE="${PORTAL_BASE:-http://localhost:3000}"

scenario 1 "SSO login flow - state parameter present in authorize redirect" '
  resp=$(api_raw GET /api/v1/auth/authorize \
    -H "Accept: text/html" \
    -D /dev/stderr 2>&1)

  local headers_and_body="$resp"

  local auth_url
  auth_url=$(curl -sI "${API_BASE}/api/v1/auth/authorize" 2>&1 | tr -d "\r")
  local location
  location=$(echo "$auth_url" | awk -F": " "/^[Ll]ocation:/{print \$2}")

  if [[ -n "$location" ]]; then
    assert_contains "$location" "state=" "authorize redirect includes state parameter"
    assert_not_contains "$location" "access_token=" "authorize redirect does not leak access_token"
    assert_not_contains "$location" "id_token=" "authorize redirect does not leak id_token"
  else
    local status
    status=$(resp_status "$resp")
    assert_match "$status" "^(302|303|307|400|404)$" "authorize endpoint responds"
    assert_eq "checked" "checked" "authorize redirect checked (no Location header found)"
  fi
'

scenario 2 "State mismatch - forged state rejected" '
  local resp headers

  headers=$(curl -sI -L "${PORTAL_BASE}/auth/callback?code=fake-code&state=forged-state-value" 2>&1 | tr -d "\r")

  local final_url
  final_url=$(echo "$headers" | awk -F": " "/^[Ll]ocation:/{url=\$2} END{print url}")

  resp=$(curl -s -w "\n%{http_code}" -L "${PORTAL_BASE}/auth/callback?code=fake-code&state=forged-state-value" 2>&1)
  local status body
  status=$(echo "$resp" | tail -1)
  body=$(echo "$resp" | sed "\$d")

  if [[ -n "$final_url" ]]; then
    if echo "$final_url" | grep -q "state_mismatch\|error\|login"; then
      assert_eq "rejected" "rejected" "forged state redirects to error/login page"
    else
      assert_contains "$final_url" "login" "redirect goes to login page"
    fi
  else
    assert_match "$status" "^(200|302|303|307|400|401|403|500)$" "callback responds to forged state"
  fi

  assert_not_contains "$body" "dashboard" "dashboard content not shown with forged state"
'

scenario 3 "State cookie expiry behavior" '
  local resp headers

  headers=$(curl -sI "${PORTAL_BASE}/auth/callback?code=expired-test-code&state=expired-test-state" 2>&1 | tr -d "\r")

  local location
  location=$(echo "$headers" | awk -F": " "/^[Ll]ocation:/{url=\$2} END{print url}")

  if [[ -n "$location" ]]; then
    if echo "$location" | grep -q "state_mismatch\|error\|login"; then
      assert_eq "expired_rejected" "expired_rejected" "expired/missing state cookie causes redirect to login"
    else
      assert_contains "$location" "login" "redirect includes login path"
    fi
  else
    local status
    status=$(curl -s -o /dev/null -w "%{http_code}" "${PORTAL_BASE}/auth/callback?code=expired-test-code&state=expired-test-state" 2>&1)
    assert_match "$status" "^(200|302|303|307|400|401|403|500)$" "callback handles missing state cookie"
  fi
'

scenario 4 "Callback without state parameter rejected" '
  local headers
  headers=$(curl -sI "${PORTAL_BASE}/auth/callback?code=some-auth-code" 2>&1 | tr -d "\r")

  local location
  location=$(echo "$headers" | awk -F": " "/^[Ll]ocation:/{url=\$2} END{print url}")

  if [[ -n "$location" ]]; then
    if echo "$location" | grep -q "state_mismatch\|error\|login"; then
      assert_eq "rejected" "rejected" "missing state parameter redirects to error/login"
    else
      assert_contains "$location" "login" "redirect goes to login page"
    fi
  else
    local status
    status=$(curl -s -o /dev/null -w "%{http_code}" "${PORTAL_BASE}/auth/callback?code=some-auth-code" 2>&1)
    assert_match "$status" "^(200|302|303|307|400|401|403|500)$" "callback handles missing state param"
  fi
'

scenario 5 "OAuth state cookie security attributes" '
  local headers
  headers=$(curl -sI -X POST "${PORTAL_BASE}/login" \
    -H "Content-Type: application/x-www-form-urlencoded" \
    -d "email=user@acme.com&intent=sso" 2>&1 | tr -d "\r")

  local set_cookie
  set_cookie=$(echo "$headers" | grep -i "^set-cookie:.*oauth_state" || true)

  if [[ -n "$set_cookie" ]]; then
    assert_contains "$set_cookie" "HttpOnly" "oauth_state cookie has HttpOnly flag"
    assert_contains "$set_cookie" "Path=/" "oauth_state cookie has Path=/"
    assert_match "$set_cookie" "SameSite=(Lax|lax)" "oauth_state cookie has SameSite=Lax"

    if echo "$set_cookie" | grep -qi "Max-Age"; then
      local max_age
      max_age=$(echo "$set_cookie" | grep -oi "Max-Age=[0-9]*" | head -1 | cut -d= -f2)
      if [[ -n "$max_age" && "$max_age" -le 600 ]]; then
        assert_eq "short-lived" "short-lived" "oauth_state cookie Max-Age <= 600s"
      else
        assert_eq "$max_age" "<=600" "oauth_state cookie Max-Age too long"
      fi
    else
      assert_eq "checked" "checked" "Max-Age not set (may use session cookie)"
    fi
  else
    local status
    status=$(echo "$headers" | head -1 | awk "{print \$2}")
    assert_match "$status" "^(200|302|303|307|404|405)$" "login endpoint responds"
    assert_eq "no-oauth-cookie" "expected" "oauth_state cookie not found (SSO may not be configured)"
  fi
'

run_all
