#!/usr/bin/env bash
# Security Auto Test: security/authentication/02-token-security
# Doc: docs/security/authentication/02-token-security.md
# Scenarios: 3
# ASVS 5.0: V9.1, V9.2, V9.3, V6.2
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/../lib/runner.sh"

require_bin jq
require_bin node

scenario 1 "JWT algorithm confusion - alg:none attack" '
  TOKEN=$(gen_default_admin_token)
  qa_set_token "$TOKEN"
  resp_ok=$(api_get /api/v1/tenants)
  assert_http_status "$(resp_status "$resp_ok")" 200 "valid token works"
  qa_set_token ""

  PAYLOAD=$(echo "$TOKEN" | cut -d. -f2)
  PADDED_PAYLOAD="$PAYLOAD"
  mod=$((${#PADDED_PAYLOAD} % 4))
  if [[ $mod -eq 2 ]]; then PADDED_PAYLOAD="${PADDED_PAYLOAD}=="; fi
  if [[ $mod -eq 3 ]]; then PADDED_PAYLOAD="${PADDED_PAYLOAD}="; fi

  NONE_HEADER=$(echo -n "{\"alg\":\"none\",\"typ\":\"JWT\"}" | base64 | tr -d "=" | tr "+/" "-_")
  FORGED_NONE="${NONE_HEADER}.${PAYLOAD}."

  qa_set_token "$FORGED_NONE"
  resp_none=$(api_get /api/v1/tenants)
  status_none=$(resp_status "$resp_none")
  assert_eq "$status_none" "401" "alg:none token rejected with 401"
  qa_set_token ""

  HS256_HEADER=$(echo -n "{\"alg\":\"HS256\",\"typ\":\"JWT\"}" | base64 | tr -d "=" | tr "+/" "-_")
  FORGED_HS256="${HS256_HEADER}.${PAYLOAD}.invalid-signature"

  qa_set_token "$FORGED_HS256"
  resp_hs256=$(api_get /api/v1/tenants)
  status_hs256=$(resp_status "$resp_hs256")
  assert_eq "$status_hs256" "401" "HS256 algorithm confusion rejected with 401"
  qa_set_token ""

  UNSUPPORTED_HEADER=$(echo -n "{\"alg\":\"ES512\",\"typ\":\"JWT\"}" | base64 | tr -d "=" | tr "+/" "-_")
  FORGED_UNSUPPORTED="${UNSUPPORTED_HEADER}.${PAYLOAD}.invalid-signature"

  qa_set_token "$FORGED_UNSUPPORTED"
  resp_unsupported=$(api_get /api/v1/tenants)
  status_unsupported=$(resp_status "$resp_unsupported")
  assert_eq "$status_unsupported" "401" "unsupported algorithm rejected with 401"
  qa_set_token ""
'

scenario 2 "JWT key exposure - JWKS only exposes public key" '
  jwks_resp=$(api_raw GET /.well-known/jwks.json)
  jwks_status=$(resp_status "$jwks_resp")
  jwks_body=$(resp_body "$jwks_resp")

  if [[ "$jwks_status" == "200" ]]; then
    assert_not_contains "$jwks_body" "\"d\"" "JWKS does not contain private exponent d"
    assert_not_contains "$jwks_body" "\"p\"" "JWKS does not contain prime p"
    assert_not_contains "$jwks_body" "\"q\"" "JWKS does not contain prime q"
    assert_not_contains "$jwks_body" "\"dp\"" "JWKS does not contain dp"
    assert_not_contains "$jwks_body" "\"dq\"" "JWKS does not contain dq"
    assert_not_contains "$jwks_body" "\"qi\"" "JWKS does not contain qi"

    if echo "$jwks_body" | jq -e ".keys[0]" >/dev/null 2>&1; then
      assert_json_exists "$jwks_body" ".keys[0].kty" "JWKS key has kty field"
      assert_json_exists "$jwks_body" ".keys[0].n" "JWKS key has public modulus n"
      assert_json_exists "$jwks_body" ".keys[0].e" "JWKS key has public exponent e"
    fi
  else
    assert_match "$jwks_status" "^(200|404)$" "JWKS endpoint responds"
  fi

  error_resp=$(api_raw GET /api/v1/nonexistent-endpoint-for-error-test)
  error_body=$(resp_body "$error_resp")
  assert_not_contains "$error_body" "private_key" "error response does not leak private key"
  assert_not_contains "$error_body" "JWT_SECRET" "error response does not leak JWT_SECRET"
  assert_not_contains "$error_body" "PRIVATE KEY" "error response does not leak PEM private key"
'

scenario 3 "Token claim tampering" '
  TOKEN=$(gen_default_admin_token)

  HEADER=$(echo "$TOKEN" | cut -d. -f1)
  PAYLOAD=$(echo "$TOKEN" | cut -d. -f2)
  SIGNATURE=$(echo "$TOKEN" | cut -d. -f3)

  PADDED="$PAYLOAD"
  mod=$((${#PADDED} % 4))
  if [[ $mod -eq 2 ]]; then PADDED="${PADDED}=="; fi
  if [[ $mod -eq 3 ]]; then PADDED="${PADDED}="; fi
  DECODED=$(echo "$PADDED" | base64 -d 2>/dev/null || echo "$PADDED" | base64 -D 2>/dev/null || echo "{}")

  TAMPERED_JSON=$(echo "$DECODED" | jq ". + {\"email\": \"hacker@evil.com\", \"roles\": [\"platform_admin\"], \"exp\": 9999999999}" 2>/dev/null || echo "$DECODED")
  TAMPERED_PAYLOAD=$(echo -n "$TAMPERED_JSON" | base64 | tr -d "=" | tr "+/" "-_" | tr -d "\n")

  TAMPERED_TOKEN="${HEADER}.${TAMPERED_PAYLOAD}.${SIGNATURE}"

  qa_set_token "$TAMPERED_TOKEN"
  resp=$(api_get /api/v1/tenants)
  status=$(resp_status "$resp")
  assert_eq "$status" "401" "tampered token rejected with 401"
  qa_set_token ""

  EMPTY_SIG_TOKEN="${HEADER}.${TAMPERED_PAYLOAD}."
  qa_set_token "$EMPTY_SIG_TOKEN"
  resp2=$(api_get /api/v1/tenants)
  status2=$(resp_status "$resp2")
  assert_eq "$status2" "401" "token with empty signature rejected with 401"
  qa_set_token ""

  TRUNCATED_TOKEN="${HEADER}.${PAYLOAD}"
  qa_set_token "$TRUNCATED_TOKEN"
  resp3=$(api_get /api/v1/tenants)
  status3=$(resp_status "$resp3")
  assert_eq "$status3" "401" "token with missing signature part rejected with 401"
  qa_set_token ""
'

run_all
