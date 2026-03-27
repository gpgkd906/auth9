#!/usr/bin/env bash
# QA Auto Test: oidc-userinfo-01
# Doc: docs/oidc/userinfo/01-userinfo-endpoint.md
# Scenarios: 4
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/../lib/runner.sh"

require_bin jq

scenario 1 "UserInfo returns user data with valid token" '
  token=$(gen_admin_token)
  qa_set_token "$token"
  resp=$(api_get "/api/v1/auth/userinfo")
  status=$(resp_status "$resp")
  body=$(resp_body "$resp")
  assert_http_status "$status" 200 "UserInfo returns 200 with valid token"
  assert_json_exists "$body" ".sub" "Response contains sub field"
  qa_set_token ""
'

scenario 2 "UserInfo response includes email field" '
  token=$(gen_admin_token)
  qa_set_token "$token"
  resp=$(api_get "/api/v1/auth/userinfo")
  status=$(resp_status "$resp")
  body=$(resp_body "$resp")
  assert_http_status "$status" 200 "UserInfo returns 200"
  assert_json_exists "$body" ".sub" "Response contains sub field"
  assert_json_exists "$body" ".email" "Response contains email field"
  qa_set_token ""
'

scenario 3 "UserInfo without token returns 401" '
  qa_set_token ""
  resp=$(api_get "/api/v1/auth/userinfo")
  status=$(resp_status "$resp")
  assert_http_status "$status" 401 "UserInfo without token returns 401"
'

scenario 4 "UserInfo with invalid token returns 401" '
  qa_set_token "invalid.jwt.token"
  resp=$(api_get "/api/v1/auth/userinfo")
  status=$(resp_status "$resp")
  assert_http_status "$status" 401 "UserInfo with invalid token returns 401"
  qa_set_token ""
'

run_all
