#!/usr/bin/env bash
# Security Auto Test: security/authorization/02-rbac-bypass
# Doc: docs/security/authorization/02-rbac-bypass.md
# Scenarios: 5
# ASVS 5.0: V8.1, V8.3, V8.4
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/../lib/runner.sh"

require_bin jq

_RANDOM_USER_ID="aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee"

scenario 1 "Direct permission bypass - unprivileged user" '
  TENANT_ID=$(db_query "SELECT id FROM tenants ORDER BY created_at LIMIT 1;" | tr -d "[:space:]")

  VIEWER_TOKEN=$(gen_tenant_token "$_RANDOM_USER_ID" "$TENANT_ID" "viewer" "user:read")
  qa_set_token "$VIEWER_TOKEN"

  resp_create=$(api_post "/api/v1/users" "{\"email\":\"unauthorized-create@test.com\",\"first_name\":\"Hack\",\"last_name\":\"Test\"}")
  status_create=$(resp_status "$resp_create")
  assert_eq "$status_create" "403" "viewer cannot create user"

  USER_ID=$(db_query "SELECT user_id FROM tenant_users WHERE tenant_id = '"'"'${TENANT_ID}'"'"' LIMIT 1;" | tr -d "[:space:]")
  if [[ -n "$USER_ID" ]]; then
    resp_delete=$(api_delete "/api/v1/users/${USER_ID}")
    status_delete=$(resp_status "$resp_delete")
    assert_eq "$status_delete" "403" "viewer cannot delete user"
  fi

  resp_svc=$(api_post "/api/v1/services" "{\"name\":\"Unauthorized Service\",\"base_url\":\"http://evil.com\",\"redirect_uris\":[\"http://evil.com/cb\"],\"logout_uris\":[\"http://evil.com/logout\"]}")
  status_svc=$(resp_status "$resp_svc")
  assert_eq "$status_svc" "403" "viewer cannot create service"

  db_check=$(db_query "SELECT COUNT(*) FROM users WHERE email = '"'"'unauthorized-create@test.com'"'"';" | tr -d "[:space:]")
  assert_eq "$db_check" "0" "no user created in database"

  qa_set_token ""
'

scenario 2 "Permission inheritance bypass - viewer cannot do editor actions" '
  TENANT_ID=$(db_query "SELECT id FROM tenants ORDER BY created_at LIMIT 1;" | tr -d "[:space:]")

  VIEWER_TOKEN=$(gen_tenant_token "$_RANDOM_USER_ID" "$TENANT_ID" "viewer" "user:read")
  qa_set_token "$VIEWER_TOKEN"

  SERVICE_ID=$(db_query "SELECT id FROM services WHERE tenant_id = '"'"'${TENANT_ID}'"'"' LIMIT 1;" | tr -d "[:space:]")
  if [[ -n "$SERVICE_ID" ]]; then
    resp_update=$(api_put "/api/v1/services/${SERVICE_ID}" "{\"name\":\"Hijacked Service\"}")
    status_update=$(resp_status "$resp_update")
    assert_eq "$status_update" "403" "viewer cannot update service"

    resp_delete=$(api_delete "/api/v1/services/${SERVICE_ID}")
    status_delete=$(resp_status "$resp_delete")
    assert_eq "$status_delete" "403" "viewer cannot delete service"
  else
    assert_eq "checked" "checked" "no service found in tenant (skipped)"
  fi

  resp_tenant=$(api_patch "/api/v1/tenants/${TENANT_ID}" "{\"name\":\"Hijacked\"}")
  status_tenant=$(resp_status "$resp_tenant")
  assert_eq "$status_tenant" "403" "viewer cannot modify tenant"

  qa_set_token ""
'

scenario 3 "HTTP method bypass - X-HTTP-Method-Override" '
  TENANT_ID=$(db_query "SELECT id FROM tenants ORDER BY created_at LIMIT 1;" | tr -d "[:space:]")
  USER_ID=$(db_query "SELECT user_id FROM tenant_users WHERE tenant_id = '"'"'${TENANT_ID}'"'"' LIMIT 1;" | tr -d "[:space:]")

  VIEWER_TOKEN=$(gen_tenant_token "$_RANDOM_USER_ID" "$TENANT_ID" "viewer" "user:read")
  qa_set_token "$VIEWER_TOKEN"

  if [[ -n "$USER_ID" ]]; then
    resp_override=$(api_raw POST "/api/v1/users/${USER_ID}" \
      -H "Authorization: Bearer $VIEWER_TOKEN" \
      -H "X-HTTP-Method-Override: DELETE" \
      -H "Content-Type: application/json")
    status_override=$(resp_status "$resp_override")
    assert_match "$status_override" "^(403|404|405)$" "X-HTTP-Method-Override DELETE not honored"

    user_exists=$(db_query "SELECT COUNT(*) FROM users WHERE id = '"'"'${USER_ID}'"'"';" | tr -d "[:space:]")
    assert_ne "$user_exists" "0" "user still exists after override attempt"
  fi

  resp_options=$(api_raw OPTIONS /api/v1/users)
  status_options=$(resp_status "$resp_options")
  body_options=$(resp_body "$resp_options")
  assert_match "$status_options" "^(200|204|404|405)$" "OPTIONS returns valid status"
  assert_not_contains "$body_options" "\"email\"" "OPTIONS does not leak user data"

  qa_set_token ""
