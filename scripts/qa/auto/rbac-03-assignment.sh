#!/usr/bin/env bash
# QA Auto Test: rbac/03-assignment
# Doc: docs/qa/rbac/03-assignment.md
# Scenarios: 5
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/../lib/runner.sh"

require_bin jq

scenario 1 "为角色分配权限" '
  TOKEN=$(gen_default_admin_token)
  qa_set_token "$TOKEN"

  SVC_ID=$(db_query "SELECT id FROM services LIMIT 1;" | tr -d "[:space:]")
  if [[ -z "$SVC_ID" ]]; then
    echo "No service found in DB" >&2
    return 1
  fi

  db_exec "DELETE FROM permissions WHERE service_id = '\''${SVC_ID}'\'' AND code = '\''qa:assign:read'\'';" || true

  perm_resp=$(api_post /api/v1/permissions "{\"service_id\":\"${SVC_ID}\",\"code\":\"qa:assign:read\",\"name\":\"QA Assign Read\"}")
  assert_http_status "$(resp_status "$perm_resp")" 201 "create test permission"
  PERM_ID=$(resp_body "$perm_resp" | jq -r ".data.id")

  role_resp=$(api_post /api/v1/roles "{\"service_id\":\"${SVC_ID}\",\"name\":\"qa-assign-role-$$\"}")
  assert_http_status "$(resp_status "$role_resp")" 201 "create test role"
  ROLE_ID=$(resp_body "$role_resp" | jq -r ".data.id")

  assign_resp=$(api_post "/api/v1/roles/${ROLE_ID}/permissions" "{\"permission_id\":\"${PERM_ID}\"}")
  assert_http_status "$(resp_status "$assign_resp")" 200 "assign permission to role returns 200"

  assert_db_not_empty "SELECT role_id FROM role_permissions WHERE role_id = '\''${ROLE_ID}'\'' AND permission_id = '\''${PERM_ID}'\'';" "role_permissions record exists"

  role_detail=$(api_get "/api/v1/roles/${ROLE_ID}")
  body=$(resp_body "$role_detail")
  assert_contains "$body" "$PERM_ID" "role detail contains assigned permission"

  db_exec "DELETE FROM role_permissions WHERE role_id = '\''${ROLE_ID}'\'';" || true
  api_delete "/api/v1/roles/${ROLE_ID}" >/dev/null 2>&1 || true
  api_delete "/api/v1/permissions/${PERM_ID}" >/dev/null 2>&1 || true
  qa_set_token ""
'

scenario 2 "从角色移除权限" '
  TOKEN=$(gen_default_admin_token)
  qa_set_token "$TOKEN"

  SVC_ID=$(db_query "SELECT id FROM services LIMIT 1;" | tr -d "[:space:]")

  perm_resp=$(api_post /api/v1/permissions "{\"service_id\":\"${SVC_ID}\",\"code\":\"qa:remove:perm\",\"name\":\"QA Remove Perm\"}")
  PERM_ID=$(resp_body "$perm_resp" | jq -r ".data.id")
  role_resp=$(api_post /api/v1/roles "{\"service_id\":\"${SVC_ID}\",\"name\":\"qa-remove-role-$$\"}")
  ROLE_ID=$(resp_body "$role_resp" | jq -r ".data.id")

  api_post "/api/v1/roles/${ROLE_ID}/permissions" "{\"permission_id\":\"${PERM_ID}\"}" >/dev/null

  del_resp=$(api_delete "/api/v1/roles/${ROLE_ID}/permissions/${PERM_ID}")
  assert_http_status "$(resp_status "$del_resp")" 200 "remove permission from role returns 200"

  assert_db "SELECT COUNT(*) FROM role_permissions WHERE role_id = '\''${ROLE_ID}'\'' AND permission_id = '\''${PERM_ID}'\'';" "0" "role_permissions record removed"

  api_delete "/api/v1/roles/${ROLE_ID}" >/dev/null 2>&1 || true
  api_delete "/api/v1/permissions/${PERM_ID}" >/dev/null 2>&1 || true
  qa_set_token ""
'

