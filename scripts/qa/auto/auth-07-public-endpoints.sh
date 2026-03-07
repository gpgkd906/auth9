#!/usr/bin/env bash
# QA Auto Test: auth/07-public-endpoints
# Doc: docs/qa/auth/07-public-endpoints.md
# Scenarios: 4 (scenario 5 requires gRPC - skipped)
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/../lib/runner.sh"

scenario 1 "Public branding endpoint (no auth)" '
  resp=$(api_raw GET /api/v1/public/branding)
  assert_http_status "$(resp_status "$resp")" 200 "GET /api/v1/public/branding returns 200"
  body=$(resp_body "$resp")
  assert_json_exists "$body" ".data.primary_color" "has primary_color in data"
'

scenario 2 "Branding update reflected immediately" '
  ADMIN_ID=$(db_query "SELECT id FROM users WHERE email = '\''admin@auth9.local'\'' LIMIT 1;")
  TENANT_ID=$(db_query "SELECT id FROM tenants LIMIT 1;")
  TOKEN=$(gen_tenant_token "$ADMIN_ID" "$TENANT_ID")
  qa_set_token "$TOKEN"

  api_put /api/v1/system/branding "{\"config\":{\"primary_color\":\"#007AFF\",\"secondary_color\":\"#5856D6\",\"background_color\":\"#F5F5F7\",\"text_color\":\"#1D1D1F\"}}" >/dev/null 2>&1 || true

  resp=$(api_put /api/v1/system/branding "{\"config\":{\"primary_color\":\"#ff6600\",\"secondary_color\":\"#5856D6\",\"background_color\":\"#F5F5F7\",\"text_color\":\"#1D1D1F\"}}")
  assert_http_status "$(resp_status "$resp")" 200 "PUT branding returns 200"

  new=$(api_raw GET /api/v1/public/branding)
  new_body=$(resp_body "$new")
  assert_contains "$new_body" "ff6600" "public branding reflects updated color"

  api_put /api/v1/system/branding "{\"config\":{\"primary_color\":\"#007AFF\",\"secondary_color\":\"#5856D6\",\"background_color\":\"#F5F5F7\",\"text_color\":\"#1D1D1F\"}}" >/dev/null 2>&1 || true
  qa_set_token ""
'

scenario 3 "Userinfo with valid identity token" '
  ID_TOKEN=$(gen_default_admin_token)
  if [[ -z "$ID_TOKEN" ]]; then
    echo "Failed to generate admin token" >&2
    return 1
  fi
  qa_set_token "$ID_TOKEN"
  resp=$(api_get /api/v1/auth/userinfo)
  assert_http_status "$(resp_status "$resp")" 200 "GET /api/v1/auth/userinfo returns 200"
  body=$(resp_body "$resp")
  assert_json_exists "$body" ".user_id" "response has user_id field"
  assert_json_exists "$body" ".email" "response has email field"
  qa_set_token ""
'

scenario 4 "Userinfo with invalid token returns 401" '
  resp=$(curl -s -w "\n%{http_code}" \
    -H "Authorization: Bearer invalid-token-12345" \
    "${API_BASE}/api/v1/auth/userinfo")
  assert_http_status "$(echo "$resp" | tail -1)" 401 "invalid token returns 401"

  resp=$(curl -s -w "\n%{http_code}" "${API_BASE}/api/v1/auth/userinfo")
  assert_http_status "$(echo "$resp" | tail -1)" 401 "no token returns 401"
'

run_all
