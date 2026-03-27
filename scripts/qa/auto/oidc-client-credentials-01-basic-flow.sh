#!/usr/bin/env bash
# QA Auto Test: oidc-client-credentials-01
# Doc: docs/oidc/client-credentials/01-basic-flow.md
# Scenarios: 4
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/../lib/runner.sh"

require_bin jq

scenario 1 "client_secret_basic returns access_token (requires OIDC client setup)" '
  echo "SKIP: requires a pre-provisioned OIDC client with known client_secret" >&2
'

scenario 2 "client_secret_post returns access_token (requires OIDC client setup)" '
  echo "SKIP: requires a pre-provisioned OIDC client with known client_secret" >&2
'

scenario 3 "Invalid credentials return 401" '
  resp=$(curl -s -w '\''\n%{http_code}'\'' \
    -u "invalid-client-id:invalid-client-secret" \
    -X POST \
    -d "grant_type=client_credentials" \
    "${API_BASE}/api/v1/auth/token")
  status=$(echo "$resp" | tail -1)
  body=$(echo "$resp" | sed '\''$d'\'')
  assert_http_status "$status" 401 "invalid credentials return 401"
'

scenario 4 "Token endpoint rejects missing credentials" '
  resp=$(curl -s -w '\''\n%{http_code}'\'' \
    -X POST \
    -d "grant_type=client_credentials" \
    "${API_BASE}/api/v1/auth/token")
  status=$(echo "$resp" | tail -1)
  body=$(echo "$resp" | sed '\''$d'\'')
  if [[ "$status" == "400" || "$status" == "401" ]]; then
    _qa_pass "missing credentials rejected" "400 or 401" "$status"
  else
    _qa_fail "missing credentials rejected" "400 or 401" "$status"
  fi
'

run_all
