#!/usr/bin/env bash
# QA Auto Test: user/02-advanced
# Doc: docs/qa/user/02-advanced.md
# Scenarios: 5
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/../lib/runner.sh"

require_bin jq

# DELETE with JSON body (needed for MFA disable endpoint)
_api_delete_with_body() {
  local path="$1" data="$2"
  _build_curl_args
  local body status_code
  body=$(curl -s -w '\n%{http_code}' -X DELETE \
    "${_CURL_AUTH_ARGS[@]+"${_CURL_AUTH_ARGS[@]}"}" \
    -H "Content-Type: application/json" \
    -d "$data" \
    "${API_BASE}${path}")
  status_code=$(echo "$body" | tail -1)
  body=$(echo "$body" | sed '$d')
  printf '%s\n%s' "$status_code" "$body"
}

scenario 1 "删除用户（级联删除）" '
  ADMIN_ID=$(qa_get_admin_id)
  TENANT_ID=$(qa_get_tenant_id)
  TOKEN=$(gen_tenant_token "$ADMIN_ID" "$TENANT_ID")
  qa_set_token "$TOKEN"

  TEST_EMAIL="qa-u02-s1@example.com"
  db_exec "DELETE FROM user_tenant_roles WHERE tenant_user_id IN (SELECT tu.id FROM tenant_users tu JOIN users u ON u.id = tu.user_id WHERE u.email = '\''$TEST_EMAIL'\'')"
  db_exec "DELETE FROM tenant_users WHERE user_id IN (SELECT id FROM users WHERE email = '\''$TEST_EMAIL'\'')"
  db_exec "DELETE FROM sessions WHERE user_id IN (SELECT id FROM users WHERE email = '\''$TEST_EMAIL'\'')"
  db_exec "DELETE FROM users WHERE email = '\''$TEST_EMAIL'\''"

  resp=$(api_post /api/v1/users "{\"email\":\"$TEST_EMAIL\",\"display_name\":\"Delete User\",\"password\":\"SecurePass123!\"}")
  assert_http_status "$(resp_status "$resp")" 201 "user created for delete test"
  USER_ID=$(echo "$(resp_body "$resp")" | jq -r ".data.id")

  api_post "/api/v1/users/$USER_ID/tenants" "{\"tenant_id\":\"$TENANT_ID\",\"role_in_tenant\":\"member\"}" >/dev/null 2>&1

  assert_db_not_empty "SELECT id FROM tenant_users WHERE user_id = '\''$USER_ID'\''" "tenant association exists before delete"

  resp2=$(api_delete "/api/v1/users/$USER_ID")
  assert_http_status "$(resp_status "$resp2")" 200 "DELETE /api/v1/users/{id} returns 200"

  assert_db "SELECT COUNT(*) FROM users WHERE id = '\''$USER_ID'\''" "0" "user removed from DB"
  assert_db "SELECT COUNT(*) FROM tenant_users WHERE user_id = '\''$USER_ID'\''" "0" "tenant_users cascade deleted"

  qa_set_token ""
'

scenario 2 "启用用户 MFA" '
  ADMIN_ID=$(qa_get_admin_id)
  TENANT_ID=$(qa_get_tenant_id)
  ADMIN_TOKEN=$(gen_tenant_token "$ADMIN_ID" "$TENANT_ID")
  qa_set_token "$ADMIN_TOKEN"

  CALLER_EMAIL="qa-u02-mfa-caller@example.com"
  TARGET_EMAIL="qa-u02-s2-target@example.com"
  CALLER_PASS="SecurePass123!"

  db_exec "DELETE FROM tenant_users WHERE user_id IN (SELECT id FROM users WHERE email = '\''$CALLER_EMAIL'\'' OR email = '\''$TARGET_EMAIL'\'')"
  db_exec "DELETE FROM users WHERE email = '\''$CALLER_EMAIL'\'' OR email = '\''$TARGET_EMAIL'\''"

  resp=$(api_post /api/v1/users "{\"email\":\"$CALLER_EMAIL\",\"display_name\":\"MFA Caller\",\"password\":\"$CALLER_PASS\"}")
  assert_http_status "$(resp_status "$resp")" 201 "MFA caller user created"
  CALLER_ID=$(echo "$(resp_body "$resp")" | jq -r ".data.id")

  resp=$(api_post /api/v1/users "{\"email\":\"$TARGET_EMAIL\",\"display_name\":\"MFA Target\",\"password\":\"SecurePass123!\"}")
  assert_http_status "$(resp_status "$resp")" 201 "MFA target user created"
  TARGET_ID=$(echo "$(resp_body "$resp")" | jq -r ".data.id")

  CALLER_TOKEN=$(gen_tenant_token "$CALLER_ID" "$TENANT_ID")
  qa_set_token "$CALLER_TOKEN"

  resp2=$(api_post "/api/v1/users/$TARGET_ID/mfa" "{\"confirm_password\":\"$CALLER_PASS\"}")
  assert_http_status "$(resp_status "$resp2")" 200 "POST /api/v1/users/{id}/mfa returns 200"

  body=$(resp_body "$resp2")
  assert_json_field "$body" ".data.mfa_enabled" "true" "MFA enabled in response"

  assert_db "SELECT mfa_enabled FROM users WHERE id = '\''$TARGET_ID'\''" "1" "MFA enabled in DB"

  qa_set_token "$ADMIN_TOKEN"
  api_delete "/api/v1/users/$TARGET_ID" >/dev/null 2>&1 || true
  api_delete "/api/v1/users/$CALLER_ID" >/dev/null 2>&1 || true
  qa_set_token ""
'

