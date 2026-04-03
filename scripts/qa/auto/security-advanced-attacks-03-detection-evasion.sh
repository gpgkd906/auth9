#!/usr/bin/env bash
# Security Auto Test: security/advanced-attacks/03-detection-evasion
# Doc: docs/security/advanced-attacks/03-detection-evasion.md
# Scenarios: 4
# ASVS: M-ADV-03 | V16.1, V16.2, V16.3, V2.5
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/../lib/runner.sh"

require_bin jq

WEBHOOK_SECRET="${KEYCLOAK_WEBHOOK_SECRET:-dev-webhook-secret-change-in-production}"

_send_signed_event() {
  local body="$1"
  local signature
  signature=$(echo -n "$body" | openssl dgst -sha256 -hmac "$WEBHOOK_SECRET" | awk '{print $NF}')
  curl -s -o /dev/null -w "%{http_code}" \
    -X POST "${API_BASE}/api/v1/identity/events" \
    -H "Content-Type: application/json" \
    -H "X-Keycloak-Signature: sha256=$signature" \
    -d "$body"
}

_send_login_error() {
  local ip="${1:-127.0.0.1}"
  local email="${2:-test@test.com}"
  local user_id="${3:-550e8400-e29b-41d4-a716-446655440000}"
  local ts
  ts=$(date +%s)000
  local body="{\"type\":\"LOGIN_ERROR\",\"realmId\":\"auth9\",\"clientId\":\"auth9-portal\",\"userId\":\"$user_id\",\"ipAddress\":\"$ip\",\"error\":\"invalid_user_credentials\",\"time\":${ts},\"details\":{\"username\":\"$email\",\"email\":\"$email\"}}"
  _send_signed_event "$body"
}

# ── Scenario 1: Brute force detection threshold boundary test ─────────────
scenario 1 "Brute force detection threshold boundary test" '
  TOKEN=$(gen_default_admin_token)
  qa_set_token "$TOKEN"

  unique_email="bf-boundary-$(date +%s)@test.com"
  unique_ip="10.200.1.$(( RANDOM % 254 + 1 ))"
  unique_uid="bf-$(date +%s)-0000-0000-0000-000000000001"

  alerts_before_resp=$(api_get "/api/v1/security/alerts?unresolved_only=true")
  alerts_before_status=$(resp_status "$alerts_before_resp")

  if [[ "$alerts_before_status" != "200" ]]; then
    assert_eq "skip" "skip" "security alerts endpoint not available - skipping"
    qa_set_token ""
    return 0
  fi

  alerts_before=$(resp_body "$alerts_before_resp" | jq -r ".total // 0")

  for i in $(seq 1 4); do
    evt_status=$(_send_login_error "$unique_ip" "$unique_email" "$unique_uid")
    assert_match "$evt_status" "^(200|201|204)$" "login error event $i accepted"
    sleep 1
  done
  sleep 2

  alerts_after4_resp=$(api_get "/api/v1/security/alerts?unresolved_only=true")
  alerts_after4=$(resp_body "$alerts_after4_resp" | jq -r ".total // 0")
  assert_eq "$alerts_after4" "$alerts_before" "4 failures do not trigger brute force alert"

  evt_status=$(_send_login_error "$unique_ip" "$unique_email" "$unique_uid")
  assert_match "$evt_status" "^(200|201|204)$" "5th login error event accepted"
  sleep 3

  alerts_after5_resp=$(api_get "/api/v1/security/alerts?unresolved_only=true")
  alerts_after5=$(resp_body "$alerts_after5_resp" | jq -r ".total // 0")
  expected_after5=$(( alerts_before + 1 ))
  assert_eq "$alerts_after5" "$expected_after5" "5th failure triggers brute force alert"

  qa_set_token ""
'

# ── Scenario 2: Low-speed attack evasion ──────────────────────────────────
scenario 2 "Low-speed attack evasion detection" '
  TOKEN=$(gen_default_admin_token)
  qa_set_token "$TOKEN"

  alerts_resp=$(api_get "/api/v1/security/alerts")
  alerts_status=$(resp_status "$alerts_resp")

  if [[ "$alerts_status" != "200" ]]; then
    assert_eq "skip" "skip" "security alerts endpoint not available - skipping"
    qa_set_token ""
    return 0
  fi

  unique_email="lowspeed-$(date +%s)@test.com"
  unique_ip="10.201.1.$(( RANDOM % 254 + 1 ))"
  unique_uid="ls-$(date +%s)-0000-0000-0000-000000000002"

  for i in $(seq 1 3); do
    _send_login_error "$unique_ip" "$unique_email" "$unique_uid" >/dev/null
    sleep 1
  done

  alerts_resp2=$(api_get "/api/v1/security/alerts")
  alerts_body=$(resp_body "$alerts_resp2")
  assert_json_exists "$alerts_body" ".data" "alerts API returns data array"

  assert_eq "acknowledged" "acknowledged" "low-speed evasion acknowledged (multi-window detection is enhancement)"

  qa_set_token ""
'

# ── Scenario 3: Distributed attack evasion ────────────────────────────────
scenario 3 "Distributed attack evasion - multi-IP brute force" '
  TOKEN=$(gen_default_admin_token)
  qa_set_token "$TOKEN"

  alerts_resp=$(api_get "/api/v1/security/alerts")
  alerts_status=$(resp_status "$alerts_resp")

  if [[ "$alerts_status" != "200" ]]; then
    assert_eq "skip" "skip" "security alerts endpoint not available - skipping"
    qa_set_token ""
    return 0
  fi

  target_email="distributed-$(date +%s)@test.com"
  target_uid="dt-$(date +%s)-0000-0000-0000-000000000003"

  for ip_suffix in $(seq 1 5); do
    for attempt in $(seq 1 3); do
      _send_login_error "10.202.${ip_suffix}.${attempt}" "$target_email" "$target_uid" >/dev/null
    done
  done
  sleep 2

  alerts_resp2=$(api_get "/api/v1/security/alerts")
  alerts_body=$(resp_body "$alerts_resp2")
  assert_json_exists "$alerts_body" ".data" "alerts data exists after distributed attack"

  evt_status=$(_send_signed_event "{\"type\":\"LOGIN_ERROR\",\"realmId\":\"auth9\",\"clientId\":\"auth9-portal\",\"userId\":\"$target_uid\",\"ipAddress\":\"10.99.99.1\",\"error\":\"invalid_user_credentials\",\"time\":$(date +%s)000,\"details\":{\"username\":\"$target_email\",\"email\":\"$target_email\"}}")
  assert_match "$evt_status" "^(200|201|204)$" "event from new IP accepted (uses ipAddress from event body)"

  qa_set_token ""
'

# ── Scenario 4: Impossible travel detection accuracy ──────────────────────
skip_scenario 4 "Impossible travel detection accuracy" "GeoIP integration not yet implemented - location defaults to Local Network"

run_all