scenario 3 "为用户分配角色" '
  TOKEN=$(gen_default_admin_token)
  qa_set_token "$TOKEN"

  SVC_ID=$(db_query "SELECT id FROM services LIMIT 1;" | tr -d "[:space:]")
  TENANT_ID=$(db_query "SELECT tenant_id FROM services WHERE id = '\''${SVC_ID}'\'' LIMIT 1;" | tr -d "[:space:]")
  if [[ -z "$TENANT_ID" ]] || [[ "$TENANT_ID" == "NULL" ]]; then
    TENANT_ID=$(db_query "SELECT id FROM tenants LIMIT 1;" | tr -d "[:space:]")
  fi
  USER_ID=$(db_query "SELECT user_id FROM tenant_users WHERE tenant_id = '\''${TENANT_ID}'\'' LIMIT 1;" | tr -d "[:space:]")
  if [[ -z "$USER_ID" ]]; then
    echo "No user in tenant" >&2
    return 1
  fi

  role_resp=$(api_post /api/v1/roles "{\"service_id\":\"${SVC_ID}\",\"name\":\"qa-user-role-$$\"}")
  assert_http_status "$(resp_status "$role_resp")" 201 "create role for user assignment"
  ROLE_ID=$(resp_body "$role_resp" | jq -r ".data.id")

  assign_resp=$(api_post /api/v1/rbac/assign "{\"user_id\":\"${USER_ID}\",\"tenant_id\":\"${TENANT_ID}\",\"role_ids\":[\"${ROLE_ID}\"]}")
  assert_http_status "$(resp_status "$assign_resp")" 200 "assign role to user returns 200"

  assert_db_not_empty "SELECT utr.id FROM user_tenant_roles utr JOIN tenant_users tu ON tu.id = utr.tenant_user_id WHERE tu.user_id = '\''${USER_ID}'\'' AND tu.tenant_id = '\''${TENANT_ID}'\'' AND utr.role_id = '\''${ROLE_ID}'\'';" "user_tenant_roles record exists"

  roles_resp=$(api_get "/api/v1/users/${USER_ID}/tenants/${TENANT_ID}/roles")
  assert_http_status "$(resp_status "$roles_resp")" 200 "get user roles returns 200"
  roles_body=$(resp_body "$roles_resp")
  assert_contains "$roles_body" "$ROLE_ID" "user roles contain assigned role"

  api_delete "/api/v1/users/${USER_ID}/tenants/${TENANT_ID}/roles/${ROLE_ID}" >/dev/null 2>&1 || true
  api_delete "/api/v1/roles/${ROLE_ID}" >/dev/null 2>&1 || true
  qa_set_token ""
'

scenario 4 "移除用户的角色" '
  TOKEN=$(gen_default_admin_token)
  qa_set_token "$TOKEN"

  SVC_ID=$(db_query "SELECT id FROM services LIMIT 1;" | tr -d "[:space:]")
  TENANT_ID=$(db_query "SELECT tenant_id FROM services WHERE id = '\''${SVC_ID}'\'' LIMIT 1;" | tr -d "[:space:]")
  if [[ -z "$TENANT_ID" ]] || [[ "$TENANT_ID" == "NULL" ]]; then
    TENANT_ID=$(db_query "SELECT id FROM tenants LIMIT 1;" | tr -d "[:space:]")
  fi
  USER_ID=$(db_query "SELECT user_id FROM tenant_users WHERE tenant_id = '\''${TENANT_ID}'\'' LIMIT 1;" | tr -d "[:space:]")

  role_resp=$(api_post /api/v1/roles "{\"service_id\":\"${SVC_ID}\",\"name\":\"qa-unassign-role-$$\"}")
  ROLE_ID=$(resp_body "$role_resp" | jq -r ".data.id")

  api_post /api/v1/rbac/assign "{\"user_id\":\"${USER_ID}\",\"tenant_id\":\"${TENANT_ID}\",\"role_ids\":[\"${ROLE_ID}\"]}" >/dev/null

  del_resp=$(api_delete "/api/v1/users/${USER_ID}/tenants/${TENANT_ID}/roles/${ROLE_ID}")
  assert_http_status "$(resp_status "$del_resp")" 200 "unassign role from user returns 200"

  assert_db "SELECT COUNT(*) FROM user_tenant_roles utr JOIN tenant_users tu ON tu.id = utr.tenant_user_id WHERE tu.user_id = '\''${USER_ID}'\'' AND tu.tenant_id = '\''${TENANT_ID}'\'' AND utr.role_id = '\''${ROLE_ID}'\'';" "0" "user_tenant_roles record removed"

  api_delete "/api/v1/roles/${ROLE_ID}" >/dev/null 2>&1 || true
  qa_set_token ""
