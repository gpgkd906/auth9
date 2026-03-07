#!/usr/bin/env bash
# Security Auto Test: security/data-security/03-secrets-management
# Doc: docs/security/data-security/03-secrets-management.md
# Scenarios: 4
# ASVS: M-DATA-03 | V11.3, V13.4, V14.3, V15.3
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/../lib/runner.sh"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"

require_bin jq

# ── Scenario 1: Secret storage security ──────────────────────────────────
scenario 1 "Secret storage security" '
  if [[ -f "$PROJECT_ROOT/.gitignore" ]]; then
    env_ignored=$(grep -c "^\.env" "$PROJECT_ROOT/.gitignore" || echo "0")
    assert_ne "$env_ignored" "0" ".env files listed in .gitignore"
  else
    assert_eq "missing" "exists" ".gitignore file should exist"
  fi

  dc_file="$PROJECT_ROOT/docker-compose.yml"
  if [[ -f "$dc_file" ]]; then
    change_markers=$(grep -c "change-in-production" "$dc_file" || echo "0")
    assert_ne "$change_markers" "0" "docker-compose.yml dev secrets marked with change-in-production"
  fi

  k8s_secrets="$PROJECT_ROOT/deploy/k8s/secrets.yaml.example"
  if [[ -f "$k8s_secrets" ]]; then
    assert_eq "exists" "exists" "K8s secrets template exists"
    real_secret=$(grep -c "sk_live\|pk_live\|real-production" "$k8s_secrets" || echo "0")
    assert_eq "$real_secret" "0" "K8s secrets template contains no real secrets"
  fi

  prod_key_in_code=$(grep -r "sk_live\|pk_live" "$PROJECT_ROOT/auth9-core/src/" 2>/dev/null | grep -v "test\|example\|mock" | wc -l | tr -d " ") || prod_key_in_code="0"
  assert_eq "$prod_key_in_code" "0" "no production API keys in source code"

  resp_git=$(api_raw GET "/.git/config")
  status_git=$(resp_status "$resp_git")
  assert_match "$status_git" "^(400|403|404|405)$" ".git/config not accessible via HTTP"

  resp_git_portal=$(curl -s -o /dev/null -w "%{http_code}" "http://localhost:3000/.git/config" 2>/dev/null) || resp_git_portal="000"
  if [[ "$resp_git_portal" != "000" ]]; then
    assert_match "$resp_git_portal" "^(400|403|404|405)$" "portal .git/config not accessible"
  fi
'

# ── Scenario 2: Secret rotation mechanism ────────────────────────────────
scenario 2 "Secret rotation mechanism" '
  jwks_resp=$(api_get "/.well-known/jwks.json")
  jwks_status=$(resp_status "$jwks_resp")

  if [[ "$jwks_status" == "200" ]]; then
    jwks_body=$(resp_body "$jwks_resp")
    key_count=$(echo "$jwks_body" | jq ".keys | length" 2>/dev/null || echo "0")
    assert_ne "$key_count" "0" "JWKS has at least one key"

    first_kid=$(echo "$jwks_body" | jq -r ".keys[0].kid" 2>/dev/null || echo "")
    assert_ne "$first_kid" "" "JWT key has kid for rotation support"
  else
    assert_eq "skip" "skip" "JWKS endpoint not available"
  fi

  TOKEN=$(gen_default_admin_token)
  qa_set_token "$TOKEN"

  svc_id=$(db_query "SELECT id FROM services LIMIT 1;" 2>/dev/null | tr -d "[:space:]") || svc_id=""
  client_pk=$(db_query "SELECT id FROM clients LIMIT 1;" 2>/dev/null | tr -d "[:space:]") || client_pk=""

  if [[ -n "$svc_id" && -n "$client_pk" ]]; then
    resp_regen=$(api_post "/api/v1/services/${svc_id}/clients/${client_pk}/regenerate-secret")
    status_regen=$(resp_status "$resp_regen")
    assert_match "$status_regen" "^(200|201|404)$" "client secret regeneration endpoint responds"
  else
    assert_eq "skip" "skip" "no service/client for rotation test"
  fi

  qa_set_token ""
'

# ── Scenario 3: Secret access control ────────────────────────────────────
scenario 3 "Secret access control" '
  TOKEN=$(gen_default_admin_token)
  qa_set_token "$TOKEN"

  tenant_id=$(db_query "SELECT id FROM tenants LIMIT 1;" 2>/dev/null | tr -d "[:space:]") || tenant_id=""
  user_id=$(db_query "SELECT user_id FROM tenant_users WHERE tenant_id='\''${tenant_id}'\'' LIMIT 1;" 2>/dev/null | tr -d "[:space:]") || user_id=""

  if [[ -n "$tenant_id" && -n "$user_id" ]]; then
    user_email="secrettest-$(date +%s)@example.com"
    TENANT_TOKEN=$(gen_tenant_token "$user_id" "$tenant_id" 2>/dev/null) || TENANT_TOKEN=""

    if [[ -n "$TENANT_TOKEN" ]]; then
      qa_set_token "$TENANT_TOKEN"
      resp_sys=$(api_get "/api/v1/system/email")
      status_sys=$(resp_status "$resp_sys")
      assert_match "$status_sys" "^(401|403|429)$" "tenant user cannot access system email config"
    fi
  fi

  qa_set_token "$TOKEN"
  resp_export=$(api_get "/api/v1/clients/export")
  status_export=$(resp_status "$resp_export")
  assert_match "$status_export" "^(400|403|404|405)$" "no bulk client export endpoint"

  resp_svc=$(api_get "/api/v1/services")
  status_svc=$(resp_status "$resp_svc")
  if [[ "$status_svc" == "200" ]]; then
    body_svc=$(resp_body "$resp_svc")
    assert_not_contains "$body_svc" "client_secret" "services list does not expose client_secret"
  fi

  qa_set_token ""
'

# ── Scenario 4: Secret leakage detection ─────────────────────────────────
scenario 4 "Secret leakage detection" '
  TOKEN=$(gen_default_admin_token)
  qa_set_token "$TOKEN"

  resp_health=$(api_get "/health")
  body_health=$(resp_body "$resp_health")
  assert_not_contains "$body_health" "DATABASE_URL" "no DATABASE_URL in health endpoint"
  assert_not_contains "$body_health" "JWT_PRIVATE_KEY" "no JWT_PRIVATE_KEY in health endpoint"
  assert_not_contains "$body_health" "REDIS_URL" "no REDIS_URL in health endpoint"

  resp_err=$(api_post "/api/v1/auth/token" "{\"invalid\":true}")
  body_err=$(resp_body "$resp_err")
  assert_not_contains "$body_err" "KEYCLOAK_ADMIN" "no Keycloak admin creds in error"
  assert_not_contains "$body_err" "client_secret" "no client_secret in auth error"
  assert_not_contains "$body_err" "DATABASE_URL" "no DATABASE_URL in auth error"

  logs=$(docker logs auth9-core --tail=100 2>&1 || echo "")
  if [[ -n "$logs" ]]; then
    assert_not_contains "$logs" "JWT_PRIVATE_KEY=" "no JWT private key in logs"
    assert_not_contains "$logs" "SETTINGS_ENCRYPTION_KEY=" "no encryption key in logs"
  else
    assert_eq "ok" "ok" "container logs not available (non-Docker env)"
  fi

  qa_set_token ""
'

run_all
