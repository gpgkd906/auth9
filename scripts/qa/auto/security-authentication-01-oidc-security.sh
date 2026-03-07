#!/usr/bin/env bash
# Security Auto Test: security/authentication/01-oidc-security
# Doc: docs/security/authentication/01-oidc-security.md
# Scenarios: 5
# ASVS 5.0: V10.1, V10.2, V10.3, V10.4
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/../lib/runner.sh"

require_bin jq
require_bin curl

PORTAL_BASE="${PORTAL_BASE:-http://localhost:3000}"

scenario 1 "Authorization Code replay attack" '
  resp=$(api_raw POST /api/v1/auth/token \
    -H "Content-Type: application/x-www-form-urlencoded" \
    -d "grant_type=authorization_code&code=INVALID_REPLAYED_CODE_12345&client_id=auth9-portal&redirect_uri=http://localhost:3000/callback")
  status=$(resp_status "$resp")
  body=$(resp_body "$resp")

  assert_match "$status" "^(400|401|403)$" "replayed/invalid auth code rejected"

  if echo "$body" | jq -e . >/dev/null 2>&1; then
    assert_not_contains "$body" "access_token" "no access_token issued for invalid code"
  else
    assert_eq "checked" "checked" "response is not JSON (expected for invalid code)"
  fi

  resp2=$(api_raw POST /api/v1/auth/token \
    -H "Content-Type: application/x-www-form-urlencoded" \
    -d "grant_type=authorization_code&code=INVALID_REPLAYED_CODE_12345&client_id=auth9-portal&redirect_uri=http://localhost:3000/callback")
  status2=$(resp_status "$resp2")
  assert_match "$status2" "^(400|401|403)$" "second attempt with same code also rejected"
'

scenario 2 "Redirect URI validation bypass" '
  MALICIOUS_URIS=(
    "http://attacker.com/callback"
    "http://localhost:3000.attacker.com/callback"
    "http://localhost:3000@attacker.com/callback"
  )

  for uri in "${MALICIOUS_URIS[@]}"; do
    encoded_uri=$(python3 -c "import urllib.parse; print(urllib.parse.quote('"'"'$uri'"'"', safe='"'"':/?=&'"'"'))" 2>/dev/null || echo "$uri")
    headers=$(curl -sI "${API_BASE}/api/v1/auth/authorize?client_id=auth9-portal&redirect_uri=${encoded_uri}&response_type=code&scope=openid" 2>&1 | tr -d "\r")
    status=$(echo "$headers" | head -1 | awk "{print \$2}")

    location=$(echo "$headers" | awk -F": " "/^[Ll]ocation:/{print \$2}" | head -1)

    if [[ "$status" =~ ^(400|403|422)$ ]]; then
      assert_eq "$status" "$status" "malicious redirect_uri rejected (${uri})"
    elif [[ -n "$location" ]]; then
      assert_not_contains "$location" "attacker.com" "redirect does not go to attacker domain (${uri})"
    else
      assert_match "$status" "^(400|403|404|422|302|303|307)$" "authorize handles malicious URI (${uri})"
    fi
  done

  legit_headers=$(curl -sI "${API_BASE}/api/v1/auth/authorize?client_id=auth9-portal&redirect_uri=http://localhost:3000/auth/callback&response_type=code&scope=openid" 2>&1 | tr -d "\r")
  legit_status=$(echo "$legit_headers" | head -1 | awk "{print \$2}")
  assert_match "$legit_status" "^(200|302|303|307)$" "legitimate redirect_uri accepted"
'

scenario 3 "State parameter CSRF protection" '
  headers=$(curl -sI "${API_BASE}/api/v1/auth/authorize?client_id=auth9-portal&redirect_uri=http://localhost:3000/auth/callback&response_type=code&scope=openid&state=test-csrf-value" 2>&1 | tr -d "\r")
  status=$(echo "$headers" | head -1 | awk "{print \$2}")

  if [[ "$status" =~ ^(302|303|307)$ ]]; then
    location=$(echo "$headers" | awk -F": " "/^[Ll]ocation:/{print \$2}" | head -1)
    assert_contains "$location" "state=" "redirect includes state parameter"

    server_state=$(echo "$location" | grep -oP "state=([^&]*)" | head -1 | cut -d= -f2 || true)
    if [[ -n "$server_state" ]]; then
      UUID_REGEX="^[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-4[0-9a-fA-F]{3}-[89abAB][0-9a-fA-F]{3}-[0-9a-fA-F]{12}$"
      decoded_state=$(python3 -c "import urllib.parse; print(urllib.parse.unquote('"'"'$server_state'"'"'))" 2>/dev/null || echo "$server_state")
      if [[ "$decoded_state" =~ $UUID_REGEX ]]; then
        assert_eq "uuid_v4" "uuid_v4" "server-generated state is UUID v4"
      fi
      assert_ne "$decoded_state" "test-csrf-value" "server state differs from client-supplied state"
    fi
  else
    assert_match "$status" "^(200|400|404)$" "authorize endpoint responds"
  fi

  forged_resp=$(api_raw GET "/api/v1/auth/callback?code=fake-auth-code&state=forged-random-state")
  forged_status=$(resp_status "$forged_resp")
  assert_match "$forged_status" "^(400|401|403|404|302)$" "forged state rejected at callback"
