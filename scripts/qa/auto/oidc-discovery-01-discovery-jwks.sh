#!/usr/bin/env bash
# QA Auto Test: oidc-discovery-01
# Doc: docs/oidc/discovery/01-discovery-jwks.md
# Scenarios: 5
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/../lib/runner.sh"

require_bin jq

scenario 1 "Discovery endpoint returns complete OpenID Configuration" '
  resp=$(api_get "/.well-known/openid-configuration")
  assert_http_status "$(resp_status "$resp")" 200 "discovery returns 200"
  body=$(resp_body "$resp")
  assert_json_exists "$body" ".issuer" "issuer field exists"
  assert_contains "$(echo "$body" | jq -r .authorization_endpoint)" "/api/v1/auth/authorize" "authorization_endpoint contains /api/v1/auth/authorize"
  assert_contains "$(echo "$body" | jq -r .token_endpoint)" "/api/v1/auth/token" "token_endpoint contains /api/v1/auth/token"
  assert_contains "$(echo "$body" | jq -r .userinfo_endpoint)" "/api/v1/auth/userinfo" "userinfo_endpoint contains /api/v1/auth/userinfo"
  assert_contains "$(echo "$body" | jq -r .jwks_uri)" "/.well-known/jwks.json" "jwks_uri contains /.well-known/jwks.json"
  assert_contains "$(echo "$body" | jq -r .end_session_endpoint)" "/api/v1/auth/logout" "end_session_endpoint contains /api/v1/auth/logout"
  assert_contains "$(echo "$body" | jq -c .response_types_supported)" "code" "response_types_supported contains code"
  assert_contains "$(echo "$body" | jq -c .grant_types_supported)" "authorization_code" "grant_types_supported contains authorization_code"
  assert_contains "$(echo "$body" | jq -c .scopes_supported)" "openid" "scopes_supported contains openid"
'

scenario 2 "Discovery endpoint URLs are reachable" '
  disc_resp=$(api_get "/.well-known/openid-configuration")
  disc_body=$(resp_body "$disc_resp")
  jwks_uri=$(echo "$disc_body" | jq -r .jwks_uri)
  jwks_path="${jwks_uri#${API_BASE}}"
  jwks_resp=$(api_get "$jwks_path")
  assert_http_status "$(resp_status "$jwks_resp")" 200 "jwks_uri is reachable"

  userinfo_resp=$(api_get "/api/v1/auth/userinfo")
  assert_http_status "$(resp_status "$userinfo_resp")" 401 "userinfo without token returns 401"

  token_resp=$(api_post "/api/v1/auth/token" "{}")
  token_status=$(resp_status "$token_resp")
  if [[ "$token_status" == "400" || "$token_status" == "415" || "$token_status" == "422" ]]; then
    _qa_pass "token endpoint rejects empty body" "4xx error" "$token_status"
  else
    _qa_fail "token endpoint rejects empty body" "4xx error" "$token_status"
  fi
'

scenario 3 "JWKS endpoint returns valid JWK Set" '
  resp=$(api_get "/.well-known/jwks.json")
  assert_http_status "$(resp_status "$resp")" 200 "jwks returns 200"
  body=$(resp_body "$resp")
  key_count=$(echo "$body" | jq '\''.keys | length'\'')
  if [[ "$key_count" -gt 0 ]]; then
    _qa_pass "keys array is non-empty" ">0" "$key_count"
  else
    _qa_fail "keys array is non-empty" ">0" "$key_count"
  fi
  assert_json_field "$body" ".keys[0].kty" "RSA" "first key kty is RSA"
  assert_json_field "$body" ".keys[0].use" "sig" "first key use is sig"
  assert_json_field "$body" ".keys[0].alg" "RS256" "first key alg is RS256"
  assert_json_field "$body" ".keys[0].kid" "auth9-current" "first key kid is auth9-current"
  assert_json_exists "$body" ".keys[0].n" "first key n field exists"
  assert_json_exists "$body" ".keys[0].e" "first key e field exists"
'

scenario 4 "JWKS key matches token signing" '
  TOKEN=$(gen_admin_token)
  header=$(echo "$TOKEN" | cut -d. -f1 | tr "_-" "/+" | base64 -d 2>/dev/null || true)
  assert_contains "$header" "RS256" "token header contains RS256"
  assert_contains "$header" "auth9-current" "token header contains auth9-current"
'

scenario 5 "JWKS key count reflects rotation config" '
  resp=$(api_get "/.well-known/jwks.json")
  body=$(resp_body "$resp")
  assert_json_exists "$body" ".keys" "keys array exists"
  key_count=$(echo "$body" | jq '\''.keys | length'\'')
  if [[ "$key_count" -ge 1 ]]; then
    _qa_pass "keys count >= 1" ">=1" "$key_count"
  else
    _qa_fail "keys count >= 1" ">=1" "$key_count"
  fi
'

run_all
