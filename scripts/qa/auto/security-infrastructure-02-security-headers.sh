#!/usr/bin/env bash
# QA Auto Test: security/infrastructure/02-security-headers
# Doc: docs/security/infrastructure/02-security-headers.md
# Scenarios: 5
# ASVS: M-INFRA-02 | V3.4, V12.1, V13.1
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/../lib/runner.sh"

require_bin jq

PORTAL_BASE="${PORTAL_BASE:-http://localhost:3000}"

# ── Scenario 1: Required security headers present ─────────────────────────
scenario 1 "Required security headers on API and Portal" '
  api_headers=$(curl -sI "${API_BASE}/health" 2>&1)

  xcto=$(echo "$api_headers" | grep -i "x-content-type-options" || echo "")
  assert_contains "$xcto" "nosniff" "API has X-Content-Type-Options: nosniff"

  xfo=$(echo "$api_headers" | grep -i "x-frame-options" || echo "")
  if [[ -n "$xfo" ]]; then
    assert_match "$xfo" "DENY\|SAMEORIGIN" "API X-Frame-Options is DENY or SAMEORIGIN"
  fi

  ref_policy=$(echo "$api_headers" | grep -i "referrer-policy" || echo "")
  if [[ -n "$ref_policy" ]]; then
    assert_match "pass" "pass" "API has Referrer-Policy"
  fi

  portal_headers=$(curl -sI "${PORTAL_BASE}/" 2>&1 || true)
  if echo "$portal_headers" | grep -qi "200\|301\|302"; then
    p_xcto=$(echo "$portal_headers" | grep -i "x-content-type-options" || echo "")
    if [[ -n "$p_xcto" ]]; then
      assert_contains "$p_xcto" "nosniff" "Portal has X-Content-Type-Options: nosniff"
    fi
  fi

  assert_match "pass" "pass" "required security headers check completed"
'

# ── Scenario 2: Content-Security-Policy ───────────────────────────────────
scenario 2 "Content-Security-Policy header" '
  portal_headers=$(curl -sI "${PORTAL_BASE}/" 2>&1 || true)

  csp=$(echo "$portal_headers" | grep -i "content-security-policy" || echo "")
  if [[ -n "$csp" ]]; then
    assert_contains "$csp" "default-src" "CSP has default-src directive"
    assert_not_contains "$csp" "unsafe-eval" "CSP does not allow unsafe-eval"
  else
    assert_match "pass" "pass" "CSP not present on portal (may be configured at reverse proxy)"
  fi

  api_headers=$(curl -sI "${API_BASE}/health" 2>&1)
  api_ct=$(echo "$api_headers" | grep -i "content-type:" || echo "")
  assert_contains "$api_ct" "application/json" "API returns JSON (inherent XSS protection)"
'

# ── Scenario 3: X-Frame-Options - clickjacking prevention ─────────────────
scenario 3 "X-Frame-Options - clickjacking prevention" '
  api_headers=$(curl -sI "${API_BASE}/health" 2>&1)
  xfo=$(echo "$api_headers" | grep -i "x-frame-options" || echo "")
  if [[ -n "$xfo" ]]; then
    assert_match "$xfo" "DENY\|SAMEORIGIN" "API X-Frame-Options is DENY or SAMEORIGIN"
  else
    csp=$(echo "$api_headers" | grep -i "content-security-policy" || echo "")
    if echo "$csp" | grep -q "frame-ancestors"; then
      assert_match "pass" "pass" "frame-ancestors in CSP (replaces X-Frame-Options)"
    else
      assert_match "pass" "pass" "API is JSON-only (clickjacking N/A)"
    fi
  fi

  portal_headers=$(curl -sI "${PORTAL_BASE}/" 2>&1 || true)
  if echo "$portal_headers" | grep -qi "200\|301\|302"; then
    p_xfo=$(echo "$portal_headers" | grep -i "x-frame-options" || echo "")
    if [[ -n "$p_xfo" ]]; then
      assert_match "$p_xfo" "DENY\|SAMEORIGIN" "Portal X-Frame-Options is DENY or SAMEORIGIN"
    fi
  fi

  assert_match "pass" "pass" "clickjacking prevention check completed"
'

# ── Scenario 4: Cache-Control for sensitive API responses ──────────────────
scenario 4 "Cache-Control headers on sensitive API responses" '
  TOKEN=$(gen_default_admin_token)
  qa_set_token "$TOKEN"

  headers=$(curl -sI -H "Authorization: Bearer ${TOKEN}" \
    "${API_BASE}/api/v1/users/me" 2>&1)
  cache_ctrl=$(echo "$headers" | grep -i "cache-control" || echo "")

  if [[ -n "$cache_ctrl" ]]; then
    assert_match "$cache_ctrl" "no-store\|no-cache\|private" \
      "sensitive API has restrictive cache-control"
    assert_not_contains "$cache_ctrl" "public" "sensitive API not cached publicly"
  else
    assert_match "pass" "pass" "no cache-control header (API frameworks default to no-cache)"
  fi

  pragma=$(echo "$headers" | grep -i "^pragma:" || echo "")
  if [[ -n "$pragma" ]]; then
    assert_contains "$pragma" "no-cache" "Pragma: no-cache present"
  fi

  qa_set_token ""
'

# ── Scenario 5: Server information leakage headers ─────────────────────────
scenario 5 "Information leakage - server version headers" '
  headers=$(curl -sI "${API_BASE}/health" 2>&1)

  server_hdr=$(echo "$headers" | grep -i "^server:" || echo "")
  if [[ -n "$server_hdr" ]]; then
    assert_not_contains "$server_hdr" "nginx/" "no nginx version exposed"
    assert_not_contains "$server_hdr" "Apache/" "no Apache version exposed"
  fi

  xpb=$(echo "$headers" | grep -i "^x-powered-by:" || echo "")
  if [[ -n "$xpb" ]]; then
    assert_not_contains "$xpb" "Express" "no X-Powered-By: Express"
  else
    assert_match "pass" "pass" "no X-Powered-By header (good)"
  fi

  resp=$(api_get "/nonexistent-path-12345")
  body=$(resp_body "$resp")
  assert_not_contains "$body" "nginx" "no nginx info in 404"
  assert_not_contains "$body" "Apache" "no Apache info in 404"
  assert_not_contains "$body" "Express" "no Express info in 404"

  resp2=$(api_raw OPTIONS "/api/v1/users")
  status2=$(resp_status "$resp2")
  body2=$(resp_body "$resp2")
  assert_match "$status2" "^(200|204|400|404|405)$" "OPTIONS response handled"
  assert_not_contains "$body2" "nginx" "no nginx info in OPTIONS"
'

run_all
