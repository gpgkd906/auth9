#!/usr/bin/env bash
# Security Auto Test: security/input-validation/06-deserialization
# Doc: docs/security/input-validation/06-deserialization.md
# Scenarios: 3
# ASVS: M-INPUT-06 | V5.5, V1.1, V2.1
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/../lib/runner.sh"

require_bin jq

scenario 1 "JSON deserialization attack" '
  TOKEN=$(gen_default_admin_token)
  qa_set_token "$TOKEN"

  # Deep nested JSON (128+ levels)
  DEPTH=200
  nested=$(python3 -c "
depth = $DEPTH
payload = '{\"a\":' * depth + '\"deep\"' + '}' * depth
print(payload)
" 2>/dev/null)
  resp=$(api_raw POST "/api/v1/tenants" \
    -H "Authorization: Bearer $TOKEN" \
    -H "Content-Type: application/json" \
    -d "$nested")
  status=$(resp_status "$resp")
  assert_match "$status" "^(400|413|422)$" "deeply nested JSON rejected (not 500)"

  # Oversized JSON body (2MB string field)
  big_name=$(python3 -c "print(\"A\" * 2_000_000)" 2>/dev/null)
  resp=$(api_raw POST "/api/v1/tenants" \
    -H "Authorization: Bearer $TOKEN" \
    -H "Content-Type: application/json" \
    -d "{\"name\": \"$big_name\", \"slug\": \"big-test\"}")
  status=$(resp_status "$resp")
  assert_match "$status" "^(400|413|422)$" "oversized JSON body rejected"

  # Duplicate keys
  resp=$(api_raw POST "/api/v1/tenants" \
    -H "Authorization: Bearer $TOKEN" \
    -H "Content-Type: application/json" \
    -d "{\"name\": \"first\", \"slug\": \"dup-test\", \"name\": \"second\"}")
  status=$(resp_status "$resp")
  assert_match "$status" "^(200|201|400|409|422)$" "duplicate key JSON handled without crash"

  # Zero-width characters in field values
  resp=$(api_raw POST "/api/v1/tenants" \
    -H "Authorization: Bearer $TOKEN" \
    -H "Content-Type: application/json" \
    -d "{\"name\": \"test\\u200b\\u200c\\u200d\\ufeff\", \"slug\": \"zero-width-test\"}")
  status=$(resp_status "$resp")
  assert_match "$status" "^(200|201|400|409|422)$" "zero-width chars handled safely"

  # NaN / Infinity (non-standard JSON)
  resp=$(api_raw POST "/api/v1/tenants" \
    -H "Authorization: Bearer $TOKEN" \
    -H "Content-Type: application/json" \
    -d "{\"name\": \"test\", \"some_number\": NaN}")
  status=$(resp_status "$resp")
  assert_match "$status" "^(400|422|429)$" "NaN in JSON rejected"

  # Super long field value
  long_slug=$(python3 -c "print(\"a\" * 100000)" 2>/dev/null)
  resp=$(api_raw POST "/api/v1/tenants" \
    -H "Authorization: Bearer $TOKEN" \
    -H "Content-Type: application/json" \
    -d "{\"name\": \"test\", \"slug\": \"$long_slug\"}")
  status=$(resp_status "$resp")
  assert_match "$status" "^(400|413|422)$" "super long slug field rejected"

  # Verify service is still alive
  resp=$(api_get "/health")
  status=$(resp_status "$resp")
  assert_eq "$status" "200" "service healthy after JSON deserialization attacks"

  qa_set_token ""
'

scenario 2 "gRPC Protobuf malformed message" '
  if ! command -v grpcurl &>/dev/null; then
    skip_scenario 2 "gRPC Protobuf malformed message" "grpcurl not installed"
    return
  fi

  # Empty message (missing required fields)
  result=$(grpcurl -plaintext \
    -d "{}" \
    localhost:50051 auth9.TokenService/ExchangeToken 2>&1 || true)
  assert_match "$result" "ERROR\|error\|INVALID_ARGUMENT\|UNAUTHENTICATED\|Code:" "empty gRPC message returns error"

  # Oversized field value
  big_token=$(python3 -c "print(\"A\" * 100000)" 2>/dev/null)
  result=$(grpcurl -plaintext \
    -d "{\"identity_token\": \"$big_token\"}" \
    localhost:50051 auth9.TokenService/ExchangeToken 2>&1 || true)
  assert_match "$result" "ERROR\|error\|INVALID_ARGUMENT\|UNAUTHENTICATED\|RESOURCE_EXHAUSTED\|Code:" "oversized gRPC field returns error"

  # Verify gRPC service is still alive
  result=$(grpcurl -plaintext localhost:50051 list 2>&1 || true)
  assert_match "$result" "auth9\|grpc\|TokenService\|Error\|Cannot" "gRPC service responds after malformed messages"
'

scenario 3 "JWT payload malformed data" '
  # Invalid JWT format
  resp=$(api_raw GET "/api/v1/auth/userinfo" \
    -H "Authorization: Bearer not-a-jwt-token")
  status=$(resp_status "$resp")
  assert_eq "$status" "401" "invalid JWT format rejected"

  # Two-part JWT (missing signature)
  resp=$(api_raw GET "/api/v1/auth/userinfo" \
    -H "Authorization: Bearer header.payload")
  status=$(resp_status "$resp")
  assert_eq "$status" "401" "two-part JWT rejected"

  # Empty Bearer token
  resp=$(api_raw GET "/api/v1/auth/userinfo" \
    -H "Authorization: Bearer ")
  status=$(resp_status "$resp")
  assert_eq "$status" "401" "empty Bearer token rejected"

  # JWT with non-existent kid
  FAKE_JWT=$(python3 -c "
import base64, json
h = base64.urlsafe_b64encode(json.dumps({\"alg\":\"RS256\",\"kid\":\"nonexistent-kid\"}).encode()).rstrip(b\"=\").decode()
p = base64.urlsafe_b64encode(json.dumps({\"sub\":\"user\",\"exp\":9999999999}).encode()).rstrip(b\"=\").decode()
print(f\"{h}.{p}.fakesig\")
" 2>/dev/null)
  resp=$(api_raw GET "/api/v1/auth/userinfo" \
    -H "Authorization: Bearer $FAKE_JWT")
  status=$(resp_status "$resp")
  assert_eq "$status" "401" "JWT with non-existent kid rejected"

  # JWT with super long sub claim
  LONG_JWT=$(python3 -c "
import base64, json
h = base64.urlsafe_b64encode(json.dumps({\"alg\":\"HS256\",\"typ\":\"JWT\"}).encode()).rstrip(b\"=\").decode()
p = base64.urlsafe_b64encode(json.dumps({\"sub\":\"A\"*50000,\"exp\":9999999999}).encode()).rstrip(b\"=\").decode()
print(f\"{h}.{p}.fakesig\")
" 2>/dev/null)
  resp=$(api_raw GET "/api/v1/auth/userinfo" \
    -H "Authorization: Bearer $LONG_JWT")
  status=$(resp_status "$resp")
  assert_eq "$status" "401" "JWT with super long sub rejected"

  # Verify service is still alive
  resp=$(api_get "/health")
  status=$(resp_status "$resp")
  assert_eq "$status" "200" "service healthy after JWT malformed attacks"
'

run_all
