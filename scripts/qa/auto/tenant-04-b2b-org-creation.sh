#!/usr/bin/env bash
# QA Auto Test: tenant/04-b2b-org-creation
# Doc: docs/qa/tenant/04-b2b-org-creation.md
# Scenarios: 5
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/../lib/runner.sh"

require_bin jq

_ADMIN_ID=""
_ID_TOKEN_MATCH=""
_ID_TOKEN_MISMATCH=""
_ORG_SLUG_1=""
_ORG_SLUG_2=""
_ORG_ID_1=""
_ORG_ID_2=""

_setup() {
  if [[ -n "$_ADMIN_ID" ]]; then return 0; fi
  _ADMIN_ID=$(db_query "SELECT id FROM users WHERE email = 'admin@auth9.local' LIMIT 1;")
  if [[ -z "$_ADMIN_ID" ]]; then
    echo "No admin user found" >&2
    return 1
  fi
  _ID_TOKEN_MATCH=$(gen_identity_token "$_ADMIN_ID" "admin@auth9.local")
  _ID_TOKEN_MISMATCH=$(gen_identity_token "$_ADMIN_ID" "admin@gmail.com")

  local ts
  ts=$(date +%s)
  _ORG_SLUG_1="qa-org-match-${ts}"
  _ORG_SLUG_2="qa-org-pending-${ts}"
}

_cleanup() {
  TOKEN=$(gen_default_admin_token)
  qa_set_token "$TOKEN"
  if [[ -n "$_ORG_ID_1" ]]; then
    db_exec "DELETE FROM tenant_users WHERE tenant_id = '\''${_ORG_ID_1}'\'';" || true
    api_delete "/api/v1/tenants/${_ORG_ID_1}" >/dev/null 2>&1 || true
    db_exec "DELETE FROM tenants WHERE id = '\''${_ORG_ID_1}'\'';" || true
  fi
  if [[ -n "$_ORG_ID_2" ]]; then
    db_exec "DELETE FROM tenant_users WHERE tenant_id = '\''${_ORG_ID_2}'\'';" || true
    api_delete "/api/v1/tenants/${_ORG_ID_2}" >/dev/null 2>&1 || true
    db_exec "DELETE FROM tenants WHERE id = '\''${_ORG_ID_2}'\'';" || true
  fi
  db_exec "DELETE FROM tenants WHERE slug LIKE '\''qa-org-%'\'';" || true
}

scenario 1 "域名匹配 — 创建组织自动 Active" '
  _setup
  qa_set_token "$_ID_TOKEN_MATCH"

  resp=$(api_post "/api/v1/organizations" \
    "{\"name\":\"QA Acme Corp\",\"slug\":\"${_ORG_SLUG_1}\",\"domain\":\"auth9.local\"}")
  assert_http_status "$(resp_status "$resp")" 201 "POST /api/v1/organizations returns 201"

  body=$(resp_body "$resp")
  _ORG_ID_1=$(echo "$body" | jq -r ".data.id")
  assert_json_exists "$body" ".data.id // .id" "response has id"
  assert_json_field "$body" ".data.status // .status" "active" "domain-match org status is active"

  assert_db \
    "SELECT status FROM tenants WHERE slug = '\''${_ORG_SLUG_1}'\'';" \
    "active" \
    "DB status is active for domain-matched org"

  assert_db_not_empty \
    "SELECT tu.role_in_tenant FROM tenant_users tu JOIN tenants t ON t.id = tu.tenant_id WHERE t.slug = '\''${_ORG_SLUG_1}'\'' AND tu.role_in_tenant = '\''owner'\'';" \
    "creator is owner in tenant_users"
'

scenario 2 "域名不匹配 — 创建组织为 Pending 状态" '
  _setup
  qa_set_token "$_ID_TOKEN_MISMATCH"

  resp=$(api_post "/api/v1/organizations" \
    "{\"name\":\"QA Acme Pending\",\"slug\":\"${_ORG_SLUG_2}\",\"domain\":\"acme.com\"}")
  assert_http_status "$(resp_status "$resp")" 201 "POST org with mismatched domain returns 201"

  body=$(resp_body "$resp")
  _ORG_ID_2=$(echo "$body" | jq -r ".data.id")
  assert_json_field "$body" ".data.status // .status" "pending" "domain-mismatch org status is pending"

  assert_db \
    "SELECT status FROM tenants WHERE slug = '\''${_ORG_SLUG_2}'\'';" \
    "pending" \
    "DB status is pending for domain-mismatched org"

  assert_db_not_empty \
    "SELECT tu.role_in_tenant FROM tenant_users tu JOIN tenants t ON t.id = tu.tenant_id WHERE t.slug = '\''${_ORG_SLUG_2}'\'' AND tu.role_in_tenant = '\''owner'\'';" \
    "creator is owner even with pending status"
