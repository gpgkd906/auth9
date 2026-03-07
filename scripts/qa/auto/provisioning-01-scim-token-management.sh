#!/usr/bin/env bash
# QA Auto Test: provisioning/01-scim-token-management
# Doc: docs/qa/provisioning/01-scim-token-management.md
# Scenarios: 5
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/../lib/runner.sh"

require_bin jq

TENANT_ID=""
CONNECTOR_ID=""
SCIM_TOKEN_RAW=""
SCIM_TOKEN_ID=""

_setup() {
  if [[ -n "$TENANT_ID" && -n "$CONNECTOR_ID" ]]; then return 0; fi

  TENANT_ID=$(db_query "SELECT id FROM tenants LIMIT 1;")
  CONNECTOR_ID=$(db_query "SELECT id FROM enterprise_sso_connectors WHERE tenant_id = '${TENANT_ID}' LIMIT 1;")

  if [[ -z "$TENANT_ID" ]]; then
    echo "No tenant found" >&2; return 1
  fi
  if [[ -z "$CONNECTOR_ID" ]]; then
    echo "No SSO connector found for tenant ${TENANT_ID} - this test requires an enterprise SSO connector" >&2
    return 1
  fi

  local admin_id
  admin_id=$(db_query "SELECT id FROM users WHERE email = 'admin@auth9.local' LIMIT 1;")
  TOKEN=$(gen_tenant_token "$admin_id" "$TENANT_ID")
  qa_set_token "$TOKEN"
}

scenario 1 "Create SCIM token with description and expiry" '
  _setup
  resp=$(api_post "/api/v1/tenants/${TENANT_ID}/sso/connectors/${CONNECTOR_ID}/scim/tokens" \
    "{\"description\":\"QA auto test token\",\"expires_in_days\":90}")
  assert_http_status "$(resp_status "$resp")" 201 "POST scim token returns 201"
  body=$(resp_body "$resp")
  assert_json_exists "$body" ".token" "response contains token"
  assert_json_exists "$body" ".token_prefix" "response contains token_prefix"
  assert_json_field "$body" ".data.description" "QA auto test token" "description matches"
  assert_json_exists "$body" ".expires_at" "response contains expires_at"

  SCIM_TOKEN_RAW=$(echo "$body" | jq -r ".token")
  SCIM_TOKEN_ID=$(echo "$body" | jq -r ".dataid")
  assert_match "$SCIM_TOKEN_RAW" "^scim_" "token starts with scim_ prefix"
'

scenario 2 "Create SCIM token without expiry" '
  _setup
  resp=$(api_post "/api/v1/tenants/${TENANT_ID}/sso/connectors/${CONNECTOR_ID}/scim/tokens" \
    "{\"description\":\"Permanent QA token\"}")
  assert_http_status "$(resp_status "$resp")" 201 "POST permanent scim token returns 201"
  body=$(resp_body "$resp")
  assert_json_field "$body" ".expires_at" "null" "expires_at is null for permanent token"
'

scenario 3 "List SCIM tokens for connector" '
  _setup
  resp=$(api_get "/api/v1/tenants/${TENANT_ID}/sso/connectors/${CONNECTOR_ID}/scim/tokens")
  assert_http_status "$(resp_status "$resp")" 200 "GET scim tokens returns 200"
  body=$(resp_body "$resp")
  assert_not_contains "$body" "token_hash" "response does not expose token_hash"
'

scenario 4 "Revoke SCIM token" '
  _setup
  if [[ -z "$SCIM_TOKEN_ID" ]]; then
    echo "No token ID from scenario 1" >&2; return 1
  fi
  resp=$(api_delete "/api/v1/tenants/${TENANT_ID}/sso/connectors/${CONNECTOR_ID}/scim/tokens/${SCIM_TOKEN_ID}")
  assert_http_status "$(resp_status "$resp")" 204 "DELETE scim token returns 204"
  assert_db \
    "SELECT revoked_at IS NOT NULL FROM scim_tokens WHERE id = '\''${SCIM_TOKEN_ID}'\'';" \
    "1" \
    "token revoked_at is set in DB"
'

scenario 5 "Revoked token rejected by SCIM endpoint" '
  _setup
  if [[ -z "$SCIM_TOKEN_RAW" ]]; then
    echo "No raw token from scenario 1" >&2; return 1
  fi
  resp=$(curl -s -w "\n%{http_code}" \
    -H "Authorization: Bearer ${SCIM_TOKEN_RAW}" \
    "${API_BASE}/api/v1/scim/v2/Users")
  assert_http_status "$(echo "$resp" | tail -1)" 401 "revoked SCIM token returns 401"
'

run_all
