#!/usr/bin/env bash
# QA Auto Test: rbac/04-advanced
# Doc: docs/qa/rbac/04-advanced.md
# Scenarios: 3
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/../lib/runner.sh"

require_bin jq

scenario 1 "角色层次视图" '
  qa_setup_service_with_tenant
  SVC_ID="$_QA_SVC_ID"
  TOKEN=$(gen_token_for_tenant "$_QA_SVC_TENANT")
  qa_set_token "$TOKEN"
  if [[ -z "$SVC_ID" ]]; then
    echo "No service found in DB" >&2
    return 1
  fi

  admin_resp=$(api_post /api/v1/roles "{\"service_id\":\"${SVC_ID}\",\"name\":\"qa-hier-admin-$$\"}")
  assert_match "$(resp_status "$admin_resp")" "^(200|201)$" "create root Admin role"
  ADMIN_ID=$(resp_body "$admin_resp" | jq -r ".data.id")

  editor_resp=$(api_post /api/v1/roles "{\"service_id\":\"${SVC_ID}\",\"name\":\"qa-hier-editor-$$\",\"parent_role_id\":\"${ADMIN_ID}\"}")
  assert_match "$(resp_status "$editor_resp")" "^(200|201)$" "create Editor (child of Admin)"
  EDITOR_ID=$(resp_body "$editor_resp" | jq -r ".data.id")

  viewer_resp=$(api_post /api/v1/roles "{\"service_id\":\"${SVC_ID}\",\"name\":\"qa-hier-viewer-$$\",\"parent_role_id\":\"${EDITOR_ID}\"}")
  assert_match "$(resp_status "$viewer_resp")" "^(200|201)$" "create Viewer (child of Editor)"
  VIEWER_ID=$(resp_body "$viewer_resp" | jq -r ".data.id")

  mod_resp=$(api_post /api/v1/roles "{\"service_id\":\"${SVC_ID}\",\"name\":\"qa-hier-mod-$$\",\"parent_role_id\":\"${ADMIN_ID}\"}")
  assert_match "$(resp_status "$mod_resp")" "^(200|201)$" "create Moderator (child of Admin)"
  MOD_ID=$(resp_body "$mod_resp" | jq -r ".data.id")

  list_resp=$(api_get "/api/v1/services/${SVC_ID}/roles")
  assert_http_status "$(resp_status "$list_resp")" 200 "list roles returns 200"
  list_body=$(resp_body "$list_resp")
  assert_contains "$list_body" "qa-hier-admin" "list contains Admin"
  assert_contains "$list_body" "qa-hier-editor" "list contains Editor"
  assert_contains "$list_body" "qa-hier-viewer" "list contains Viewer"
  assert_contains "$list_body" "qa-hier-mod" "list contains Moderator"

  editor_detail=$(api_get "/api/v1/roles/${EDITOR_ID}")
  editor_body=$(resp_body "$editor_detail")
  assert_json_field "$editor_body" ".data.role.parent_role_id" "$ADMIN_ID" "Editor parent is Admin"

  viewer_detail=$(api_get "/api/v1/roles/${VIEWER_ID}")
  viewer_body=$(resp_body "$viewer_detail")
  assert_json_field "$viewer_body" ".data.role.parent_role_id" "$EDITOR_ID" "Viewer parent is Editor"

  assert_db_not_empty "SELECT r.name FROM roles r WHERE r.id = '\''${EDITOR_ID}'\'' AND r.parent_role_id = '\''${ADMIN_ID}'\'';" "DB confirms Editor->Admin parent"
  assert_db_not_empty "SELECT r.name FROM roles r WHERE r.id = '\''${VIEWER_ID}'\'' AND r.parent_role_id = '\''${EDITOR_ID}'\'';" "DB confirms Viewer->Editor parent"
  assert_db_not_empty "SELECT r.name FROM roles r WHERE r.id = '\''${MOD_ID}'\'' AND r.parent_role_id = '\''${ADMIN_ID}'\'';" "DB confirms Moderator->Admin parent"

  api_delete "/api/v1/roles/${VIEWER_ID}" >/dev/null 2>&1 || true
  api_delete "/api/v1/roles/${MOD_ID}" >/dev/null 2>&1 || true
  api_delete "/api/v1/roles/${EDITOR_ID}" >/dev/null 2>&1 || true
  api_delete "/api/v1/roles/${ADMIN_ID}" >/dev/null 2>&1 || true
  qa_set_token ""
'

