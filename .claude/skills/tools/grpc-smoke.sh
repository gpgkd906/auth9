#!/usr/bin/env bash
# gRPC smoke checks for auth9-grpc-tls.
#
# What this validates:
# - Reflection is disabled (list should fail).
# - API key is required (call should fail with "Missing API key" without header).
# - With API key, request gets past API key check (no longer "Missing API key").
# - Plaintext against mTLS endpoint should fail.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
GRPCURL="$SCRIPT_DIR/grpcurl-docker.sh"

GRPC_TARGET="${GRPC_TARGET:-auth9-grpc-tls:50051}"
GRPC_API_KEY="${GRPC_API_KEY:-dev-grpc-api-key}"
GRPC_PROTO="${GRPC_PROTO:-auth9.proto}"
GRPC_CA_CERT="${GRPC_CA_CERT:-/certs/ca.crt}"
GRPC_CLIENT_CERT="${GRPC_CLIENT_CERT:-/certs/client.crt}"
GRPC_CLIENT_KEY="${GRPC_CLIENT_KEY:-/certs/client.key}"

REQ_BODY='{"identity_token":"dummy","tenant_id":"dummy","service_id":"dummy"}'

fail() {
  echo "FAIL: $*" >&2
  exit 1
}

run_expect_fail_contains() {
  local desc="$1"
  local needle="$2"
  shift 2

  local out
  set +e
  out="$("$@" 2>&1)"
  local code=$?
  set -e

  if [ $code -eq 0 ]; then
    fail "$desc: expected non-zero exit code"
  fi
  if ! echo "$out" | grep -Fq "$needle"; then
    echo "$out" >&2
    fail "$desc: expected output to contain: $needle"
  fi
}

run_expect_fail_not_contains() {
  local desc="$1"
  local needle="$2"
  shift 2

  local out
  set +e
  out="$("$@" 2>&1)"
  local code=$?
  set -e

  if [ $code -eq 0 ]; then
    fail "$desc: expected non-zero exit code"
  fi
  if echo "$out" | grep -Fq "$needle"; then
    echo "$out" >&2
    fail "$desc: expected output NOT to contain: $needle"
  fi
}

run_expect_fail_any() {
  local desc="$1"
  shift 1

  set +e
  "$@" >/dev/null 2>&1
  local code=$?
  set -e

  if [ $code -eq 0 ]; then
    fail "$desc: expected non-zero exit code"
  fi
}

echo "[1/4] Reflection should be disabled"
run_expect_fail_contains \
  "reflection disabled" \
  "server does not support the reflection API" \
  "$GRPCURL" \
    -cacert "$GRPC_CA_CERT" \
    -cert "$GRPC_CLIENT_CERT" \
    -key "$GRPC_CLIENT_KEY" \
    "$GRPC_TARGET" list

echo "[2/4] Missing API key should be rejected"
run_expect_fail_contains \
  "missing api key" \
  "Missing API key" \
  "$GRPCURL" \
    -cacert "$GRPC_CA_CERT" \
    -cert "$GRPC_CLIENT_CERT" \
    -key "$GRPC_CLIENT_KEY" \
    -import-path /proto -proto "$GRPC_PROTO" \
    -d "$REQ_BODY" \
    "$GRPC_TARGET" auth9.TokenExchange/ExchangeToken

echo "[3/4] With API key, should get past API key check"
run_expect_fail_not_contains \
  "api key accepted" \
  "Missing API key" \
  "$GRPCURL" \
    -cacert "$GRPC_CA_CERT" \
    -cert "$GRPC_CLIENT_CERT" \
    -key "$GRPC_CLIENT_KEY" \
    -H "x-api-key: $GRPC_API_KEY" \
    -import-path /proto -proto "$GRPC_PROTO" \
    -d "$REQ_BODY" \
    "$GRPC_TARGET" auth9.TokenExchange/ExchangeToken

echo "[4/4] Plaintext to TLS endpoint should fail"
run_expect_fail_any \
  "plaintext should fail" \
  "$GRPCURL" -plaintext \
    -import-path /proto -proto "$GRPC_PROTO" \
    -d "$REQ_BODY" \
    "$GRPC_TARGET" auth9.TokenExchange/ExchangeToken

echo "OK: gRPC smoke checks passed"
