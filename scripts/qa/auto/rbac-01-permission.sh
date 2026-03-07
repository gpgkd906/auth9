#!/usr/bin/env bash
# QA Auto Test: rbac/01-permission
# Doc: docs/qa/rbac/01-permission.md
# Scenarios: 4
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/../lib/runner.sh"

require_bin jq

scenario 1 "创建权限" '
  TOKEN=$(gen_default_admin_token)
  qa_set_token "$TOKEN"

  SVC_ID=$(db_query "SELECT id FROM services LIMIT 1;" | tr -d "[:space:]")
  if [[ -z "$SVC_ID" ]]; then
    echo "No service found in DB" >&2
    return 1
  fi

  db_exec "DELETE FROM permissions WHERE service_id = '\''${SVC_ID}'\'' AND code = '\''qa:perm:read'\'';" || true

  resp=$(api_post /api/v1/permissions "{\"service_id\":\"${SVC_ID}\",\"code\":\"qa:perm:read\",\"name\":\"QA Read Permission\",\"description\":\"QA test permission for read access\"}")
  assert_http_status "$(resp_status "$resp")" 201 "POST /api/v1/permissions returns 201"

  body=$(resp_body "$resp")
  assert_json_field "$body" ".data.code" "qa:perm:read" "permission code matches"
  assert_json_field "$body" ".data.name" "QA Read Permission" "permission name matches"
  assert_json_exists "$body" ".data.id" "permission id exists"

  PERM_ID=$(echo "$body" | jq -r ".data.id")
  assert_db_not_empty "SELECT id FROM permissions WHERE id = '\''${PERM_ID}'\'' AND code = '\''qa:perm:read'\'';" "permission exists in DB"

  db_exec "DELETE FROM permissions WHERE id = '\''${PERM_ID}'\'';" || true
  qa_set_token ""
'

scenario 2 "创建重复 code 的权限" '
  TOKEN=$(gen_default_admin_token)
  qa_set_token "$TOKEN"

  SVC_ID=$(db_query "SELECT id FROM services LIMIT 1;" | tr -d "[:space:]")
  if [[ -z "$SVC_ID" ]]; then
    echo "No service found in DB" >&2
    return 1
  fi

  db_exec "DELETE FROM permissions WHERE service_id = '\''${SVC_ID}'\'' AND code = '\''qa:perm:dup'\'';" || true

  resp=$(api_post /api/v1/permissions "{\"service_id\":\"${SVC_ID}\",\"code\":\"qa:perm:dup\",\"name\":\"QA Dup Permission\"}")
  assert_http_status "$(resp_status "$resp")" 201 "first creation returns 201"

  resp2=$(api_post /api/v1/permissions "{\"service_id\":\"${SVC_ID}\",\"code\":\"qa:perm:dup\",\"name\":\"QA Dup Permission Again\"}")
  status2=$(resp_status "$resp2")
  assert_match "$status2" "^(400|409|422)$" "duplicate code rejected"

  assert_db "SELECT COUNT(*) FROM permissions WHERE service_id = '\''${SVC_ID}'\'' AND code = '\''qa:perm:dup'\'';" "1" "only one permission with that code"

  db_exec "DELETE FROM permissions WHERE service_id = '\''${SVC_ID}'\'' AND code = '\''qa:perm:dup'\'';" || true
  qa_set_token ""
'

scenario 3 "删除权限" '
  TOKEN=$(gen_default_admin_token)
  qa_set_token "$TOKEN"

  SVC_ID=$(db_query "SELECT id FROM services LIMIT 1;" | tr -d "[:space:]")
  if [[ -z "$SVC_ID" ]]; then
    echo "No service found in DB" >&2
    return 1
  fi

  db_exec "DELETE FROM permissions WHERE service_id = '\''${SVC_ID}'\'' AND code = '\''qa:perm:del'\'';" || true

  resp=$(api_post /api/v1/permissions "{\"service_id\":\"${SVC_ID}\",\"code\":\"qa:perm:del\",\"name\":\"QA Delete Permission\"}")
  assert_http_status "$(resp_status "$resp")" 201 "create permission for deletion"
  PERM_ID=$(resp_body "$resp" | jq -r ".data.id")

  ROLE_RESP=$(api_post /api/v1/roles "{\"service_id\":\"${SVC_ID}\",\"name\":\"qa-del-role-$$\"}")
  ROLE_ID=$(resp_body "$ROLE_RESP" | jq -r ".data.id")

  api_post "/api/v1/roles/${ROLE_ID}/permissions" "{\"permission_id\":\"${PERM_ID}\"}" >/dev/null 2>&1 || true

  del_resp=$(api_delete "/api/v1/permissions/${PERM_ID}")
  assert_http_status "$(resp_status "$del_resp")" 200 "DELETE /api/v1/permissions/{id} returns 200"

  assert_db "SELECT COUNT(*) FROM permissions WHERE id = '\''${PERM_ID}'\'';" "0" "permission deleted from DB"
  assert_db "SELECT COUNT(*) FROM role_permissions WHERE permission_id = '\''${PERM_ID}'\'';" "0" "role_permissions cascade cleared"

  api_delete "/api/v1/roles/${ROLE_ID}" >/dev/null 2>&1 || true
  qa_set_token ""
'

scenario 4 "权限代码格式验证" '
  TOKEN=$(gen_default_admin_token)
  qa_set_token "$TOKEN"

  SVC_ID=$(db_query "SELECT id FROM services LIMIT 1;" | tr -d "[:space:]")
  if [[ -z "$SVC_ID" ]]; then
    echo "No service found in DB" >&2
    return 1
  fi

  resp=$(api_post /api/v1/permissions "{\"service_id\":\"${SVC_ID}\",\"code\":\"report:export\",\"name\":\"QA Valid Code 1\"}")
  status=$(resp_status "$resp")
  assert_eq "$status" "201" "standard format report:export accepted"
  VALID1_ID=$(resp_body "$resp" | jq -r ".data.id")
  db_exec "DELETE FROM permissions WHERE id = '\''${VALID1_ID}'\'';" || true

  resp=$(api_post /api/v1/permissions "{\"service_id\":\"${SVC_ID}\",\"code\":\"admin:user:delete\",\"name\":\"QA Valid Code 2\"}")
  status=$(resp_status "$resp")
  assert_eq "$status" "201" "namespaced format admin:user:delete accepted"
  VALID2_ID=$(resp_body "$resp" | jq -r ".data.id")
  db_exec "DELETE FROM permissions WHERE id = '\''${VALID2_ID}'\'';" || true

  resp=$(api_post /api/v1/permissions "{\"service_id\":\"${SVC_ID}\",\"code\":\"user@read\",\"name\":\"QA Invalid Code 1\"}")
  status=$(resp_status "$resp")
  assert_match "$status" "^(400|422)$" "illegal char user@read rejected"

  resp=$(api_post /api/v1/permissions "{\"service_id\":\"${SVC_ID}\",\"code\":\"user read\",\"name\":\"QA Invalid Code 2\"}")
  status=$(resp_status "$resp")
  assert_match "$status" "^(400|422)$" "space in code user read rejected"

  qa_set_token ""
'

run_all