scenario 3 "禁用用户 MFA" '
  ADMIN_ID=$(qa_get_admin_id)
  TENANT_ID=$(qa_get_tenant_id)
  ADMIN_TOKEN=$(gen_tenant_token "$ADMIN_ID" "$TENANT_ID")
  qa_set_token "$ADMIN_TOKEN"

  CALLER_EMAIL="qa-u02-mfa-caller3@example.com"
  TARGET_EMAIL="qa-u02-s3-target@example.com"
  CALLER_PASS="SecurePass123!"

  db_exec "DELETE FROM tenant_users WHERE user_id IN (SELECT id FROM users WHERE email = '\''$CALLER_EMAIL'\'' OR email = '\''$TARGET_EMAIL'\'')"
  db_exec "DELETE FROM users WHERE email = '\''$CALLER_EMAIL'\'' OR email = '\''$TARGET_EMAIL'\''"

  resp=$(api_post /api/v1/users "{\"email\":\"$CALLER_EMAIL\",\"display_name\":\"MFA Caller\",\"password\":\"$CALLER_PASS\"}")
  CALLER_ID=$(echo "$(resp_body "$resp")" | jq -r ".data.id")

  resp=$(api_post /api/v1/users "{\"email\":\"$TARGET_EMAIL\",\"display_name\":\"MFA Target\",\"password\":\"SecurePass123!\"}")
  TARGET_ID=$(echo "$(resp_body "$resp")" | jq -r ".data.id")

  CALLER_TOKEN=$(gen_tenant_token "$CALLER_ID" "$TENANT_ID")
  qa_set_token "$CALLER_TOKEN"

  api_post "/api/v1/users/$TARGET_ID/mfa" "{\"confirm_password\":\"$CALLER_PASS\"}" >/dev/null 2>&1
  assert_db "SELECT mfa_enabled FROM users WHERE id = '\''$TARGET_ID'\''" "1" "MFA was enabled before disable"

  resp2=$(_api_delete_with_body "/api/v1/users/$TARGET_ID/mfa" "{\"confirm_password\":\"$CALLER_PASS\"}")
  assert_http_status "$(resp_status "$resp2")" 200 "DELETE /api/v1/users/{id}/mfa returns 200"

  body=$(resp_body "$resp2")
  assert_json_field "$body" ".data.mfa_enabled" "false" "MFA disabled in response"

  assert_db "SELECT mfa_enabled FROM users WHERE id = '\''$TARGET_ID'\''" "0" "MFA disabled in DB"

  qa_set_token "$ADMIN_TOKEN"
  api_delete "/api/v1/users/$TARGET_ID" >/dev/null 2>&1 || true
  api_delete "/api/v1/users/$CALLER_ID" >/dev/null 2>&1 || true
  qa_set_token ""
'

scenario 4 "用户列表分页和搜索" '
  ADMIN_ID=$(qa_get_admin_id)
  TENANT_ID=$(qa_get_tenant_id)
  TOKEN=$(gen_tenant_token "$ADMIN_ID" "$TENANT_ID")
  qa_set_token "$TOKEN"

  resp=$(api_get "/api/v1/users?page=1&per_page=5")
  assert_http_status "$(resp_status "$resp")" 200 "GET /api/v1/users returns 200"

  body=$(resp_body "$resp")
  assert_json_exists "$body" ".data" "response has data array"
  assert_json_exists "$body" ".pagination" "response has pagination"
  assert_json_exists "$body" ".pagination.total" "pagination has total"
  assert_json_exists "$body" ".pagination.page" "pagination has page"
  assert_json_field "$body" ".pagination.page" "1" "page is 1"
  assert_json_field "$body" ".pagination.per_page" "5" "per_page is 5"

  resp2=$(api_get "/api/v1/users?search=admin")
  assert_http_status "$(resp_status "$resp2")" 200 "GET /api/v1/users?search=admin returns 200"
  body2=$(resp_body "$resp2")
  assert_json_exists "$body2" ".data" "search response has data"

  qa_set_token ""
'

scenario 5 "查看用户的租户列表" '
  ADMIN_ID=$(qa_get_admin_id)
  TENANT_ID=$(qa_get_tenant_id)
  TOKEN=$(gen_tenant_token "$ADMIN_ID" "$TENANT_ID")
  qa_set_token "$TOKEN"

  TEST_EMAIL="qa-u02-s5@example.com"
  db_exec "DELETE FROM tenant_users WHERE user_id IN (SELECT id FROM users WHERE email = '\''$TEST_EMAIL'\'')"
  db_exec "DELETE FROM users WHERE email = '\''$TEST_EMAIL'\''"

  resp=$(api_post /api/v1/users "{\"email\":\"$TEST_EMAIL\",\"display_name\":\"Tenant List User\",\"password\":\"SecurePass123!\"}")
  assert_http_status "$(resp_status "$resp")" 201 "user created"
  USER_ID=$(echo "$(resp_body "$resp")" | jq -r ".data.id")

  api_post "/api/v1/users/$USER_ID/tenants" "{\"tenant_id\":\"$TENANT_ID\",\"role_in_tenant\":\"member\"}" >/dev/null 2>&1

  resp2=$(api_get "/api/v1/users/$USER_ID/tenants")
  assert_http_status "$(resp_status "$resp2")" 200 "GET /api/v1/users/{id}/tenants returns 200"

  body=$(resp_body "$resp2")
  assert_json_exists "$body" ".data" "response has data"
  TENANT_COUNT=$(echo "$body" | jq ".data | length")
  assert_eq "$TENANT_COUNT" "1" "user has 1 tenant association"

  api_delete "/api/v1/users/$USER_ID/tenants/$TENANT_ID" >/dev/null 2>&1 || true
  api_delete "/api/v1/users/$USER_ID" >/dev/null 2>&1 || true
  qa_set_token ""
'

run_all
