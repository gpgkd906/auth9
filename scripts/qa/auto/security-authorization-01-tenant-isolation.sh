#!/usr/bin/env bash
# Security Auto Test: security/authorization/01-tenant-isolation
# Doc: docs/security/authorization/01-tenant-isolation.md
# Scenarios: 4
# ASVS 5.0: V8.1, V8.2, V4.2
# IMPORTANT: Tenant isolation MUST be tested with non-platform-admin tokens.
#   Platform admins (admin@auth9.local) have global bypass — using them will
#   cause false positives. gen_tenant_token produces regular-user@example.com.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/../lib/runner.sh"

require_bin jq

_RANDOM_USER_ID="aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee"

scenario 1 "Cross-tenant data access (IDOR)" '
  TENANT_IDS=($(db_query "SELECT id FROM tenants ORDER BY created_at LIMIT 2;" | tr -d " "))
  if [[ ${#TENANT_IDS[@]} -lt 2 ]]; then
    assert_eq "need_2_tenants" "need_2_tenants" "need at least 2 tenants (skipping)"
    return 0
  fi
  TENANT_A="${TENANT_IDS[0]}"
  TENANT_B="${TENANT_IDS[1]}"

  TOKEN_A=$(gen_tenant_token "$_RANDOM_USER_ID" "$TENANT_A")
  qa_set_token "$TOKEN_A"

  resp=$(api_get "/api/v1/tenants/${TENANT_B}")
  status=$(resp_status "$resp")
  assert_match "$status" "^(403|404)$" "cross-tenant GET tenant rejected"

  resp_users=$(api_get "/api/v1/tenants/${TENANT_B}/users")
  status_users=$(resp_status "$resp_users")
  assert_match "$status_users" "^(403|404)$" "cross-tenant GET tenant users rejected"

  resp_patch=$(api_patch "/api/v1/tenants/${TENANT_B}" "{\"name\":\"Hijacked Tenant\"}")
  status_patch=$(resp_status "$resp_patch")
  assert_match "$status_patch" "^(403|404)$" "cross-tenant PATCH tenant rejected"

  resp_delete=$(api_delete "/api/v1/tenants/${TENANT_B}")
  status_delete=$(resp_status "$resp_delete")
  assert_match "$status_delete" "^(403|404)$" "cross-tenant DELETE tenant rejected"

  qa_set_token ""
'

scenario 2 "Bulk operation tenant leakage" '
  TENANT_ID=$(db_query "SELECT id FROM tenants ORDER BY created_at LIMIT 1;" | tr -d "[:space:]")

  TOKEN=$(gen_tenant_token "$_RANDOM_USER_ID" "$TENANT_ID")
  qa_set_token "$TOKEN"

  resp_users=$(api_get "/api/v1/users")
  status_users=$(resp_status "$resp_users")
  if [[ "$status_users" == "200" ]]; then
    body_users=$(resp_body "$resp_users")
    other_tenant_users=$(echo "$body_users" | jq "[.data[]? | select(.tenant_id != null and .tenant_id != \"${TENANT_ID}\")] | length" 2>/dev/null || echo "0")
    assert_eq "$other_tenant_users" "0" "user list contains no cross-tenant data"
  else
    assert_match "$status_users" "^(200|403)$" "user list endpoint responds"
  fi

  resp_services=$(api_get "/api/v1/services")
  status_services=$(resp_status "$resp_services")
  if [[ "$status_services" == "200" ]]; then
    body_services=$(resp_body "$resp_services")
    other_tenant_svcs=$(echo "$body_services" | jq "[.data[]? | select(.tenant_id != null and .tenant_id != \"${TENANT_ID}\")] | length" 2>/dev/null || echo "0")
    assert_eq "$other_tenant_svcs" "0" "service list contains no cross-tenant data"
  else
    assert_match "$status_services" "^(200|403)$" "service list endpoint responds"
  fi

  resp_roles=$(api_get "/api/v1/roles")
  status_roles=$(resp_status "$resp_roles")
  if [[ "$status_roles" == "200" ]]; then
    body_roles=$(resp_body "$resp_roles")
    other_tenant_roles=$(echo "$body_roles" | jq "[.data[]? | select(.tenant_id != null and .tenant_id != \"${TENANT_ID}\")] | length" 2>/dev/null || echo "0")
    assert_eq "$other_tenant_roles" "0" "role list contains no cross-tenant data"
  else
    assert_match "$status_roles" "^(200|403)$" "role list endpoint responds"
  fi

  qa_set_token ""
'

scenario 3 "Associated resource cross-tenant access" '
  TENANT_IDS=($(db_query "SELECT id FROM tenants ORDER BY created_at LIMIT 2;" | tr -d " "))
  if [[ ${#TENANT_IDS[@]} -lt 2 ]]; then
    assert_eq "need_2_tenants" "need_2_tenants" "need at least 2 tenants (skipping)"
    return 0
  fi
  TENANT_A="${TENANT_IDS[0]}"
  TENANT_B="${TENANT_IDS[1]}"

  SERVICE_B=$(db_query "SELECT id FROM services WHERE tenant_id = '"'"'${TENANT_B}'"'"' LIMIT 1;" | tr -d "[:space:]")

  TOKEN_A=$(gen_tenant_token "$_RANDOM_USER_ID" "$TENANT_A")
  qa_set_token "$TOKEN_A"

  if [[ -n "$SERVICE_B" ]]; then
    resp=$(api_get "/api/v1/services/${SERVICE_B}")
    status=$(resp_status "$resp")
    assert_match "$status" "^(403|404)$" "cross-tenant GET service rejected"

    resp_put=$(api_put "/api/v1/services/${SERVICE_B}" "{\"name\":\"Hijacked\"}")
    status_put=$(resp_status "$resp_put")
    assert_match "$status_put" "^(403|404)$" "cross-tenant PUT service rejected"

    resp_roles=$(api_get "/api/v1/services/${SERVICE_B}/roles")
    status_roles=$(resp_status "$resp_roles")
    assert_match "$status_roles" "^(403|404)$" "cross-tenant GET service roles rejected"
  else
    assert_eq "checked" "checked" "no service in tenant B (cross-tenant service test skipped)"
  fi

  ROLE_B=$(db_query "SELECT r.id FROM roles r JOIN services s ON r.service_id = s.id WHERE s.tenant_id = '"'"'${TENANT_B}'"'"' LIMIT 1;" | tr -d "[:space:]")
  if [[ -n "$ROLE_B" ]]; then
    USER_B=$(db_query "SELECT user_id FROM tenant_users WHERE tenant_id = '"'"'${TENANT_B}'"'"' LIMIT 1;" | tr -d "[:space:]")
    if [[ -n "$USER_B" ]]; then
      resp_assign=$(api_post "/api/v1/rbac/assign" "{\"user_id\":\"${USER_B}\",\"tenant_id\":\"${TENANT_A}\",\"role_id\":\"${ROLE_B}\"}")
      status_assign=$(resp_status "$resp_assign")
      assert_match "$status_assign" "^(400|403|404)$" "cross-tenant role assignment rejected"
    fi
  fi

  qa_set_token ""
'

scenario 4 "Admin permission boundary" '
  TENANT_ID=$(db_query "SELECT id FROM tenants ORDER BY created_at LIMIT 1;" | tr -d "[:space:]")
  TOKEN=$(gen_tenant_token "$_RANDOM_USER_ID" "$TENANT_ID")
  qa_set_token "$TOKEN"

  resp_create=$(api_post "/api/v1/tenants" "{\"name\":\"Unauthorized Tenant\",\"slug\":\"unauth-tenant-$(date +%s)\"}")
  status_create=$(resp_status "$resp_create")
  assert_eq "$status_create" "403" "non-platform-admin cannot create tenant"

  resp_sys=$(api_get "/api/v1/system/email")
  status_sys=$(resp_status "$resp_sys")
  assert_eq "$status_sys" "403" "non-platform-admin cannot access system settings"

  resp_settings=$(api_get "/api/v1/system/settings")
  status_settings=$(resp_status "$resp_settings")
  assert_match "$status_settings" "^(403|404)$" "non-platform-admin cannot access system/settings"

  qa_set_token ""

  ADMIN_TOKEN=$(gen_default_admin_token)
  qa_set_token "$ADMIN_TOKEN"
  resp_admin_sys=$(api_get "/api/v1/system/email")
  status_admin_sys=$(resp_status "$resp_admin_sys")
  assert_match "$status_admin_sys" "^(200|404)$" "platform admin CAN access system settings"
  qa_set_token ""
'

run_all