'

scenario 3 "Slug 重复 — 创建被拒绝" '
  _setup
  qa_set_token "$_ID_TOKEN_MATCH"

  if [[ -z "$_ORG_SLUG_1" ]]; then
    echo "Requires org from scenario 1" >&2
    return 1
  fi

  local count_before
  count_before=$(db_query "SELECT COUNT(*) FROM tenants WHERE slug = '\''${_ORG_SLUG_1}'\'';" | tr -d '[:space:]')

  resp=$(api_post "/api/v1/organizations" \
    "{\"name\":\"Another QA Acme\",\"slug\":\"${_ORG_SLUG_1}\",\"domain\":\"another.com\"}")
  local status
  status=$(resp_status "$resp")
  assert_match "$status" "^(409|400|422)$" "duplicate slug rejected with 409/400/422"

  local count_after
  count_after=$(db_query "SELECT COUNT(*) FROM tenants WHERE slug = '\''${_ORG_SLUG_1}'\'';" | tr -d '[:space:]')
  assert_eq "$count_after" "$count_before" "no duplicate tenant created"
'

scenario 4 "域名格式验证" '
  _setup
  qa_set_token "$_ID_TOKEN_MATCH"

  resp_empty=$(api_post "/api/v1/organizations" \
    "{\"name\":\"QA Test Empty\",\"slug\":\"qa-empty-domain\",\"domain\":\"\"}")
  local status_empty
  status_empty=$(resp_status "$resp_empty")
  assert_match "$status_empty" "^(400|422)$" "empty domain rejected"

  resp_invalid=$(api_post "/api/v1/organizations" \
    "{\"name\":\"QA Test Invalid\",\"slug\":\"qa-bad-domain\",\"domain\":\"not a domain!\"}")
  local status_invalid
  status_invalid=$(resp_status "$resp_invalid")
  assert_match "$status_invalid" "^(400|422)$" "invalid domain format rejected"

  resp_proto=$(api_post "/api/v1/organizations" \
    "{\"name\":\"QA Test Proto\",\"slug\":\"qa-proto-domain\",\"domain\":\"https://acme.com\"}")
  local status_proto
  status_proto=$(resp_status "$resp_proto")
  assert_match "$status_proto" "^(400|422)$" "domain with protocol rejected"

  assert_db \
    "SELECT COUNT(*) FROM tenants WHERE slug IN ('\''qa-empty-domain'\'', '\''qa-bad-domain'\'', '\''qa-proto-domain'\'');" \
    "0" \
    "no tenants created with invalid domains"
'

scenario 5 "GET /api/v1/users/me/tenants — 获取当前用户的租户列表" '
  _setup
  qa_set_token "$_ID_TOKEN_MATCH"

  resp=$(api_get "/api/v1/users/me/tenants")
  assert_http_status "$(resp_status "$resp")" 200 "GET /api/v1/users/me/tenants returns 200"

  body=$(resp_body "$resp")
  local count
  count=$(echo "$body" | jq ".data | length // length")
  assert_match "$count" "^[1-9][0-9]*$" "user has at least 1 tenant membership"

  local first
  first=$(echo "$body" | jq ".data[0] // .[0]")
  assert_json_exists "$first" ".tenant_id" "entry has tenant_id"
  assert_json_exists "$first" ".role_in_tenant" "entry has role_in_tenant"
  assert_json_exists "$first" ".tenant" "entry has nested tenant object"
  assert_json_exists "$first" ".tenant.name" "nested tenant has name"
  assert_json_exists "$first" ".tenant.slug" "nested tenant has slug"
  assert_json_exists "$first" ".tenant.status" "nested tenant has status"

  _cleanup
'

run_all
