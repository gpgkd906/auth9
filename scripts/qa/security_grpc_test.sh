#!/bin/bash
# security_grpc_test.sh - Automated gRPC Security Tests (using Docker helper)

set -euo pipefail

GRPC_HELPER=".claude/skills/tools/grpcurl-docker.sh"
GRPC_TARGET="auth9-grpc-tls:50051"
PROTO="auth9.proto"
API_KEY="dev-grpc-api-key"  # pragma: allowlist secret

# Cert args for mTLS
CERT_ARGS="-cacert /certs/ca.crt -cert /certs/client.crt -key /certs/client.key"
PROTO_ARGS="-import-path /proto -proto $PROTO"

echo "=========================================="
echo "🔐 gRPC Security Tests (Dockerized)"
echo "=========================================="

echo ""
echo "=========================================="
echo "🧪 场景 1: 未认证 gRPC 访问"
echo "=========================================="

echo "Trying to list services without API Key (but with mTLS certs)..."
# Even with valid mTLS, if x-api-key is required and missing, it should fail.
# Note: auth9-core might require API Key even if Nginx allowed mTLS.
STATUS_OUT=$($GRPC_HELPER $CERT_ARGS $PROTO_ARGS "$GRPC_TARGET" list 2>&1 || true)
if echo "$STATUS_OUT" | grep -q "Unauthenticated"; then
    echo "✅ PASS: Unauthenticated request rejected (Unauthenticated)."
else
    echo "⚠️  INFO: Service listing allowed with mTLS but without API Key. Checking actual method call..."
fi

echo "Trying to call ExchangeToken without API Key..."
EXCHANGE_OUT=$($GRPC_HELPER $CERT_ARGS $PROTO_ARGS -d '{"identity_token":"invalid"}' "$GRPC_TARGET" auth9.TokenExchange/ExchangeToken 2>&1 || true)
if echo "$EXCHANGE_OUT" | grep -q "Unauthenticated"; then
    echo "✅ PASS: Unauthenticated method call rejected (Unauthenticated)."
else
    echo "❌ FAIL: Unexpected response for unauthenticated method call: $EXCHANGE_OUT"
fi

echo ""
echo "=========================================="
echo "🧪 场景 2: Token Exchange 滥用 (跨租户)"
echo "=========================================="

# IDs
USER1="408a0cdb-1582-47dc-bd44-cc442af245e8"
TENANT2="9dca1bb5-1156-4716-9fa9-18628f25b5ce"

echo "Generating valid Identity Token for User 1..."
ID_TOKEN=$(node .claude/skills/tools/gen-test-tokens.js identity-user --user-id $USER1)

echo "Attempting to exchange for OTHER tenant (Tenant 2)..."
EXCHANGE_OUT=$($GRPC_HELPER $CERT_ARGS $PROTO_ARGS -H "x-api-key: $API_KEY" \
  -d "{\"identity_token\": \"$ID_TOKEN\", \"tenant_id\": \"$TENANT2\", \"service_id\": \"auth9-portal\"}" \
  "$GRPC_TARGET" auth9.TokenExchange/ExchangeToken 2>&1 || true)

if echo "$EXCHANGE_OUT" | grep -q "PermissionDenied"; then
    echo "✅ PASS: Cross-tenant exchange rejected (PermissionDenied)."
elif echo "$EXCHANGE_OUT" | grep -q "User not member of tenant"; then
    echo "✅ PASS: Cross-tenant exchange rejected (User not member of tenant)."
else
    echo "❌ FAIL: Cross-tenant exchange might have worked or returned unexpected error: $EXCHANGE_OUT"
fi

echo ""
echo "=========================================="
echo "🧪 场景 5: gRPC 传输安全 (Plaintext check)"
echo "=========================================="

echo "Trying to connect via plaintext to auth9-core directly (bypassing Nginx)..."
# auth9-core:50051 is the internal port
if $GRPC_HELPER -plaintext $PROTO_ARGS auth9-core:50051 list > /dev/null 2>&1; then
    echo "⚠️  INFO: Internal gRPC (auth9-core) is plaintext. (Acceptable if restricted to Docker network)."
else
    echo "✅ PASS: Internal gRPC also requires TLS."
fi

echo "Trying to connect via plaintext to auth9-grpc-tls (Nginx)..."
if $GRPC_HELPER -plaintext $PROTO_ARGS auth9-grpc-tls:50051 list > /dev/null 2>&1; then
    echo "❌ FAIL: Nginx proxy accepts plaintext connections!"
else
    echo "✅ PASS: Nginx proxy rejected plaintext connection."
fi

echo ""
echo "=========================================="
echo "📊 测试总结"
echo "=========================================="
echo "已执行 3 个 gRPC 安全场景。"
