#!/usr/bin/env bash
# QA Auto Test: auth/06-client-credentials
# Doc: docs/qa/auth/06-client-credentials.md
# Scenarios: 5
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/../lib/runner.sh"

require_bin jq

_CLIENT_ID=""
_CLIENT_SECRET=""

_setup_service_client() {
  if [[ -n "$_CLIENT_ID" ]]; then return 0; fi
  TOKEN=$(gen_default_admin_token)
  qa_set_token "$TOKEN"

  local tenant_id
  tenant_id=$(db_query "SELECT id FROM tenants LIMIT 1;")
  if [[ -z "$tenant_id" ]]; then
    echo "No tenant found in DB" >&2
    return 1
  fi

  local service_resp
  service_resp=$(api_post "/api/v1/tenants/${tenant_id}/services" \
    "{\"name\":\"cc-test-svc-$(date +%s)\",\"description\":\"Client Credentials QA test\"}")
  local service_id
  service_id=$(resp_body "$service_resp" | jq -r '.id // .data.id // empty')
  if [[ -z "$service_id" ]]; then
    echo "Failed to create service: $(resp_body "$service_resp")" >&2
    return 1
  fi

  local client_resp
  client_resp=$(api_post "/api/v1/services/${service_id}/clients" \
    "{\"name\":\"cc-test-client\",\"grant_types\":[\"client_credentials\"]}")
  _CLIENT_ID=$(resp_body "$client_resp" | jq -r '.client_id // .data.client_id // empty')
  _CLIENT_SECRET=$(resp_body "$client_resp" | jq -r '.client_secret // .data.client_secret // empty')

  if [[ -z "$_CLIENT_ID" || -z "$_CLIENT_SECRET" ]]; then
    echo "Failed to create client: $(resp_body "$client_resp")" >&2
    return 1
  fi
  qa_set_token ""
}

scenario 1 "Valid credentials - obtain service token" '
  _setup_service_client
  resp=$(api_raw POST /api/v1/auth/token \
    -H "Content-Type: application/json" \
    -d "{\"grant_type\":\"client_credentials\",\"client_id\":\"$_CLIENT_ID\",\"client_secret\":\"$_CLIENT_SECRET\"}")
  assert_http_status "$(resp_status "$resp")" 200 "client_credentials returns 200"
  body=$(resp_body "$resp")
  assert_json_exists "$body" ".access_token" "response has access_token"
  assert_json_field "$body" ".token_type" "Bearer" "token_type = Bearer"
'

scenario 2 "Wrong client secret rejected" '
  _setup_service_client
  resp=$(api_raw POST /api/v1/auth/token \
    -H "Content-Type: application/json" \
    -d "{\"grant_type\":\"client_credentials\",\"client_id\":\"$_CLIENT_ID\",\"client_secret\":\"wrong-secret-12345\"}")
  assert_http_status "$(resp_status "$resp")" 401 "wrong secret returns 401"
'

scenario 3 "Non-existent client ID rejected" '
  resp=$(api_raw POST /api/v1/auth/token \
    -H "Content-Type: application/json" \
    -d "{\"grant_type\":\"client_credentials\",\"client_id\":\"non-existent-client\",\"client_secret\":\"any-secret\"}")
  local status
  status=$(resp_status "$resp")
  assert_match "$status" "^(401|404)$" "non-existent client returns 401 or 404"
'

scenario 4 "Missing required parameters" '
  resp=$(api_raw POST /api/v1/auth/token \
    -H "Content-Type: application/json" \
    -d "{\"grant_type\":\"client_credentials\"}")
  assert_http_status "$(resp_status "$resp")" 400 "missing client_id returns 400"

  _setup_service_client
  resp=$(api_raw POST /api/v1/auth/token \
    -H "Content-Type: application/json" \
    -d "{\"grant_type\":\"client_credentials\",\"client_id\":\"$_CLIENT_ID\"}")
  assert_http_status "$(resp_status "$resp")" 400 "missing client_secret returns 400"
'

scenario 5 "Unsupported grant type" '
  resp=$(api_raw POST /api/v1/auth/token \
    -H "Content-Type: application/json" \
    -d "{\"grant_type\":\"password\",\"username\":\"test\",\"password\":\"test\"}")
  assert_http_status "$(resp_status "$resp")" 400 "password grant_type returns 400"
'

run_all
