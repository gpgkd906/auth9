#!/usr/bin/env bash
# QA Auto Test: oidc-token-validation-01
# Doc: docs/oidc/token-validation/01-claims-signing.md
# Scenarios: 5
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/../lib/runner.sh"

require_bin jq

scenario 1 "Token is valid RS256 signed JWT" '
  TOKEN=$(gen_admin_token)
  header=$(echo "$TOKEN" | cut -d. -f1 | tr "_-" "/+" | base64 -d 2>/dev/null || true)
  alg=$(echo "$header" | jq -r .alg 2>/dev/null || echo "")
  assert_eq "$alg" "RS256" "token alg is RS256"
'

scenario 2 "Token header references JWKS kid" '
  TOKEN=$(gen_admin_token)
  header=$(echo "$TOKEN" | cut -d. -f1 | tr "_-" "/+" | base64 -d 2>/dev/null || true)
  kid=$(echo "$header" | jq -r .kid 2>/dev/null || echo "")
  assert_eq "$kid" "auth9-current" "token kid is auth9-current"
'

scenario 3 "Token contains required claims" '
  TOKEN=$(gen_admin_token)
  payload=$(echo "$TOKEN" | cut -d. -f2 | tr "_-" "/+" | base64 -d 2>/dev/null || true)
  assert_json_exists "$payload" ".iss" "claim iss exists"
  assert_json_exists "$payload" ".sub" "claim sub exists"
  assert_json_exists "$payload" ".aud" "claim aud exists"
  assert_json_exists "$payload" ".exp" "claim exp exists"
  assert_json_exists "$payload" ".iat" "claim iat exists"
'

scenario 4 "Token issuer matches discovery" '
  disc_resp=$(api_get "/.well-known/openid-configuration")
  disc_body=$(resp_body "$disc_resp")
  discovery_issuer=$(echo "$disc_body" | jq -r .issuer)

  TOKEN=$(gen_admin_token)
  payload=$(echo "$TOKEN" | cut -d. -f2 | tr "_-" "/+" | base64 -d 2>/dev/null || true)
  token_iss=$(echo "$payload" | jq -r .iss 2>/dev/null || echo "")
  assert_eq "$token_iss" "$discovery_issuer" "token issuer matches discovery issuer"
'

scenario 5 "Token timestamps are valid" '
  TOKEN=$(gen_admin_token)
  payload=$(echo "$TOKEN" | cut -d. -f2 | tr "_-" "/+" | base64 -d 2>/dev/null || true)
  exp=$(echo "$payload" | jq -r .exp 2>/dev/null || echo "0")
  iat=$(echo "$payload" | jq -r .iat 2>/dev/null || echo "0")
  now=$(date +%s)
  if [[ "$exp" -gt "$now" ]]; then
    _qa_pass "exp is in the future" ">$now" "$exp"
  else
    _qa_fail "exp is in the future" ">$now" "$exp"
  fi
  if [[ "$iat" -le "$now" ]]; then
    _qa_pass "iat is not in the future" "<=$now" "$iat"
  else
    _qa_fail "iat is not in the future" "<=$now" "$iat"
  fi
'

run_all
