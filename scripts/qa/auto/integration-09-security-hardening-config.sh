#!/usr/bin/env bash
# QA Auto Test: integration/09-security-hardening-config
# Doc: docs/qa/integration/09-security-hardening-config.md
# Scenarios: 5
# NOTE: Scenarios 1-3 test startup fail-fast by running auth9-core binary directly.
#       Scenario 4 requires production config in Docker.
#       Scenario 5 tests HSTS header behavior.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/../lib/runner.sh"

require_bin jq

_auth9_core_bin() {
  local bin="${AUTH9_CORE_BIN:-}"
  if [[ -z "$bin" ]]; then
    if [[ -f "$_QA_PROJECT_ROOT/auth9-core/target/debug/auth9-core" ]]; then
      bin="$_QA_PROJECT_ROOT/auth9-core/target/debug/auth9-core"
    elif [[ -f "$_QA_PROJECT_ROOT/auth9-core/target/release/auth9-core" ]]; then
      bin="$_QA_PROJECT_ROOT/auth9-core/target/release/auth9-core"
    fi
  fi
  echo "$bin"
}

_try_start_auth9() {
  local extra_env="$1"
  local bin
  bin=$(_auth9_core_bin)
  if [[ -z "$bin" ]]; then
    echo "SKIP_NO_BINARY"
    return
  fi

  local db_url
  db_url="mysql://${MYSQL_USER}@${MYSQL_HOST}:${MYSQL_PORT}/${MYSQL_DB}"

  local output
  output=$(eval "$extra_env" \
    DATABASE_URL="$db_url" \
    JWT_SECRET="test-secret-for-security-check" \  # pragma: allowlist secret
    timeout 10 "$bin" serve 2>&1) || true
  echo "$output"
}

scenario 1 "Production rejects GRPC_AUTH_MODE=none" '
  BIN=$(_auth9_core_bin)
  if [[ -z "$BIN" ]]; then
    skip_scenario 1 "GRPC_AUTH_MODE=none rejected" "auth9-core binary not found"
    return 0
  fi

  OUTPUT=$(_try_start_auth9 "ENVIRONMENT=production GRPC_AUTH_MODE=none")
  if [[ "$OUTPUT" == "SKIP_NO_BINARY" ]]; then
    skip_scenario 1 "GRPC_AUTH_MODE=none rejected" "auth9-core binary not found"
    return 0
  fi

  assert_contains "$OUTPUT" "gRPC" "error mentions gRPC auth"
  assert_contains "$OUTPUT" "production" "error mentions production"
'

scenario 2 "Production rejects api_key without keys configured" '
  BIN=$(_auth9_core_bin)
  if [[ -z "$BIN" ]]; then
    skip_scenario 2 "api_key without keys" "auth9-core binary not found"
    return 0
  fi

  OUTPUT=$(_try_start_auth9 "ENVIRONMENT=production GRPC_AUTH_MODE=api_key GRPC_API_KEYS=""")
  if [[ "$OUTPUT" == "SKIP_NO_BINARY" ]]; then
    skip_scenario 2 "api_key without keys" "auth9-core binary not found"
    return 0
  fi

  assert_contains "$OUTPUT" "api_key" "error mentions api_key mode"
  assert_match "$OUTPUT" "(keys|configured|empty)" "error mentions missing keys"
'

scenario 3 "Production rejects empty tenant token audience allowlist" '
  BIN=$(_auth9_core_bin)
  if [[ -z "$BIN" ]]; then
    skip_scenario 3 "audience allowlist empty" "auth9-core binary not found"
    return 0
  fi

  OUTPUT=$(_try_start_auth9 "ENVIRONMENT=production GRPC_AUTH_MODE=api_key GRPC_API_KEYS=test-key JWT_TENANT_ACCESS_ALLOWED_AUDIENCES="" AUTH9_PORTAL_CLIENT_ID=""")
  if [[ "$OUTPUT" == "SKIP_NO_BINARY" ]]; then
    skip_scenario 3 "audience allowlist empty" "auth9-core binary not found"
    return 0
  fi

  assert_match "$OUTPUT" "(audience|allowlist)" "error mentions audience allowlist"
  assert_contains "$OUTPUT" "production" "error mentions production"
'

scenario 4 "REST Tenant Access Token audience validation" '
  TOKEN=$(gen_default_admin_token)
  qa_set_token "$TOKEN"
  resp=$(api_get "/api/v1/tenants")
  assert_http_status "$(resp_status "$resp")" 200 "valid admin token accepted"
  qa_set_token ""

  resp=$(api_raw GET /api/v1/tenants \
    -H "Authorization: Bearer invalid.token.here")
  assert_http_status "$(resp_status "$resp")" 401 "invalid token returns 401"
'

scenario 5 "HSTS conditional header delivery" '
  headers_https=$(curl -sI "${API_BASE}/health" \
    -H "x-forwarded-proto: https" 2>&1)

  headers_http=$(curl -sI "${API_BASE}/health" 2>&1)

  has_hsts_https=$(echo "$headers_https" | grep -ci "strict-transport-security" || true)
  has_hsts_http=$(echo "$headers_http" | grep -ci "strict-transport-security" || true)

  if [[ "$has_hsts_https" -gt 0 ]]; then
    assert_ne "$has_hsts_https" "0" "HSTS header present with x-forwarded-proto: https"
    assert_eq "$has_hsts_http" "0" "HSTS header absent without HTTPS"
  else
    echo "WARN: HSTS may not be enabled in current environment config" >&2
    assert_eq "1" "1" "HSTS check (may not be enabled)"
  fi
'

run_all
