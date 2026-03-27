#!/usr/bin/env bash
# QA Auto Test: oidc-security-01
# Doc: docs/oidc/security/01-state-nonce-redirect.md
# Scenarios: 5
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/../lib/runner.sh"

require_bin jq

scenario 1 "Empty state parameter is rejected" '
  resp=$(curl -s -w '\''\n%{http_code}'\'' \
    "${API_BASE}/api/v1/auth/authorize?response_type=code&client_id=test-client&redirect_uri=http://localhost:3000/callback&scope=openid&state=")
  status=$(echo "$resp" | tail -1)
  body=$(echo "$resp" | sed '\''$d'\'')
  assert_http_status "$status" 400 "empty state returns 400"
'

scenario 2 "State parameter passthrough (requires browser)" '
  echo "SKIP: requires browser interaction to complete auth flow" >&2
'

scenario 3 "Redirect URI must match registered value exactly" '
  resp=$(curl -s -w '\''\n%{http_code}'\'' \
    "${API_BASE}/api/v1/auth/authorize?response_type=code&client_id=test-client&redirect_uri=https://evil.example.com/callback&scope=openid&state=random-state-123")
  status=$(echo "$resp" | tail -1)
  body=$(echo "$resp" | sed '\''$d'\'')
  assert_http_status "$status" 400 "mismatched redirect_uri returns 400"
'

scenario 4 "Nonce binds to ID token (requires browser)" '
  echo "SKIP: requires browser interaction to complete auth code flow" >&2
'

scenario 5 "Replay detection with nonce (requires browser)" '
  echo "SKIP: requires browser interaction to complete auth code flow" >&2
'

run_all
