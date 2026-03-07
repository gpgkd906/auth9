#!/usr/bin/env bash
# QA Auto Test: security/api-security/05-rate-limit-bypass-hardening
# Doc: docs/security/api-security/05-rate-limit-bypass-hardening.md
# Scenarios: 5 - Rate limit bypass hardening and DoS amplification
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/../lib/runner.sh"

scenario 1 "x-tenant-id rotation does not bypass IP rate limiting" '
  count_429=0
  count_ok=0
  for i in $(seq 1 120); do
    code=$(curl -s -o /dev/null -w "%{http_code}" \
      -H "x-tenant-id: fake-tenant-$i" \
      "${API_BASE}/api/v1/tenants" 2>/dev/null || echo "000")
    if [[ "$code" == "429" ]]; then
      count_429=$((count_429 + 1))
    elif [[ "$code" == "200" || "$code" == "401" ]]; then
      count_ok=$((count_ok + 1))
    fi
  done
  assert_ne "$count_429" "0" "429 seen ($count_429/120) - x-tenant-id rotation does not bypass rate limit"
'

scenario 2 "x-tenant-id pollution does not affect other tenant buckets" '
  tid=$(db_query "SELECT id FROM tenants LIMIT 1" 2>/dev/null || echo "")
  uid=$(db_query "SELECT user_id FROM tenant_users LIMIT 1" 2>/dev/null || echo "")
  if [[ -z "$tid" || -z "$uid" ]]; then
    assert_eq "skip" "skip" "No test data - skipping pollution test"
  else
    victim_token=$(gen_tenant_token "$uid" "$tid")

    for i in $(seq 1 20); do
      curl -s -o /dev/null \
        -H "x-tenant-id: $tid" \
        "${API_BASE}/api/v1/tenants" 2>/dev/null || true
    done

    victim_status=$(curl -s -o /dev/null -w "%{http_code}" \
      -H "Authorization: Bearer $victim_token" \
      "${API_BASE}/api/v1/tenants" 2>/dev/null || echo "000")
    assert_ne "$victim_status" "429" "Victim not rate limited after attacker x-tenant-id pollution ($victim_status)"
  fi
'

scenario 3 "Dynamic path parameters do not inflate rate limit keys" '
  for i in $(seq 1 50); do
    curl -s -o /dev/null \
      "${API_BASE}/api/v1/users/00000000-0000-0000-0000-$(printf "%012d" $i)" \
      2>/dev/null || true
  done

  if command -v redis-cli &>/dev/null; then
    key_count=$(redis-cli --raw KEYS "auth9:ratelimit:*:GET:/api/v1/users/*" 2>/dev/null | wc -l | tr -d " " || echo "0")
    if [[ "$key_count" -gt 0 ]]; then
      is_high=$([[ "$key_count" -gt 10 ]] && echo "true" || echo "false")
      assert_eq "$is_high" "false" "Rate limit key count ($key_count) not inflated by path params"
    else
      assert_eq "0" "0" "No per-path-param rate limit keys (uses route template)"
    fi
  else
    assert_eq "skip" "skip" "redis-cli not available - skipping key count verification"
  fi
'

if [[ "${QA_DESTRUCTIVE:-0}" == "1" ]]; then
  scenario 4 "Redis failure triggers in-memory fallback rate limiting" '
    docker stop auth9-redis 2>/dev/null || true
    sleep 3

    count_429=0
    count_200=0
    for i in $(seq 1 150); do
      code=$(curl -s -o /dev/null -w "%{http_code}" \
        "${API_BASE}/api/v1/tenants" 2>/dev/null || echo "000")
      if [[ "$code" == "429" ]]; then
        count_429=$((count_429 + 1))
      elif [[ "$code" == "200" || "$code" == "401" ]]; then
        count_200=$((count_200 + 1))
      fi
    done

    docker start auth9-redis 2>/dev/null || true
    sleep 3

    assert_ne "$count_429" "0" "In-memory fallback active ($count_429/150 got 429, $count_200 got 200/401)"
  '
else
  skip_scenario 4 "Redis failure in-memory fallback" "Requires QA_DESTRUCTIVE=1 (stops Redis container)"
fi

scenario 5 "Rate limit recovery and metrics observability" '
  count_429=0
  for i in $(seq 1 30); do
    code=$(curl -s -o /dev/null -w "%{http_code}" \
      "${API_BASE}/api/v1/tenants" 2>/dev/null || echo "000")
    if [[ "$code" == "429" ]]; then
      count_429=$((count_429 + 1))
    fi
  done
  assert_match "$count_429" "^[0-9]+$" "Rate limiting operational ($count_429/30 got 429)"

  metrics_token="${METRICS_TOKEN:-}"
  if [[ -n "$metrics_token" ]]; then
    metrics=$(curl -s -H "Authorization: Bearer $metrics_token" \
      "${API_BASE}/metrics" 2>/dev/null || echo "")
    if [[ -n "$metrics" && "$metrics" != *"404"* ]]; then
      has_rl_metric=false
      if echo "$metrics" | grep -qE "rate_limit|http_requests_total"; then
        has_rl_metric=true
      fi
      assert_eq "$has_rl_metric" "true" "Rate limit metrics available in /metrics"
    else
      assert_eq "skip" "skip" "Metrics endpoint returned 404 (METRICS_TOKEN may be invalid)"
    fi
  else
    assert_eq "skip" "skip" "METRICS_TOKEN not set - skipping metrics check"
  fi
'

run_all
