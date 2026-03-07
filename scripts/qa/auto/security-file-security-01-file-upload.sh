#!/usr/bin/env bash
# Security Auto Test: security/file-security/01-file-upload
# Doc: docs/security/file-security/01-file-upload.md
# Scenarios: 3
# ASVS: M-FILE-01 | V5.1, V5.2, V5.3, V5.4
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/../lib/runner.sh"

require_bin jq

# ── Scenario 1: URL path traversal attack ────────────────────────────────
scenario 1 "URL path traversal attack" '
  TOKEN=$(gen_default_admin_token)
  qa_set_token "$TOKEN"

  resp=$(api_put "/api/v1/users/me" "{\"avatar_url\":\"../../etc/passwd\"}")
  status=$(resp_status "$resp")
  assert_match "$status" "^(400|422|429)$" "avatar_url without scheme rejected"

  resp2=$(api_put "/api/v1/users/me" "{\"avatar_url\":\"https://example.com/../../etc/passwd\"}")
  status2=$(resp_status "$resp2")
  assert_match "$status2" "^(400|422|429)$" "avatar_url with path traversal rejected"

  resp3=$(api_put "/api/v1/users/me" "{\"avatar_url\":\"..%2F..%2Fetc%2Fpasswd\"}")
  status3=$(resp_status "$resp3")
  assert_match "$status3" "^(400|422|429)$" "URL-encoded path traversal rejected"

  printf -v null_url "https://example.com/avatar\x00.png"
  resp4=$(api_put "/api/v1/users/me" "{\"avatar_url\":\"https://example.com/avatar\\u0000.png\"}")
  status4=$(resp_status "$resp4")
  assert_match "$status4" "^(400|422|429)$" "null byte in avatar_url rejected"

  tenant_id=$(db_query "SELECT id FROM tenants LIMIT 1;" 2>/dev/null | tr -d "[:space:]") || tenant_id=""
  if [[ -n "$tenant_id" ]]; then
    resp5=$(api_put "/api/v1/tenants/${tenant_id}" "{\"logo_url\":\"https://example.com/../../etc/passwd\"}")
    status5=$(resp_status "$resp5")
    assert_match "$status5" "^(200|400|422)$" "tenant logo_url path traversal handled"

    resp6=$(api_put "/api/v1/tenants/${tenant_id}" \
      "{\"settings\":{\"branding\":{\"logo_url\":\"https://example.com/../../etc/passwd\"}}}")
    status6=$(resp_status "$resp6")
    assert_match "$status6" "^(400|422|429)$" "tenant branding logo_url path traversal rejected"

    resp7=$(api_put "/api/v1/tenants/${tenant_id}" \
      "{\"settings\":{\"branding\":{\"logo_url\":\"https://example.com/logo\\u0000.png\"}}}")
    status7=$(resp_status "$resp7")
    assert_match "$status7" "^(400|422|429)$" "tenant branding logo_url null byte rejected"
  fi

  resp_ok=$(api_put "/api/v1/users/me" "{\"avatar_url\":\"https://cdn.example.com/avatars/user123.png\"}")
  status_ok=$(resp_status "$resp_ok")
  assert_eq "$status_ok" "200" "valid avatar_url accepted"

  qa_set_token ""
'

# ── Scenario 2: URL scheme injection ─────────────────────────────────────
scenario 2 "URL scheme injection" '
  TOKEN=$(gen_default_admin_token)
  qa_set_token "$TOKEN"

  resp_js=$(api_put "/api/v1/users/me" "{\"avatar_url\":\"javascript:alert(document.cookie)\"}")
  status_js=$(resp_status "$resp_js")
  assert_match "$status_js" "^(400|422|429)$" "javascript: scheme rejected for avatar_url"

  tenant_id=$(db_query "SELECT id FROM tenants LIMIT 1;" 2>/dev/null | tr -d "[:space:]") || tenant_id=""
  if [[ -n "$tenant_id" ]]; then
    resp_data=$(api_put "/api/v1/tenants/${tenant_id}" \
      "{\"logo_url\":\"data:text/html,<script>alert(1)</script>\"}")
    status_data=$(resp_status "$resp_data")
    assert_match "$status_data" "^(400|422|429)$" "data: scheme rejected for tenant logo_url"

    resp_svg=$(api_put "/api/v1/tenants/${tenant_id}" \
      "{\"logo_url\":\"data:image/svg+xml;base64,PHN2ZyB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciPjxzY3JpcHQ+YWxlcnQoMSk8L3NjcmlwdD48L3N2Zz4=\"}")
    status_svg=$(resp_status "$resp_svg")
    assert_match "$status_svg" "^(400|422|429)$" "base64 SVG data: URI rejected"

    resp_ftp=$(api_put "/api/v1/tenants/${tenant_id}/branding" \
      "{\"config\":{\"favicon_url\":\"ftp://evil.com/malware.exe\",\"primary_color\":\"#007AFF\",\"secondary_color\":\"#5856D6\",\"background_color\":\"#F5F5F7\",\"text_color\":\"#1D1D1F\"}}")
    status_ftp=$(resp_status "$resp_ftp")
    assert_match "$status_ftp" "^(400|422|429)$" "ftp: scheme rejected for favicon_url"

    resp_file=$(api_put "/api/v1/tenants/${tenant_id}" \
      "{\"logo_url\":\"file:///etc/passwd\"}")
    status_file=$(resp_status "$resp_file")
    assert_match "$status_file" "^(400|422|429)$" "file: scheme rejected for logo_url"
  fi

  resp_valid=$(api_put "/api/v1/users/me" "{\"avatar_url\":\"https://cdn.example.com/logo.png\"}")
  status_valid=$(resp_status "$resp_valid")
  assert_eq "$status_valid" "200" "valid https URL accepted"

  qa_set_token ""
