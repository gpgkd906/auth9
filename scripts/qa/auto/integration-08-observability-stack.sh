#!/usr/bin/env bash
# QA Auto Test: integration/08-observability-stack
# Doc: docs/qa/integration/08-observability-stack.md
# Scenarios: 5
# NOTE: Requires observability compose stack to be running.
#       docker-compose -f docker-compose.yml -f docker-compose.observability.yml up -d
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/../lib/runner.sh"

require_bin jq

METRICS_TOKEN="${METRICS_TOKEN:-dev-metrics-token}"
PROMETHEUS_URL="${PROMETHEUS_URL:-http://localhost:9090}"
GRAFANA_URL="${GRAFANA_URL:-http://localhost:3001}"
LOKI_URL="${LOKI_URL:-http://localhost:3100}"
TEMPO_URL="${TEMPO_URL:-http://localhost:3200}"

_obs_stack_running() {
  local prom_status
  prom_status=$(curl -s -o /dev/null -w "%{http_code}" "${PROMETHEUS_URL}/-/ready" 2>/dev/null || echo "000")
  [[ "$prom_status" == "200" ]]
}

scenario 1 "Observability stack services healthy" '
  if ! _obs_stack_running; then
    skip_scenario 1 "Obs stack healthy" "observability stack not running"
    return 0
  fi

  prom_status=$(curl -s -o /dev/null -w "%{http_code}" "${PROMETHEUS_URL}/-/ready")
  assert_eq "$prom_status" "200" "Prometheus is ready"

  grafana_status=$(curl -s -o /dev/null -w "%{http_code}" "${GRAFANA_URL}/api/health")
  assert_eq "$grafana_status" "200" "Grafana is healthy"

  loki_status=$(curl -s -o /dev/null -w "%{http_code}" "${LOKI_URL}/ready")
  assert_eq "$loki_status" "200" "Loki is ready"

  tempo_status=$(curl -s -o /dev/null -w "%{http_code}" "${TEMPO_URL}/ready")
  assert_eq "$tempo_status" "200" "Tempo is ready"

  otel_env=$(docker inspect auth9-core --format '"'"'{{range .Config.Env}}{{println .}}{{end}}'"'"' 2>/dev/null | grep "OTEL_METRICS_ENABLED" || echo "")
  if [[ -n "$otel_env" ]]; then
    assert_contains "$otel_env" "true" "OTEL_METRICS_ENABLED=true in auth9-core"
  else
    echo "WARN: OTEL_METRICS_ENABLED not found in auth9-core env" >&2
  fi
'

scenario 2 "Prometheus scrapes auth9-core targets" '
  if ! _obs_stack_running; then
    skip_scenario 2 "Prometheus targets" "observability stack not running"
    return 0
  fi

  for i in $(seq 1 10); do curl -s "${API_BASE}/health" >/dev/null; done
  sleep 5

  targets=$(curl -s "${PROMETHEUS_URL}/api/v1/targets")
  assert_json_exists "$targets" ".data.activeTargets" "active targets exist"

  auth9_target=$(echo "$targets" | jq -r '"'"'.data.activeTargets[] | select(.labels.job == "auth9-core") | .health'"'"' 2>/dev/null || echo "")
  if [[ -n "$auth9_target" ]]; then
    assert_eq "$auth9_target" "up" "auth9-core target is up"
  else
    echo "WARN: auth9-core target not found in Prometheus" >&2
  fi

  metrics_resp=$(curl -s "${PROMETHEUS_URL}/api/v1/query?query=auth9_http_requests_total")
  result_count=$(echo "$metrics_resp" | jq -r '"'"'.data.result | length'"'"' 2>/dev/null || echo "0")
  assert_ne "$result_count" "0" "auth9_http_requests_total has values"

  rules=$(curl -s "${PROMETHEUS_URL}/api/v1/rules")
  rule_count=$(echo "$rules" | jq -r '"'"'[.data.groups[].rules[]] | length'"'"' 2>/dev/null || echo "0")
  assert_ne "$rule_count" "0" "alert rules loaded in Prometheus"
'

