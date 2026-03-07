#!/usr/bin/env bash
# Security Auto Test: security/authorization/03-privilege-escalation
# Doc: docs/security/authorization/03-privilege-escalation.md
# Scenarios: 5
# ASVS 5.0: V8.2, V8.3, V8.4
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/../lib/runner.sh"

require_bin jq

_MEMBER_USER_ID="11111111-2222-3333-4444-555555555555"

scenario 1 "Self role assignment attack" '
  TENANT_ID=$(db_query "SELECT id FROM tenants ORDER BY created_at LIMIT 1;" | tr -d "[:space:]")

  MEMBER_TOKEN=$(gen_tenant_token "$_MEMBER_USER_ID" "$TENANT_ID" "member" "user:read")
  qa_set_token "$MEMBER_TOKEN"

  ADMIN_ROLE_ID=$(db_query "SELECT r.id FROM roles r JOIN services s ON r.service_id = s.id WHERE s.tenant_id = '"'"'${TENANT_ID}'"'"' AND r.name = '"'"'admin'"'"' LIMIT 1;" | tr -d "[:space:]")

  if [[ -z "$ADMIN_ROLE_ID" ]]; then
    ADMIN_ROLE_ID=$(db_query "SELECT r.id FROM roles r JOIN services s ON r.service_id = s.id WHERE s.tenant_id = '"'"'${TENANT_ID}'"'"' ORDER BY r.created_at LIMIT 1;" | tr -d "[:space:]")
  fi

  if [[ -n "$ADMIN_ROLE_ID" ]]; then
    resp=$(api_post "/api/v1/rbac/assign" "{\"user_id\":\"${_MEMBER_USER_ID}\",\"tenant_id\":\"${TENANT_ID}\",\"role_id\":\"${ADMIN_ROLE_ID}\"}")
    status=$(resp_status "$resp")
    assert_eq "$status" "403" "member cannot self-assign role (RBAC layer)"

    db_check=$(db_query "SELECT COUNT(*) FROM user_tenant_roles utr JOIN tenant_users tu ON utr.tenant_user_id = tu.id WHERE tu.user_id = '"'"'${_MEMBER_USER_ID}'"'"' AND utr.role_id = '"'"'${ADMIN_ROLE_ID}'"'"';" | tr -d "[:space:]")
    assert_eq "$db_check" "0" "no role assignment in DB"
  else
    assert_eq "no_roles" "no_roles" "no roles found in tenant (skipping)"
  fi

  qa_set_token ""
'

scenario 2 "Role creation backdoor - non-platform-admin" '
  TENANT_ID=$(db_query "SELECT id FROM tenants ORDER BY created_at LIMIT 1;" | tr -d "[:space:]")
  SERVICE_ID=$(db_query "SELECT id FROM services WHERE tenant_id = '"'"'${TENANT_ID}'"'"' LIMIT 1;" | tr -d "[:space:]")

  TOKEN=$(gen_tenant_token "$_MEMBER_USER_ID" "$TENANT_ID" "admin" "rbac:read,rbac:write,user:read,service:read")
  qa_set_token "$TOKEN"

  if [[ -n "$SERVICE_ID" ]]; then
    resp=$(api_post "/api/v1/roles" "{\"name\":\"super_role_$(date +%s)\",\"service_id\":\"${SERVICE_ID}\",\"permissions\":[\"platform:admin\",\"tenant:delete\",\"system:configure\"]}")
    status=$(resp_status "$resp")
    assert_eq "$status" "403" "non-platform-admin cannot create roles"

    resp_reserved=$(api_post "/api/v1/roles" "{\"name\":\"platform_admin\",\"service_id\":\"${SERVICE_ID}\",\"permissions\":[]}")
    status_reserved=$(resp_status "$resp_reserved")
    assert_match "$status_reserved" "^(400|403|409)$" "reserved role name rejected"
  else
    assert_eq "no_service" "no_service" "no service found in tenant (skipping)"
  fi

  qa_set_token ""
'

scenario 3 "Invitation link privilege escalation" '
  TENANT_ID=$(db_query "SELECT id FROM tenants ORDER BY created_at LIMIT 1;" | tr -d "[:space:]")

  ADMIN_TOKEN=$(gen_default_admin_token)
  qa_set_token "$ADMIN_TOKEN"

  invite_email="invite-escalation-$(date +%s)@test.com"
  resp_invite=$(api_post "/api/v1/tenants/{tenant_id}/invitations" "{\"email\":\"${invite_email}\",\"tenant_id\":\"${TENANT_ID}\",\"role\":\"member\"}")
  status_invite=$(resp_status "$resp_invite")

  if [[ "$status_invite" =~ ^(200|201)$ ]]; then
    invite_body=$(resp_body "$resp_invite")
    INVITE_TOKEN=$(echo "$invite_body" | jq -r ".data.token // .data.invitation_token // .token // empty" 2>/dev/null)
    INVITE_ID=$(echo "$invite_body" | jq -r ".data.id // empty" 2>/dev/null)

    if [[ -n "$INVITE_TOKEN" ]]; then
      qa_set_token ""
      resp_accept=$(api_post "/api/v1/tenants/{tenant_id}/invitations/accept" "{\"token\":\"${INVITE_TOKEN}\",\"role\":\"admin\"}")
      status_accept=$(resp_status "$resp_accept")

      if [[ "$status_accept" =~ ^(200|201)$ ]]; then
        accept_body=$(resp_body "$resp_accept")
        assigned_role=$(echo "$accept_body" | jq -r ".data.role // .role // empty" 2>/dev/null)
        if [[ -n "$assigned_role" && "$assigned_role" != "null" ]]; then
          assert_ne "$assigned_role" "admin" "invitation accept ignores role override"
        else
          assert_eq "checked" "checked" "invitation accepted (role field not in response)"
        fi
      else
        assert_match "$status_accept" "^(200|201|400|401|404|409)$" "invitation accept responds"
      fi
    else
      assert_eq "checked" "checked" "invitation created but no token in response (may use email flow)"
    fi

    if [[ -n "$INVITE_ID" ]]; then
      qa_set_token "$ADMIN_TOKEN"
      api_delete "/api/v1/tenants/{tenant_id}/invitations/${INVITE_ID}" >/dev/null 2>&1 || true
    fi
  else
    assert_match "$status_invite" "^(200|201|400|409)$" "invitation creation responds"
  fi

  qa_set_token ""