'

# ── Scenario 3: SSRF - internal network probing via URL fields ───────────
scenario 3 "SSRF - internal network probing via URL fields" '
  TOKEN=$(gen_default_admin_token)
  qa_set_token "$TOKEN"

  tenant_id=$(db_query "SELECT id FROM tenants LIMIT 1;" 2>/dev/null | tr -d "[:space:]") || tenant_id=""
  if [[ -z "$tenant_id" ]]; then
    assert_eq "skip" "skip" "no tenant for SSRF test"
    qa_set_token ""
    return 0
  fi

  resp_lo=$(api_put "/api/v1/tenants/${tenant_id}" \
    "{\"logo_url\":\"http://127.0.0.1:8080/admin\"}")
  status_lo=$(resp_status "$resp_lo")
  assert_match "$status_lo" "^(400|422|429)$" "localhost URL rejected for tenant logo_url"

  resp_priv=$(api_put "/api/v1/tenants/${tenant_id}" \
    "{\"logo_url\":\"https://192.168.1.1/internal\"}")
  status_priv=$(resp_status "$resp_priv")
  assert_match "$status_priv" "^(400|422|429)$" "private IP rejected for tenant logo_url"

  resp_aws=$(api_put "/api/v1/tenants/${tenant_id}/branding" \
    "{\"config\":{\"logo_url\":\"http://169.254.169.254/latest/meta-data/\",\"primary_color\":\"#007AFF\",\"secondary_color\":\"#5856D6\",\"background_color\":\"#F5F5F7\",\"text_color\":\"#1D1D1F\"}}")
  status_aws=$(resp_status "$resp_aws")
  assert_match "$status_aws" "^(400|422|429)$" "AWS metadata URL rejected for branding logo"

  resp_http=$(api_put "/api/v1/tenants/${tenant_id}" \
    "{\"logo_url\":\"http://example.com/logo.png\"}")
  status_http=$(resp_status "$resp_http")
  assert_match "$status_http" "^(400|422|429)$" "HTTP (non-HTTPS) URL rejected for tenant logo_url"

  resp_av_lo=$(api_put "/api/v1/users/me" "{\"avatar_url\":\"http://127.0.0.1:8080/admin\"}")
  status_av_lo=$(resp_status "$resp_av_lo")
  assert_match "$status_av_lo" "^(400|422|429)$" "localhost rejected for avatar_url"

  resp_av_aws=$(api_put "/api/v1/users/me" \
    "{\"avatar_url\":\"http://169.254.169.254/latest/meta-data/\"}")
  status_av_aws=$(resp_status "$resp_av_aws")
  assert_match "$status_av_aws" "^(400|422|429)$" "AWS metadata URL rejected for avatar_url"

  resp_av_priv=$(api_put "/api/v1/users/me" \
    "{\"avatar_url\":\"http://192.168.1.1/internal-dashboard\"}")
  status_av_priv=$(resp_status "$resp_av_priv")
  assert_match "$status_av_priv" "^(400|422|429)$" "private IP rejected for avatar_url"

  resp_av_ipv6=$(api_put "/api/v1/users/me" \
    "{\"avatar_url\":\"http://[::1]/admin\"}")
  status_av_ipv6=$(resp_status "$resp_av_ipv6")
  assert_match "$status_av_ipv6" "^(400|422|429)$" "IPv6 localhost rejected for avatar_url"

  resp_av_zero=$(api_put "/api/v1/users/me" \
    "{\"avatar_url\":\"http://0.0.0.0/admin\"}")
  status_av_zero=$(resp_status "$resp_av_zero")
  assert_match "$status_av_zero" "^(400|422|429)$" "0.0.0.0 rejected for avatar_url"

  qa_set_token ""
'

run_all