'

scenario 5 "查询用户的有效权限（含继承）" '
  TOKEN=$(gen_default_admin_token)
  qa_set_token "$TOKEN"

  SVC_ID=$(db_query "SELECT id FROM services LIMIT 1;" | tr -d "[:space:]")
  TENANT_ID=$(db_query "SELECT tenant_id FROM services WHERE id = '\''${SVC_ID}'\'' LIMIT 1;" | tr -d "[:space:]")
  if [[ -z "$TENANT_ID" ]] || [[ "$TENANT_ID" == "NULL" ]]; then
    TENANT_ID=$(db_query "SELECT id FROM tenants LIMIT 1;" | tr -d "[:space:]")
  fi
  USER_ID=$(db_query "SELECT user_id FROM tenant_users WHERE tenant_id = '\''${TENANT_ID}'\'' LIMIT 1;" | tr -d "[:space:]")

  perm1_resp=$(api_post /api/v1/permissions "{\"service_id\":\"${SVC_ID}\",\"code\":\"qa:inherit:read\",\"name\":\"QA Inherit Read\"}")
  PERM1_ID=$(resp_body "$perm1_resp" | jq -r ".data.id")

  perm2_resp=$(api_post /api/v1/permissions "{\"service_id\":\"${SVC_ID}\",\"code\":\"qa:inherit:write\",\"name\":\"QA Inherit Write\"}")
  PERM2_ID=$(resp_body "$perm2_resp" | jq -r ".data.id")

  viewer_resp=$(api_post /api/v1/roles "{\"service_id\":\"${SVC_ID}\",\"name\":\"qa-viewer-$$\"}")
  VIEWER_ID=$(resp_body "$viewer_resp" | jq -r ".data.id")

  editor_resp=$(api_post /api/v1/roles "{\"service_id\":\"${SVC_ID}\",\"name\":\"qa-editor-$$\",\"parent_role_id\":\"${VIEWER_ID}\"}")
  EDITOR_ID=$(resp_body "$editor_resp" | jq -r ".data.id")

  api_post "/api/v1/roles/${VIEWER_ID}/permissions" "{\"permission_id\":\"${PERM1_ID}\"}" >/dev/null
  api_post "/api/v1/roles/${EDITOR_ID}/permissions" "{\"permission_id\":\"${PERM2_ID}\"}" >/dev/null

  api_post /api/v1/rbac/assign "{\"user_id\":\"${USER_ID}\",\"tenant_id\":\"${TENANT_ID}\",\"role_ids\":[\"${EDITOR_ID}\"]}" >/dev/null

  roles_resp=$(api_get "/api/v1/users/${USER_ID}/tenants/${TENANT_ID}/roles")
  assert_http_status "$(resp_status "$roles_resp")" 200 "get user effective roles returns 200"
  roles_body=$(resp_body "$roles_resp")
  assert_contains "$roles_body" "qa:inherit:read" "effective permissions include inherited qa:inherit:read"
  assert_contains "$roles_body" "qa:inherit:write" "effective permissions include own qa:inherit:write"

  api_delete "/api/v1/users/${USER_ID}/tenants/${TENANT_ID}/roles/${EDITOR_ID}" >/dev/null 2>&1 || true
  db_exec "DELETE FROM role_permissions WHERE role_id IN ('\''${VIEWER_ID}'\'', '\''${EDITOR_ID}'\'');" || true
  api_delete "/api/v1/roles/${EDITOR_ID}" >/dev/null 2>&1 || true
  api_delete "/api/v1/roles/${VIEWER_ID}" >/dev/null 2>&1 || true
  api_delete "/api/v1/permissions/${PERM1_ID}" >/dev/null 2>&1 || true
  api_delete "/api/v1/permissions/${PERM2_ID}" >/dev/null 2>&1 || true
  qa_set_token ""
'

run_all