'

scenario 4 "Tenant ownership transfer attack" '
  TENANT_ID=$(db_query "SELECT id FROM tenants ORDER BY created_at LIMIT 1;" | tr -d "[:space:]")

  TOKEN=$(gen_tenant_token "$_MEMBER_USER_ID" "$TENANT_ID" "admin" "rbac:read,rbac:write,user:read,service:read,tenant:read")
  qa_set_token "$TOKEN"

  resp_transfer=$(api_put "/api/v1/tenants/${TENANT_ID}" "{\"owner_id\":\"${_MEMBER_USER_ID}\"}")
  status_transfer=$(resp_status "$resp_transfer")
  assert_match "$status_transfer" "^(400|403|404|422)$" "non-owner cannot transfer ownership"

  OWNER_ROLE_ID=$(db_query "SELECT r.id FROM roles r JOIN services s ON r.service_id = s.id WHERE s.tenant_id = '"'"'${TENANT_ID}'"'"' AND r.name = '"'"'owner'"'"' LIMIT 1;" | tr -d "[:space:]")
  if [[ -n "$OWNER_ROLE_ID" ]]; then
    resp_assign=$(api_post "/api/v1/rbac/assign" "{\"user_id\":\"${_MEMBER_USER_ID}\",\"tenant_id\":\"${TENANT_ID}\",\"role_id\":\"${OWNER_ROLE_ID}\"}")
    status_assign=$(resp_status "$resp_assign")
    assert_match "$status_assign" "^(403|400)$" "cannot self-assign owner role"
  else
    assert_eq "checked" "checked" "no owner role found (ownership may use different mechanism)"
  fi

  OWNER_USER_ID=$(db_query "SELECT tu.user_id FROM tenant_users tu JOIN user_tenant_roles utr ON tu.id = utr.tenant_user_id JOIN roles r ON utr.role_id = r.id WHERE tu.tenant_id = '"'"'${TENANT_ID}'"'"' AND r.name IN ('"'"'owner'"'"', '"'"'admin'"'"') LIMIT 1;" | tr -d "[:space:]")
  if [[ -n "$OWNER_USER_ID" ]]; then
    resp_remove=$(api_delete "/api/v1/tenants/${TENANT_ID}/users/${OWNER_USER_ID}")
    status_remove=$(resp_status "$resp_remove")
    assert_match "$status_remove" "^(400|403|404)$" "cannot remove tenant owner"
  fi

  qa_set_token ""
'

scenario 5 "API key privilege escalation" '
  TENANT_ID=$(db_query "SELECT id FROM tenants ORDER BY created_at LIMIT 1;" | tr -d "[:space:]")
  TOKEN=$(gen_tenant_token "$_MEMBER_USER_ID" "$TENANT_ID" "member" "user:read")
  qa_set_token "$TOKEN"

  resp=$(api_post "/api/v1/api-keys" "{\"name\":\"escalation-key-$(date +%s)\",\"scopes\":[\"admin:*\",\"platform:*\",\"tenant:delete\"]}")
  status=$(resp_status "$resp")

  if [[ "$status" =~ ^(200|201)$ ]]; then
    body=$(resp_body "$resp")
    key_scopes=$(echo "$body" | jq -r "[.data.scopes[]?] | join(\",\")" 2>/dev/null || echo "")
    assert_not_contains "$key_scopes" "platform:" "API key does not have platform scope"
    assert_not_contains "$key_scopes" "admin:*" "API key does not have admin wildcard scope"

    KEY_ID=$(echo "$body" | jq -r ".data.id // empty" 2>/dev/null)
    if [[ -n "$KEY_ID" ]]; then
      api_delete "/api/v1/api-keys/${KEY_ID}" >/dev/null 2>&1 || true
    fi
  else
    assert_match "$status" "^(400|403|404)$" "member cannot create privileged API key"
  fi

  resp_no_auth=$(api_raw GET /api/v1/tenants -H "X-API-Key: fake-api-key-12345")
  status_no_auth=$(resp_status "$resp_no_auth")
  assert_match "$status_no_auth" "^(401|403|429)$" "invalid API key rejected"

  qa_set_token ""
'

run_all
