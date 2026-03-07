#!/usr/bin/env bash
# QA Auto Test: integration/04-health-check
# Doc: docs/qa/integration/04-health-check.md
# Scenarios: 5
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/../lib/runner.sh"

scenario 1 "Health check normal response" '
  resp=$(api_raw GET /health)
  assert_http_status "$(resp_status "$resp")" 200 "GET /health returns 200"
  body=$(resp_body "$resp")
  assert_json_field "$body" ".status" "healthy" "status = healthy"
  assert_json_exists "$body" ".version" "version field present"
'

scenario 2 "Readiness probe - all deps normal" '
  resp=$(api_raw GET /ready)
  assert_http_status "$(resp_status "$resp")" 200 "GET /ready returns 200"
'

scenario 3 "Readiness probe - DB down" '
  docker stop auth9-tidb >/dev/null 2>&1
  sleep 3
  resp=$(api_raw GET /ready)
  assert_http_status "$(resp_status "$resp")" 503 "GET /ready returns 503 with DB down"
  health=$(api_raw GET /health)
  assert_http_status "$(resp_status "$health")" 200 "/health still 200 with DB down"
  docker start auth9-tidb >/dev/null 2>&1
  sleep 5
'

scenario 4 "Readiness probe - Redis down" '
  docker stop auth9-redis >/dev/null 2>&1
  sleep 3
  resp=$(api_raw GET /ready)
  assert_http_status "$(resp_status "$resp")" 503 "GET /ready returns 503 with Redis down"
  health=$(api_raw GET /health)
  assert_http_status "$(resp_status "$health")" 200 "/health still 200 with Redis down"
  docker start auth9-redis >/dev/null 2>&1
  sleep 5
'

scenario 5 "Health endpoints require no auth" '
  resp=$(api_raw GET /health)
  assert_http_status "$(resp_status "$resp")" 200 "No-auth /health returns 200"
  resp=$(api_raw GET /ready)
  assert_http_status "$(resp_status "$resp")" 200 "No-auth /ready returns 200"
  resp=$(curl -s -w "\n%{http_code}" -H "Authorization: Bearer invalid-token" "${API_BASE}/health")
  assert_http_status "$(echo "$resp" | tail -1)" 200 "Invalid-auth /health returns 200"
  resp=$(curl -s -w "\n%{http_code}" -H "Authorization: Bearer invalid-token" "${API_BASE}/ready")
  assert_http_status "$(echo "$resp" | tail -1)" 200 "Invalid-auth /ready returns 200"
'

run_all
