#!/usr/bin/env bash
# QA Auto Test: service/04-tenant-service-toggle
# Doc: docs/qa/service/04-tenant-service-toggle.md
# Scenarios: 5
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/../lib/runner.sh"

require_bin jq

_TENANT_ID=""
_SERVICE_ID=""

_setup() {
  if [[ -n "$_TENANT_ID" ]]; then return 0; fi

  _TENANT_ID=$(qa_get_tenant_id)
  if [[ -z "$_TENANT_ID" ]]; then
    echo "No active tenant found" >&2; return 1
  fi

  TOKEN=$(gen_token_for_tenant "$_TENANT_ID")
  qa_set_token "$TOKEN"

  _SERVICE_ID=$(db_query "SELECT id FROM services WHERE tenant_id IS NULL LIMIT 1;" | tr -d "[:space:]")
  if [[ -z "$_SERVICE_ID" ]]; then
    echo "No global service found" >&2; return 1
  fi

  db_exec "DELETE FROM tenant_services WHERE tenant_id = '${_TENANT_ID}' AND service_id = '${_SERVICE_ID}';" || true
}

scenario 1 "List tenant available services" '
  _setup

  resp=$(api_get "/api/v1/tenants/${_TENANT_ID}/services")
  assert_http_status "$(resp_status "$resp")" 200 "GET /api/v1/tenants/{id}/services returns 200"

  body=$(resp_body "$resp")
  count=$(echo "$body" | jq ".data | length // 0" 2>/dev/null || echo "0")
  assert_match "$count" "^[0-9]+$" "response returns numeric service count"

  if [[ "$count" -gt 0 ]]; then
    assert_json_exists "$body" ".data[0].id" "service has id"
    assert_json_exists "$body" ".data[0].name" "service has name"
  fi

  global_count=$(db_query "SELECT COUNT(*) FROM services WHERE tenant_id IS NULL;")
  global_count=$(echo "$global_count" | tr -d "[:space:]")
  assert_match "$global_count" "^[0-9]+$" "global services count is numeric"
'

scenario 2 "Enable service for tenant" '
  _setup

  db_exec "DELETE FROM tenant_services WHERE tenant_id = '\''${_TENANT_ID}'\'' AND service_id = '\''${_SERVICE_ID}'\'';" || true

  resp=$(api_post "/api/v1/tenants/${_TENANT_ID}/services" \
    "{\"service_id\":\"${_SERVICE_ID}\",\"enabled\":true}")
  status=$(resp_status "$resp")
  assert_match "$status" "^(200|201)$" "enable service returns 200/201"

  resp_enabled=$(api_get "/api/v1/tenants/${_TENANT_ID}/services/enabled")
  assert_http_status "$(resp_status "$resp_enabled")" 200 "GET enabled services returns 200"

  body_enabled=$(resp_body "$resp_enabled")
  found=$(echo "$body_enabled" | jq "[.data[]? | select(.id == \"${_SERVICE_ID}\")] | length" 2>/dev/null || echo "0")
  assert_match "$found" "^[1-9]" "enabled service appears in list"

  assert_db \
    "SELECT enabled FROM tenant_services WHERE tenant_id = '\''${_TENANT_ID}'\'' AND service_id = '\''${_SERVICE_ID}'\'';" \
    "1" \
    "tenant_services.enabled = true in DB"
'

scenario 3 "Disable enabled service for tenant" '
  _setup

  resp=$(api_post "/api/v1/tenants/${_TENANT_ID}/services" \
    "{\"service_id\":\"${_SERVICE_ID}\",\"enabled\":false}")
  status=$(resp_status "$resp")
  assert_match "$status" "^(200|201)$" "disable service returns 200/201"

  resp_enabled=$(api_get "/api/v1/tenants/${_TENANT_ID}/services/enabled")
  assert_http_status "$(resp_status "$resp_enabled")" 200 "GET enabled services returns 200"

  body_enabled=$(resp_body "$resp_enabled")
  found=$(echo "$body_enabled" | jq "[.data[]? | select(.id == \"${_SERVICE_ID}\")] | length" 2>/dev/null || echo "0")
  assert_eq "$found" "0" "disabled service not in enabled list"

  enabled_val=$(db_query "SELECT enabled FROM tenant_services WHERE tenant_id = '\''${_TENANT_ID}'\'' AND service_id = '\''${_SERVICE_ID}'\'';" | tr -d "[:space:]")
  assert_eq "$enabled_val" "0" "tenant_services.enabled = false in DB"
'

scenario 4 "Enable non-existent service" '
  _setup

  fake_svc="99999999-9999-9999-9999-999999999999"
  resp=$(api_post "/api/v1/tenants/${_TENANT_ID}/services" \
    "{\"service_id\":\"${fake_svc}\",\"enabled\":true}")
  assert_http_status "$(resp_status "$resp")" 404 "non-existent service returns 404"

  assert_db \
    "SELECT COUNT(*) FROM tenant_services WHERE tenant_id = '\''${_TENANT_ID}'\'' AND service_id = '\''${fake_svc}'\'';" \
    "0" \
    "no record created for non-existent service"
'

scenario 5 "Idempotent enable (duplicate enable)" '
  _setup

  db_exec "DELETE FROM tenant_services WHERE tenant_id = '\''${_TENANT_ID}'\'' AND service_id = '\''${_SERVICE_ID}'\'';" || true

  resp1=$(api_post "/api/v1/tenants/${_TENANT_ID}/services" \
    "{\"service_id\":\"${_SERVICE_ID}\",\"enabled\":true}")
  status1=$(resp_status "$resp1")
  assert_match "$status1" "^(200|201)$" "first enable succeeds"

  resp2=$(api_post "/api/v1/tenants/${_TENANT_ID}/services" \
    "{\"service_id\":\"${_SERVICE_ID}\",\"enabled\":true}")
  status2=$(resp_status "$resp2")
  assert_match "$status2" "^(200|201)$" "second enable succeeds (idempotent)"

  row_count=$(db_query "SELECT COUNT(*) FROM tenant_services WHERE tenant_id = '\''${_TENANT_ID}'\'' AND service_id = '\''${_SERVICE_ID}'\'';" | tr -d "[:space:]")
  assert_eq "$row_count" "1" "only 1 record in tenant_services (not duplicated)"

  db_exec "DELETE FROM tenant_services WHERE tenant_id = '\''${_TENANT_ID}'\'' AND service_id = '\''${_SERVICE_ID}'\'';" || true
'

run_all
