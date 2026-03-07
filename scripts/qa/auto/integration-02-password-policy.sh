#!/usr/bin/env bash
# QA Auto Test: integration/02-password-policy
# Doc: docs/qa/integration/02-password-policy.md
# Scenarios: 5
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

scenario 1 "Password min length and character type enforcement" '
  TENANT_ID=$(qa_get_tenant_id)
  TOKEN=$(gen_token_for_tenant "$TENANT_ID")
  qa_set_token "$TOKEN"

  resp=$(api_put "/api/v1/tenants/${TENANT_ID}/password-policy" \
    "{\"min_length\":12,\"require_uppercase\":true,\"require_lowercase\":true,\"require_numbers\":true,\"require_symbols\":true}")
  policy_status=$(resp_status "$resp")
  assert_match "$policy_status" "^(200|204)$" "set password policy"

  UNIQUE_EMAIL="policy-$(date +%s)@example.com"

  resp=$(api_post "/api/v1/users" \
    "{\"email\":\"${UNIQUE_EMAIL}\",\"display_name\":\"Policy Test\",\"password\":\"password\",\"tenant_id\":\"${TENANT_ID}\"}")
  assert_match "$(resp_status "$resp")" "^(400|422)$" "weak password rejected"

  resp=$(api_post "/api/v1/users" \
    "{\"email\":\"${UNIQUE_EMAIL}\",\"display_name\":\"Policy Test\",\"password\":\"Password1\",\"tenant_id\":\"${TENANT_ID}\"}")
  assert_match "$(resp_status "$resp")" "^(400|422)$" "missing symbol rejected"

  resp=$(api_post "/api/v1/users" \
    "{\"email\":\"${UNIQUE_EMAIL}\",\"display_name\":\"Policy Test\",\"password\":\"Password!\",\"tenant_id\":\"${TENANT_ID}\"}")
  assert_match "$(resp_status "$resp")" "^(400|422)$" "missing number rejected"

  resp=$(api_post "/api/v1/users" \
    "{\"email\":\"${UNIQUE_EMAIL}\",\"display_name\":\"Policy Test\",\"password\":\"Pass1!\",\"tenant_id\":\"${TENANT_ID}\"}")
  assert_match "$(resp_status "$resp")" "^(400|422)$" "too short rejected"

  resp=$(api_post "/api/v1/users" \
    "{\"email\":\"${UNIQUE_EMAIL}\",\"display_name\":\"Policy Test\",\"password\":\"MySecurePass123!\",\"tenant_id\":\"${TENANT_ID}\"}")
  assert_http_status "$(resp_status "$resp")" 201 "valid password accepted"
  USER_ID=$(resp_body "$resp" | jq -r '"'"'.data.id // .id'"'"')

  DB_COUNT=$(db_query "SELECT COUNT(*) FROM users WHERE email='"'"'${UNIQUE_EMAIL}'"'"';" | tr -d '[:space:]')
  assert_eq "$DB_COUNT" "1" "exactly 1 user created in DB"

  db_exec "DELETE FROM tenant_users WHERE user_id='"'"'${USER_ID}'"'"';" || true
  db_exec "DELETE FROM users WHERE id='"'"'${USER_ID}'"'"';" || true
  qa_set_token ""
'

scenario 2 "Password history sync to Keycloak realm" '
  TENANT_ID=$(qa_get_tenant_id)
  TOKEN=$(gen_token_for_tenant "$TENANT_ID")
  qa_set_token "$TOKEN"

  resp=$(api_put "/api/v1/tenants/${TENANT_ID}/password-policy" \
    "{\"history_count\":5}")
  assert_match "$(resp_status "$resp")" "^(200|204)$" "set history_count=5"

  HISTORY_VAL=$(db_query "SELECT JSON_EXTRACT(password_policy, '"'"'$.history_count'"'"') FROM tenants WHERE id='"'"'${TENANT_ID}'"'"';" | tr -d '[:space:]')
  assert_eq "$HISTORY_VAL" "5" "DB password_policy.history_count = 5"

  KC_TOKEN=$(_get_kc_admin_token)
  if [[ -n "$KC_TOKEN" && "$KC_TOKEN" != "null" ]]; then
    KC_POLICY=$(curl -s "http://localhost:8081/admin/realms/auth9" \
      -H "Authorization: Bearer $KC_TOKEN" \
      | jq -r '"'"'.passwordPolicy // ""'"'"')
    assert_contains "$KC_POLICY" "passwordHistory" "Keycloak realm has passwordHistory policy"
  else
    echo "WARN: could not get Keycloak admin token, skipping realm check" >&2
  fi

  qa_set_token ""
'

