#!/usr/bin/env bash
# QA Auto Test: tenant/03-status-lifecycle
# Doc: docs/qa/tenant/03-status-lifecycle.md
# Scenarios: 5
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/../lib/runner.sh"

require_bin jq

_TENANT_ID=""
_TENANT_SLUG=""
_ADMIN_ID=""

_setup() {
  if [[ -n "$_TENANT_ID" ]]; then return 0; fi
  TOKEN=$(gen_default_admin_token)
  qa_set_token "$TOKEN"

  _ADMIN_ID=$(db_query "SELECT id FROM users WHERE email = 'admin@auth9.local' LIMIT 1;")
  if [[ -z "$_ADMIN_ID" ]]; then
    echo "No admin user found" >&2
    return 1
  fi

  _TENANT_SLUG="qa-status-$(date +%s)"
  local resp
  resp=$(api_post "/api/v1/tenants" \
    "{\"name\":\"QA Status Lifecycle\",\"slug\":\"${_TENANT_SLUG}\"}")
  assert_http_status "$(resp_status "$resp")" 201 "create lifecycle test tenant"
  _TENANT_ID=$(resp_body "$resp" | jq -r ".dataid")
}

scenario 1 "入口可见性 - 从租户列表进入状态编辑并设为 Inactive" '
  _setup

  resp_list=$(api_get "/api/v1/tenants?search=${_TENANT_SLUG}")
  assert_http_status "$(resp_status "$resp_list")" 200 "list tenants returns 200"
  body_list=$(resp_body "$resp_list")
  local found
  found=$(echo "$body_list" | jq "[.data[] | select(.slug == \"${_TENANT_SLUG}\")] | length")
  assert_eq "$found" "1" "test tenant found in list"

  resp_detail=$(api_get "/api/v1/tenants/${_TENANT_ID}")
  assert_http_status "$(resp_status "$resp_detail")" 200 "get tenant detail returns 200"
  assert_json_field "$(resp_body "$resp_detail")" ".status" "active" "initial status is active"

  resp_update=$(api_put "/api/v1/tenants/${_TENANT_ID}" "{\"status\":\"inactive\"}")
  assert_http_status "$(resp_status "$resp_update")" 200 "PUT status=inactive returns 200"
  assert_json_field "$(resp_body "$resp_update")" ".status" "inactive" "status updated to inactive"

  assert_db \
    "SELECT status FROM tenants WHERE id = '\''${_TENANT_ID}'\'';" \
    "inactive" \
    "DB status is inactive"
'

scenario 2 "将租户状态设为 Suspended" '
  _setup

  resp_restore=$(api_put "/api/v1/tenants/${_TENANT_ID}" "{\"status\":\"active\"}")
  assert_http_status "$(resp_status "$resp_restore")" 200 "restore to active first"

  resp=$(api_put "/api/v1/tenants/${_TENANT_ID}" "{\"status\":\"suspended\"}")
  assert_http_status "$(resp_status "$resp")" 200 "PUT status=suspended returns 200"
  assert_json_field "$(resp_body "$resp")" ".status" "suspended" "status updated to suspended"

  assert_db \
    "SELECT status FROM tenants WHERE id = '\''${_TENANT_ID}'\'';" \
    "suspended" \
    "DB status is suspended"
'

scenario 3 "恢复 Suspended 租户为 Active" '
  _setup

  db_exec "UPDATE tenants SET status = '\''suspended'\'' WHERE id = '\''${_TENANT_ID}'\'';" || true

  resp=$(api_put "/api/v1/tenants/${_TENANT_ID}" "{\"status\":\"active\"}")
  assert_http_status "$(resp_status "$resp")" 200 "PUT status=active returns 200"
  assert_json_field "$(resp_body "$resp")" ".status" "active" "status restored to active"

  assert_db \
    "SELECT status FROM tenants WHERE id = '\''${_TENANT_ID}'\'';" \
    "active" \
    "DB status is active after restore"
'

scenario 4 "Inactive 租户的 Token Exchange 行为" '
  _setup

  api_put "/api/v1/tenants/${_TENANT_ID}" "{\"status\":\"inactive\"}" >/dev/null 2>&1

  local admin_id
  admin_id=$(db_query "SELECT id FROM users WHERE email = '\''admin@auth9.local'\'' LIMIT 1;")
  db_exec "INSERT IGNORE INTO tenant_users (id, tenant_id, user_id, role_in_tenant, joined_at) VALUES (UUID(), '\''${_TENANT_ID}'\'', '\''${admin_id}'\'', '\''admin'\'', NOW());" || true

  ID_TOKEN=$(gen_identity_token "$admin_id" "admin@auth9.local")

  resp=$(api_raw POST /api/v1/auth/token-exchange \
    -H "Content-Type: application/json" \
    -d "{\"identity_token\":\"${ID_TOKEN}\",\"tenant_id\":\"${_TENANT_ID}\"}")
  local status
  status=$(resp_status "$resp")
  assert_match "$status" "^(403|400)$" "token exchange for inactive tenant returns 403 or 400"

  local body
  body=$(resp_body "$resp")
  assert_contains "$body" "not active" "error mentions tenant not active"
'

scenario 5 "租户状态对管理操作的影响" '
  _setup

  api_put "/api/v1/tenants/${_TENANT_ID}" "{\"status\":\"suspended\"}" >/dev/null 2>&1

  local admin_id
  admin_id=$(db_query "SELECT id FROM users WHERE email = '\''admin@auth9.local'\'' LIMIT 1;")
  TENANT_TOKEN=$(gen_tenant_token "$admin_id" "$_TENANT_ID")
  qa_set_token "$TENANT_TOKEN"

  resp_inv=$(api_post "/api/v1/tenants/${_TENANT_ID}/invitations" \
    "{\"email\":\"qa-invite-test@example.com\",\"role\":\"member\"}")
  local status_inv
  status_inv=$(resp_status "$resp_inv")
  assert_match "$status_inv" "^(403|400|422)$" "invitation on suspended tenant blocked"

  resp_wh=$(api_post "/api/v1/tenants/${_TENANT_ID}/webhooks" \
    "{\"url\":\"https://example.com/qa-hook\",\"events\":[\"user.created\"]}")
  local status_wh
  status_wh=$(resp_status "$resp_wh")
  assert_match "$status_wh" "^(403|400|422)$" "webhook on suspended tenant blocked"

  TOKEN=$(gen_default_admin_token)
  qa_set_token "$TOKEN"
  api_put "/api/v1/tenants/${_TENANT_ID}" "{\"status\":\"active\"}" >/dev/null 2>&1

  db_exec "DELETE FROM tenant_users WHERE tenant_id = '\''${_TENANT_ID}'\'' AND user_id != '\''${admin_id}'\'';" || true
  api_delete "/api/v1/tenants/${_TENANT_ID}" >/dev/null 2>&1 || true
  db_exec "DELETE FROM tenants WHERE slug = '\''${_TENANT_SLUG}'\'';" || true
'

run_all
