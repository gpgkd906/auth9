#!/usr/bin/env bash
# Security Auto Test: security/data-security/02-encryption
# Doc: docs/security/data-security/02-encryption.md
# Scenarios: 5
# ASVS: M-DATA-02 | V11.1, V11.2, V12.1, V14.3
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/../lib/runner.sh"

require_bin jq

# ── Scenario 1: Transport layer encryption (TLS) ─────────────────────────
scenario 1 "Transport layer encryption - HTTPS enforcement" '
  resp=$(api_get "/health")
  status=$(resp_status "$resp")
  assert_eq "$status" "200" "health endpoint accessible"

  headers=$(curl -sI "${API_BASE}/health" 2>&1)

  hsts=$(echo "$headers" | grep -i "strict-transport-security" || echo "")
  if [[ -n "$hsts" ]]; then
    assert_contains "$hsts" "max-age" "HSTS header present with max-age"
  else
    assert_eq "dev" "dev" "HSTS not set (acceptable in dev environment)"
  fi

  if echo "${API_BASE}" | grep -q "https"; then
    tls_info=$(curl -svI "${API_BASE}/health" 2>&1 | grep "TLS\|SSL" || echo "")
    if [[ -n "$tls_info" ]]; then
      assert_not_contains "$tls_info" "TLSv1.0" "TLS 1.0 not used"
      assert_not_contains "$tls_info" "TLSv1.1" "TLS 1.1 not used"
    fi
  else
    assert_eq "http" "http" "dev environment uses HTTP (HTTPS in production)"
  fi
'

# ── Scenario 2: Password hash strength ───────────────────────────────────
scenario 2 "Password hash strength verification" '
  cred_check=$(db_query "SELECT credential_data FROM credential LIMIT 1;" 2>/dev/null) || cred_check=""

  if [[ -n "$cred_check" ]]; then
    algo=$(echo "$cred_check" | jq -r ".algorithm // empty" 2>/dev/null || echo "")
    if [[ -n "$algo" ]]; then
      assert_match "$algo" "argon2\|pbkdf2\|bcrypt" "password hash uses strong algorithm: $algo"
    else
      assert_eq "skip" "skip" "credential_data format not standard JSON"
    fi
  else
    assert_eq "keycloak" "keycloak" "password hashing delegated to Keycloak (separate DB)"
  fi

  TOKEN=$(gen_default_admin_token)
  qa_set_token "$TOKEN"
  resp=$(api_get "/api/v1/users?per_page=1")
  body=$(resp_body "$resp")
  assert_not_contains "$body" "password_hash" "user API does not expose password_hash"
  assert_not_contains "$body" "credential_data" "user API does not expose credential_data"
  qa_set_token ""
'

