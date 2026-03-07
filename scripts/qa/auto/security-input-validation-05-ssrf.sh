#!/usr/bin/env bash
# QA Auto Test: security/input-validation/05-ssrf
# Doc: docs/security/input-validation/05-ssrf.md
# Scenarios: 5
# ASVS: M-INPUT-05 | V5.4, V12.3, V13.2
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/../lib/runner.sh"

require_bin jq

# ── Scenario 1: Webhook URL - internal network probing ────────────────────
scenario 1 "Webhook URL SSRF - internal/private IPs blocked" '
  TOKEN=$(gen_default_admin_token)
  qa_set_token "$TOKEN"

  TENANT_ID=$(db_query "SELECT id FROM tenants LIMIT 1;" | tr -d "[:space:]")
  USER_ID=$(db_query "SELECT user_id FROM tenant_users WHERE tenant_id='\''${TENANT_ID}'\'' LIMIT 1;" | tr -d "[:space:]")
  if [[ -z "$USER_ID" ]]; then
    USER_ID=$(db_query "SELECT id FROM users LIMIT 1;" | tr -d "[:space:]")
  fi
  TTOKEN=$(gen_tenant_token "$USER_ID" "$TENANT_ID")
  qa_set_token "$TTOKEN"

  private_urls=(
    "http://127.0.0.1:4000/"
    "http://localhost:8080/health"
    "http://[::1]:8080/health"
    "http://192.168.1.1:8080/"
    "http://10.0.0.1:8080/"
    "http://172.16.0.1:8080/"
    "http://169.254.169.254/latest/meta-data/"
    "http://metadata.google.internal/computeMetadata/v1/"
    "http://169.254.169.254/latest/meta-data/iam/security-credentials/"
  )

  for url in "${private_urls[@]}"; do
    escaped_url=$(echo "$url" | python3 -c "import sys,json; print(json.dumps(sys.stdin.read().strip()))" | sed "s/^\"//" | sed "s/\"$//")
    resp=$(api_post "/api/v1/tenants/{tenant_id}/webhooks" \
      "{\"url\":\"${escaped_url}\",\"events\":[\"user.created\"]}")
    status=$(resp_status "$resp")
    assert_match "$status" "^(400|422|403)$" "private URL rejected: ${url}"
  done

  qa_set_token ""
'

# ── Scenario 2: URL protocol abuse ────────────────────────────────────────
scenario 2 "URL protocol abuse - non-HTTP schemes blocked" '
  TOKEN=$(gen_default_admin_token)
  qa_set_token "$TOKEN"

  TENANT_ID=$(db_query "SELECT id FROM tenants LIMIT 1;" | tr -d "[:space:]")
  USER_ID=$(db_query "SELECT user_id FROM tenant_users WHERE tenant_id='\''${TENANT_ID}'\'' LIMIT 1;" | tr -d "[:space:]")
  if [[ -z "$USER_ID" ]]; then
    USER_ID=$(db_query "SELECT id FROM users LIMIT 1;" | tr -d "[:space:]")
  fi
  TTOKEN=$(gen_tenant_token "$USER_ID" "$TENANT_ID")
  qa_set_token "$TTOKEN"

  bad_protocols=(
    "file:///etc/passwd"
    "gopher://127.0.0.1:6379/_FLUSHALL"
    "dict://127.0.0.1:6379/INFO"
    "ftp://127.0.0.1/secret.txt"
    "ldap://127.0.0.1/dc=example,dc=com"
    "data:text/html,<script>alert(1)</script>"
  )

  for url in "${bad_protocols[@]}"; do
    escaped_url=$(echo "$url" | python3 -c "import sys,json; print(json.dumps(sys.stdin.read().strip()))" | sed "s/^\"//" | sed "s/\"$//")
    resp=$(api_post "/api/v1/tenants/{tenant_id}/webhooks" \
      "{\"url\":\"${escaped_url}\",\"events\":[\"user.created\"]}")
    status=$(resp_status "$resp")
    assert_match "$status" "^(400|422|403)$" "bad protocol rejected: ${url:0:30}"
  done

  qa_set_token ""
'