scenario 3 "Password max age sync to Keycloak realm" '
  TENANT_ID=$(qa_get_tenant_id)
  TOKEN=$(gen_token_for_tenant "$TENANT_ID")
  qa_set_token "$TOKEN"

  resp=$(api_put "/api/v1/tenants/${TENANT_ID}/password-policy" \
    "{\"max_age_days\":90}")
  assert_match "$(resp_status "$resp")" "^(200|204)$" "set max_age_days=90"

  KC_TOKEN=$(_get_kc_admin_token)
  if [[ -n "$KC_TOKEN" && "$KC_TOKEN" != "null" ]]; then
    KC_POLICY=$(curl -s "http://localhost:8081/admin/realms/auth9" \
      -H "Authorization: Bearer $KC_TOKEN" \
      | jq -r '"'"'.passwordPolicy // ""'"'"')
    assert_contains "$KC_POLICY" "forceExpiredPasswordChange" "Keycloak has forceExpiredPasswordChange"
  else
    echo "WARN: could not get Keycloak admin token, skipping realm check" >&2
  fi

  qa_set_token ""
'

scenario 4 "Account lockout policy sync to Keycloak realm" '
  TENANT_ID=$(qa_get_tenant_id)
  TOKEN=$(gen_token_for_tenant "$TENANT_ID")
  qa_set_token "$TOKEN"

  resp=$(api_put "/api/v1/tenants/${TENANT_ID}/password-policy" \
    "{\"lockout_threshold\":5,\"lockout_duration_mins\":30}")
  assert_match "$(resp_status "$resp")" "^(200|204)$" "set lockout policy"

  KC_TOKEN=$(_get_kc_admin_token)
  if [[ -n "$KC_TOKEN" && "$KC_TOKEN" != "null" ]]; then
    REALM_JSON=$(curl -s "http://localhost:8081/admin/realms/auth9" \
      -H "Authorization: Bearer $KC_TOKEN")
    BF_ENABLED=$(echo "$REALM_JSON" | jq -r '"'"'.bruteForceProtected // false'"'"')
    FAILURE_FACTOR=$(echo "$REALM_JSON" | jq -r '"'"'.failureFactor // 0'"'"')

    assert_eq "$BF_ENABLED" "true" "bruteForceProtected is enabled"
    assert_eq "$FAILURE_FACTOR" "5" "failureFactor = 5"
  else
    echo "WARN: could not get Keycloak admin token, skipping realm check" >&2
  fi

  qa_set_token ""
'

scenario 5 "Admin bypass password policy via Auth9 API" '
  TENANT_ID=$(qa_get_tenant_id)
  TOKEN=$(gen_token_for_tenant "$TENANT_ID")
  qa_set_token "$TOKEN"

  resp=$(api_put "/api/v1/tenants/${TENANT_ID}/password-policy" \
    "{\"min_length\":12,\"require_uppercase\":true,\"require_lowercase\":true,\"require_numbers\":true,\"require_symbols\":true}")
  assert_match "$(resp_status "$resp")" "^(200|204)$" "set strict policy"

  BYPASS_EMAIL="admin-bypass-$(date +%s)@example.com"
  resp=$(api_post "/api/v1/users" \
    "{\"email\":\"${BYPASS_EMAIL}\",\"display_name\":\"Bypass Test\",\"password\":\"MySecurePass123!\",\"tenant_id\":\"${TENANT_ID}\"}")
  assert_http_status "$(resp_status "$resp")" 201 "create user with valid password"
  USER_ID=$(resp_body "$resp" | jq -r '"'"'.data.id // .id'"'"')

  resp=$(api_put "/api/v1/admin/users/${USER_ID}/password" \
    "{\"password\":\"Temp123!\",\"temporary\":true}")
  admin_set_status=$(resp_status "$resp")
  assert_match "$admin_set_status" "^(200|204)$" "admin set temp password bypasses policy"

  KC_TOKEN=$(_get_kc_admin_token)
  if [[ -n "$KC_TOKEN" && "$KC_TOKEN" != "null" ]]; then
    KC_USER_ID=$(db_query "SELECT keycloak_id FROM users WHERE id='"'"'${USER_ID}'"'"';" | tr -d '[:space:]')
    if [[ -n "$KC_USER_ID" ]]; then
      KC_USER=$(curl -s "http://localhost:8081/admin/realms/auth9/users/${KC_USER_ID}" \
        -H "Authorization: Bearer $KC_TOKEN")
      REQUIRED_ACTIONS=$(echo "$KC_USER" | jq -r '"'"'.requiredActions // [] | join(",")'"'"')
      assert_contains "$REQUIRED_ACTIONS" "UPDATE_PASSWORD" "Keycloak user has UPDATE_PASSWORD action"
    fi
  fi

  db_exec "DELETE FROM tenant_users WHERE user_id='"'"'${USER_ID}'"'"';" || true
  db_exec "DELETE FROM users WHERE id='"'"'${USER_ID}'"'"';" || true
  qa_set_token ""
'

run_all