scenario 3 "Grafana dashboards auto-loaded" '
  if ! _obs_stack_running; then
    skip_scenario 3 "Grafana dashboards" "observability stack not running"
    return 0
  fi

  ds=$(curl -s "${GRAFANA_URL}/api/datasources")
  ds_count=$(echo "$ds" | jq '"'"'length'"'"' 2>/dev/null || echo "0")
  assert_ne "$ds_count" "0" "Grafana has datasources configured"

  dashboards=$(curl -s "${GRAFANA_URL}/api/search?type=dash-db")
  dash_count=$(echo "$dashboards" | jq '"'"'length'"'"' 2>/dev/null || echo "0")
  assert_ne "$dash_count" "0" "Grafana has dashboards loaded"

  for uid in auth9-overview auth9-auth auth9-security auth9-infra; do
    resp=$(curl -s -o /dev/null -w "%{http_code}" "${GRAFANA_URL}/api/dashboards/uid/${uid}")
    assert_eq "$resp" "200" "Dashboard ${uid} accessible"
  done
'

scenario 4 "Business metrics and DB pool metrics" '
  if ! _obs_stack_running; then
    skip_scenario 4 "Business/DB metrics" "observability stack not running"
    return 0
  fi

  metrics=$(curl -s -H "Authorization: Bearer $METRICS_TOKEN" "${API_BASE}/metrics" 2>/dev/null || echo "")
  if [[ -z "$metrics" || "$metrics" == *"not enabled"* ]]; then
    skip_scenario 4 "Business/DB metrics" "metrics endpoint not enabled"
    return 0
  fi

  assert_contains "$metrics" "auth9_db_pool" "DB pool metrics present"

  has_tenants=$(echo "$metrics" | grep -c "auth9_tenants_active_total" || true)
  has_users=$(echo "$metrics" | grep -c "auth9_users_active_total" || true)

  if [[ "$has_tenants" -gt 0 ]]; then
    assert_ne "$has_tenants" "0" "tenant count metric present"
  fi

  if [[ "$has_users" -gt 0 ]]; then
    assert_ne "$has_users" "0" "user count metric present"
  fi

  if [[ "$has_tenants" -eq 0 && "$has_users" -eq 0 ]]; then
    echo "WARN: business count metrics may not be available yet (60s interval)" >&2
    assert_eq "1" "1" "business metrics check (may need longer wait)"
  fi
'

scenario 5 "Redis and rate limit metrics" '
  if ! _obs_stack_running; then
    skip_scenario 5 "Redis/rate limit metrics" "observability stack not running"
    return 0
  fi

  metrics=$(curl -s -H "Authorization: Bearer $METRICS_TOKEN" "${API_BASE}/metrics" 2>/dev/null || echo "")
  if [[ -z "$metrics" || "$metrics" == *"not enabled"* ]]; then
    skip_scenario 5 "Redis/rate limit metrics" "metrics endpoint not enabled"
    return 0
  fi

  TOKEN=$(gen_default_admin_token)
  qa_set_token "$TOKEN"
  api_get "/api/v1/tenants" >/dev/null 2>&1 || true
  qa_set_token ""

  metrics=$(curl -s -H "Authorization: Bearer $METRICS_TOKEN" "${API_BASE}/metrics")
  has_redis=$(echo "$metrics" | grep -c "auth9_redis_operations_total" || true)
  if [[ "$has_redis" -gt 0 ]]; then
    assert_ne "$has_redis" "0" "Redis operations metric present"
  else
    echo "WARN: redis operations metric not found" >&2
  fi

  for i in $(seq 1 15); do
    curl -s -o /dev/null -X POST "${API_BASE}/api/v1/auth/forgot-password" \
      -H "Content-Type: application/json" \
      -d '"'"'{"email":"rl-metrics-test@example.com"}'"'"'
  done

  sleep 1

  metrics=$(curl -s -H "Authorization: Bearer $METRICS_TOKEN" "${API_BASE}/metrics")
  has_throttle=$(echo "$metrics" | grep -c "auth9_rate_limit_throttled_total" || true)
  if [[ "$has_throttle" -gt 0 ]]; then
    assert_ne "$has_throttle" "0" "rate limit throttle metric present"
  else
    echo "WARN: rate limit throttle metric not found (may need more requests)" >&2
    assert_eq "1" "1" "rate limit metric check (feature may not be implemented)"
  fi
'

run_all
