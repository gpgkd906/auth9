#!/usr/bin/env bash
# QA Auto Test: session/08-identity-token-whitelist-tenant-token-enforcement
# Doc: docs/qa/session/08-identity-token-whitelist-tenant-token-enforcement.md
# Scenarios: 4 (scenario 5 tests Portal UI switching - requires browser)
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/../lib/runner.sh"

require_bin jq

ADMIN_ID=""
TENANT_ID=""

_setup() {
  if [[ -n "$ADMIN_ID" ]]; then return 0; fi
  ADMIN_ID=$(db_query "SELECT id FROM users WHERE email = 'admin@auth9.local' LIMIT 1;")
  TENANT_ID=$(db_query "SELECT id FROM tenants LIMIT 1;")
  if [[ -z "$ADMIN_ID" || -z "$TENANT_ID" ]]; then
    echo "Missing admin user or tenant in DB" >&2
    return 1
  fi
}

scenario 1 "Identity token can access whitelist endpoints" '
  _setup
  ID_TOKEN=$(gen_identity_token "$ADMIN_ID" "admin@auth9.local")
  qa_set_token "$ID_TOKEN"

  resp=$(api_get /api/v1/users/me/tenants)
  assert_http_status "$(resp_status "$resp")" 200 "GET /users/me/tenants with identity token"

  resp=$(api_get /api/v1/auth/userinfo)
  assert_http_status "$(resp_status "$resp")" 200 "GET /auth/userinfo with identity token"
  qa_set_token ""
'

scenario 2 "Identity token rejected on tenant business endpoints" '
  _setup
  ID_TOKEN=$(gen_identity_token "$ADMIN_ID" "admin@auth9.local")
  qa_set_token "$ID_TOKEN"

  resp=$(api_get "/api/v1/tenants/${TENANT_ID}")
  assert_http_status "$(resp_status "$resp")" 403 "GET /tenants/{id} with identity token returns 403"

  resp=$(api_put "/api/v1/tenants/${TENANT_ID}" "{\"name\":\"hacked\"}")
  assert_http_status "$(resp_status "$resp")" 403 "PUT /tenants/{id} with identity token returns 403"
  qa_set_token ""
'

scenario 3 "Tenant access token can access tenant business endpoints" '
  _setup
  TENANT_TOKEN=$(gen_tenant_token "$ADMIN_ID" "$TENANT_ID")
  qa_set_token "$TENANT_TOKEN"

  resp=$(api_get "/api/v1/tenants/${TENANT_ID}")
  assert_http_status "$(resp_status "$resp")" 200 "GET /tenants/{id} with tenant token returns 200"
  qa_set_token ""
'

scenario 4 "Identity token can access GET /tenants (whitelist, filtered)" '
  _setup
  ID_TOKEN=$(gen_identity_token "$ADMIN_ID" "admin@auth9.local")
  qa_set_token "$ID_TOKEN"

  resp=$(api_get /api/v1/tenants)
  assert_http_status "$(resp_status "$resp")" 200 "GET /tenants with identity token returns 200"
  qa_set_token ""
'

run_all
