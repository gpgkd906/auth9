#!/usr/bin/env bash
# QA Auto Test: security/input-validation/02-xss
# Doc: docs/security/input-validation/02-xss.md
# Scenarios: 5
# ASVS: M-INPUT-02 | V1.2, V3.1, V3.2
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/../lib/runner.sh"

require_bin jq

# ── Scenario 1: Stored XSS - user profile fields ──────────────────────────
scenario 1 "Stored XSS - user profile fields" '
  TOKEN=$(gen_default_admin_token)
  qa_set_token "$TOKEN"

  TENANT_ID=$(db_query "SELECT id FROM tenants LIMIT 1;" | tr -d "[:space:]")
  USER_ID=$(db_query "SELECT user_id FROM tenant_users WHERE tenant_id='\''${TENANT_ID}'\'' LIMIT 1;" | tr -d "[:space:]")
  if [[ -z "$USER_ID" ]]; then
    USER_ID=$(db_query "SELECT id FROM users LIMIT 1;" | tr -d "[:space:]")
  fi

  xss_payloads=(
    "<script>alert(document.cookie)</script>"
    "<img src=x onerror=alert(1)>"
    "<svg onload=alert(1)>"
    "<body onload=alert(1)>"
    "javascript:alert(1)"
  )

  for p in "${xss_payloads[@]}"; do
    escaped=$(echo "$p" | python3 -c "import sys,json; print(json.dumps(sys.stdin.read().strip()))" | sed "s/^\"//" | sed "s/\"$//")
    resp=$(api_put "/api/v1/users/me" "{\"display_name\":\"${escaped}\"}")
    status=$(resp_status "$resp")
    body=$(resp_body "$resp")
    assert_match "$status" "^(200|400|422)$" "XSS payload in display_name handled: ${p:0:25}"

    if [[ "$status" == "200" ]]; then
      assert_not_contains "$body" "<script>" "no raw script tag in response"
    fi
  done

  api_put "/api/v1/users/me" "{\"display_name\":\"QA Test User\"}" >/dev/null 2>&1 || true
  qa_set_token ""
'

# ── Scenario 2: Stored XSS - tenant/service configuration ─────────────────
scenario 2 "Stored XSS - tenant and service configuration" '
  TOKEN=$(gen_default_admin_token)
  qa_set_token "$TOKEN"

  resp=$(api_post "/api/v1/tenants" \
    "{\"name\":\"<svg onload=alert(1)>\",\"slug\":\"xss-test-tenant\"}")
  status=$(resp_status "$resp")
  body=$(resp_body "$resp")
  assert_match "$status" "^(201|400|422)$" "XSS in tenant name handled"
  if [[ "$status" == "201" ]]; then
    tenant_name=$(echo "$body" | jq -r ".data.name // .name // empty")
    assert_not_contains "$tenant_name" "<svg" "tenant name sanitized or stored safely"
    tid=$(echo "$body" | jq -r ".data.id // .id // empty")
    if [[ -n "$tid" ]]; then
      api_delete "/api/v1/tenants/${tid}" >/dev/null 2>&1 || true
    fi
  fi

  SVC_ID=$(db_query "SELECT id FROM services LIMIT 1;" | tr -d "[:space:]")
  if [[ -n "$SVC_ID" ]]; then
    resp=$(api_put "/api/v1/services/${SVC_ID}" \
      "{\"name\":\"<img src=x onerror=alert(1)>\"}")
    status=$(resp_status "$resp")
    assert_match "$status" "^(200|400|422)$" "XSS in service name handled"
  fi

  qa_set_token ""
'

# ── Scenario 3: Reflected XSS - search and error responses ────────────────
scenario 3 "Reflected XSS - search parameters and error messages" '
  TOKEN=$(gen_default_admin_token)
  qa_set_token "$TOKEN"

  xss_search_payloads=(
    "<script>alert(1)</script>"
    "\"><img src=x onerror=alert(1)>"
    "<ScRiPt>alert(1)</ScRiPt>"
  )

  for p in "${xss_search_payloads[@]}"; do
    encoded=$(python3 -c "import urllib.parse; print(urllib.parse.quote(\"${p}\"))" 2>/dev/null || echo "$p")
    resp=$(api_get "/api/v1/users?search=${encoded}")
    status=$(resp_status "$resp")
    body=$(resp_body "$resp")
    assert_match "$status" "^(200|400|422)$" "reflected XSS search handled: ${p:0:25}"
    assert_not_contains "$body" "<script>" "no reflected script tag"
    assert_not_contains "$body" "onerror=" "no reflected onerror"
  done

  resp=$(api_get "/api/v1/nonexistent-path-<script>alert(1)</script>")
  status=$(resp_status "$resp")
  body=$(resp_body "$resp")
  assert_not_contains "$body" "<script>" "no XSS in 404 error body"

  qa_set_token ""
'

# ── Scenario 4: DOM XSS - API content-type safety ─────────────────────────
scenario 4 "DOM XSS prevention - API returns JSON content-type" '
  TOKEN=$(gen_default_admin_token)
  qa_set_token "$TOKEN"

  headers=$(curl -sI -H "Authorization: Bearer ${TOKEN}" \
    "${API_BASE}/api/v1/users?search=test" 2>&1)
  assert_contains "$headers" "application/json" "API returns application/json content-type"
  assert_not_contains "$headers" "text/html" "API does not return text/html"

  headers2=$(curl -sI "${API_BASE}/health" 2>&1)
  assert_contains "$headers2" "application/json" "health endpoint returns JSON"

  qa_set_token ""
'

# ── Scenario 5: XSS via file upload (SVG/HTML) ────────────────────────────
scenario 5 "XSS via malicious file upload" '
  TOKEN=$(gen_default_admin_token)
  qa_set_token "$TOKEN"

  resp=$(api_raw POST "/api/v1/users/me/avatar" \
    -H "Authorization: Bearer ${TOKEN}" \
    -F "file=@-;filename=xss.svg;type=image/svg+xml" \
    <<< "<svg xmlns=\"http://www.w3.org/2000/svg\"><script>alert(document.domain)</script></svg>")
  status=$(resp_status "$resp")
  body=$(resp_body "$resp")
  assert_match "$status" "^(200|400|404|405|413|415|422)$" "SVG upload handled"

  if [[ "$status" == "200" || "$status" == "201" ]]; then
    file_url=$(echo "$body" | jq -r ".data.avatar_url // .avatar_url // empty")
    if [[ -n "$file_url" && "$file_url" != "null" ]]; then
      file_headers=$(curl -sI "${file_url}" 2>&1)
      assert_not_contains "$file_headers" "text/html" "SVG not served as text/html"
    fi
  fi

  resp2=$(api_raw POST "/api/v1/users/me/avatar" \
    -H "Authorization: Bearer ${TOKEN}" \
    -F "file=@-;filename=evil.html;type=text/html" \
    <<< "<html><script>alert(1)</script></html>")
  status2=$(resp_status "$resp2")
  assert_match "$status2" "^(400|404|405|413|415|422)$" "HTML file upload rejected"

  qa_set_token ""
'

run_all
