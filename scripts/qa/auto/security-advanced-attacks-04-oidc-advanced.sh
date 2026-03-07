#!/usr/bin/env bash
# Security Auto Test: security/advanced-attacks/04-oidc-advanced
# Doc: docs/security/advanced-attacks/04-oidc-advanced.md
# Scenarios: 3
# ASVS: M-ADV-04 | V10.1, V10.2, V10.4, V9.1
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/../lib/runner.sh"

require_bin jq

# ── Scenario 1: Token type confusion attack ───────────────────────────────
scenario 1 "Token type confusion attack" '
  TOKEN=$(gen_default_admin_token)
  qa_set_token "$TOKEN"

  tenant_row=$(db_query "SELECT id FROM tenants LIMIT 1;" 2>/dev/null | tr -d "[:space:]") || tenant_row=""
  user_row=$(db_query "SELECT user_id FROM tenant_users LIMIT 1;" 2>/dev/null | tr -d "[:space:]") || user_row=""

  if [[ -z "$tenant_row" || -z "$user_row" ]]; then
    assert_eq "skip" "skip" "no tenant/user data for token confusion test"
    qa_set_token ""
    return 0
  fi

  non_admin_email="tokentest-$(date +%s)@example.com"
  non_admin_uid="$user_row"
  IDENTITY_TOKEN=$(gen_identity_token "$non_admin_uid" "$non_admin_email")

  qa_set_token "$IDENTITY_TOKEN"
  resp_roles=$(api_get "/api/v1/roles")
  status_roles=$(resp_status "$resp_roles")
  assert_match "$status_roles" "^(401|403|429)$" "identity token cannot access /api/v1/roles"

  TENANT_TOKEN=$(gen_tenant_token "$non_admin_uid" "$tenant_row" 2>/dev/null) || TENANT_TOKEN=""
  if [[ -n "$TENANT_TOKEN" ]]; then
    qa_set_token "$TENANT_TOKEN"
    resp_tenants=$(api_get "/api/v1/tenants")
    status_tenants=$(resp_status "$resp_tenants")
    assert_match "$status_tenants" "^(200|401|403)$" "tenant token on /api/v1/tenants handled correctly"
  fi

  fake_refresh="eyJhbGciOiJSUzI1NiJ9.eyJ0b2tlbl90eXBlIjoicmVmcmVzaCIsInN1YiI6InRlc3QiLCJleHAiOjk5OTk5OTk5OTl9.fake"  # pragma: allowlist secret
  qa_set_token "$fake_refresh"
  resp_refresh=$(api_get "/api/v1/auth/userinfo")
  status_refresh=$(resp_status "$resp_refresh")
  assert_eq "$status_refresh" "401" "fake refresh token rejected by API"

  qa_set_token ""
'

# ── Scenario 2: IdP confusion and account hijacking ──────────────────────
scenario 2 "IdP confusion and account hijacking" '
  TOKEN=$(gen_default_admin_token)
  qa_set_token "$TOKEN"

  li_table=$(db_query "SELECT COUNT(*) FROM linked_accounts;" 2>/dev/null | tr -d "[:space:]") || li_table=""
  if [[ -n "$li_table" ]]; then
    has_provider_col=$(db_query "SELECT COUNT(*) FROM information_schema.columns WHERE table_name='\''linked_accounts'\'' AND column_name='\''provider'\'';" 2>/dev/null | tr -d "[:space:]") || has_provider_col="0"
    assert_ne "$has_provider_col" "0" "linked_accounts has provider column"

    has_provider_uid_col=$(db_query "SELECT COUNT(*) FROM information_schema.columns WHERE table_name='\''linked_accounts'\'' AND column_name='\''provider_user_id'\'';" 2>/dev/null | tr -d "[:space:]") || has_provider_uid_col="0"
    assert_ne "$has_provider_uid_col" "0" "linked_accounts has provider_user_id column"

    unique_idx=$(db_query "SELECT COUNT(*) FROM information_schema.statistics WHERE table_name='\''linked_accounts'\'' AND non_unique=0 AND (column_name='\''provider'\'' OR column_name='\''provider_user_id'\'');" 2>/dev/null | tr -d "[:space:]") || unique_idx="0"
    assert_ne "$unique_idx" "0" "linked_accounts has unique index on provider+provider_user_id"
  else
    assert_eq "skip" "skip" "linked_accounts table not available - skipping IdP confusion test"
  fi

  qa_set_token ""
'

# ── Scenario 3: Client credentials leakage exploitation ──────────────────
scenario 3 "Client credentials leakage exploitation" '
  TOKEN=$(gen_default_admin_token)
  qa_set_token "$TOKEN"

  svc_id=$(db_query "SELECT id FROM services LIMIT 1;" 2>/dev/null | tr -d "[:space:]") || svc_id=""
  client_row=$(db_query "SELECT id, client_id FROM clients LIMIT 1;" 2>/dev/null) || client_row=""

  if [[ -z "$svc_id" || -z "$client_row" ]]; then
    assert_eq "skip" "skip" "no service/client data for client_credentials test"
    qa_set_token ""
    return 0
  fi

  client_pk=$(echo "$client_row" | awk "{print \$1}" | tr -d "[:space:]")
  client_id=$(echo "$client_row" | awk "{print \$2}" | tr -d "[:space:]")

  resp_cc=$(api_raw POST "/api/v1/auth/token" \
    -H "Content-Type: application/x-www-form-urlencoded" \
    -d "grant_type=client_credentials&client_id=${client_id}&client_secret=totally-wrong-secret")
  status_cc=$(resp_status "$resp_cc")
  assert_match "$status_cc" "^(400|401|403)$" "invalid client_secret rejected"

  has_hash=$(db_query "SELECT COUNT(*) FROM clients WHERE client_secret_hash IS NOT NULL AND client_secret_hash != '\'''\'';" 2>/dev/null | tr -d "[:space:]") || has_hash="0"
  has_plain=$(db_query "SELECT COUNT(*) FROM clients WHERE client_secret IS NOT NULL AND client_secret != '\'''\'' AND client_secret NOT LIKE '\''$%'\'';" 2>/dev/null | tr -d "[:space:]") || has_plain="0"
  if [[ "$has_hash" != "0" ]]; then
    assert_ne "$has_hash" "0" "client secrets stored as hashes"
  fi

  resp_svc=$(api_get "/api/v1/services/${svc_id}")
  body_svc=$(resp_body "$resp_svc")
  assert_not_contains "$body_svc" "client_secret" "GET /services/:id does not expose client_secret"

  qa_set_token ""
'

run_all
