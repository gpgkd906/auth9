#!/usr/bin/env bash
# Security Auto Test: security/advanced-attacks/05-webhook-forgery
# Doc: docs/security/advanced-attacks/05-webhook-forgery.md
# Scenarios: 2
# ASVS: M-ADV-05 | V10.5, V13.2, V16.2
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/../lib/runner.sh"

require_bin jq
require_bin openssl

WEBHOOK_ENDPOINT="/api/v1/identity/events"
VALID_SECRET="${KEYCLOAK_WEBHOOK_SECRET:-dev-webhook-secret-change-in-production}"

scenario 1 "Inbound Webhook signature forgery" '
  CURRENT_TIME_MILLIS=$(($(date +%s) * 1000))
  EVENT="{\"type\":\"LOGIN\",\"realmId\":\"auth9\",\"userId\":\"test-user\",\"time\":${CURRENT_TIME_MILLIS}}"

  # No signature header
  resp=$(api_raw POST "$WEBHOOK_ENDPOINT" \
    -H "Content-Type: application/json" \
    -d "$EVENT")
  status=$(resp_status "$resp")
  assert_match "$status" "^(401|403|429)$" "missing signature header rejected"

  # Empty signature header
  resp=$(api_raw POST "$WEBHOOK_ENDPOINT" \
    -H "Content-Type: application/json" \
    -H "X-Keycloak-Signature: " \
    -d "$EVENT")
  status=$(resp_status "$resp")
  assert_match "$status" "^(401|403|429)$" "empty signature rejected"

  # Wrong signature
  resp=$(api_raw POST "$WEBHOOK_ENDPOINT" \
    -H "Content-Type: application/json" \
    -H "X-Keycloak-Signature: sha256=0000000000000000000000000000000000000000000000000000000000000000" \
    -d "$EVENT")
  status=$(resp_status "$resp")
  assert_match "$status" "^(401|403|429)$" "wrong signature rejected"

  # Correct signature should succeed
  SIGNATURE=$(echo -n "$EVENT" | openssl dgst -sha256 -hmac "$VALID_SECRET" | awk "{print \$2}")
  resp=$(api_raw POST "$WEBHOOK_ENDPOINT" \
    -H "Content-Type: application/json" \
    -H "X-Keycloak-Signature: sha256=$SIGNATURE" \
    -d "$EVENT")
  status=$(resp_status "$resp")
  assert_match "$status" "^(200|204)$" "valid signature accepted"

  # Signature with wrong format (no sha256= prefix)
  resp=$(api_raw POST "$WEBHOOK_ENDPOINT" \
    -H "Content-Type: application/json" \
    -H "X-Keycloak-Signature: $SIGNATURE" \
    -d "$EVENT")
  status=$(resp_status "$resp")
  assert_match "$status" "^(200|204|401|403|429)$" "signature without sha256= prefix (server may accept)"

  # Signature computed on different payload
  DIFFERENT_EVENT="{\"type\":\"LOGOUT\",\"realmId\":\"auth9\",\"userId\":\"other\",\"time\":${CURRENT_TIME_MILLIS}}"
  resp=$(api_raw POST "$WEBHOOK_ENDPOINT" \
    -H "Content-Type: application/json" \
    -H "X-Keycloak-Signature: sha256=$SIGNATURE" \
    -d "$DIFFERENT_EVENT")
  status=$(resp_status "$resp")
  assert_match "$status" "^(401|403|429)$" "signature mismatch (different payload) rejected"
'

scenario 2 "Webhook replay attack" '
  CURRENT_TIME_MILLIS=$(($(date +%s) * 1000))
  EVENT_ID="event-replay-test-$(date +%s)"
  EVENT="{\"type\":\"LOGIN\",\"realmId\":\"auth9\",\"userId\":\"test-user\",\"time\":${CURRENT_TIME_MILLIS},\"id\":\"${EVENT_ID}\"}"
  SIGNATURE=$(echo -n "$EVENT" | openssl dgst -sha256 -hmac "$VALID_SECRET" | awk "{print \$2}")

  # First send should succeed
  resp=$(api_raw POST "$WEBHOOK_ENDPOINT" \
    -H "Content-Type: application/json" \
    -H "X-Keycloak-Signature: sha256=$SIGNATURE" \
    -d "$EVENT")
  status=$(resp_status "$resp")
  assert_match "$status" "^(200|204)$" "first webhook send accepted"

  # Immediate replay (should be deduplicated - returns 204 idempotently)
  resp=$(api_raw POST "$WEBHOOK_ENDPOINT" \
    -H "Content-Type: application/json" \
    -H "X-Keycloak-Signature: sha256=$SIGNATURE" \
    -d "$EVENT")
  status=$(resp_status "$resp")
  assert_match "$status" "^(200|204)$" "replay returns idempotent response (deduplicated)"

  # Old event timestamp (should be rejected as expired)
  OLD_EVENT="{\"type\":\"LOGIN\",\"realmId\":\"auth9\",\"userId\":\"test-user\",\"time\":1600000000000,\"id\":\"event-old-replay\"}"
  OLD_SIGNATURE=$(echo -n "$OLD_EVENT" | openssl dgst -sha256 -hmac "$VALID_SECRET" | awk "{print \$2}")
  resp=$(api_raw POST "$WEBHOOK_ENDPOINT" \
    -H "Content-Type: application/json" \
    -H "X-Keycloak-Signature: sha256=$OLD_SIGNATURE" \
    -d "$OLD_EVENT")
  status=$(resp_status "$resp")
  body=$(resp_body "$resp")
  assert_match "$status" "^(400|401|403)$" "old timestamp event rejected"
'

run_all