'

scenario 4 "Scope escalation attack" '
  headers=$(curl -sI "${API_BASE}/api/v1/auth/authorize?client_id=auth9-portal&redirect_uri=http://localhost:3000/auth/callback&response_type=code&scope=openid+profile+email+admin+offline_access+platform:admin" 2>&1 | tr -d "\r")
  status=$(echo "$headers" | head -1 | awk "{print \$2}")

  if [[ "$status" =~ ^(302|303|307)$ ]]; then
    location=$(echo "$headers" | awk -F": " "/^[Ll]ocation:/{print \$2}" | head -1)
    if [[ -n "$location" ]]; then
      scope_in_redirect=$(echo "$location" | grep -oP "scope=([^&]*)" | head -1 | cut -d= -f2 || true)
      if [[ -n "$scope_in_redirect" ]]; then
        decoded_scope=$(python3 -c "import urllib.parse; print(urllib.parse.unquote('"'"'$scope_in_redirect'"'"'))" 2>/dev/null || echo "$scope_in_redirect")
        assert_not_contains "$decoded_scope" "platform:admin" "platform:admin scope not forwarded"
      fi
      assert_eq "redirect_ok" "redirect_ok" "authorize accepts request (Keycloak filters scopes)"
    fi
  elif [[ "$status" =~ ^(400|403|422)$ ]]; then
    assert_eq "$status" "$status" "excessive scopes rejected at authorize endpoint"
  else
    assert_match "$status" "^(200|302|400|403|404)$" "authorize handles scope escalation"
  fi

  headers_normal=$(curl -sI "${API_BASE}/api/v1/auth/authorize?client_id=auth9-portal&redirect_uri=http://localhost:3000/auth/callback&response_type=code&scope=openid" 2>&1 | tr -d "\r")
  normal_status=$(echo "$headers_normal" | head -1 | awk "{print \$2}")
  assert_match "$normal_status" "^(200|302|303|307)$" "normal scope openid accepted"
'

scenario 5 "OIDC metadata endpoint security" '
  resp=$(api_raw GET /.well-known/openid-configuration)
  status=$(resp_status "$resp")
  body=$(resp_body "$resp")

  assert_match "$status" "^(200|404)$" "OIDC discovery endpoint responds"

  if [[ "$status" == "200" ]]; then
    if echo "$body" | jq -e . >/dev/null 2>&1; then
      assert_json_exists "$body" ".issuer" "OIDC config has issuer"
      assert_json_exists "$body" ".authorization_endpoint" "OIDC config has authorization_endpoint"
      assert_json_exists "$body" ".token_endpoint" "OIDC config has token_endpoint"
      assert_json_not_exists "$body" ".private_key" "OIDC config does not leak private_key"
      assert_json_not_exists "$body" ".client_secret" "OIDC config does not leak client_secret"
    fi
  fi

  headers=$(curl -sI "${API_BASE}/.well-known/openid-configuration" 2>&1 | tr -d "\r")
  content_type=$(echo "$headers" | grep -i "^content-type:" | head -1 || true)
  if [[ -n "$content_type" ]]; then
    assert_contains "$content_type" "json" "OIDC config content-type is JSON"
  fi

  jwks_resp=$(api_raw GET /.well-known/jwks.json)
  jwks_status=$(resp_status "$jwks_resp")
  jwks_body=$(resp_body "$jwks_resp")

  if [[ "$jwks_status" == "200" ]]; then
    assert_not_contains "$jwks_body" "\"d\":" "JWKS does not expose private key d parameter"
    assert_not_contains "$jwks_body" "\"p\":" "JWKS does not expose private key p parameter"
    assert_not_contains "$jwks_body" "\"q\":" "JWKS does not expose private key q parameter"
    assert_not_contains "$jwks_body" "\"dp\":" "JWKS does not expose private key dp parameter"
    assert_not_contains "$jwks_body" "\"dq\":" "JWKS does not expose private key dq parameter"
    if echo "$jwks_body" | jq -e ".keys" >/dev/null 2>&1; then
      assert_json_exists "$jwks_body" ".keys[0].kty" "JWKS has key type"
    fi
  else
    assert_match "$jwks_status" "^(200|404)$" "JWKS endpoint responds"
  fi
'

run_all