scenario 2 "循环继承检测" '
  qa_setup_service_with_tenant
  SVC_ID="$_QA_SVC_ID"
  TOKEN=$(gen_token_for_tenant "$_QA_SVC_TENANT")
  qa_set_token "$TOKEN"

  role_a_resp=$(api_post /api/v1/roles "{\"service_id\":\"${SVC_ID}\",\"name\":\"qa-cycle-a-$$\"}")
  ROLE_A=$(resp_body "$role_a_resp" | jq -r ".data.id")

  role_b_resp=$(api_post /api/v1/roles "{\"service_id\":\"${SVC_ID}\",\"name\":\"qa-cycle-b-$$\",\"parent_role_id\":\"${ROLE_A}\"}")
  assert_match "$(resp_status "$resp")" "^(200|201)$" "create B (child of A)"
  ROLE_B=$(resp_body "$role_b_resp" | jq -r ".data.id")

  cycle_resp=$(api_put "/api/v1/roles/${ROLE_A}" "{\"parent_role_id\":\"${ROLE_B}\"}")
  cycle_status=$(resp_status "$cycle_resp")
  assert_match "$cycle_status" "^(400|409|422)$" "setting A parent to B rejected (cycle)"

  actual_parent=$(db_query "SELECT COALESCE(parent_role_id, '\''NULL'\'') FROM roles WHERE id = '\''${ROLE_A}'\'';" | tr -d "[:space:]")
  assert_eq "$actual_parent" "NULL" "Role A parent unchanged after cycle rejection"

  api_delete "/api/v1/roles/${ROLE_B}" >/dev/null 2>&1 || true
  api_delete "/api/v1/roles/${ROLE_A}" >/dev/null 2>&1 || true
  qa_set_token ""
'

scenario 3 "跨服务权限分配验证" '
  TOKEN=$(gen_default_admin_token)
  qa_set_token "$TOKEN"

  SVC_IDS=$(db_query "SELECT id FROM services LIMIT 2;")
  SVC_A=$(echo "$SVC_IDS" | head -1 | tr -d "[:space:]")
  SVC_B=$(echo "$SVC_IDS" | tail -1 | tr -d "[:space:]")

  if [[ -z "$SVC_A" ]] || [[ -z "$SVC_B" ]] || [[ "$SVC_A" == "$SVC_B" ]]; then
    perm_resp=$(api_post /api/v1/permissions "{\"service_id\":\"${SVC_A}\",\"code\":\"qa:cross:perm\",\"name\":\"QA Cross Perm\"}")
    PERM_A=$(resp_body "$perm_resp" | jq -r ".data.id")
    role_resp=$(api_post /api/v1/roles "{\"service_id\":\"${SVC_A}\",\"name\":\"qa-cross-role-$$\"}")
    ROLE_A=$(resp_body "$role_resp" | jq -r ".data.id")

    assign_resp=$(api_post "/api/v1/roles/${ROLE_A}/permissions" "{\"permission_id\":\"${PERM_A}\"}")
    assert_http_status "$(resp_status "$assign_resp")" 200 "same-service assignment succeeds"

    db_exec "DELETE FROM role_permissions WHERE role_id = '\''${ROLE_A}'\'';" || true
    api_delete "/api/v1/roles/${ROLE_A}" >/dev/null 2>&1 || true
    api_delete "/api/v1/permissions/${PERM_A}" >/dev/null 2>&1 || true
    qa_set_token ""
    return 0
  fi

  perm_resp=$(api_post /api/v1/permissions "{\"service_id\":\"${SVC_A}\",\"code\":\"qa:cross:a\",\"name\":\"QA Cross Perm A\"}")
  PERM_A=$(resp_body "$perm_resp" | jq -r ".data.id")

  role_resp=$(api_post /api/v1/roles "{\"service_id\":\"${SVC_B}\",\"name\":\"qa-cross-b-$$\"}")
  ROLE_B=$(resp_body "$role_resp" | jq -r ".data.id")

  cross_resp=$(api_post "/api/v1/roles/${ROLE_B}/permissions" "{\"permission_id\":\"${PERM_A}\"}")
  cross_status=$(resp_status "$cross_resp")
  assert_eq "$cross_status" "400" "cross-service permission assignment returns 400"

  cross_body=$(resp_body "$cross_resp")
  assert_contains "$cross_body" "annot assign" "error message mentions cannot assign"

  assert_db "SELECT COUNT(*) FROM role_permissions rp JOIN roles r ON r.id = rp.role_id JOIN permissions p ON p.id = rp.permission_id WHERE r.service_id != p.service_id AND rp.role_id = '\''${ROLE_B}'\'' AND rp.permission_id = '\''${PERM_A}'\'';" "0" "no cross-service record in DB"

  api_delete "/api/v1/roles/${ROLE_B}" >/dev/null 2>&1 || true
  api_delete "/api/v1/permissions/${PERM_A}" >/dev/null 2>&1 || true
  qa_set_token ""
'

run_all
