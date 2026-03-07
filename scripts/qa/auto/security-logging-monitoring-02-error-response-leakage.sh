#!/usr/bin/env bash
# Security Auto Test: security/logging-monitoring/02-error-response-leakage
# Doc: docs/security/logging-monitoring/02-error-response-leakage.md
# Scenarios: 3
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/../lib/runner.sh"

require_bin jq
require_bin node

scenario 1 "Parse error does not leak internals" '
  resp=$(api_raw POST /api/v1/tenants \
    -H "Authorization: Bearer $(gen_admin_token)" \
    -H "Content-Type: application/json" \
    -d "{invalid json here}")
  status=$(resp_status "$resp")
  body=$(resp_body "$resp")

  assert_match "$status" "^(400|415|422)$" "Malformed JSON returns 400/415/422"
  assert_not_contains "$body" "line " "No line number in parse error"
  assert_not_contains "$body" "column " "No column number in parse error"
  assert_not_contains "$body" "at position" "No byte offset in parse error"
  assert_not_contains "$body" "stack" "No stack trace in parse error"

  resp2=$(api_raw POST /api/v1/tenants \
    -H "Authorization: Bearer $(gen_admin_token)" \
    -H "Content-Type: text/plain" \
    -d "not json")
  status2=$(resp_status "$resp2")
  assert_match "$status2" "^(400|415|422)$" "Wrong content-type returns 400/415/422"
'

scenario 2 "5xx errors do not leak SQL, paths, or module names" '
  resp=$(api_raw GET "/api/v1/tenants/00000000-0000-0000-0000-000000000000" \
    -H "Authorization: Bearer $(gen_admin_token)")
  status=$(resp_status "$resp")
  body=$(resp_body "$resp")
  assert_match "$status" "^(403|404|400|500)$" "Zero-UUID tenant returns error"
  assert_not_contains "$body" "SELECT" "No SQL SELECT in error"
  assert_not_contains "$body" "FROM " "No SQL FROM in error"
  assert_not_contains "$body" "src/" "No source path in error"
  assert_not_contains "$body" "mod.rs" "No Rust module name in error"
  assert_not_contains "$body" "thread " "No Rust thread info in error"

  resp2=$(api_raw POST /api/v1/tenants \
    -H "Authorization: Bearer $(gen_admin_token)" \
    -H "Content-Type: application/json" \
    -d "{\"name\":\"\"}")
  body2=$(resp_body "$resp2")
  assert_not_contains "$body2" "panicked" "Empty name error does not contain panic info"
  assert_not_contains "$body2" "unwrap()" "Empty name error does not contain unwrap info"
'

scenario 3 "Error response format consistency across status codes" '
  local admin_token
  admin_token=$(gen_admin_token)

  resp_401=$(api_raw GET /api/v1/tenants)
  body_401=$(resp_body "$resp_401")
  status_401=$(resp_status "$resp_401")
  assert_http_status "$status_401" 401 "Unauthenticated returns 401"
  assert_json_exists "$body_401" ".error" "401 has error field"

  resp_404=$(api_raw GET /api/v1/nonexistent-path-xyz \
    -H "Authorization: Bearer $admin_token")
  body_404=$(resp_body "$resp_404")
  status_404=$(resp_status "$resp_404")
  assert_match "$status_404" "^(404|405)$" "Nonexistent path returns 404/405"

  resp_400=$(api_raw POST /api/v1/tenants \
    -H "Authorization: Bearer $admin_token" \
    -H "Content-Type: application/json" \
    -d "{bad}")
  body_400=$(resp_body "$resp_400")
  status_400=$(resp_status "$resp_400")
  assert_match "$status_400" "^(400|415|422)$" "Bad JSON returns 400/415/422"

  resp_405=$(api_raw PATCH /api/v1/auth/token \
    -H "Authorization: Bearer $admin_token" \
    -H "Content-Type: application/json" \
    -d "{}")
  status_405=$(resp_status "$resp_405")
  assert_match "$status_405" "^(404|405)$" "Wrong method returns 404/405"

  for b in "$body_401" "$body_400"; do
    assert_not_contains "$b" "stack" "Error body does not contain stack trace"
    assert_not_contains "$b" "backtrace" "Error body does not contain backtrace"
  done
'

run_all
