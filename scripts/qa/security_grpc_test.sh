#!/usr/bin/env bash
# security_grpc_test.sh - gRPC security regression checks (dynamic fixtures).

set -euo pipefail

GRPC_HELPER="${GRPC_HELPER:-.claude/skills/tools/grpcurl-docker.sh}"
GRPC_TARGET="${GRPC_TARGET:-auth9-grpc-tls:50051}"
PROTO="${PROTO:-auth9.proto}"
API_KEY="${API_KEY:-dev-grpc-api-key}" # pragma: allowlist secret
JWT_PRIVATE_KEY="${JWT_PRIVATE_KEY:-deploy/dev-certs/jwt/private.key}"
PORTAL_CLIENT_ID="${PORTAL_CLIENT_ID:-auth9-portal}"

MYSQL_HOST="${MYSQL_HOST:-127.0.0.1}"
MYSQL_PORT="${MYSQL_PORT:-4000}"
MYSQL_USER="${MYSQL_USER:-root}"
MYSQL_DB="${MYSQL_DB:-auth9}"
TEST_TENANT_SLUG="${TEST_TENANT_SLUG:-demo}"
OTHER_TENANT_SLUG="${OTHER_TENANT_SLUG:-auth9-platform}"

GRPC_USER_ID="55555555-5555-5555-5555-555555555555"
GRPC_USER_TU_ID="66666666-6666-6666-6666-666666666666"

PASS_COUNT=0
FAIL_COUNT=0
SKIP_COUNT=0

CERT_ARGS=(-cacert /certs/ca.crt -cert /certs/client.crt -key /certs/client.key)
PROTO_ARGS=(-import-path /proto -proto "$PROTO")

mysql_q() {
  mysql -h "$MYSQL_HOST" -P "$MYSQL_PORT" -u "$MYSQL_USER" "$MYSQL_DB" -N -e "$1"
}

mark_pass() { PASS_COUNT=$((PASS_COUNT + 1)); echo "✅ PASS: $1"; }
mark_fail() { FAIL_COUNT=$((FAIL_COUNT + 1)); echo "❌ FAIL: $1"; }
mark_skip() { SKIP_COUNT=$((SKIP_COUNT + 1)); echo "⏭️  SKIP: $1"; }

require_bin() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "Missing dependency: $1"
    exit 2
  fi
}

gen_identity_token() {
  local user_id="$1"
  local email="$2"
  node -e '
const jwt=require("jsonwebtoken");
const fs=require("fs");
const now=Math.floor(Date.now()/1000);
const privateKey=fs.readFileSync(process.argv[1], "utf8");
const payload={
  sub: process.argv[2],
  email: process.argv[3],
  iss: "http://localhost:8080",
  aud: "auth9",
  token_type: "identity",
  iat: now,
  exp: now + 3600,
  sid: "sid-" + process.argv[2].slice(0, 8)
};
process.stdout.write(jwt.sign(payload, privateKey, {algorithm:"RS256", keyid:"auth9-current"}));
' "$JWT_PRIVATE_KEY" "$user_id" "$email"
}

echo "=========================================="
echo "🔐 gRPC Security Tests (Dockerized, Dynamic)"
echo "=========================================="

require_bin mysql
require_bin node
require_bin grep

if [[ ! -x "$GRPC_HELPER" ]]; then
  echo "Missing helper: $GRPC_HELPER"
  exit 2
fi
if [[ ! -f "$JWT_PRIVATE_KEY" ]]; then
  echo "Missing key: $JWT_PRIVATE_KEY"
  exit 2
fi

DEMO_TENANT_ID="$(mysql_q "SELECT id FROM tenants WHERE slug='$TEST_TENANT_SLUG' LIMIT 1;")"
OTHER_TENANT_ID="$(mysql_q "SELECT id FROM tenants WHERE slug='$OTHER_TENANT_SLUG' LIMIT 1;")"

if [[ -z "$DEMO_TENANT_ID" || -z "$OTHER_TENANT_ID" ]]; then
  echo "Missing tenant data in DB; run reset/init first."
  exit 2
