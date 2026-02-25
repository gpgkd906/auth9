#!/bin/bash
# security_isolation_test.sh - Automated Tenant Isolation Tests

set -euo pipefail

API_BASE="http://localhost:8080"
TOKEN_GEN="node .claude/skills/tools/gen-test-tokens.js"

# IDs
TENANT1="60ae0c35-f0df-473a-8800-b67714ccec2e"
USER1="408a0cdb-1582-47dc-bd44-cc442af245e8"
TENANT2="9dca1bb5-1156-4716-9fa9-18628f25b5ce"
USER2="24052573-8597-4eba-a545-0430f1d545e1"

echo "=========================================="
echo "🔐 Tenant Isolation Security Tests"
echo "=========================================="

# 1. Generate tokens
echo "Generating tokens..."
TOKEN1=$($TOKEN_GEN tenant-access --tenant-id $TENANT1 --user-id $USER1)
TOKEN2=$($TOKEN_GEN tenant-access --tenant-id $TENANT2 --user-id $USER2)

echo ""
echo "=========================================="
echo "🧪 场景 1: 跨租户基础数据访问 (GET /tenants/{other_id})"
echo "=========================================="

echo "User 1 (Tenant 1) trying to access Tenant 2 details..."
STATUS=$(curl -s -o /dev/null -w "%{http_code}" -H "Authorization: Bearer $TOKEN1" "$API_BASE/api/v1/tenants/$TENANT2")
if [ "$STATUS" == "403" ]; then
    echo "✅ PASS: Access to other tenant rejected (HTTP 403)"
else
    echo "❌ FAIL: Access to other tenant allowed or returned unexpected status: $STATUS"
fi

echo ""
echo "=========================================="
echo "🧪 场景 2: 跨租户用户列表访问 (GET /tenants/{other_id}/users)"
echo "=========================================="

echo "User 1 (Tenant 1) trying to list users of Tenant 2..."
STATUS=$(curl -s -o /dev/null -w "%{http_code}" -H "Authorization: Bearer $TOKEN1" "$API_BASE/api/v1/tenants/$TENANT2/users")
if [ "$STATUS" == "403" ]; then
    echo "✅ PASS: Listing users of other tenant rejected (HTTP 403)"
else
    echo "❌ FAIL: Listing users of other tenant allowed or returned unexpected status: $STATUS"
fi

echo ""
echo "=========================================="
echo "🧪 场景 3: IDOR - 跨租户更新用户信息 (PUT /users/{other_user_id})"
echo "=========================================="

echo "User 1 (Tenant 1) trying to update User 2 (Tenant 2) profile..."
STATUS=$(curl -s -X PUT -o /dev/null -w "%{http_code}" -H "Authorization: Bearer $TOKEN1" -H "Content-Type: application/json" -d '{"display_name": "Hacked by User 1"}' "$API_BASE/api/v1/users/$USER2")

if [ "$STATUS" == "403" ]; then
    echo "✅ PASS: Updating other user profile rejected (HTTP 403)"
else
    echo "❌ FAIL: Updating other user profile allowed or returned unexpected status: $STATUS"
fi

echo ""
echo "=========================================="
echo "🧪 场景 4: 跨租户服务列表访问 (GET /api/v1/services)"
echo "=========================================="

echo "User 1 (Tenant 1) listing services..."
PLATFORM_TOKEN=$($TOKEN_GEN tenant-owner --tenant-id a4ccb751-f62c-44c9-8f45-19f5d2fd6491 --user-id 51d09a19-0eb8-4d2c-89fb-43bfa82c5381)
echo "Creating a dummy service in Tenant 2..."
S2_ID=$(curl -s -X POST "$API_BASE/api/v1/tenants/$TENANT2/services" -H "Authorization: Bearer $PLATFORM_TOKEN" -H "Content-Type: application/json" -d '{"name": "Tenant 2 Private Service", "slug": "t2-service"}' | jq -r '.data.id')

echo "User 1 listing services (should exclude Tenant 2 service)..."
SERVICES=$(curl -s -H "Authorization: Bearer $TOKEN1" "$API_BASE/api/v1/services")
if echo "$SERVICES" | grep -q "$S2_ID"; then
    echo "❌ FAIL: User 1 can see Tenant 2's service!"
else
    echo "✅ PASS: User 1 cannot see Tenant 2's service."
fi

echo ""
echo "=========================================="
echo "📊 测试总结"
echo "=========================================="
echo "已执行 4 个隔离场景。"
