#!/usr/bin/env bash
# QA Auto Test: integration/07-observability-metrics
# Doc: docs/qa/integration/07-observability-metrics.md
# Scenarios: 5
# NOTE: Scenarios 1-4 require OTEL_METRICS_ENABLED=true (observability compose).
#       Scenario 5 tests the disabled case (standard compose).
#       If metrics are not enabled, scenarios 1-4 are skipped gracefully.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/../lib/runner.sh"

METRICS_TOKEN="${METRICS_TOKEN:-dev-metrics-token}"

_metrics_enabled() {
  local status
  status=$(curl -s -o /dev/null -w "%{http_code}" \
    -H "Authorization: Bearer $METRICS_TOKEN" \
    "${API_BASE}/metrics")
  [[ "$status" == "200" ]]
}

scenario 1 "/metrics returns Prometheus format" '
  if ! _metrics_enabled; then
    skip_scenario 1 "/metrics Prometheus format" "OTEL_METRICS_ENABLED not set"
    return 0
  fi
  resp=$(curl -s -w "\n%{http_code}" \
    -H "Authorization: Bearer $METRICS_TOKEN" \
    "${API_BASE}/metrics")
  assert_http_status "$(echo "$resp" | tail -1)" 200 "/metrics returns 200"
  body=$(echo "$resp" | sed "\$d")
  assert_contains "$body" "auth9_http_requests_total" "has http_requests_total metric"
  assert_contains "$body" "# HELP" "has HELP lines"
  assert_contains "$body" "# TYPE" "has TYPE lines"
'

scenario 2 "HTTP request metrics increment" '
  if ! _metrics_enabled; then
    skip_scenario 2 "HTTP request metrics" "OTEL_METRICS_ENABLED not set"
    return 0
  fi
  curl -s "${API_BASE}/health" >/dev/null
  resp=$(curl -s -H "Authorization: Bearer $METRICS_TOKEN" "${API_BASE}/metrics")
  assert_contains "$resp" "auth9_http_requests_total" "http_requests_total present after requests"
  assert_contains "$resp" "auth9_http_request_duration_seconds" "http_request_duration present"
'

scenario 3 "X-Request-ID propagation" '
  headers=$(curl -sI "${API_BASE}/health")
  assert_match "$headers" "[Xx]-[Rr]equest-[Ii][Dd]" "auto-generated X-Request-ID present"

  custom_headers=$(curl -sI -H "X-Request-ID: test-req-12345" "${API_BASE}/health")
  assert_contains "$custom_headers" "test-req-12345" "custom X-Request-ID echoed back"
'

scenario 4 "UUID path segments collapsed to {id}" '
  if ! _metrics_enabled; then
    skip_scenario 4 "UUID path collapse" "OTEL_METRICS_ENABLED not set"
    return 0
  fi
  TENANT_ID=$(db_query "SELECT id FROM tenants LIMIT 1;")
  if [[ -z "$TENANT_ID" ]]; then
    echo "No tenant found" >&2
    return 1
  fi
  TOKEN=$(gen_default_admin_token)
  qa_set_token "$TOKEN"
  api_get "/api/v1/tenants/${TENANT_ID}" >/dev/null 2>&1 || true
  qa_set_token ""

  metrics=$(curl -s -H "Authorization: Bearer $METRICS_TOKEN" "${API_BASE}/metrics")
  assert_not_contains "$metrics" "$TENANT_ID" "raw UUID not in metrics path labels"
'

scenario 5 "Metrics disabled returns 404" '
  if _metrics_enabled; then
    skip_scenario 5 "Metrics disabled" "OTEL_METRICS_ENABLED is currently true"
    return 0
  fi
  resp=$(curl -s -w "\n%{http_code}" "${API_BASE}/metrics")
  assert_http_status "$(echo "$resp" | tail -1)" 404 "/metrics returns 404 when disabled"
  health=$(api_raw GET /health)
  assert_http_status "$(resp_status "$health")" 200 "/health still works when metrics disabled"
'

run_all
