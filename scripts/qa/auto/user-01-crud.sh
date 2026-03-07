#!/usr/bin/env bash
# QA Auto Test: user/01-crud
# Doc: docs/qa/user/01-crud.md
# Scenarios: 5
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/../lib/runner.sh"

require_bin jq

scenario 1 "创建用户" '
  ADMIN_ID=$(qa_get_admin_id)
  TENANT_ID=$(qa_get_tenant_id)
  TOKEN=$(gen_tenant_token "$ADMIN_ID" "$TENANT_ID")
  qa_set_token "$TOKEN"

  TEST_EMAIL="qa-u01-s1@example.com"
  db_exec "DELETE FROM tenant_users WHERE user_id IN (SELECT id FROM users WHERE email = '\''$TEST_EMAIL'\'')"
  db_exec "DELETE FROM users WHERE email = '\''$TEST_EMAIL'\''"

  resp=$(api_post /api/v1/users "{\"email\":\"$TEST_EMAIL\",\"display_name\":\"QA Create User\",\"password\":\"SecurePass123!\"}")
  assert_http_status "$(resp_status "$resp")" 201 "POST /api/v1/users returns 201"

  body=$(resp_body "$resp")
  assert_json_exists "$body" ".data.id" "response has user id"
  assert_json_field "$body" ".data.email" "$TEST_EMAIL" "response email matches"
  assert_json_field "$body" ".data.mfa_enabled" "false" "MFA default off"

  assert_db_not_empty "SELECT id FROM users WHERE email = '\''$TEST_EMAIL'\''" "user exists in DB"
  assert_db_not_empty "SELECT keycloak_id FROM users WHERE email = '\''$TEST_EMAIL'\'' AND keycloak_id IS NOT NULL AND LENGTH(keycloak_id) > 0" "keycloak_id is not empty"

  USER_ID=$(echo "$body" | jq -r ".data.id")
  api_delete "/api/v1/users/$USER_ID" >/dev/null 2>&1 || true
  qa_set_token ""
'

scenario 2 "创建重复邮箱的用户" '
  ADMIN_ID=$(qa_get_admin_id)
  TENANT_ID=$(qa_get_tenant_id)
  TOKEN=$(gen_tenant_token "$ADMIN_ID" "$TENANT_ID")
  qa_set_token "$TOKEN"

  TEST_EMAIL="qa-u01-s2@example.com"
  db_exec "DELETE FROM tenant_users WHERE user_id IN (SELECT id FROM users WHERE email = '\''$TEST_EMAIL'\'')"
  db_exec "DELETE FROM users WHERE email = '\''$TEST_EMAIL'\''"

  resp=$(api_post /api/v1/users "{\"email\":\"$TEST_EMAIL\",\"display_name\":\"First User\",\"password\":\"SecurePass123!\"}")
  assert_http_status "$(resp_status "$resp")" 201 "first user created"
  USER_ID=$(echo "$(resp_body "$resp")" | jq -r ".data.id")

  resp2=$(api_post /api/v1/users "{\"email\":\"$TEST_EMAIL\",\"display_name\":\"Duplicate User\",\"password\":\"SecurePass123!\"}")
  assert_http_status "$(resp_status "$resp2")" 409 "duplicate email returns 409"

  assert_db "SELECT COUNT(*) FROM users WHERE email = '\''$TEST_EMAIL'\''" "1" "only one user with this email"

  api_delete "/api/v1/users/$USER_ID" >/dev/null 2>&1 || true
  qa_set_token ""
'

scenario 3 "更新用户信息" '
  ADMIN_ID=$(qa_get_admin_id)
  TENANT_ID=$(qa_get_tenant_id)
  TOKEN=$(gen_tenant_token "$ADMIN_ID" "$TENANT_ID")
  qa_set_token "$TOKEN"

  TEST_EMAIL="qa-u01-s3@example.com"
  db_exec "DELETE FROM tenant_users WHERE user_id IN (SELECT id FROM users WHERE email = '\''$TEST_EMAIL'\'')"
  db_exec "DELETE FROM users WHERE email = '\''$TEST_EMAIL'\''"

  resp=$(api_post /api/v1/users "{\"email\":\"$TEST_EMAIL\",\"display_name\":\"Old Name\",\"password\":\"SecurePass123!\"}")
  assert_http_status "$(resp_status "$resp")" 201 "user created for update test"
  USER_ID=$(echo "$(resp_body "$resp")" | jq -r ".data.id")

  resp2=$(api_put "/api/v1/users/$USER_ID" "{\"display_name\":\"New Name\"}")
  assert_http_status "$(resp_status "$resp2")" 200 "PUT /api/v1/users/{id} returns 200"

  body=$(resp_body "$resp2")
  assert_json_field "$body" ".data.display_name" "New Name" "display_name updated in response"

  assert_db "SELECT display_name FROM users WHERE id = '\''$USER_ID'\''" "New Name" "display_name updated in DB"

  api_delete "/api/v1/users/$USER_ID" >/dev/null 2>&1 || true
  qa_set_token ""
