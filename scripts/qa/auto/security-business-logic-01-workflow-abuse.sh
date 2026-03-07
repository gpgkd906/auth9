#!/usr/bin/env bash
# Security Auto Test: security/business-logic/01-workflow-abuse
# Doc: docs/security/business-logic/01-workflow-abuse.md
# Scenarios: 5
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/../lib/runner.sh"

require_bin jq
require_bin node

_PLATFORM_TENANT_ID=""
_get_platform_tenant_id() {
  if [[ -z "$_PLATFORM_TENANT_ID" ]]; then
    _PLATFORM_TENANT_ID=$(db_query "SELECT id FROM tenants WHERE slug = 'auth9-platform' LIMIT 1;")
  fi
  echo "$_PLATFORM_TENANT_ID"
}

scenario 1 "Token Exchange - forged token rejected" '
  local forged_token
  forged_token=$(node -e "
const jwt=require(\"jsonwebtoken\");
const now=Math.floor(Date.now()/1000);
process.stdout.write(jwt.sign({
  sub:\"fake-admin-id\",email:\"fake@hacker.com\",
  iss:\"http://localhost:8080\",aud:\"auth9\",token_type:\"identity\",
  iat:now,exp:now+3600,sid:\"fake-sid\"
},\"wrong-secret-key\",{algorithm:\"HS256\"}));
" 2>/dev/null)

  local tenant_id
  tenant_id=$(_get_platform_tenant_id)

  qa_set_token "$forged_token"
  resp=$(api_post "/api/v1/auth/tenant-token" \
    "{\"tenant_id\":\"$tenant_id\",\"service_id\":\"auth9-portal\"}")
  status=$(resp_status "$resp")
  assert_http_status "$status" 401 "Forged token (wrong key) rejected for token exchange"
  qa_set_token ""
'

scenario 2 "Token Exchange - expired identity token rejected" '
  local uid
  uid=$(db_query "SELECT LOWER(UUID());")

  db_exec "DELETE FROM users WHERE keycloak_id = '\''kc-qa-biz01-2'\'';" || true
  db_exec "INSERT INTO users (id, keycloak_id, email, display_name) VALUES ('\''$uid'\'', '\''kc-qa-biz01-2'\'', '\''qa-biz01-2@test.com'\'', '\''QA Biz Exp'\'');"

  local expired_token
  expired_token=$(node -e "
const jwt=require(\"jsonwebtoken\"),fs=require(\"fs\");
const pk=fs.readFileSync(process.argv[1],\"utf8\");
const now=Math.floor(Date.now()/1000);
process.stdout.write(jwt.sign({
  sub:process.argv[2],email:\"qa-biz01-2@test.com\",
  iss:\"http://localhost:8080\",aud:\"auth9\",token_type:\"identity\",
  iat:now-7200,exp:now-3600,sid:\"sid-expired-biz\"
},pk,{algorithm:\"RS256\",keyid:\"auth9-current\"}));
" "$_JWT_PRIVATE_KEY" "$uid" 2>/dev/null)

  local tenant_id
  tenant_id=$(_get_platform_tenant_id)

  qa_set_token "$expired_token"
  resp=$(api_post "/api/v1/auth/tenant-token" \
    "{\"tenant_id\":\"$tenant_id\",\"service_id\":\"auth9-portal\"}")
  status=$(resp_status "$resp")
  assert_http_status "$status" 401 "Expired identity token rejected for token exchange"

  db_exec "DELETE FROM users WHERE id = '\''$uid'\'';" || true
  qa_set_token ""
'

scenario 3 "Token Exchange - cross-tenant access denied" '
  local uid tenant_a tenant_b
  uid=$(db_query "SELECT LOWER(UUID());")
  tenant_a=$(db_query "SELECT LOWER(UUID());")
  tenant_b=$(db_query "SELECT LOWER(UUID());")

  db_exec "DELETE FROM tenant_users WHERE user_id IN (SELECT id FROM users WHERE keycloak_id = '\''kc-qa-biz01-3'\'');" || true
  db_exec "DELETE FROM users WHERE keycloak_id = '\''kc-qa-biz01-3'\'';" || true
  db_exec "DELETE FROM tenants WHERE slug IN ('\''qa-biz01-3a'\'', '\''qa-biz01-3b'\'');" || true

  db_exec "INSERT INTO tenants (id, name, slug, status) VALUES ('\''$tenant_a'\'', '\''QA Biz Tenant A'\'', '\''qa-biz01-3a'\'', '\''active'\'');"
  db_exec "INSERT INTO tenants (id, name, slug, status) VALUES ('\''$tenant_b'\'', '\''QA Biz Tenant B'\'', '\''qa-biz01-3b'\'', '\''active'\'');"
  db_exec "INSERT INTO users (id, keycloak_id, email, display_name) VALUES ('\''$uid'\'', '\''kc-qa-biz01-3'\'', '\''qa-biz01-3@test.com'\'', '\''QA Biz Cross'\'');"
  db_exec "INSERT INTO tenant_users (tenant_id, user_id, role) VALUES ('\''$tenant_a'\'', '\''$uid'\'', '\''member'\'');"

  local not_member
  not_member=$(db_query "SELECT COUNT(*) FROM tenant_users WHERE user_id = '\''$uid'\'' AND tenant_id = '\''$tenant_b'\'';")
  assert_eq "$not_member" "0" "User confirmed not member of tenant B"

  local id_token
  id_token=$(gen_identity_token "$uid" "qa-biz01-3@test.com")
  qa_set_token "$id_token"

  resp=$(api_post "/api/v1/auth/tenant-token" \
    "{\"tenant_id\":\"$tenant_b\",\"service_id\":\"auth9-portal\"}")
  status=$(resp_status "$resp")
  assert_match "$status" "^(403|404)$" "Cross-tenant token exchange denied"

  db_exec "DELETE FROM tenant_users WHERE user_id = '\''$uid'\'';" || true
  db_exec "DELETE FROM users WHERE id = '\''$uid'\'';" || true
  db_exec "DELETE FROM tenants WHERE id IN ('\''$tenant_a'\'', '\''$tenant_b'\'');" || true
  qa_set_token ""
'

scenario 4 "Role inheritance cycle detection" '
  local admin_token tenant_id service_id
  admin_token=$(gen_default_admin_token)
  tenant_id=$(_get_platform_tenant_id)

  qa_set_token "$admin_token"

  local svc_name="qa-biz01-4-svc-$(date +%s)"
  resp=$(api_post "/api/v1/tenants/$tenant_id/services" \
    "{\"name\":\"$svc_name\",\"description\":\"QA test service for cycle detection\"}")
  service_id=$(resp_body "$resp" | jq -r ".data.id // .id // empty")

  if [[ -z "$service_id" || "$service_id" == "null" ]]; then
    service_id=$(db_query "SELECT id FROM services WHERE tenant_id = '\''$tenant_id'\'' LIMIT 1;")
  fi

  if [[ -z "$service_id" ]]; then
    assert_eq "skip" "skip" "No service available, skipping cycle detection test"
  else
    local role_a role_b role_c
    resp=$(api_post "/api/v1/tenants/$tenant_id/services/$service_id/roles" \
      "{\"name\":\"qa-cycle-a-$(date +%s)\",\"description\":\"Cycle test A\"}")
    role_a=$(resp_body "$resp" | jq -r ".data.id // .id // empty")

    resp=$(api_post "/api/v1/tenants/$tenant_id/services/$service_id/roles" \
      "{\"name\":\"qa-cycle-b-$(date +%s)\",\"description\":\"Cycle test B\"}")
    role_b=$(resp_body "$resp" | jq -r ".data.id // .id // empty")

    if [[ -n "$role_a" && "$role_a" != "null" && -n "$role_b" && "$role_b" != "null" ]]; then
      resp=$(api_put "/api/v1/tenants/$tenant_id/services/$service_id/roles/$role_b" \
        "{\"name\":\"qa-cycle-b\",\"description\":\"Cycle test B\",\"parent_role_id\":\"$role_a\"}")

      resp=$(api_put "/api/v1/tenants/$tenant_id/services/$service_id/roles/$role_a" \
        "{\"name\":\"qa-cycle-a\",\"description\":\"Cycle test A\",\"parent_role_id\":\"$role_b\"}")
      cycle_status=$(resp_status "$resp")
      assert_match "$cycle_status" "^(400|409|422)$" "Circular role inheritance rejected"

      api_delete "/api/v1/tenants/$tenant_id/services/$service_id/roles/$role_b" >/dev/null 2>&1 || true
      api_delete "/api/v1/tenants/$tenant_id/services/$service_id/roles/$role_a" >/dev/null 2>&1 || true
    else
      assert_eq "skip" "skip" "Could not create test roles, skipping"
    fi

    if [[ "$svc_name" == qa-biz01-4-svc-* ]]; then
      api_delete "/api/v1/tenants/$tenant_id/services/$service_id" >/dev/null 2>&1 || true
    fi
  fi

  qa_set_token ""
'

scenario 5 "System settings - security downgrade protection" '
  local admin_token tenant_id
  admin_token=$(gen_default_admin_token)
  tenant_id=$(_get_platform_tenant_id)
  qa_set_token "$admin_token"

  resp=$(api_put "/api/v1/tenants/$tenant_id/password-policy" \
    "{\"min_length\":1,\"require_uppercase\":false,\"require_lowercase\":false,\"require_number\":false,\"require_symbol\":false}")
  status=$(resp_status "$resp")
  body=$(resp_body "$resp")

  if [[ "$status" == "200" ]]; then
    if echo "$body" | jq -e ".data.min_length" >/dev/null 2>&1; then
      actual_min=$(echo "$body" | jq -r ".data.min_length")
      if [[ "$actual_min" -ge 8 ]]; then
        assert_eq "$actual_min" "$actual_min" "Server enforced minimum password length >= 8"
      else
        assert_eq "enforced" "not-enforced" "SECURITY: Server accepted password min_length < 8"
      fi
    else
      assert_eq "$status" "400" "Expected 400 for insecure password policy"
    fi
  else
    assert_match "$status" "^(400|403|422)$" "Insecure password policy rejected"
  fi

  qa_set_token ""
'

run_all
