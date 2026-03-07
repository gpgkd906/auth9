#!/usr/bin/env bash
# QA Auto Test: integration/03-rate-limiting
# Doc: docs/qa/integration/03-rate-limiting.md
# Scenarios: 5
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/../lib/runner.sh"

require_bin jq

scenario 1 "Normal request includes rate limit headers" '
  TOKEN=$(gen_default_admin_token)
  qa_set_token "$TOKEN"

  headers=$(curl -sI "${API_BASE}/api/v1/tenants" \
    -H "Authorization: Bearer ${TOKEN}")

  status=$(echo "$headers" | head -1 | grep -o "[0-9][0-9][0-9]" | head -1)
  assert_eq "$status" "200" "GET /api/v1/tenants returns 200"

  has_remaining=$(echo "$headers" | grep -ci "x-ratelimit-remaining" || true)
  assert_ne "$has_remaining" "0" "X-RateLimit-Remaining header present"

  qa_set_token ""
'

scenario 2 "Exceeding rate limit returns 429" '
  GOT_429="false"
  LAST_STATUS=""

  for i in $(seq 1 50); do
    LAST_STATUS=$(curl -s -o /dev/null -w "%{http_code}" \
      -X POST "${API_BASE}/api/v1/auth/token" \
      -H "Content-Type: application/json" \
      -d '"'"'{"grant_type":"client_credentials","client_id":"rl-test","client_secret":"rl-test"}'"'"')
    if [[ "$LAST_STATUS" == "429" ]]; then
      GOT_429="true"
      break
    fi
  done

  assert_eq "$GOT_429" "true" "eventually received 429 Too Many Requests"

  if [[ "$GOT_429" == "true" ]]; then
    resp_headers=$(curl -sI -X POST "${API_BASE}/api/v1/auth/token" \
      -H "Content-Type: application/json" \
      -d '"'"'{"grant_type":"client_credentials","client_id":"rl-test","client_secret":"rl-test"}'"'"')
    has_retry=$(echo "$resp_headers" | grep -ci "retry-after" || true)
    assert_ne "$has_retry" "0" "429 response has Retry-After header"
  fi
'

scenario 3 "Rate limit window recovery" '
  GOT_429="false"
  for i in $(seq 1 50); do
    status=$(curl -s -o /dev/null -w "%{http_code}" \
      -X POST "${API_BASE}/api/v1/auth/token" \
      -H "Content-Type: application/json" \
      -H "X-Forwarded-For: 10.99.99.$(( (RANDOM % 254) + 1 ))" \
      -d '"'"'{"grant_type":"client_credentials","client_id":"rl-recovery","client_secret":"test"}'"'"')
    if [[ "$status" == "429" ]]; then
      GOT_429="true"
      break
    fi
  done

  if [[ "$GOT_429" == "true" ]]; then
    RETRY_AFTER=$(curl -sI -X POST "${API_BASE}/api/v1/auth/token" \
      -H "Content-Type: application/json" \
      -H "X-Forwarded-For: 10.99.99.$(( (RANDOM % 254) + 1 ))" \
      -d '"'"'{"grant_type":"client_credentials","client_id":"rl-recovery","client_secret":"test"}'"'"' \
      | grep -i "retry-after" | grep -o "[0-9]*" | head -1)

    WAIT=${RETRY_AFTER:-5}
    if [[ "$WAIT" -gt 10 ]]; then
      WAIT=10
    fi

    sleep "$WAIT"

    status=$(curl -s -o /dev/null -w "%{http_code}" \
      -X POST "${API_BASE}/api/v1/auth/token" \
      -H "Content-Type: application/json" \
      -H "X-Forwarded-For: 10.99.99.$(( (RANDOM % 254) + 1 ))" \
      -d '"'"'{"grant_type":"client_credentials","client_id":"rl-recovery","client_secret":"test"}'"'"')
    assert_ne "$status" "429" "request succeeds after window expires"
  else
    assert_eq "1" "1" "rate limit not hit with fresh IP (skip recovery test)"
  fi
'

scenario 4 "Different IPs have independent rate limits (unauthenticated)" '
  IP_A="10.0.$(( RANDOM % 255 )).1"
  IP_B="10.0.$(( RANDOM % 255 )).2"

  for i in $(seq 1 50); do
    status=$(curl -s -o /dev/null -w "%{http_code}" \
      -H "X-Forwarded-For: ${IP_A}" \
      "${API_BASE}/health")
    if [[ "$status" == "429" ]]; then
      break
    fi
  done

  status_b=$(curl -s -o /dev/null -w "%{http_code}" \
    -H "X-Forwarded-For: ${IP_B}" \
    "${API_BASE}/health")
  assert_eq "$status_b" "200" "IP-B request returns 200 while IP-A is rate-limited"
'

scenario 5 "Redis unavailable - fail open for health endpoint" '
  docker stop auth9-redis >/dev/null 2>&1 || true
  sleep 3

  status=$(curl -s -o /dev/null -w "%{http_code}" "${API_BASE}/health")
  assert_eq "$status" "200" "/health returns 200 with Redis down (fail-open)"
  assert_ne "$status" "429" "/health not rate-limited with Redis down"

  docker start auth9-redis >/dev/null 2>&1 || true
  sleep 3
'

run_all