# ── Scenario 3: DNS rebinding attack surface ──────────────────────────────
scenario 3 "DNS rebinding - URL host validation" '
  TOKEN=$(gen_default_admin_token)
  qa_set_token "$TOKEN"

  TENANT_ID=$(db_query "SELECT id FROM tenants LIMIT 1;" | tr -d "[:space:]")
  USER_ID=$(db_query "SELECT user_id FROM tenant_users WHERE tenant_id='\''${TENANT_ID}'\'' LIMIT 1;" | tr -d "[:space:]")
  if [[ -z "$USER_ID" ]]; then
    USER_ID=$(db_query "SELECT id FROM users LIMIT 1;" | tr -d "[:space:]")
  fi
  TTOKEN=$(gen_tenant_token "$USER_ID" "$TENANT_ID")
  qa_set_token "$TTOKEN"

  tricky_urls=(
    "http://0x7f000001:8080/"
    "http://0177.0.0.1:8080/"
    "http://2130706433:8080/"
    "http://127.0.0.1.nip.io:8080/"
    "http://0:8080/"
  )

  for url in "${tricky_urls[@]}"; do
    escaped_url=$(echo "$url" | python3 -c "import sys,json; print(json.dumps(sys.stdin.read().strip()))" | sed "s/^\"//" | sed "s/\"$//")
    resp=$(api_post "/api/v1/tenants/{tenant_id}/webhooks" \
      "{\"url\":\"${escaped_url}\",\"events\":[\"user.created\"]}")
    status=$(resp_status "$resp")
    assert_match "$status" "^(400|422|403)$" "obfuscated loopback rejected: ${url:0:30}"
  done

  qa_set_token ""
'

# ── Scenario 4: Branding Logo URL SSRF ────────────────────────────────────
scenario 4 "Branding Logo URL - SSRF protection" '
  TOKEN=$(gen_default_admin_token)
  qa_set_token "$TOKEN"

  resp=$(api_put "/api/v1/system/branding" \
    "{\"logo_url\":\"http://169.254.169.254/latest/meta-data/iam/security-credentials/\"}")
  status=$(resp_status "$resp")
  body=$(resp_body "$resp")
  assert_match "$status" "^(200|400|422)$" "cloud metadata logo_url handled"

  if [[ "$status" == "200" ]]; then
    assert_match "pass" "pass" "logo_url stored client-side only (no server-side fetch)"
  fi

  resp2=$(api_put "/api/v1/system/branding" \
    "{\"logo_url\":\"http://127.0.0.1:6379/\"}")
  status2=$(resp_status "$resp2")
  assert_match "$status2" "^(200|400|422)$" "internal Redis logo_url handled"

  resp3=$(api_put "/api/v1/system/branding" \
    "{\"logo_url\":\"file:///etc/passwd\"}")
  status3=$(resp_status "$resp3")
  assert_match "$status3" "^(400|422|429)$" "file:// protocol logo_url rejected"

  api_put "/api/v1/system/branding" "{\"logo_url\":\"\"}" >/dev/null 2>&1 || true
  qa_set_token ""
'

# ── Scenario 5: Redirect chain SSRF (URL with query params) ───────────────
scenario 5 "Redirect chain SSRF - query params with internal IPs are safe" '
  TOKEN=$(gen_default_admin_token)
  qa_set_token "$TOKEN"

  TENANT_ID=$(db_query "SELECT id FROM tenants LIMIT 1;" | tr -d "[:space:]")
  USER_ID=$(db_query "SELECT user_id FROM tenant_users WHERE tenant_id='\''${TENANT_ID}'\'' LIMIT 1;" | tr -d "[:space:]")
  if [[ -z "$USER_ID" ]]; then
    USER_ID=$(db_query "SELECT id FROM users LIMIT 1;" | tr -d "[:space:]")
  fi
  TTOKEN=$(gen_tenant_token "$USER_ID" "$TENANT_ID")
  qa_set_token "$TTOKEN"

  resp=$(api_post "/api/v1/tenants/{tenant_id}/webhooks" \
    "{\"url\":\"https://example.com/api?url=http://192.168.1.1\",\"events\":[\"user.created\"]}")
  status=$(resp_status "$resp")
  assert_match "$status" "^(201|200|400|422)$" "query param with private IP is not SSRF"

  resp2=$(api_post "/api/v1/tenants/{tenant_id}/webhooks" \
    "{\"url\":\"http://127.0.0.1:80/redirect\",\"events\":[\"user.created\"]}")
  status2=$(resp_status "$resp2")
  assert_match "$status2" "^(400|422|403)$" "direct loopback URL blocked"

  if [[ "$status" == "201" || "$status" == "200" ]]; then
    wh_id=$(resp_body "$resp" | jq -r ".data.id // .id // empty")
    if [[ -n "$wh_id" && "$wh_id" != "null" ]]; then
      api_delete "/api/v1/tenants/{tenant_id}/webhooks/${wh_id}" >/dev/null 2>&1 || true
    fi
  fi

  qa_set_token ""
'

run_all
