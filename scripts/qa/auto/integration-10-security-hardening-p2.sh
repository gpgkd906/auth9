#!/usr/bin/env bash
# QA Auto Test: integration/10-security-hardening-p2
# Doc: docs/qa/integration/10-security-hardening-p2.md
# Scenarios: 4
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/../lib/runner.sh"

require_bin jq

_lookup_demo_tenant() {
  db_query "SELECT id FROM tenants WHERE slug='demo' LIMIT 1;" | tr -d '[:space:]'
}

_get_kc_admin_token() {
  curl -s -X POST "http://localhost:8081/realms/master/protocol/openid-connect/token" \
    -d "grant_type=password&client_id=admin-cli&username=admin&password=admin" \
    | jq -r '.access_token'
}

_auth9_core_bin() {
  local bin="${AUTH9_CORE_BIN:-}"
  if [[ -z "$bin" ]]; then
    if [[ -f "$_QA_PROJECT_ROOT/auth9-core/target/debug/auth9-core" ]]; then
      bin="$_QA_PROJECT_ROOT/auth9-core/target/debug/auth9-core"
    elif [[ -f "$_QA_PROJECT_ROOT/auth9-core/target/release/auth9-core" ]]; then
      bin="$_QA_PROJECT_ROOT/auth9-core/target/release/auth9-core"
    fi
  fi
  echo "$bin"
}

scenario 1 "User delete - cascade atomicity" '
  TENANT_ID=$(_lookup_demo_tenant)
  if [[ -z "$TENANT_ID" ]]; then
    echo "No demo tenant found" >&2
    return 1
  fi

  TOKEN=$(gen_default_admin_token)
  qa_set_token "$TOKEN"

  DEL_EMAIL="cascade-user-$(date +%s)@example.com"
  resp=$(api_post "/api/v1/users" \
    "{\"email\":\"${DEL_EMAIL}\",\"display_name\":\"Cascade Test\",\"password\":\"Test123!\",\"tenant_id\":\"${TENANT_ID}\"}")
  assert_http_status "$(resp_status "$resp")" 201 "create user for cascade test"
  USER_ID=$(resp_body "$resp" | jq -r '"'"'.data.id // .id'"'"')

  TU_COUNT=$(db_query "SELECT COUNT(*) FROM tenant_users WHERE user_id='"'"'${USER_ID}'"'"';" | tr -d '[:space:]')
  assert_ne "$TU_COUNT" "0" "tenant_users record exists before delete"

  resp=$(api_delete "/api/v1/tenants/${TENANT_ID}/users/${USER_ID}")
  assert_match "$(resp_status "$resp")" "^(200|204)$" "delete user returns success"

  sleep 1

  assert_db "SELECT COUNT(*) FROM users WHERE id='"'"'${USER_ID}'"'"';" "0" "user deleted from users table"
  assert_db "SELECT COUNT(*) FROM tenant_users WHERE user_id='"'"'${USER_ID}'"'"';" "0" "tenant_users cleaned up"
  assert_db "SELECT COUNT(*) FROM sessions WHERE user_id='"'"'${USER_ID}'"'"';" "0" "sessions cleaned up"
  assert_db "SELECT COUNT(*) FROM login_events WHERE user_id='"'"'${USER_ID}'"'"';" "0" "login_events cleaned up"
  assert_db "SELECT COUNT(*) FROM security_alerts WHERE user_id='"'"'${USER_ID}'"'"';" "0" "security_alerts cleaned up"

  qa_set_token ""
'

