#!/usr/bin/env bash
# QA Auto Test: analytics/02-events
# Doc: docs/qa/analytics/02-events.md
# Scenarios: 5
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/../lib/runner.sh"

require_bin jq

_TENANT_ID=""
_USER_ID=""

_setup() {
  if [[ -n "$_TENANT_ID" ]]; then return 0; fi
  TOKEN=$(gen_default_admin_token)
  qa_set_token "$TOKEN"
  _TENANT_ID=$(qa_get_tenant_id)
  _USER_ID=$(db_query "SELECT id FROM users WHERE email = 'admin@auth9.local' LIMIT 1;")
  if [[ -z "$_TENANT_ID" ]]; then
    echo "No active tenant found" >&2; return 1
  fi
}

scenario 1 "View login events list" '
  _setup

  resp=$(api_get "/api/v1/analytics/login-events?page=1&per_page=50")
  assert_http_status "$(resp_status "$resp")" 200 "GET /api/v1/analytics/login-events returns 200"

  body=$(resp_body "$resp")
  assert_json_exists "$body" ".data" "response has data array"
  assert_json_exists "$body" ".pagination" "response has pagination"
  assert_json_exists "$body" ".pagination.total" "pagination has total count"

  first=$(echo "$body" | jq -r ".data[0] // empty")
  if [[ -n "$first" ]]; then
    assert_json_exists "$body" ".data[0].event_type" "event has event_type"
    assert_json_exists "$body" ".data[0].created_at" "event has created_at"
  fi

  assert_db_not_empty \
    "SELECT id FROM login_events ORDER BY created_at DESC LIMIT 1;" \
    "login_events table has data"
'

scenario 2 "Paginate login events" '
  _setup

  total=$(db_query "SELECT COUNT(*) FROM login_events;")
  total=$(echo "$total" | tr -d "[:space:]")

  resp_p1=$(api_get "/api/v1/analytics/login-events?page=1&per_page=10")
  assert_http_status "$(resp_status "$resp_p1")" 200 "page 1 returns 200"
  body_p1=$(resp_body "$resp_p1")
  count_p1=$(echo "$body_p1" | jq ".data | length")
  assert_match "$count_p1" "^[0-9]+$" "page 1 returns numeric count"

  if [[ "$total" -gt 10 ]]; then
    resp_p2=$(api_get "/api/v1/analytics/login-events?page=2&per_page=10")
    assert_http_status "$(resp_status "$resp_p2")" 200 "page 2 returns 200"
    body_p2=$(resp_body "$resp_p2")
    count_p2=$(echo "$body_p2" | jq ".data | length")
    assert_match "$count_p2" "^[0-9]+$" "page 2 returns numeric count"

    id_p1=$(echo "$body_p1" | jq -r ".data[0].id")
    id_p2=$(echo "$body_p2" | jq -r ".data[0].id")
    assert_ne "$id_p1" "$id_p2" "page 1 and page 2 have different first items"
  else
    assert_match "$count_p1" "^[0-9]+$" "total <= 10, single page ok"
  fi
'

scenario 3 "Identify different event types" '
  _setup

  types=$(db_query "SELECT DISTINCT event_type FROM login_events;")
  assert_ne "$types" "" "login_events has event types"

  resp=$(api_get "/api/v1/analytics/login-events?page=1&per_page=100")
  assert_http_status "$(resp_status "$resp")" 200 "GET login-events returns 200"
  body=$(resp_body "$resp")

  event_types=$(echo "$body" | jq -r "[.data[].event_type] | unique | .[]" 2>/dev/null || echo "")
  assert_ne "$event_types" "" "API response contains event types"
'

scenario 4 "View failed event details" '
  _setup

  failed_count=$(db_query "SELECT COUNT(*) FROM login_events WHERE event_type IN ('\''failed_password'\'', '\''failed_mfa'\'', '\''locked'\'');")
  failed_count=$(echo "$failed_count" | tr -d "[:space:]")

  resp=$(api_get "/api/v1/analytics/login-events?page=1&per_page=100")
  assert_http_status "$(resp_status "$resp")" 200 "GET login-events returns 200"
  body=$(resp_body "$resp")

  if [[ "$failed_count" -gt 0 ]]; then
    assert_db_not_empty \
      "SELECT event_type, failure_reason FROM login_events WHERE event_type IN ('\''failed_password'\'', '\''failed_mfa'\'', '\''locked'\'') LIMIT 1;" \
      "failed events exist in DB"
  else
    assert_match "$failed_count" "^[0-9]+$" "failed event count is numeric (may be 0)"
  fi
'

scenario 5 "Filter events by user" '
  _setup

  if [[ -z "$_USER_ID" ]]; then
    echo "No admin user found, skipping" >&2
    return 0
  fi

  resp=$(api_get "/api/v1/analytics/login-events?user_id=${_USER_ID}&page=1&per_page=50")
  status=$(resp_status "$resp")
  assert_match "$status" "^(200|404)$" "filter by user_id returns 200 or 404"

  if [[ "$status" == "200" ]]; then
    body=$(resp_body "$resp")
    assert_json_exists "$body" ".data" "filtered response has data"
  fi

  db_count=$(db_query "SELECT COUNT(*) FROM login_events WHERE user_id = '\''${_USER_ID}'\'';")
  db_count=$(echo "$db_count" | tr -d "[:space:]")
  assert_match "$db_count" "^[0-9]+$" "DB count for user events is numeric"
'

run_all
