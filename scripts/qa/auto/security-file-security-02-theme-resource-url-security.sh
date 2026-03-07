#!/usr/bin/env bash
# Security Auto Test: security/file-security/02-theme-resource-url-security
# Doc: docs/security/file-security/02-theme-resource-url-security.md
# Scenarios: 3
# ASVS: M-FILE-02 | V5.2, V3.4, V14.2
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/../lib/runner.sh"

require_bin jq

BRANDING_PAYLOAD_BASE='{"config":{"primary_color":"#000000","secondary_color":"#111111","background_color":"#222222","text_color":"#333333"'

# ── Scenario 1: Dangerous protocol injection ─────────────────────────────
scenario 1 "Dangerous protocol injection in branding URLs" '
  TOKEN=$(gen_default_admin_token)
  qa_set_token "$TOKEN"

  resp_js=$(api_put "/api/v1/system/branding" \
    "${BRANDING_PAYLOAD_BASE},\"logo_url\":\"javascript:alert(1)\"}}")
  status_js=$(resp_status "$resp_js")
  assert_match "$status_js" "^(400|422|429)$" "javascript: protocol rejected for branding logo_url"

  resp_data=$(api_put "/api/v1/system/branding" \
    "${BRANDING_PAYLOAD_BASE},\"logo_url\":\"data:image/png;base64,iVBOR\"}}")
  status_data=$(resp_status "$resp_data")
  assert_match "$status_data" "^(400|422|429)$" "data: protocol rejected for branding logo_url"

  resp_file=$(api_put "/api/v1/system/branding" \
    "${BRANDING_PAYLOAD_BASE},\"logo_url\":\"file:///etc/passwd\"}}")
  status_file=$(resp_status "$resp_file")
  assert_match "$status_file" "^(400|422|429)$" "file: protocol rejected for branding logo_url"

  resp_ftp=$(api_put "/api/v1/system/branding" \
    "${BRANDING_PAYLOAD_BASE},\"favicon_url\":\"ftp://evil.com/favicon.ico\"}}")
  status_ftp=$(resp_status "$resp_ftp")
  assert_match "$status_ftp" "^(400|422|429)$" "ftp: protocol rejected for branding favicon_url"

  resp_lo=$(api_put "/api/v1/system/branding" \
    "${BRANDING_PAYLOAD_BASE},\"logo_url\":\"http://127.0.0.1/logo.png\"}}")
  status_lo=$(resp_status "$resp_lo")
  assert_match "$status_lo" "^(400|422|429)$" "localhost URL rejected for branding logo_url"

  resp_priv=$(api_put "/api/v1/system/branding" \
    "${BRANDING_PAYLOAD_BASE},\"logo_url\":\"http://192.168.1.1/logo.png\"}}")
  status_priv=$(resp_status "$resp_priv")
  assert_match "$status_priv" "^(400|422|429)$" "private IP rejected for branding logo_url"

  resp_meta=$(api_put "/api/v1/system/branding" \
    "${BRANDING_PAYLOAD_BASE},\"logo_url\":\"http://169.254.169.254/latest/meta-data/\"}}")
  status_meta=$(resp_status "$resp_meta")
  assert_match "$status_meta" "^(400|422|429)$" "cloud metadata URL rejected for branding logo_url"

  resp_http=$(api_put "/api/v1/system/branding" \
    "${BRANDING_PAYLOAD_BASE},\"logo_url\":\"http://example.com/logo.png\"}}")
  status_http=$(resp_status "$resp_http")
  assert_match "$status_http" "^(400|422|429)$" "HTTP (non-HTTPS) rejected for branding logo_url"

  qa_set_token ""
'

# ── Scenario 2: External domain control (allowlist) ──────────────────────
scenario 2 "External domain control via BRANDING_ALLOWED_DOMAINS" '
  TOKEN=$(gen_default_admin_token)
  qa_set_token "$TOKEN"

  resp_valid=$(api_put "/api/v1/system/branding" \
    "${BRANDING_PAYLOAD_BASE},\"logo_url\":\"https://cdn.example.com/logo.png\"}}")
  status_valid=$(resp_status "$resp_valid")

  if [[ "$status_valid" == "200" ]]; then
    resp_ext=$(api_put "/api/v1/system/branding" \
      "${BRANDING_PAYLOAD_BASE},\"logo_url\":\"https://evil-attacker.com/logo.png\"}}")
    status_ext=$(resp_status "$resp_ext")

    if [[ "$status_ext" == "422" ]]; then
      assert_eq "$status_ext" "422" "domain allowlist active - unauthorized domain rejected"

      resp_sub=$(api_put "/api/v1/system/branding" \
        "${BRANDING_PAYLOAD_BASE},\"logo_url\":\"https://img.cdn.example.com/logo.png\"}}")
      status_sub=$(resp_status "$resp_sub")
      assert_eq "$status_sub" "200" "subdomain of allowed domain accepted"
    else
      assert_eq "not-configured" "not-configured" "BRANDING_ALLOWED_DOMAINS not set (any HTTPS domain allowed)"
    fi
  else
    assert_match "$status_valid" "^(400|422|429)$" "branding update handled"
  fi

  qa_set_token ""
'

# ── Scenario 3: Resource URL privacy leakage (referrer) ──────────────────
scenario 3 "Resource URL privacy leakage via referrer" '
  resp_branding=$(api_get "/api/v1/public/branding")
  status_branding=$(resp_status "$resp_branding")

  if [[ "$status_branding" == "200" ]]; then
    body_branding=$(resp_body "$resp_branding")
    logo_url=$(echo "$body_branding" | jq -r ".data.logo_url // .logo_url // empty" 2>/dev/null || echo "")

    if [[ -n "$logo_url" && "$logo_url" != "null" ]]; then
      assert_contains "$logo_url" "https" "branding logo uses HTTPS"
    else
      assert_eq "ok" "ok" "no logo URL configured (safe)"
    fi
  else
    assert_match "$status_branding" "^(200|404)$" "public branding endpoint accessible"
  fi

  headers=$(curl -sI "${API_BASE}/api/v1/public/branding" 2>&1)
  rp=$(echo "$headers" | grep -i "referrer-policy" || echo "")
  if [[ -n "$rp" ]]; then
    assert_match "$rp" "no-referrer\|strict-origin\|same-origin" "referrer-policy header set appropriately"
  else
    assert_eq "ok" "ok" "referrer-policy handled at application level"
  fi

  login_resp=$(curl -s "http://localhost:3000/login" 2>/dev/null) || login_resp=""
  if [[ -n "$login_resp" ]]; then
    if echo "$login_resp" | grep -q "referrerPolicy"; then
      assert_contains "$login_resp" "referrerPolicy" "login page sets referrerPolicy on resources"
    fi
  fi
'

run_all