scenario 2 "Tenant delete - cascade atomicity" '
  TOKEN=$(gen_default_admin_token)
  qa_set_token "$TOKEN"

  SLUG="cascade-tenant-$(date +%s)"
  resp=$(api_post "/api/v1/tenants" \
    "{\"name\":\"Cascade Tenant Test\",\"slug\":\"${SLUG}\"}")
  assert_http_status "$(resp_status "$resp")" 201 "create tenant for cascade test"
  TENANT_ID=$(resp_body "$resp" | jq -r '"'"'.data.id // .id'"'"')

  resp=$(api_post "/api/v1/tenants/${TENANT_ID}/services" \
    "{\"name\":\"Cascade Service\",\"description\":\"test\"}")
  svc_status=$(resp_status "$resp")
  if [[ "$svc_status" == "201" || "$svc_status" == "200" ]]; then
    SVC_ID=$(resp_body "$resp" | jq -r '"'"'.data.id // .id'"'"')
  fi

  SVC_COUNT=$(db_query "SELECT COUNT(*) FROM services WHERE tenant_id='"'"'${TENANT_ID}'"'"';" | tr -d '[:space:]')

  resp=$(api_delete "/api/v1/tenants/${TENANT_ID}")
  assert_match "$(resp_status "$resp")" "^(200|204)$" "delete tenant returns success"

  sleep 1

  assert_db "SELECT COUNT(*) FROM tenants WHERE id='"'"'${TENANT_ID}'"'"';" "0" "tenant deleted"
  assert_db "SELECT COUNT(*) FROM services WHERE tenant_id='"'"'${TENANT_ID}'"'"';" "0" "services cleaned up"
  assert_db "SELECT COUNT(*) FROM tenant_users WHERE tenant_id='"'"'${TENANT_ID}'"'"';" "0" "tenant_users cleaned up"
  assert_db "SELECT COUNT(*) FROM webhooks WHERE tenant_id='"'"'${TENANT_ID}'"'"';" "0" "webhooks cleaned up"
  assert_db "SELECT COUNT(*) FROM invitations WHERE tenant_id='"'"'${TENANT_ID}'"'"';" "0" "invitations cleaned up"

  qa_set_token ""
'

scenario 3 "Delete syncs to Keycloak" '
  TENANT_ID=$(_lookup_demo_tenant)
  if [[ -z "$TENANT_ID" ]]; then
    echo "No demo tenant found" >&2
    return 1
  fi

  TOKEN=$(gen_default_admin_token)
  qa_set_token "$TOKEN"

  SYNC_EMAIL="kc-sync-del-$(date +%s)@example.com"
  resp=$(api_post "/api/v1/users" \
    "{\"email\":\"${SYNC_EMAIL}\",\"display_name\":\"KC Sync Del\",\"password\":\"Test123!\",\"tenant_id\":\"${TENANT_ID}\"}")
  assert_http_status "$(resp_status "$resp")" 201 "create user for KC sync test"
  USER_ID=$(resp_body "$resp" | jq -r '"'"'.data.id // .id'"'"')

  KC_ID=$(db_query "SELECT keycloak_id FROM users WHERE id='"'"'${USER_ID}'"'"';" | tr -d '[:space:]')

  resp=$(api_delete "/api/v1/tenants/${TENANT_ID}/users/${USER_ID}")
  assert_match "$(resp_status "$resp")" "^(200|204)$" "delete user returns success"

  sleep 2

  assert_db "SELECT COUNT(*) FROM users WHERE id='"'"'${USER_ID}'"'"';" "0" "user deleted from DB"

  if [[ -n "$KC_ID" ]]; then
    KC_TOKEN=$(_get_kc_admin_token)
    if [[ -n "$KC_TOKEN" && "$KC_TOKEN" != "null" ]]; then
      kc_status=$(curl -s -o /dev/null -w "%{http_code}" \
        "http://localhost:8081/admin/realms/auth9/users/${KC_ID}" \
        -H "Authorization: Bearer $KC_TOKEN")
      assert_eq "$kc_status" "404" "user deleted from Keycloak"
    else
      echo "WARN: could not get Keycloak admin token" >&2
    fi
  else
    echo "WARN: no keycloak_id for user, skipping Keycloak check" >&2
  fi

  qa_set_token ""
'

scenario 4 "Production rejects missing KEYCLOAK_WEBHOOK_SECRET" '
  BIN=$(_auth9_core_bin)
  if [[ -z "$BIN" ]]; then
    skip_scenario 4 "Missing webhook secret" "auth9-core binary not found"
    return 0
  fi

  local db_url="mysql://${MYSQL_USER}@${MYSQL_HOST}:${MYSQL_PORT}/${MYSQL_DB}"

  OUTPUT=$(ENVIRONMENT=production \
    DATABASE_URL="$db_url" \
    JWT_SECRET="test-secret" \  # pragma: allowlist secret
    GRPC_AUTH_MODE=api_key \
    GRPC_API_KEYS="test-key" \  # pragma: allowlist secret
    JWT_TENANT_ACCESS_ALLOWED_AUDIENCES="auth9-portal" \
    KEYCLOAK_WEBHOOK_SECRET="" \
    timeout 10 "$BIN" serve 2>&1) || true

  assert_match "$OUTPUT" "(KEYCLOAK_WEBHOOK_SECRET|webhook.*secret)" "error mentions webhook secret"
  assert_contains "$OUTPUT" "production" "error mentions production"
'

run_all
