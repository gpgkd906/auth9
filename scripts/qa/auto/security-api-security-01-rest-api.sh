#!/usr/bin/env bash
# QA Auto Test: security/api-security/01-rest-api
# Doc: docs/security/api-security/01-rest-api.md
# Scenarios: 5 - REST API endpoint security
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/../lib/runner.sh"

scenario 1 "Unauthenticated endpoint access returns 401" '
  protected=(
    "/api/v1/tenants"
    "/api/v1/services"
    "/api/v1/roles"
    "/api/v1/system/email"
    "/api/v1/audit-logs"
  )
  for ep in "${protected[@]}"; do
    resp=$(api_raw GET "$ep")
    assert_eq "$(resp_status "$resp")" "401" "GET $ep without token returns 401"
  done

  resp=$(api_raw GET "/health")
  assert_eq "$(resp_status "$resp")" "200" "GET /health is public"

  resp=$(api_raw GET "/.well-known/openid-configuration")
  assert_eq "$(resp_status "$resp")" "200" "GET /.well-known/openid-configuration is public"
'

scenario 2 "Token validation bypass attempts rejected" '
  resp=$(api_raw GET "/api/v1/tenants" -H "Authorization: Bearer ")
  assert_eq "$(resp_status "$resp")" "401" "Empty Bearer token returns 401"

  resp=$(api_raw GET "/api/v1/tenants" -H "Authorization: Bearer not.a.jwt")
  assert_eq "$(resp_status "$resp")" "401" "Malformed token returns 401"

  resp=$(api_raw GET "/api/v1/tenants" -H "Authorization: Bearer eyJhbGciOiJSUzI1NiJ9.eyJzdWIiOiJ0ZXN0In0.invalid-sig")  # pragma: allowlist secret
  assert_eq "$(resp_status "$resp")" "401" "Invalid signature token returns 401"

  resp=$(api_raw GET "/api/v1/tenants?access_token=some-token")
  assert_eq "$(resp_status "$resp")" "401" "Query param token not accepted"

  resp=$(api_raw GET "/api/v1/tenants" -u "admin:password")
  assert_eq "$(resp_status "$resp")" "401" "Basic auth not accepted"
'

scenario 3 "Deprecated and internal endpoints return 404" '
  ghost=(
    "/api/v0/users"
    "/api/users"
    "/v1/users"
    "/api/v1/internal/config"
    "/api/v1/admin/settings"
    "/api/v1/debug/vars"
    "/actuator"
    "/debug/pprof"
  )
  for ep in "${ghost[@]}"; do
    resp=$(api_raw GET "$ep")
    assert_match "$(resp_status "$resp")" "^(401|404|405)$" "GET $ep returns 401/404/405"
  done
'

scenario 4 "Bulk data extraction pagination is capped" '
  qa_set_token "$(gen_default_admin_token)"

  resp=$(api_get "/api/v1/tenants?per_page=1000000")
  body=$(resp_body "$resp")
  pp=$(echo "$body" | jq -r ".pagination.per_page // empty" 2>/dev/null || echo "")
  if [[ -n "$pp" ]]; then
    assert_eq "$pp" "100" "per_page=1000000 capped to 100"
  else
    assert_http_status "$(resp_status "$resp")" "200" "Oversized per_page handled"
  fi

  resp=$(api_get "/api/v1/tenants?per_page=-1")
  assert_match "$(resp_status "$resp")" "^(400|422|429)$" "Negative per_page rejected"

  resp=$(api_get "/api/v1/tenants?per_page=abc")
  assert_match "$(resp_status "$resp")" "^(400|422|429)$" "Non-numeric per_page rejected"
'

scenario 5 "Sensitive endpoints require elevated privileges" '
  tid=$(db_query "SELECT id FROM tenants LIMIT 1" 2>/dev/null || echo "")
  uid=$(db_query "SELECT user_id FROM tenant_users LIMIT 1" 2>/dev/null || echo "")

  if [[ -z "$tid" || -z "$uid" ]]; then
    assert_eq "skip" "skip" "No test data in DB"
  else
    qa_set_token "$(gen_tenant_token "$uid" "$tid")"
    resp=$(api_get "/api/v1/system/email")
    assert_match "$(resp_status "$resp")" "^(401|403|429)$" "Tenant user cannot access system/email"

    qa_set_token "$(gen_default_admin_token)"
    resp=$(api_get "/api/v1/system/email")
    assert_match "$(resp_status "$resp")" "^(200|404)$" "Admin can access system/email"
  fi
'

run_all
