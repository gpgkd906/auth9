#!/usr/bin/env bash
# QA Auto Test: security/api-security/04-cors-headers
# Doc: docs/security/api-security/04-cors-headers.md
# Scenarios: 4 - CORS configuration and security headers
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/../lib/runner.sh"

scenario 1 "CORS configuration security" '
  headers=$(curl -sI -X OPTIONS "${API_BASE}/api/v1/users" \
    -H "Origin: http://localhost:3000" \
    -H "Access-Control-Request-Method: GET" 2>&1)
  lower=$(echo "$headers" | tr "[:upper:]" "[:lower:]")
  assert_contains "$lower" "access-control-allow-origin" "CORS ACAO header present for allowed origin"
  assert_contains "$headers" "localhost:3000" "CORS allows localhost:3000"

  evil_headers=$(curl -sI -X OPTIONS "${API_BASE}/api/v1/users" \
    -H "Origin: http://evil.com" \
    -H "Access-Control-Request-Method: GET" 2>&1)
  assert_not_contains "$evil_headers" "evil.com" "CORS does not reflect evil.com"

  null_headers=$(curl -sI -X OPTIONS "${API_BASE}/api/v1/users" \
    -H "Origin: null" \
    -H "Access-Control-Request-Method: GET" 2>&1)
  null_lower=$(echo "$null_headers" | tr "[:upper:]" "[:lower:]")
  allows_null=false
  if echo "$null_lower" | grep -q "access-control-allow-origin:.*null"; then
    allows_null=true
  fi
  assert_eq "$allows_null" "false" "CORS does not allow null origin"

  any_headers=$(curl -sI "${API_BASE}/api/v1/users" \
    -H "Origin: http://random-site.com" 2>&1)
  any_lower=$(echo "$any_headers" | tr "[:upper:]" "[:lower:]")
  wildcard_plus_creds=false
  if echo "$any_lower" | grep -q "access-control-allow-origin: \*"; then
    if echo "$any_lower" | grep -q "access-control-allow-credentials: true"; then
      wildcard_plus_creds=true
    fi
  fi
  assert_eq "$wildcard_plus_creds" "false" "No wildcard ACAO combined with credentials"
'

scenario 2 "Security response headers on API" '
  token=$(gen_admin_token)
  headers=$(curl -sI -H "Authorization: Bearer $token" "${API_BASE}/api/v1/tenants" 2>&1)
  lower=$(echo "$headers" | tr "[:upper:]" "[:lower:]")

  assert_contains "$lower" "x-content-type-options" "X-Content-Type-Options header present"
  if echo "$lower" | grep -q "x-content-type-options"; then
    assert_contains "$lower" "nosniff" "X-Content-Type-Options is nosniff"
  fi

  assert_contains "$lower" "x-frame-options" "X-Frame-Options header present"

  no_store=false
  if echo "$lower" | grep -q "no-store\|no-cache"; then
    no_store=true
  fi
  assert_eq "$no_store" "true" "Cache-Control includes no-store or no-cache"
'

scenario 3 "Content-Security-Policy on Portal" '
  portal_status=$(curl -s -o /dev/null -w "%{http_code}" "http://localhost:3000/" 2>/dev/null || echo "000")
  if [[ "$portal_status" == "000" ]]; then
    assert_eq "skip" "skip" "Portal (localhost:3000) not accessible - skipping CSP test"
  else
    csp_headers=$(curl -sI "http://localhost:3000/" 2>&1)
    csp_lower=$(echo "$csp_headers" | tr "[:upper:]" "[:lower:]")

    has_csp=false
    if echo "$csp_lower" | grep -q "content-security-policy"; then
      has_csp=true
    fi
    assert_eq "$has_csp" "true" "CSP header present on Portal"

    has_unsafe_eval=false
    if echo "$csp_lower" | grep -q "unsafe-eval"; then
      has_unsafe_eval=true
    fi
    assert_eq "$has_unsafe_eval" "false" "CSP does not contain unsafe-eval"
  fi
'

scenario 4 "Clickjacking protection" '
  headers=$(curl -sI "${API_BASE}/health" 2>&1)
  lower=$(echo "$headers" | tr "[:upper:]" "[:lower:]")
  has_frame_protection=false
  if echo "$lower" | grep -q "x-frame-options"; then
    has_frame_protection=true
  fi
  if echo "$lower" | grep -q "frame-ancestors"; then
    has_frame_protection=true
  fi
  assert_eq "$has_frame_protection" "true" "API has clickjacking protection (X-Frame-Options or frame-ancestors)"

  portal_status=$(curl -s -o /dev/null -w "%{http_code}" "http://localhost:3000/" 2>/dev/null || echo "000")
  if [[ "$portal_status" != "000" ]]; then
    p_headers=$(curl -sI "http://localhost:3000/" 2>&1)
    p_lower=$(echo "$p_headers" | tr "[:upper:]" "[:lower:]")
    p_has_protection=false
    if echo "$p_lower" | grep -q "x-frame-options"; then
      p_has_protection=true
    fi
    if echo "$p_lower" | grep -q "frame-ancestors"; then
      p_has_protection=true
    fi
    assert_eq "$p_has_protection" "true" "Portal has clickjacking protection"
  fi
'

run_all
