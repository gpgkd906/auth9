#!/usr/bin/env bash
# Security Auto Test: security/business-logic/03-admin-operational-endpoint-abuse
# Doc: docs/security/business-logic/03-admin-operational-endpoint-abuse.md
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

_gen_member_token() {
  local tenant_id="$1"
  local uid="16daa93d-06e8-479c-867d-f9b6184e06c7"
  local email="member-qa-test@test.com"
  node -e '
const jwt=require("jsonwebtoken"),fs=require("fs");
const pk=fs.readFileSync(process.argv[1],"utf8");
const now=Math.floor(Date.now()/1000);
process.stdout.write(jwt.sign({
  sub:process.argv[2],email:process.argv[3],
  iss:"http://localhost:8080",aud:"auth9-portal",token_type:"access",
  tenant_id:process.argv[4],roles:["member"],permissions:[],
  iat:now,exp:now+3600
},pk,{algorithm:"RS256",keyid:"auth9-current"}));
' "$_JWT_PRIVATE_KEY" "$uid" "$email" "$tenant_id" 2>/dev/null
}

scenario 1 "Normal user cannot force logout other users" '
  local tenant_id victim_uid
  tenant_id=$(_get_platform_tenant_id)
  victim_uid=$(db_query "SELECT LOWER(UUID());")

  db_exec "DELETE FROM users WHERE keycloak_id = '\''kc-qa-admin03-victim1'\'';" || true
  db_exec "INSERT INTO users (id, keycloak_id, email, display_name) VALUES ('\''$victim_uid'\'', '\''kc-qa-admin03-victim1'\'', '\''qa-admin03-victim1@test.com'\'', '\''QA Victim'\'');"
  db_exec "INSERT INTO sessions (id, user_id, device_type, ip_address, location, last_active_at) VALUES (LOWER(UUID()), '\''$victim_uid'\'', '\''desktop'\'', '\''192.168.1.1'\'', '\''Test'\'', NOW());"

  local member_token
  member_token=$(_gen_member_token "$tenant_id")
  qa_set_token "$member_token"

  resp=$(api_post "/api/v1/admin/users/$victim_uid/logout" "{}")
  status=$(resp_status "$resp")
  assert_http_status "$status" 403 "Normal user cannot force logout other users"

  local active_count
  active_count=$(db_query "SELECT COUNT(*) FROM sessions WHERE user_id = '\''$victim_uid'\'' AND revoked_at IS NULL;")
  assert_eq "$active_count" "1" "Victim session still active after unauthorized logout attempt"

  db_exec "DELETE FROM sessions WHERE user_id = '\''$victim_uid'\'';" || true
  db_exec "DELETE FROM users WHERE id = '\''$victim_uid'\'';" || true
  qa_set_token ""
'

scenario 2 "Normal user cannot read audit logs" '
  local tenant_id member_token
  tenant_id=$(_get_platform_tenant_id)
  member_token=$(_gen_member_token "$tenant_id")

  qa_set_token "$member_token"
  resp=$(api_get "/api/v1/audit-logs?limit=20")
  status=$(resp_status "$resp")
  assert_http_status "$status" 403 "Normal user cannot access audit logs"
  qa_set_token ""
'

scenario 3 "Normal user cannot read/resolve security alerts" '
  local tenant_id member_token
  tenant_id=$(_get_platform_tenant_id)
  member_token=$(_gen_member_token "$tenant_id")

  qa_set_token "$member_token"

  resp=$(api_get "/api/v1/security/alerts?page=1&per_page=20")
  status=$(resp_status "$resp")
  assert_http_status "$status" 403 "Normal user cannot list security alerts"

  resp2=$(api_post "/api/v1/security/alerts/00000000-0000-0000-0000-000000000001/resolve" "{}")
  status2=$(resp_status "$resp2")
  assert_match "$status2" "^(403|404)$" "Normal user cannot resolve security alerts"

  qa_set_token ""
'

scenario 4 "Cross-tenant service toggle denied" '
  local tenant_a tenant_b member_token_a
  tenant_a=$(_get_platform_tenant_id)
  tenant_b=$(db_query "SELECT id FROM tenants WHERE slug != 'auth9-platform' LIMIT 1;" || echo "")

  if [[ -z "$tenant_b" ]]; then
    tenant_b=$(db_query "SELECT LOWER(UUID());")
    db_exec "INSERT INTO tenants (id, name, slug, status) VALUES ('\''$tenant_b'\'', '\''QA Cross Tenant B'\'', '\''qa-cross-tenant-b'\'', '\''active'\'');"
  fi

  member_token_a=$(_gen_member_token "$tenant_a")
  qa_set_token "$member_token_a"

  local service_id
  service_id=$(db_query "SELECT id FROM services WHERE tenant_id = '\''$tenant_b'\'' LIMIT 1;" || echo "")
  local test_service_id="${service_id:-00000000-0000-0000-0000-000000000099}"

  resp=$(api_post "/api/v1/tenants/$tenant_b/services" \
    "{\"service_id\":\"$test_service_id\",\"enabled\":false}")
  status=$(resp_status "$resp")
  assert_match "$status" "^(403|404|400|422)$" "Cross-tenant service toggle denied"

  db_exec "DELETE FROM tenants WHERE slug = '\''qa-cross-tenant-b'\'';" || true
  qa_set_token ""
'

scenario 5 "Cross-tenant webhook tampering denied" '
  local tenant_a tenant_b member_token_a
  tenant_a=$(_get_platform_tenant_id)
  tenant_b=$(db_query "SELECT id FROM tenants WHERE slug != 'auth9-platform' LIMIT 1;" || echo "")

  if [[ -z "$tenant_b" ]]; then
    tenant_b=$(db_query "SELECT LOWER(UUID());")
    db_exec "INSERT INTO tenants (id, name, slug, status) VALUES ('\''$tenant_b'\'', '\''QA Cross WH B'\'', '\''qa-cross-wh-b'\'', '\''active'\'');"
  fi

  member_token_a=$(_gen_member_token "$tenant_a")
  qa_set_token "$member_token_a"

  local fake_wh_id="00000000-0000-0000-0000-000000000088"

  resp=$(api_put "/api/v1/tenants/$tenant_b/webhooks/$fake_wh_id" \
    "{\"name\":\"hijacked\",\"url\":\"https://attacker.example/webhook\",\"events\":[\"user.created\"],\"enabled\":true}")
  status=$(resp_status "$resp")
  assert_match "$status" "^(403|404)$" "Cross-tenant webhook update denied"

  resp2=$(api_delete "/api/v1/tenants/$tenant_b/webhooks/$fake_wh_id")
  status2=$(resp_status "$resp2")
  assert_match "$status2" "^(403|404)$" "Cross-tenant webhook delete denied"

  resp3=$(api_post "/api/v1/tenants/$tenant_b/webhooks/$fake_wh_id/regenerate-secret" "{}")
  status3=$(resp_status "$resp3")
  assert_match "$status3" "^(403|404)$" "Cross-tenant webhook secret regeneration denied"

  db_exec "DELETE FROM tenants WHERE slug = '\''qa-cross-wh-b'\'';" || true
  qa_set_token ""
'

run_all
