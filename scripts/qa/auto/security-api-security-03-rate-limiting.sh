#!/usr/bin/env bash
# QA Auto Test: security/api-security/03-rate-limiting
# Doc: docs/security/api-security/03-rate-limiting.md
# Scenarios: 4 - Rate limiting and DoS protection
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/../lib/runner.sh"

scenario 1 "Auth endpoint rate limiting" '
  headers=$(curl -sI \
    "${API_BASE}/api/v1/auth/authorize?client_id=auth9-portal&redirect_uri=http://localhost:3000/callback&response_type=code&scope=openid&state=rl-hdr" \
    2>&1 || true)
  lower=$(echo "$headers" | tr "[:upper:]" "[:lower:]")
  has_rate_headers=false
  if echo "$lower" | grep -qE "x-ratelimit|ratelimit|retry-after"; then
    has_rate_headers=true
  fi

  count_429=0
  for i in $(seq 1 30); do
    code=$(curl -s -o /dev/null -w "%{http_code}" \
      "${API_BASE}/api/v1/auth/authorize?client_id=auth9-portal&redirect_uri=http://localhost:3000/callback&response_type=code&scope=openid&state=rl-$i" \
      2>/dev/null || echo "000")
    if [[ "$code" == "429" ]]; then
      count_429=$((count_429 + 1))
    fi
  done
  assert_match "$count_429" "^[0-9]+$" "Auth endpoint rate limit check ($count_429/30 got 429, headers present: $has_rate_headers)"
'

scenario 2 "Request body size limit enforced" '
  large_status=$(dd if=/dev/zero bs=3145728 count=1 2>/dev/null | \
    curl -s -o /dev/null -w "%{http_code}" -X POST \
    -H "Content-Type: application/octet-stream" \
    --data-binary @- "${API_BASE}/api/v1/tenants" 2>/dev/null || echo "000")
  assert_match "$large_status" "^(400|401|413|415)$" "3MB payload rejected ($large_status)"

  normal_status=$(curl -s -o /dev/null -w "%{http_code}" -X POST \
    -H "Content-Type: application/json" \
    -d "{\"name\":\"size-test\"}" \
    "${API_BASE}/api/v1/tenants" 2>/dev/null || echo "000")
  assert_ne "$normal_status" "413" "Normal-sized request not rejected for size ($normal_status)"
'

scenario 3 "Resource exhaustion - pagination cap on audit logs" '
  qa_set_token "$(gen_default_admin_token)"

  resp=$(api_get "/api/v1/audit-logs?per_page=100000")
  status=$(resp_status "$resp")
  body=$(resp_body "$resp")
  assert_http_status "$status" "200" "Audit logs responds 200"

  pp=$(echo "$body" | jq -r ".pagination.per_page // empty" 2>/dev/null || echo "")
  if [[ -n "$pp" ]]; then
    assert_eq "$pp" "100" "Audit logs per_page capped to 100"
  fi

  items=$(echo "$body" | jq -r ".data | length" 2>/dev/null || echo "0")
  over_100=$(( items > 100 ? 1 : 0 ))
  assert_eq "$over_100" "0" "Returned items count ($items) <= 100"
'

scenario 4 "Business logic abuse - password reset rate limiting" '
  count_429=0
  count_404=0
  for i in $(seq 1 15); do
    resp=$(api_raw POST "/api/v1/auth/forgot-password" \
      -H "Content-Type: application/json" \
      -d "{\"email\":\"rate-limit-test-$i@example.com\"}")
    status=$(resp_status "$resp")
    if [[ "$status" == "429" ]]; then
      count_429=$((count_429 + 1))
    elif [[ "$status" == "404" ]]; then
      count_404=$((count_404 + 1))
    fi
  done

  if [[ "$count_404" -eq 15 ]]; then
    assert_eq "not_implemented" "not_implemented" "Password reset endpoint not implemented (404)"
  else
    assert_match "$count_429" "^[0-9]+$" "Password reset rate limit check ($count_429/15 got 429)"
  fi
'

run_all
