#!/usr/bin/env bash
# Security Auto Test: security/authentication/03-mfa-security
# Doc: docs/security/authentication/03-mfa-security.md
# Scenarios: 5
# ASVS: M-AUTH-03 | V6.7, V6.8, V7.3
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

scenario 1 "TOTP brute force protection" '
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
  assert_eq "$failure_factor" "5" "failureFactor is 5"

  max_delta=$(echo "$realm_config" | jq -r '"'"'.maxDeltaTimeSeconds // "null"'"'"')
  assert_ne "$max_delta" "null" "maxDeltaTimeSeconds is configured"

  wait_increment=$(echo "$realm_config" | jq -r '"'"'.waitIncrementSeconds // "null"'"'"')
  assert_ne "$wait_increment" "null" "waitIncrementSeconds is configured"
'

scenario 2 "TOTP time window attack prevention" '
  KC_TOKEN=$(_get_kc_token)

  if [[ -z "$KC_TOKEN" || "$KC_TOKEN" == "null" ]]; then
    assert_eq "fail" "pass" "cannot obtain Keycloak admin token"
    return
  fi

  realm_config=$(docker exec auth9-core curl -s \
    "http://keycloak:8080/admin/realms/auth9" \
    -H "Authorization: Bearer $KC_TOKEN" 2>/dev/null)

  otp_type=$(echo "$realm_config" | jq -r '"'"'.otpPolicyType // "null"'"'"')
  assert_eq "$otp_type" "totp" "OTP policy type is TOTP"

  otp_digits=$(echo "$realm_config" | jq -r '"'"'.otpPolicyDigits // "null"'"'"')
  assert_eq "$otp_digits" "6" "OTP digits is 6"

  otp_period=$(echo "$realm_config" | jq -r '"'"'.otpPolicyPeriod // "null"'"'"')
  assert_eq "$otp_period" "30" "OTP period is 30 seconds"

  otp_window=$(echo "$realm_config" | jq -r '"'"'.otpPolicyLookAheadWindow // "null"'"'"')
  assert_eq "$otp_window" "1" "OTP look-ahead window is 1"
'

scenario 3 "MFA bypass prevention" '
  TOKEN=$(gen_default_admin_token)
  qa_set_token "$TOKEN"

  resp=$(api_get "/api/v1/tenants")
  status=$(resp_status "$resp")
  assert_eq "$status" "200" "authenticated request with valid token succeeds"

  qa_set_token ""
  resp=$(api_get "/api/v1/tenants")
  status=$(resp_status "$resp")
  assert_eq "$status" "401" "unauthenticated request returns 401"

  qa_set_token "pre-mfa-fake-session-token"
  resp=$(api_get "/api/v1/tenants")
  status=$(resp_status "$resp")
  assert_eq "$status" "401" "pre-MFA session token rejected"

  qa_set_token ""
'

scenario 4 "MFA registration flow security" '
  TOKEN=$(gen_default_admin_token)
  qa_set_token "$TOKEN"

  resp=$(api_post "/api/v1/users/me/mfa" "{\"type\":\"totp\"}")
  status=$(resp_status "$resp")
  body=$(resp_body "$resp")
  assert_match "$status" "^(400|401|403|404|405)$" "MFA registration without password confirmation rejected or endpoint not exposed"

  qa_set_token ""

  resp=$(api_post "/api/v1/users/me/mfa" "{\"type\":\"totp\"}")
  status=$(resp_status "$resp")
  assert_match "$status" "^(401|403|404|405)$" "unauthenticated MFA registration rejected"
'

scenario 5 "MFA recovery mechanism security" '
  TOKEN=$(gen_default_admin_token)
  qa_set_token "$TOKEN"

  resp=$(api_raw DELETE "/api/v1/users/non-existent-user-id/mfa")
  status=$(resp_status "$resp")
  assert_match "$status" "^(400|401|403|404|405)$" "admin disable other user MFA requires proper authorization"

  qa_set_token ""
  resp=$(api_raw DELETE "/api/v1/users/some-user-id/mfa")
  status=$(resp_status "$resp")
  assert_match "$status" "^(401|403|404|405)$" "unauthenticated MFA disable rejected"
'

run_all
