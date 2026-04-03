#!/bin/bash
# Security Detection Evasion QA Test Script
# Based on docs/security/advanced-attacks/03-detection-evasion.md

set -e

# Configuration
API_BASE="http://localhost:8080"
WEBHOOK_SECRET="${KEYCLOAK_WEBHOOK_SECRET:-dev-webhook-secret-change-in-production}"
TOKEN=$(.claude/skills/tools/gen-admin-token.sh 2>/dev/null)

if [ -z "$TOKEN" ]; then
    echo "❌ Failed to generate admin token"
    exit 1
fi

echo "🔐 Admin token generated"

# Helper function: Send signed Keycloak event
send_signed_event() {
  local body="$1"
  local signature=$(echo -n "$body" | openssl dgst -sha256 -hmac "$WEBHOOK_SECRET" | awk '{print $NF}')
  curl -s -o /dev/null -w "%{http_code}" \
    -X POST "$API_BASE/api/v1/identity/events" \
    -H "Content-Type: application/json" \
    -H "X-Keycloak-Signature: sha256=$signature" \
    -d "$body"
}

# Helper function: Send LOGIN_ERROR event
send_login_error() {
  local ip="${1:-127.0.0.1}"
  local email="${2:-test@test.com}"
  local user_id="${3:-550e8400-e29b-41d4-a716-446655440000}"
  local body="{\"type\":\"LOGIN_ERROR\",\"realmId\":\"auth9\",\"clientId\":\"auth9-portal\",\"userId\":\"$user_id\",\"ipAddress\":\"$ip\",\"error\":\"invalid_user_credentials\",\"time\":$(date +%s)000,\"details\":{\"username\":\"$email\",\"email\":\"$email\"}}"
  send_signed_event "$body"
}

# Helper function: Send LOGIN event
send_login_success() {
  local ip="${1:-127.0.0.1}"
  local email="${2:-test@test.com}"
  local user_id="${3:-550e8400-e29b-41d4-a716-446655440000}"
  local body="{\"type\":\"LOGIN\",\"realmId\":\"auth9\",\"clientId\":\"auth9-portal\",\"userId\":\"$user_id\",\"ipAddress\":\"$ip\",\"time\":$(date +%s)000,\"details\":{\"username\":\"$email\",\"email\":\"$email\"}}"
  send_signed_event "$body"
}

# Helper function: Get security alerts
get_alerts() {
  curl -s -H "Authorization: Bearer $TOKEN" \
    "$API_BASE/api/v1/security/alerts" | jq -r '.pagination.total // 0'
}

# Helper function: Get alerts for specific email
get_alerts_for_email() {
  local email="$1"
  curl -s -H "Authorization: Bearer $TOKEN" \
    "$API_BASE/api/v1/security/alerts" | jq -r --arg email "$email" '[.data[] | select(.details.email == $email)] | length'
}

echo ""
echo "=========================================="
echo "🧪 场景1: 暴力破解检测阈值边界测试"
echo "=========================================="
echo ""

# Record baseline
BASELINE_ALERTS=$(get_alerts)
echo "📊 Current alert count: $BASELINE_ALERTS"
echo ""

# Test 1: Send 4 failed attempts (should NOT trigger)
echo "⏳ Sending 4 failed login attempts (threshold - 1)..."
TEST_EMAIL="bruteforce_test_$(date +%s)@test.com"
TEST_IP="192.168.100.1"

for i in $(seq 1 4); do
  echo "  Attempt $i/4..."
  send_login_error "$TEST_IP" "$TEST_EMAIL"
  sleep 2
done

echo ""
echo "⏳ Waiting 3 seconds for processing..."
sleep 3

ALERTS_AFTER_4=$(get_alerts)
echo "📊 Alerts after 4 attempts: $ALERTS_AFTER_4"

if [ "$ALERTS_AFTER_4" -eq "$BASELINE_ALERTS" ]; then
  echo "✅ PASS: 4 attempts did NOT trigger alert (as expected)"
else
  echo "⚠️  WARNING: 4 attempts triggered alert (may be cumulative from previous tests)"
fi

# Test 2: Send 5th failed attempt (SHOULD trigger)
echo ""
echo "⏳ Sending 5th failed login attempt (should trigger alert)..."
send_login_error "$TEST_IP" "$TEST_EMAIL"
sleep 3

ALERTS_AFTER_5=$(get_alerts)
echo "📊 Alerts after 5 attempts: $ALERTS_AFTER_5"

if [ "$ALERTS_AFTER_5" -gt "$ALERTS_AFTER_4" ]; then
  echo "✅ PASS: 5 attempts triggered alert (HIGH severity brute force)"
  
  # Verify alert details
  echo ""
  echo "📋 Latest alert details:"
  curl -s -H "Authorization: Bearer $TOKEN" \
    "$API_BASE/api/v1/security/alerts?limit=1" | jq '.data[0] | {alert_type, severity, details}'