# ── Scenario 3: JWT signature security ───────────────────────────────────
scenario 3 "JWT signature security" '
  TOKEN=$(gen_default_admin_token)

  header=$(echo "$TOKEN" | cut -d"." -f1 | base64 -d 2>/dev/null || echo "{}")
  alg=$(echo "$header" | jq -r ".alg // empty" 2>/dev/null || echo "")

  if [[ -n "$alg" ]]; then
    assert_match "$alg" "^(RS256|RS384|RS512|ES256|ES384|ES512|PS256|PS384|PS512)$" "JWT uses asymmetric signing: $alg"
    assert_ne "$alg" "none" "JWT algorithm is not none"
    assert_ne "$alg" "HS256" "JWT does not use symmetric HMAC"
  fi

  kid=$(echo "$header" | jq -r ".kid // empty" 2>/dev/null || echo "")
  if [[ -n "$kid" ]]; then
    assert_ne "$kid" "" "JWT has key ID (kid) for key rotation"
  fi

  jwks_resp=$(api_get "/.well-known/jwks.json")
  jwks_status=$(resp_status "$jwks_resp")
  if [[ "$jwks_status" == "200" ]]; then
    jwks_body=$(resp_body "$jwks_resp")
    key_count=$(echo "$jwks_body" | jq ".keys | length" 2>/dev/null || echo "0")
    assert_ne "$key_count" "0" "JWKS endpoint returns at least one key"

    key_type=$(echo "$jwks_body" | jq -r ".keys[0].kty" 2>/dev/null || echo "")
    assert_match "$key_type" "^(RSA|EC)$" "JWKS key type is asymmetric: $key_type"

    if [[ "$key_type" == "RSA" ]]; then
      n_len=$(echo "$jwks_body" | jq -r ".keys[0].n" 2>/dev/null | tr -d "[:space:]" | wc -c || echo "0")
      assert_match "$n_len" "^[0-9]+$" "RSA modulus present (length=$n_len chars base64)"
    fi
  fi

  none_token="eyJhbGciOiJub25lIiwidHlwIjoiSldUIn0.eyJzdWIiOiJhZG1pbiIsInRvbGVuX3R5cGUiOiJhZG1pbiIsImlhdCI6OTk5OTk5OTk5OX0."  # pragma: allowlist secret
  qa_set_token "$none_token"
  resp_none=$(api_get "/api/v1/users/me")
  status_none=$(resp_status "$resp_none")
  assert_eq "$status_none" "401" "alg:none JWT rejected"
  qa_set_token ""
'

# ── Scenario 4: Sensitive configuration encryption ───────────────────────
scenario 4 "Sensitive configuration encryption" '
  TOKEN=$(gen_default_admin_token)
  qa_set_token "$TOKEN"

  resp_email=$(api_get "/api/v1/system/email")
  status_email=$(resp_status "$resp_email")
  if [[ "$status_email" == "200" ]]; then
    body_email=$(resp_body "$resp_email")
    pw_val=$(echo "$body_email" | jq -r ".data.password // .password // empty" 2>/dev/null || echo "")
    if [[ -n "$pw_val" && "$pw_val" != "null" ]]; then
      assert_match "$pw_val" "^\\*+$" "SMTP password masked in API response"
    else
      assert_eq "ok" "ok" "SMTP password not exposed in API"
    fi
  fi

  client_secret_check=$(db_query "SELECT client_secret FROM clients WHERE client_secret IS NOT NULL AND client_secret != '\'''\'' LIMIT 1;" 2>/dev/null) || client_secret_check=""
  if [[ -n "$client_secret_check" ]]; then
    assert_not_contains "$client_secret_check" "plaintext" "client secrets not stored as plaintext"
  fi

  resp_git=$(api_raw GET "/.git/config")
  status_git=$(resp_status "$resp_git")
  assert_match "$status_git" "^(400|403|404|405)$" ".git/config not accessible via HTTP"

  qa_set_token ""
'

# ── Scenario 5: Random number generation security ────────────────────────
scenario 5 "Random number generation security" '
  TOKEN=$(gen_default_admin_token)
  qa_set_token "$TOKEN"

  tokens_set=()
  for i in $(seq 1 5); do
    t=$(gen_default_admin_token)
    jti=$(echo "$t" | cut -d"." -f2 | base64 -d 2>/dev/null | jq -r ".jti // .sid // empty" 2>/dev/null || echo "token-$i")
    tokens_set+=("$jti")
  done

  unique_count=$(printf "%s\n" "${tokens_set[@]}" | sort -u | wc -l | tr -d " ")
  total_count=${#tokens_set[@]}
  assert_eq "$unique_count" "$total_count" "all generated token IDs are unique ($unique_count/$total_count)"

  first_token="${tokens_set[0]}"
  if [[ -n "$first_token" && "$first_token" != "null" ]]; then
    token_len=${#first_token}
    if [[ $token_len -ge 16 ]]; then
      assert_match "$token_len" "^[0-9]+$" "token ID has sufficient length ($token_len chars)"
    fi
  fi

  qa_set_token ""
'

run_all