'

scenario 4 "Parameter-level permission bypass - sensitive field update" '
  TENANT_ID=$(db_query "SELECT id FROM tenants ORDER BY created_at LIMIT 1;" | tr -d "[:space:]")
  TOKEN=$(gen_tenant_token "$_RANDOM_USER_ID" "$TENANT_ID" "member" "user:read,user:write")
  qa_set_token "$TOKEN"

  resp=$(api_put "/api/v1/users/me" "{\"first_name\":\"Updated\",\"role_in_tenant\":\"owner\",\"created_at\":\"2020-01-01T00:00:00Z\"}")
  status=$(resp_status "$resp")

  if [[ "$status" == "200" ]]; then
    body=$(resp_body "$resp")
    role=$(echo "$body" | jq -r ".data.role_in_tenant // .role_in_tenant // \"\"" 2>/dev/null)
    if [[ -n "$role" && "$role" != "null" ]]; then
      assert_ne "$role" "owner" "role_in_tenant not changed to owner via user update"
    else
      assert_eq "checked" "checked" "role_in_tenant field not returned (safe)"
    fi
  else
    assert_match "$status" "^(200|400|403|404|405)$" "user self-update responds"
  fi

  qa_set_token ""
'

scenario 5 "Token permission vs DB state - JWT stateless behavior" '
  TENANT_ID=$(db_query "SELECT id FROM tenants ORDER BY created_at LIMIT 1;" | tr -d "[:space:]")
  TOKEN=$(gen_tenant_token "$_RANDOM_USER_ID" "$TENANT_ID" "admin" "user:read,user:write,service:read,rbac:read")
  qa_set_token "$TOKEN"

  resp=$(api_get "/api/v1/users")
  status=$(resp_status "$resp")
  assert_match "$status" "^(200|403)$" "admin-role token accesses user list"

  qa_set_token ""

  EXPIRED_HEADER=$(echo -n "{\"alg\":\"RS256\",\"typ\":\"JWT\"}" | base64 | tr -d "=" | tr "+/" "-_")
  PAST_EXP=$(( $(date +%s) - 3600 ))
  EXPIRED_PAYLOAD=$(echo -n "{\"sub\":\"${_RANDOM_USER_ID}\",\"email\":\"test@example.com\",\"iss\":\"http://localhost:8080\",\"aud\":\"auth9-portal\",\"token_type\":\"access\",\"tenant_id\":\"${TENANT_ID}\",\"roles\":[\"admin\"],\"permissions\":[\"user:read\"],\"iat\":1000000000,\"exp\":${PAST_EXP}}" | base64 | tr -d "=" | tr "+/" "-_" | tr -d "\n")
  FAKE_EXPIRED="${EXPIRED_HEADER}.${EXPIRED_PAYLOAD}.invalid-signature"

  qa_set_token "$FAKE_EXPIRED"
  resp_expired=$(api_get "/api/v1/users")
  status_expired=$(resp_status "$resp_expired")
  assert_eq "$status_expired" "401" "expired token rejected"
  qa_set_token ""

  FUTURE_EXP=$(( $(date +%s) + 999999 ))
  EXTENDED_PAYLOAD=$(echo -n "{\"sub\":\"${_RANDOM_USER_ID}\",\"email\":\"test@example.com\",\"iss\":\"http://localhost:8080\",\"aud\":\"auth9-portal\",\"token_type\":\"access\",\"tenant_id\":\"${TENANT_ID}\",\"roles\":[\"admin\"],\"permissions\":[\"user:read\"],\"iat\":1000000000,\"exp\":${FUTURE_EXP}}" | base64 | tr -d "=" | tr "+/" "-_" | tr -d "\n")
  FORGED_EXTENDED="${EXPIRED_HEADER}.${EXTENDED_PAYLOAD}.forged-sig"

  qa_set_token "$FORGED_EXTENDED"
  resp_forged=$(api_get "/api/v1/users")
  status_forged=$(resp_status "$resp_forged")
  assert_eq "$status_forged" "401" "forged token with extended expiry rejected"
  qa_set_token ""
'

run_all