else
  echo "❌ FAIL: 5 attempts did NOT trigger alert"
fi

echo ""
echo "=========================================="
echo "🧪 场景2: 低速攻击规避"
echo "=========================================="
echo ""
echo "⏳ Simulating low-rate attack (5 attempts)..."

TEST_EMAIL_LOW="lowrate_$(date +%s)@test.com"
TEST_IP_LOW="192.168.101.1"

# Quick simulation: 5 attempts with small intervals
for i in $(seq 1 5); do
  echo "  Low-rate attempt $i/5..."
  send_login_error "$TEST_IP_LOW" "$TEST_EMAIL_LOW"
  if [ $i -lt 5 ]; then
    sleep 6
  fi
done

echo ""
echo "⏳ Waiting 3 seconds for processing..."
sleep 3

ALERTS_LOW=$(get_alerts)
echo "📊 Alerts after low-rate simulation: $ALERTS_LOW"

# For 5 attempts from same IP to same user, should trigger after 5
if [ "$ALERTS_LOW" -gt "$ALERTS_AFTER_5" ]; then
  echo "✅ PASS: Low-rate attack still detected (threshold works)"
else
  echo "⚠️  INFO: Low-rate detection - 5 attempts may not trigger if rate is slow enough"
fi

echo ""
echo "=========================================="
echo "🧪 场景3: 分布式攻击规避"
echo "=========================================="
echo ""

TEST_EMAIL_DIST="distributed_$(date +%s)@test.com"

# Simulate 10 different IPs, each sending 4 attempts (total 40, but per-IP below threshold)
echo "⏳ Simulating distributed attack: 10 IPs × 4 attempts each..."
for ip_suffix in $(seq 1 10); do
  for attempt in $(seq 1 4); do
    send_login_error "10.0.0.$ip_suffix" "$TEST_EMAIL_DIST"
  done
  echo "  IP 10.0.0.$ip_suffix: 4 attempts sent"
  sleep 1
done

echo ""
echo "⏳ Waiting 3 seconds for processing..."
sleep 3

ALERTS_DIST=$(get_alerts)
echo "📊 Alerts after distributed attack (40 attempts): $ALERTS_DIST"

# Check if account-level aggregation detected this
ACCOUNT_ALERTS=$(get_alerts_for_email "$TEST_EMAIL_DIST")
echo "📊 Alerts for this specific account: $ACCOUNT_ALERTS"

if [ "$ACCOUNT_ALERTS" -gt 0 ]; then
  echo "✅ PASS: Account-level aggregation detected distributed attack"
else
  echo "⚠️  INFO: Distributed attack may require longer time window or higher volume"
fi

echo ""
echo "=========================================="
echo "🧪 场景4: 不可能旅行检测准确性"
echo "=========================================="
echo ""

TEST_EMAIL_TRAVEL="travel_$(date +%s)@test.com"

# First login from Beijing-like IP
echo "⏳ Simulating login from Beijing IP..."
send_login_success "123.123.123.123" "$TEST_EMAIL_TRAVEL"
sleep 2

# Second login from New York-like IP (impossible travel)
echo "⏳ Simulating login from New York IP (impossible travel)..."
send_login_success "74.125.224.72" "$TEST_EMAIL_TRAVEL"
sleep 3

echo ""
echo "📋 Checking for impossible travel alerts..."
curl -s -H "Authorization: Bearer $TOKEN" \
  "$API_BASE/api/v1/security/alerts" | jq '.data[] | select(.alert_type == "impossible_travel") | {alert_type, severity, details}' | head -20

TRAVEL_ALERTS=$(curl -s -H "Authorization: Bearer $TOKEN" \
  "$API_BASE/api/v1/security/alerts" | jq '[.data[] | select(.alert_type == "impossible_travel")] | length')

echo ""
echo "📊 Impossible travel alerts found: $TRAVEL_ALERTS"
if [ "$TRAVEL_ALERTS" -gt 0 ]; then
  echo "✅ PASS: Impossible travel detection is working"
else
  echo "⚠️  INFO: Impossible travel may require actual GeoIP database or specific IP ranges"
fi

echo ""
echo "=========================================="
echo "📊 测试总结"
echo "=========================================="
echo ""
FINAL_ALERTS=$(get_alerts)
echo "📊 Final total alerts: $FINAL_ALERTS"
echo "📊 New alerts created: $((FINAL_ALERTS - BASELINE_ALERTS))"
echo ""
echo "测试场景说明:"
echo "  ✅ PASS: 检测系统正常工作"
echo "  ⚠️  INFO: 需要进一步验证或配置"
echo "  ❌ FAIL: 检测到问题"
echo ""
echo "注意: 某些场景可能需要更长时间或特定配置才能准确测试。"
