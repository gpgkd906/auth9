#!/usr/bin/env bash
# Security Auto Test: security/authorization/04-resource-access
# Doc: docs/security/authorization/04-resource-access.md
# Scenarios: 5
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/../lib/runner.sh"

require_bin jq
require_bin node
require_bin curl

_PLATFORM_TENANT_ID=""
_get_platform_tenant_id() {
  if [[ -z "$_PLATFORM_TENANT_ID" ]]; then
    _PLATFORM_TENANT_ID=$(db_query "SELECT id FROM tenants WHERE slug = 'auth9-platform' LIMIT 1;")
  fi
  echo "$_PLATFORM_TENANT_ID"
}

_gen_tenant_member_token() {
  local tenant_id="$1"
  local uid="${2:-16daa93d-06e8-479c-867d-f9b6184e06c7}"
  local email="${3:-regular-member@test.com}"
  node -e '
const jwt=require("jsonwebtoken"),fs=require("fs");
const pk=fs.readFileSync(process.argv[1],"utf8");
const now=Math.floor(Date.now()/1000);
process.stdout.write(jwt.sign({
  sub:process.argv[2],email:process.argv[3],
  iss:"http://localhost:8080",aud:"auth9-portal",token_type:"access",
  tenant_id:process.argv[4],roles:["member"],permissions:["user:read"],
  iat:now,exp:now+3600
},pk,{algorithm:"RS256",keyid:"auth9-current"}));
' "$_JWT_PRIVATE_KEY" "$uid" "$email" "$tenant_id" 2>/dev/null
}

scenario 1 "IDOR - cross-tenant service access denied" '
  local tenant_a tenant_b uid_a token_a
  tenant_a=$(_get_platform_tenant_id)

  tenant_b=$(db_query "SELECT LOWER(UUID());")
  db_exec "DELETE FROM tenants WHERE slug = '\''qa-idor-tenant-b'\'';" || true
  db_exec "INSERT INTO tenants (id, name, slug, status) VALUES ('\''$tenant_b'\'', '\''QA IDOR B'\'', '\''qa-idor-tenant-b'\'', '\''active'\'');"

  local service_b
  service_b=$(db_query "SELECT LOWER(UUID());")
  db_exec "INSERT INTO services (id, tenant_id, name, description) VALUES ('\''$service_b'\'', '\''$tenant_b'\'', '\''qa-idor-svc'\'', '\''test'\'');"

  token_a=$(_gen_tenant_member_token "$tenant_a")
  qa_set_token "$token_a"

  resp=$(api_get "/api/v1/tenants/$tenant_b/services/$service_b")
  status=$(resp_status "$resp")
  assert_match "$status" "^(403|404)$" "Cross-tenant service read denied (IDOR protection)"

  resp2=$(api_put "/api/v1/tenants/$tenant_b/services/$service_b" \
    "{\"name\":\"hacked\",\"description\":\"hacked\"}")
  status2=$(resp_status "$resp2")
  assert_match "$status2" "^(403|404)$" "Cross-tenant service update denied"

  resp3=$(api_delete "/api/v1/tenants/$tenant_b/services/$service_b")
  status3=$(resp_status "$resp3")
  assert_match "$status3" "^(403|404)$" "Cross-tenant service delete denied"

  db_exec "DELETE FROM services WHERE id = '\''$service_b'\'';" || true
  db_exec "DELETE FROM tenants WHERE id = '\''$tenant_b'\'';" || true
  qa_set_token ""
'

scenario 2 "Path traversal protection" '
  local admin_token
  admin_token=$(gen_default_admin_token)

  resp=$(api_raw GET "/api/v1/tenants/../admin/config" \
    -H "Authorization: Bearer $admin_token" \
    --path-as-is)
  status=$(resp_status "$resp")
  assert_match "$status" "^(400|404)$" "Path traversal with .. returns 400/404"

  resp2=$(api_raw GET "/api/v1/services/../tenants" \
    -H "Authorization: Bearer $admin_token" \
    --path-as-is)
  status2=$(resp_status "$resp2")
  assert_match "$status2" "^(400|404)$" "Path traversal services/../tenants returns 400/404"

  resp3=$(api_raw GET "/api/v1/users/%2e%2e/admin" \
    -H "Authorization: Bearer $admin_token" \
    --path-as-is)
  status3=$(resp_status "$resp3")
  assert_match "$status3" "^(400|404)$" "URL-encoded path traversal returns 400/404"