fi
if [[ "$DEMO_TENANT_ID" == "$OTHER_TENANT_ID" ]]; then
  echo "Test tenant and other tenant must be different."
  exit 2
fi

# Prepare deterministic gRPC test user: member in demo tenant only.
mysql -h "$MYSQL_HOST" -P "$MYSQL_PORT" -u "$MYSQL_USER" "$MYSQL_DB" <<SQL
DELETE FROM tenant_users WHERE user_id = '$GRPC_USER_ID';
DELETE FROM users WHERE id = '$GRPC_USER_ID';
INSERT INTO users (id,identity_subject,email,display_name,mfa_enabled)
VALUES ('$GRPC_USER_ID','grpc-member-555','grpc.member@test.local','gRPC Member',0);
INSERT INTO tenant_users (id,tenant_id,user_id,role_in_tenant)
VALUES ('$GRPC_USER_TU_ID','$DEMO_TENANT_ID','$GRPC_USER_ID','member');
SQL

ID_TOKEN="$(gen_identity_token "$GRPC_USER_ID" "grpc.member@test.local")"

echo
echo "=========================================="
echo "🧪 场景 1: 未认证 gRPC 访问"
echo "=========================================="
NO_APIKEY_OUT="$("$GRPC_HELPER" "${CERT_ARGS[@]}" "${PROTO_ARGS[@]}" \
  -d '{"identity_token":"dummy","tenant_id":"dummy","service_id":"dummy"}' \
  "$GRPC_TARGET" auth9.TokenExchange/ExchangeToken 2>&1 || true)"
echo "$NO_APIKEY_OUT"
if echo "$NO_APIKEY_OUT" | grep -Eiq 'Unauthenticated|missing api key|invalid api key'; then
  mark_pass "ExchangeToken without API key was rejected"
else
  mark_fail "Expected unauthenticated/missing-api-key rejection"
fi

echo
echo "=========================================="
echo "🧪 场景 2: Token Exchange 跨租户滥用"
echo "=========================================="
CROSS_TENANT_OUT="$("$GRPC_HELPER" "${CERT_ARGS[@]}" "${PROTO_ARGS[@]}" \
  -H "x-api-key: $API_KEY" \
  -d "{\"identity_token\":\"$ID_TOKEN\",\"tenant_id\":\"$OTHER_TENANT_ID\",\"service_id\":\"$PORTAL_CLIENT_ID\"}" \
  "$GRPC_TARGET" auth9.TokenExchange/ExchangeToken 2>&1 || true)"
echo "$CROSS_TENANT_OUT"
if echo "$CROSS_TENANT_OUT" | grep -Eiq 'PermissionDenied|not a member of this tenant'; then
  mark_pass "Cross-tenant token exchange was blocked"
else
  mark_fail "Expected PermissionDenied for cross-tenant exchange"
fi

echo
echo "=========================================="
echo "🧪 场景 3: gRPC TLS 传输安全"
echo "=========================================="
PLAINTEXT_OUT="$("$GRPC_HELPER" -plaintext "${PROTO_ARGS[@]}" \
  -d '{"identity_token":"dummy","tenant_id":"dummy","service_id":"dummy"}' \
  "$GRPC_TARGET" auth9.TokenExchange/ExchangeToken 2>&1 || true)"
echo "$PLAINTEXT_OUT"
if echo "$PLAINTEXT_OUT" | grep -Eiq 'tls|handshake|transport|connection|first record does not look like a TLS handshake|DeadlineExceeded|Unavailable'; then
  mark_pass "Plaintext connection to TLS endpoint failed as expected"
elif echo "$PLAINTEXT_OUT" | grep -Eiq 'Unauthenticated|PermissionDenied|Invalid identity token'; then
  mark_fail "Plaintext reached application layer on TLS endpoint"
else
  mark_skip "Could not classify plaintext result; inspect output above"
fi

echo
echo "=========================================="
echo "📊 测试总结"
echo "=========================================="
echo "PASS: $PASS_COUNT"
echo "FAIL: $FAIL_COUNT"
echo "SKIP: $SKIP_COUNT"

if [[ "$FAIL_COUNT" -gt 0 ]]; then
  exit 1
fi
