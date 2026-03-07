#!/usr/bin/env bash
# QA Auto Test: user/03-validation
# Doc: docs/qa/user/03-validation.md
# Scenarios: 3
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/../lib/runner.sh"

require_bin jq

scenario 1 "用户重复加入同一租户" '
  ADMIN_ID=$(qa_get_admin_id)
  TENANT_ID=$(qa_get_tenant_id)
  TOKEN=$(gen_tenant_token "$ADMIN_ID" "$TENANT_ID")
  qa_set_token "$TOKEN"

  TEST_EMAIL="qa-u03-s1@example.com"
  db_exec "DELETE FROM tenant_users WHERE user_id IN (SELECT id FROM users WHERE email = '\''$TEST_EMAIL'\'')"
  db_exec "DELETE FROM users WHERE email = '\''$TEST_EMAIL'\''"

  resp=$(api_post /api/v1/users "{\"email\":\"$TEST_EMAIL\",\"display_name\":\"Dup Tenant User\",\"password\":\"SecurePass123!\"}")
  assert_http_status "$(resp_status "$resp")" 201 "user created"
  USER_ID=$(echo "$(resp_body "$resp")" | jq -r ".data.id")

  resp2=$(api_post "/api/v1/users/$USER_ID/tenants" "{\"tenant_id\":\"$TENANT_ID\",\"role_in_tenant\":\"member\"}")
  assert_http_status "$(resp_status "$resp2")" 409 "add to same tenant returns 409 (auto-added on create)"

  assert_db "SELECT COUNT(*) FROM tenant_users WHERE user_id = '\''$USER_ID'\'' AND tenant_id = '\''$TENANT_ID'\''" "1" "still only one tenant_users record"

  api_delete "/api/v1/users/$USER_ID" >/dev/null 2>&1 || true
  qa_set_token ""
'

scenario 2 "修改用户在租户中的角色" '
  ADMIN_ID=$(qa_get_admin_id)
  TENANT_ID=$(qa_get_tenant_id)
  TOKEN=$(gen_tenant_token "$ADMIN_ID" "$TENANT_ID")
  qa_set_token "$TOKEN"

  TEST_EMAIL="qa-u03-s2@example.com"
  db_exec "DELETE FROM user_tenant_roles WHERE tenant_user_id IN (SELECT tu.id FROM tenant_users tu JOIN users u ON u.id = tu.user_id WHERE u.email = '\''$TEST_EMAIL'\'')"
  db_exec "DELETE FROM tenant_users WHERE user_id IN (SELECT id FROM users WHERE email = '\''$TEST_EMAIL'\'')"
  db_exec "DELETE FROM users WHERE email = '\''$TEST_EMAIL'\''"

  resp=$(api_post /api/v1/users "{\"email\":\"$TEST_EMAIL\",\"display_name\":\"Role User\",\"password\":\"SecurePass123!\"}")
  assert_http_status "$(resp_status "$resp")" 201 "user created"
  USER_ID=$(echo "$(resp_body "$resp")" | jq -r ".data.id")

  assert_db "SELECT role_in_tenant FROM tenant_users WHERE user_id = '\''$USER_ID'\'' AND tenant_id = '\''$TENANT_ID'\''" "member" "auto-added as member"

  resp3=$(api_put "/api/v1/users/$USER_ID/tenants/$TENANT_ID" "{\"role_in_tenant\":\"admin\"}")
  assert_http_status "$(resp_status "$resp3")" 200 "PUT /api/v1/users/{id}/tenants/{tenant_id} returns 200"

  body=$(resp_body "$resp3")
  assert_json_field "$body" ".data.role_in_tenant" "admin" "role updated to admin in response"

  assert_db "SELECT role_in_tenant FROM tenant_users WHERE user_id = '\''$USER_ID'\'' AND tenant_id = '\''$TENANT_ID'\''" "admin" "role updated to admin in DB"

  api_delete "/api/v1/users/$USER_ID" >/dev/null 2>&1 || true
  qa_set_token ""
'

scenario 3 "邮箱格式验证" '
  ADMIN_ID=$(qa_get_admin_id)
  TENANT_ID=$(qa_get_tenant_id)
  TOKEN=$(gen_tenant_token "$ADMIN_ID" "$TENANT_ID")
  qa_set_token "$TOKEN"

  resp=$(api_post /api/v1/users "{\"email\":\"invalidemail\",\"password\":\"SecurePass123!\"}")
  STATUS=$(resp_status "$resp")
  assert_match "$STATUS" "^4[0-9][0-9]$" "no-@ email rejected (status $STATUS)"

  resp=$(api_post /api/v1/users "{\"email\":\"test@\",\"password\":\"SecurePass123!\"}")
  STATUS=$(resp_status "$resp")
  assert_match "$STATUS" "^4[0-9][0-9]$" "no-domain email rejected (status $STATUS)"

  resp=$(api_post /api/v1/users "{\"email\":\"@example.com\",\"password\":\"SecurePass123!\"}")
  STATUS=$(resp_status "$resp")
  assert_match "$STATUS" "^4[0-9][0-9]$" "no-user email rejected (status $STATUS)"

  resp=$(api_post /api/v1/users "{\"email\":\"test<script>@example.com\",\"password\":\"SecurePass123!\"}")
  STATUS=$(resp_status "$resp")
  assert_match "$STATUS" "^4[0-9][0-9]$" "special-char email rejected (status $STATUS)"

  assert_db "SELECT COUNT(*) FROM users WHERE email IN ('\''invalidemail'\'', '\''test@'\'', '\''@example.com'\'', '\''test<script>@example.com'\'')" "0" "no invalid users created in DB"

  qa_set_token ""
'

run_all