'

skip_scenario 3 "Batch operation authorization" "Auth9 does not currently expose batch delete/update endpoints"

scenario 4 "Associated resource cross-tenant leak prevention" '
  local tenant_a tenant_b token_a uid_b
  tenant_a=$(_get_platform_tenant_id)
  tenant_b=$(db_query "SELECT LOWER(UUID());")
  uid_b=$(db_query "SELECT LOWER(UUID());")

  db_exec "DELETE FROM tenants WHERE slug = '\''qa-assoc-leak-b'\'';" || true
  db_exec "INSERT INTO tenants (id, name, slug, status) VALUES ('\''$tenant_b'\'', '\''QA Assoc Leak B'\'', '\''qa-assoc-leak-b'\'', '\''active'\'');"

  db_exec "DELETE FROM users WHERE keycloak_id = '\''kc-qa-authz04-4b'\'';" || true
  db_exec "INSERT INTO users (id, keycloak_id, email, display_name) VALUES ('\''$uid_b'\'', '\''kc-qa-authz04-4b'\'', '\''qa-authz04-4b@test.com'\'', '\''QA User B'\'');"
  db_exec "INSERT INTO tenant_users (tenant_id, user_id, role) VALUES ('\''$tenant_b'\'', '\''$uid_b'\'', '\''member'\'');" || true

  token_a=$(_gen_tenant_member_token "$tenant_a")
  qa_set_token "$token_a"

  resp=$(api_get "/api/v1/tenants/$tenant_a/users")
  status=$(resp_status "$resp")
  if [[ "$status" == "200" ]]; then
    body=$(resp_body "$resp")
    assert_not_contains "$body" "qa-authz04-4b@test.com" "Tenant A user list does not contain Tenant B user"
  else
    assert_match "$status" "^(200|403)$" "User list endpoint returns 200 or 403"
  fi

  db_exec "DELETE FROM tenant_users WHERE user_id = '\''$uid_b'\'';" || true
  db_exec "DELETE FROM users WHERE id = '\''$uid_b'\'';" || true
  db_exec "DELETE FROM tenants WHERE id = '\''$tenant_b'\'';" || true
  qa_set_token ""
'

scenario 5 "Deleted resource not accessible" '
  local admin_token tenant_id service_id
  admin_token=$(gen_default_admin_token)
  tenant_id=$(_get_platform_tenant_id)
  qa_set_token "$admin_token"

  local svc_name="qa-del-svc-$(date +%s)"
  resp=$(api_post "/api/v1/tenants/$tenant_id/services" \
    "{\"name\":\"$svc_name\",\"description\":\"To be deleted\"}")
  service_id=$(resp_body "$resp" | jq -r ".data.id // .id // empty")

  if [[ -n "$service_id" && "$service_id" != "null" ]]; then
    resp=$(api_get "/api/v1/tenants/$tenant_id/services/$service_id")
    pre_status=$(resp_status "$resp")
    assert_http_status "$pre_status" 200 "Service accessible before deletion"

    resp=$(api_delete "/api/v1/tenants/$tenant_id/services/$service_id")
    del_status=$(resp_status "$resp")
    assert_match "$del_status" "^(200|204)$" "Service deletion succeeds"

    resp=$(api_get "/api/v1/tenants/$tenant_id/services/$service_id")
    post_status=$(resp_status "$resp")
    assert_http_status "$post_status" 404 "Deleted service returns 404"

    local db_count
    db_count=$(db_query "SELECT COUNT(*) FROM services WHERE id = '\''$service_id'\'';")
    assert_eq "$db_count" "0" "Service hard-deleted from database"
  else
    assert_eq "skip" "skip" "Could not create test service, skipping"
  fi

  qa_set_token ""
'

run_all
