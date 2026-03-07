#!/usr/bin/env bash
# Security Auto Test: security/authentication/04-password-security
# Doc: docs/security/authentication/04-password-security.md
# Scenarios: 4
# ASVS: M-AUTH-04 | V6.1, V6.2, V6.3, V6.6
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/../lib/runner.sh"

require_bin jq

_get_kc_token() {
  docker exec auth9-core curl -s -X POST \
    "http://keycloak:8080/realms/master/protocol/openid-connect/token" \
    -d "client_id=admin-cli" -d "username=admin" -d "password=admin" \
    -d "grant_type=password" 2>/dev/null | jq -r '.access_token'
}

scenario 1 "Password brute force protection" '
  KC_TOKEN=$(_get_kc_token)

  if [[ -z "$KC_TOKEN" || "$KC_TOKEN" == "null" ]]; then
    assert_eq "fail" "pass" "cannot obtain Keycloak admin token (is Docker running?)"
    return
  fi

  realm_config=$(docker exec auth9-core curl -s \
    "http://keycloak:8080/admin/realms/auth9" \
    -H "Authorization: Bearer $KC_TOKEN" 2>/dev/null)

  brute_force=$(echo "$realm_config" | jq -r '"'"'.bruteForceProtected // "null"'"'"')
  assert_eq "$brute_force" "true" "bruteForceProtected is enabled"

  failure_factor=$(echo "$realm_config" | jq -r '"'"'.failureFactor // "null"'"'"')
  assert_eq "$failure_factor" "5" "failureFactor is 5 (lock after 5 failed attempts)"

  max_wait=$(echo "$realm_config" | jq -r '"'"'.maxFailureWaitSeconds // "null"'"'"')
  assert_ne "$max_wait" "null" "maxFailureWaitSeconds is configured"

  wait_increment=$(echo "$realm_config" | jq -r '"'"'.waitIncrementSeconds // "null"'"'"')
  assert_ne "$wait_increment" "null" "waitIncrementSeconds is configured"

  # Verify error responses do not leak user existence
  resp=$(api_post "/api/v1/auth/forgot-password" "{\"email\":\"existing@test.com\"}")
  status1=$(resp_status "$resp")
  body1=$(resp_body "$resp")

  resp=$(api_post "/api/v1/auth/forgot-password" "{\"email\":\"nonexistent-xyz-abc@test.com\"}")
  status2=$(resp_status "$resp")
  body2=$(resp_body "$resp")

  assert_eq "$status1" "$status2" "forgot-password returns same status for existing and non-existing email"
'

scenario 2 "Password reset flow security" '
  # Verify consistent response for existing vs non-existing email
  resp=$(api_post "/api/v1/auth/forgot-password" "{\"email\":\"admin@test.com\"}")
  status=$(resp_status "$resp")
  assert_match "$status" "^(200|202|204|400|404)$" "forgot-password endpoint responds"

  resp=$(api_post "/api/v1/auth/forgot-password" "{\"email\":\"nobody-at-all-xyz@test.com\"}")
  status2=$(resp_status "$resp")
  assert_eq "$status" "$status2" "same response for non-existing email (no user enumeration)"

  # Token reuse test
  resp=$(api_post "/api/v1/auth/reset-password" \
    "{\"token\":\"used-or-invalid-token\",\"new_password\":\"NewPass123!\"}")
  status=$(resp_status "$resp")
  assert_match "$status" "^(400|401|404|405)$" "invalid/used reset token rejected"
'

scenario 3 "Password storage security" '
  KC_TOKEN=$(_get_kc_token)

  if [[ -z "$KC_TOKEN" || "$KC_TOKEN" == "null" ]]; then
    assert_eq "fail" "pass" "cannot obtain Keycloak admin token"
    return
  fi

  realm_config=$(docker exec auth9-core curl -s \
    "http://keycloak:8080/admin/realms/auth9" \
    -H "Authorization: Bearer $KC_TOKEN" 2>/dev/null)

  password_policy=$(echo "$realm_config" | jq -r '"'"'.passwordPolicy // "null"'"'"')
  assert_ne "$password_policy" "null" "password policy is configured"

  if [[ "$password_policy" != "null" ]]; then  # pragma: allowlist secret
    assert_contains "$password_policy" "hashAlgorithm" "password policy specifies hash algorithm"
    assert_contains "$password_policy" "hashIterations" "password policy specifies hash iterations"
  fi
'

scenario 4 "Password change security" '
  TOKEN=$(gen_default_admin_token)
  qa_set_token "$TOKEN"

  # Attempt password change without current password
  resp=$(api_put "/api/v1/users/me/password" "{\"new_password\":\"NewPass123!\"}")
  status=$(resp_status "$resp")
  assert_match "$status" "^(400|401|403|404|405)$" "password change without current password rejected or endpoint not exposed"

  qa_set_token ""

  # Unauthenticated password change attempt
  resp=$(api_put "/api/v1/users/me/password" \
    "{\"current_password\":\"OldPass\",\"new_password\":\"NewPass123!\"}")
  status=$(resp_status "$resp")
  assert_match "$status" "^(401|403|404|405)$" "unauthenticated password change rejected"
'

run_all
