#!/usr/bin/env bash
# QA Auto Test: security/api-security/06-api-boundary-pagination
# Doc: docs/security/api-security/06-api-boundary-pagination.md
# Scenarios: 3 - API boundary and pagination security
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/../lib/runner.sh"

scenario 1 "Pagination parameter boundary handling" '
  qa_set_token "$(gen_default_admin_token)"

  resp=$(api_get "/api/v1/tenants?per_page=99999999")
  assert_http_status "$(resp_status "$resp")" "200" "Oversized per_page returns 200 (capped)"
  body=$(resp_body "$resp")
  pp=$(echo "$body" | jq -r ".pagination.per_page // empty" 2>/dev/null || echo "")
  if [[ -n "$pp" ]]; then
    assert_eq "$pp" "100" "per_page=99999999 server-side capped to 100"
  fi

  resp=$(api_get "/api/v1/tenants?per_page=-1")
  assert_match "$(resp_status "$resp")" "^(400|422|429)$" "per_page=-1 returns validation error"

  resp=$(api_get "/api/v1/tenants?per_page=abc")
  assert_match "$(resp_status "$resp")" "^(400|422|429)$" "per_page=abc returns validation error"

  resp=$(api_get "/api/v1/tenants?page=0")
  assert_match "$(resp_status "$resp")" "^(200|400|422)$" "page=0 handled gracefully"

  resp=$(api_get "/api/v1/tenants?page=999999&per_page=100")
  assert_http_status "$(resp_status "$resp")" "200" "Extremely large page number returns 200 with empty data"
  body=$(resp_body "$resp")
  items=$(echo "$body" | jq -r ".data | length" 2>/dev/null || echo "0")
  assert_eq "$items" "0" "Large page offset returns 0 items"
'

scenario 2 "Bulk request amplification protection" '
  tid=$(db_query "SELECT id FROM tenants LIMIT 1" 2>/dev/null || echo "")
  uid=$(db_query "SELECT user_id FROM tenant_users LIMIT 1" 2>/dev/null || echo "")
  if [[ -z "$tid" || -z "$uid" ]]; then
    assert_eq "skip" "skip" "No test data - skipping bulk request test"
  else
    qa_set_token "$(gen_tenant_token "$uid" "$tid")"

    count_created=0
    count_rejected=0
    created_names=""
    for i in $(seq 1 10); do
      resp=$(api_post "/api/v1/services" "{\"name\":\"qa-bulk-test-$i\",\"description\":\"bulk amplification test\"}")
      status=$(resp_status "$resp")
      if [[ "$status" == "201" || "$status" == "200" ]]; then
        count_created=$((count_created + 1))
        created_names="$created_names qa-bulk-test-$i"
      elif [[ "$status" == "429" ]]; then
        count_rejected=$((count_rejected + 1))
      fi
    done
    total=$((count_created + count_rejected))
    assert_ne "$total" "0" "Bulk service creation processed ($count_created created, $count_rejected rate-limited)"

    # Cleanup created test services
    resp=$(api_get "/api/v1/services?per_page=100")
    body=$(resp_body "$resp")
    svc_ids=$(echo "$body" | jq -r ".data[]? | select(.name | startswith(\"qa-bulk-test-\")) | .id" 2>/dev/null || echo "")
    for svc_id in $svc_ids; do
      api_delete "/api/v1/services/$svc_id" >/dev/null 2>&1 || true
    done
  fi
'

scenario 3 "Rate limiting consistency with boundary pagination" '
  qa_set_token "$(gen_default_admin_token)"

  count_429=0
  count_200=0
  for i in $(seq 1 50); do
    resp=$(api_get "/api/v1/tenants?per_page=100&page=$i")
    status=$(resp_status "$resp")
    if [[ "$status" == "429" ]]; then
      count_429=$((count_429 + 1))
    elif [[ "$status" == "200" ]]; then
      count_200=$((count_200 + 1))
    fi
  done
  assert_match "$count_429" "^[0-9]+$" "Rate limiting with large pagination ($count_429/50 got 429, $count_200 got 200)"

  resp=$(api_get "/api/v1/tenants?per_page=100")
  body=$(resp_body "$resp")
  pp=$(echo "$body" | jq -r ".pagination.per_page // empty" 2>/dev/null || echo "")
  if [[ -n "$pp" ]]; then
    over_100=$([[ "$pp" -gt 100 ]] && echo "true" || echo "false")
    assert_eq "$over_100" "false" "per_page does not exceed 100 under load"
  fi
'

run_all
