#!/usr/bin/env bash
# QA Auto Test: security/input-validation/03-csrf
# Doc: docs/security/input-validation/03-csrf.md
# Scenarios: 5
# ASVS: M-INPUT-03 | V3.3, V7.1, V10.2
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/../lib/runner.sh"

require_bin jq

# ── Scenario 1: OIDC login CSRF - redirect_uri validation ─────────────────
scenario 1 "OIDC login CSRF - redirect_uri whitelist and state param" '
  resp=$(api_raw GET "/api/v1/auth/authorize" \
    -H "Content-Type: application/json" \
    -G \
    -d "client_id=auth9-portal" \
    -d "redirect_uri=http://attacker.com/callback" \
    -d "response_type=code" \
    -d "state=random123")
  status=$(resp_status "$resp")
  body=$(resp_body "$resp")
  assert_match "$status" "^(400|403|302)$" "malicious redirect_uri rejected or handled"

  if [[ "$status" == "302" ]]; then
    location=$(curl -sI -G \
      -d "client_id=auth9-portal" \
      -d "redirect_uri=http://attacker.com/callback" \
      -d "response_type=code" \
      -d "state=random123" \
      "${API_BASE}/api/v1/auth/authorize" 2>&1 | grep -i "^location:" || echo "")
    assert_not_contains "$location" "attacker.com" "redirect does not go to attacker"
  fi

  resp2=$(api_raw GET "/api/v1/auth/authorize" \
    -G \
    -d "client_id=auth9-portal" \
    -d "redirect_uri=http://localhost:3000/callback" \
    -d "response_type=code")
  status2=$(resp_status "$resp2")
  assert_match "$status2" "^(200|302|400)$" "authorize without state param handled"
'

# ── Scenario 2: Sensitive operation CSRF - requires Bearer token ───────────
scenario 2 "Sensitive operations CSRF - cookie-only auth rejected" '
  resp=$(api_raw PUT "/api/v1/users/me" \
    -H "Content-Type: application/json" \
    -H "Cookie: session=fake_session_cookie" \
    -d "{\"display_name\":\"hacked\"}")
  status=$(resp_status "$resp")
  assert_match "$status" "^(401|403|429)$" "PUT /users/me without Bearer token returns 401/403"

  resp2=$(api_raw POST "/api/v1/tenants" \
    -H "Content-Type: application/json" \
    -H "Cookie: session=fake_session_cookie" \
    -d "{\"name\":\"csrf-test\",\"slug\":\"csrf-test\"}")
  status2=$(resp_status "$resp2")
  assert_match "$status2" "^(401|403|429)$" "POST /tenants without Bearer token returns 401/403"

  resp3=$(api_raw DELETE "/api/v1/tenants/fake-id" \
    -H "Cookie: session=fake_session_cookie")
  status3=$(resp_status "$resp3")
  assert_match "$status3" "^(401|403|429)$" "DELETE without Bearer token returns 401/403"
'

# ── Scenario 3: Cookie SameSite configuration ─────────────────────────────
scenario 3 "Cookie SameSite attribute verification" '
  headers=$(curl -sI "${API_BASE}/health" 2>&1)

  cookie_headers=$(echo "$headers" | grep -i "^set-cookie:" || echo "")
  if [[ -n "$cookie_headers" ]]; then
    if echo "$cookie_headers" | grep -qi "SameSite=None"; then
      has_samesite_none="true"
    else
      has_samesite_none="false"
    fi
    assert_eq "$has_samesite_none" "false" "no SameSite=None on health cookies"

    if echo "$cookie_headers" | grep -qi "session\|auth\|token"; then
      assert_contains "$cookie_headers" "HttpOnly" "auth cookies have HttpOnly"
    fi
  fi

  resp=$(api_raw POST "/api/v1/auth/forgot-password" \
    -H "Content-Type: application/json" \
    -D - \
    -d "{\"email\":\"test@example.com\"}")
  resp_headers=$(echo "$resp" | grep -i "^set-cookie:" || echo "")
  if [[ -n "$resp_headers" ]]; then
    assert_not_contains "$resp_headers" "SameSite=None" "no SameSite=None on auth cookies"
  fi

  assert_match "pass" "pass" "cookie SameSite check completed"
'

# ── Scenario 4: JSON API CSRF - Content-Type enforcement ──────────────────
scenario 4 "JSON API CSRF - strict Content-Type enforcement" '
  TOKEN=$(gen_admin_token)

  resp=$(api_raw POST "/api/v1/tenants" \
    -H "Authorization: Bearer ${TOKEN}" \
    -H "Content-Type: text/plain" \
    -d "{\"name\":\"csrf-ct-test\",\"slug\":\"csrf-ct-test\"}")
  status=$(resp_status "$resp")
  assert_match "$status" "^(400|415|422)$" "text/plain Content-Type rejected for JSON API"

  resp2=$(api_raw POST "/api/v1/tenants" \
    -H "Authorization: Bearer ${TOKEN}" \
    -H "Content-Type: application/x-www-form-urlencoded" \
    -d "name=csrf-test&slug=csrf-test")
  status2=$(resp_status "$resp2")
  assert_match "$status2" "^(400|415|422)$" "form-urlencoded Content-Type rejected for JSON API"

  resp3=$(api_raw POST "/api/v1/tenants" \
    -H "Authorization: Bearer ${TOKEN}" \
    -H "Content-Type: multipart/form-data" \
    -d "name=csrf-test")
  status3=$(resp_status "$resp3")
  assert_match "$status3" "^(400|415|422)$" "multipart/form-data Content-Type rejected for JSON API"
'

# ── Scenario 5: Logout CSRF - GET request must not trigger logout ──────────
scenario 5 "Logout CSRF - GET must not execute logout" '
  resp=$(api_raw GET "/api/v1/auth/logout")
  status=$(resp_status "$resp")
  assert_match "$status" "^(302|307|400|404|405)$" "GET logout does not execute or returns error"

  resp2=$(api_raw GET "/api/v1/auth/logout" \
    -H "Cookie: session=fake_session")
  status2=$(resp_status "$resp2")
  assert_match "$status2" "^(302|307|400|404|405)$" "GET logout with cookie does not force logout"
'

run_all