'

scenario 4 "添加用户到租户" '
  ADMIN_ID=$(qa_get_admin_id)
  TENANT_ID=$(qa_get_tenant_id)
  TOKEN=$(gen_tenant_token "$ADMIN_ID" "$TENANT_ID")
  qa_set_token "$TOKEN"

  SECOND_TENANT=$(db_query "SELECT id FROM tenants WHERE id != '\''$TENANT_ID'\'' AND status = '\''active'\'' LIMIT 1;" | tr -d "[:space:]")
  if [[ -z "$SECOND_TENANT" ]]; then
    echo "No second tenant for add-to-tenant test" >&2
    return 1
  fi

  TEST_EMAIL="qa-u01-s4@example.com"
  db_exec "DELETE FROM tenant_users WHERE user_id IN (SELECT id FROM users WHERE email = '\''$TEST_EMAIL'\'')"
  db_exec "DELETE FROM users WHERE email = '\''$TEST_EMAIL'\''"

  resp=$(api_post /api/v1/users "{\"email\":\"$TEST_EMAIL\",\"display_name\":\"Tenant User\",\"password\":\"SecurePass123!\"}")
  assert_http_status "$(resp_status "$resp")" 201 "user created for tenant test"
  USER_ID=$(echo "$(resp_body "$resp")" | jq -r ".data.id")

  resp2=$(api_post "/api/v1/users/$USER_ID/tenants" "{\"tenant_id\":\"$SECOND_TENANT\",\"role_in_tenant\":\"admin\"}")
  status2=$(resp_status "$resp2")
  assert_match "$status2" "^(200|201)$" "POST /api/v1/users/{id}/tenants returns 200/201"

  assert_db_not_empty "SELECT id FROM tenant_users WHERE user_id = '\''$USER_ID'\'' AND tenant_id = '\''$SECOND_TENANT'\''" "tenant_users record exists for second tenant"

  api_delete "/api/v1/users/$USER_ID/tenants/$SECOND_TENANT" >/dev/null 2>&1 || true
  api_delete "/api/v1/users/$USER_ID" >/dev/null 2>&1 || true
  qa_set_token ""
'

scenario 5 "从租户移除用户" '
  ADMIN_ID=$(qa_get_admin_id)
  TENANT_ID=$(qa_get_tenant_id)
  TOKEN=$(gen_tenant_token "$ADMIN_ID" "$TENANT_ID")
  qa_set_token "$TOKEN"

  SECOND_TENANT=$(db_query "SELECT id FROM tenants WHERE id != '\''$TENANT_ID'\'' AND status = '\''active'\'' LIMIT 1;" | tr -d "[:space:]")

  TEST_EMAIL="qa-u01-s5@example.com"
  db_exec "DELETE FROM tenant_users WHERE user_id IN (SELECT id FROM users WHERE email = '\''$TEST_EMAIL'\'')"
  db_exec "DELETE FROM users WHERE email = '\''$TEST_EMAIL'\''"

  resp=$(api_post /api/v1/users "{\"email\":\"$TEST_EMAIL\",\"display_name\":\"Remove User\",\"password\":\"SecurePass123!\"}")
  assert_http_status "$(resp_status "$resp")" 201 "user created"
  USER_ID=$(echo "$(resp_body "$resp")" | jq -r ".data.id")

  resp2=$(api_post "/api/v1/users/$USER_ID/tenants" "{\"tenant_id\":\"$SECOND_TENANT\",\"role_in_tenant\":\"member\"}")
  status2=$(resp_status "$resp2")
  assert_match "$status2" "^(200|201)$" "user added to second tenant"

  resp3=$(api_delete "/api/v1/users/$USER_ID/tenants/$SECOND_TENANT")
  del_status=$(resp_status "$resp3")
  assert_match "$del_status" "^(200|204)$" "DELETE /api/v1/users/{id}/tenants/{tenant_id} returns 200/204"

  assert_db "SELECT COUNT(*) FROM tenant_users WHERE user_id = '\''$USER_ID'\'' AND tenant_id = '\''$SECOND_TENANT'\''" "0" "tenant_users record removed"

  api_delete "/api/v1/users/$USER_ID" >/dev/null 2>&1 || true
  qa_set_token ""
'

run_all
