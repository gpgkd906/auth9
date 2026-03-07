#!/usr/bin/env bash
# QA Auto Test: tenant/02-list-settings
# Doc: docs/qa/tenant/02-list-settings.md
# Scenarios: 5
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/../lib/runner.sh"

require_bin jq

_TENANT_ID=""

_setup() {
  if [[ -n "$_TENANT_ID" ]]; then return 0; fi
  TOKEN=$(gen_default_admin_token)
  qa_set_token "$TOKEN"
  _TENANT_ID=$(db_query "SELECT id FROM tenants WHERE status != 'deleted' LIMIT 1;")
  if [[ -z "$_TENANT_ID" ]]; then
    echo "No tenant found in DB" >&2
    return 1
  fi
}

scenario 1 "租户列表分页" '
  _setup
  resp=$(api_get "/api/v1/tenants?page=1&per_page=20")
  assert_http_status "$(resp_status "$resp")" 200 "GET /api/v1/tenants returns 200"
  body=$(resp_body "$resp")
  assert_json_exists "$body" ".data" "response has data array"
  assert_json_exists "$body" ".pagination" "response has pagination"
  assert_json_exists "$body" ".pagination.total" "pagination has total"
  assert_json_exists "$body" ".pagination.total_pages" "pagination has total_pages"

  local count
  count=$(echo "$body" | jq ".data | length")
  local total
  total=$(echo "$body" | jq ".pagination.total")
  if [[ "$total" -gt 20 ]]; then
    assert_eq "$count" "20" "first page has 20 items when total > 20"
    resp2=$(api_get "/api/v1/tenants?page=2&per_page=20")
    assert_http_status "$(resp_status "$resp2")" 200 "GET page 2 returns 200"
    local count2
    count2=$(resp_body "$resp2" | jq ".data | length")
    assert_match "$count2" "^[0-9]+$" "page 2 returns numeric count"
  else
    assert_match "$count" "^[0-9]+$" "page returns items"
  fi
'

scenario 2 "查看租户详情" '
  _setup
  resp=$(api_get "/api/v1/tenants/${_TENANT_ID}")
  assert_http_status "$(resp_status "$resp")" 200 "GET /api/v1/tenants/{id} returns 200"
  body=$(resp_body "$resp")
  assert_json_exists "$body" ".data.id" "response has id"
  assert_json_exists "$body" ".data.name" "response has name"
  assert_json_exists "$body" ".data.slug" "response has slug"
  assert_json_exists "$body" ".data.status" "response has status"
  assert_json_exists "$body" ".data.created_at" "response has created_at"
  assert_json_exists "$body" ".data.updated_at" "response has updated_at"

  assert_db_not_empty \
    "SELECT id FROM tenants WHERE id = '\''${_TENANT_ID}'\'';" \
    "tenant exists in DB"
'

scenario 3 "搜索租户" '
  _setup

  local slug_prefix="qa-search-$(date +%s)"

  api_post "/api/v1/tenants" \
    "{\"name\":\"QA Acme Corporation\",\"slug\":\"${slug_prefix}-acme\"}" >/dev/null 2>&1 || true
  api_post "/api/v1/tenants" \
    "{\"name\":\"QA Beta Company\",\"slug\":\"${slug_prefix}-beta\"}" >/dev/null 2>&1 || true
  api_post "/api/v1/tenants" \
    "{\"name\":\"QA Acme Labs\",\"slug\":\"${slug_prefix}-acme-labs\"}" >/dev/null 2>&1 || true

  resp=$(api_get "/api/v1/tenants?search=QA+Acme")
  assert_http_status "$(resp_status "$resp")" 200 "search returns 200"
  body=$(resp_body "$resp")
  local count
  count=$(echo "$body" | jq ".data | length")
  assert_match "$count" "^[2-9][0-9]*$" "search for QA Acme returns at least 2 results"

  resp_beta=$(api_get "/api/v1/tenants?search=${slug_prefix}-beta")
  assert_http_status "$(resp_status "$resp_beta")" 200 "search by slug returns 200"
  body_beta=$(resp_body "$resp_beta")
  local count_beta
  count_beta=$(echo "$body_beta" | jq ".data | length")
  assert_match "$count_beta" "^[1-9][0-9]*$" "search by slug returns at least 1 result"

  db_exec "DELETE FROM tenants WHERE slug IN ('\''${slug_prefix}-acme'\'', '\''${slug_prefix}-beta'\'', '\''${slug_prefix}-acme-labs'\'');" || true
'

scenario 4 "租户设置更新" '
  _setup

  local ts
  ts=$(date +%s)
  local settings_slug="qa-settings-${ts}"

  resp_create=$(api_post "/api/v1/tenants" \
    "{\"name\":\"QA Settings Test\",\"slug\":\"${settings_slug}\",\"settings\":{\"require_mfa\":false}}")
  assert_http_status "$(resp_status "$resp_create")" 201 "create test tenant returns 201"
  local tid
  tid=$(resp_body "$resp_create" | jq -r ".data.id")

  resp=$(api_get "/api/v1/tenants/${tid}")
  body=$(resp_body "$resp")
  assert_json_field "$body" ".data.settings.require_mfa" "false" "initial require_mfa is false"

  resp_update=$(api_put "/api/v1/tenants/${tid}" \
    "{\"settings\":{\"require_mfa\":true,\"session_timeout_secs\":1800}}")
  assert_http_status "$(resp_status "$resp_update")" 200 "PUT settings update returns 200"
  body_update=$(resp_body "$resp_update")
  assert_json_field "$body_update" ".data.settings.require_mfa" "true" "require_mfa updated to true"

  resp_verify=$(api_get "/api/v1/tenants/${tid}")
  body_verify=$(resp_body "$resp_verify")
  assert_json_field "$body_verify" ".data.settings.require_mfa" "true" "require_mfa persisted as true"

  api_delete "/api/v1/tenants/${tid}" >/dev/null 2>&1 || true
  db_exec "DELETE FROM tenants WHERE slug = '\''${settings_slug}'\'';" || true
'

scenario 5 "Slug 格式验证" '
  _setup

  resp1=$(api_post "/api/v1/tenants" "{\"name\":\"Bad Slug 1\",\"slug\":\"TestCompany\"}")
  status1=$(resp_status "$resp1")
  assert_match "$status1" "^(400|422)$" "uppercase slug rejected"

  resp2=$(api_post "/api/v1/tenants" "{\"name\":\"Bad Slug 2\",\"slug\":\"test@company\"}")
  status2=$(resp_status "$resp2")
  assert_match "$status2" "^(400|422)$" "special char slug rejected"

  resp3=$(api_post "/api/v1/tenants" "{\"name\":\"Bad Slug 3\",\"slug\":\"-test-company\"}")
  status3=$(resp_status "$resp3")
  assert_match "$status3" "^(400|422)$" "leading-hyphen slug rejected"

  local long_slug
  long_slug=$(head -c 64 < /dev/zero | tr "\0" "a")
  resp4=$(api_post "/api/v1/tenants" "{\"name\":\"Bad Slug 4\",\"slug\":\"${long_slug}\"}")
  status4=$(resp_status "$resp4")
  assert_match "$status4" "^(400|422)$" "64-char slug rejected"

  assert_db \
    "SELECT COUNT(*) FROM tenants WHERE slug IN ('\''TestCompany'\'', '\''test@company'\'', '\''-test-company'\'');" \
    "0" \
    "invalid slugs not created in DB"
'

run_all
