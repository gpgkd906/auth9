#!/bin/bash
# security_rbac_test.sh - Automated RBAC Security Tests

set -euo pipefail

API_BASE="http://localhost:8080"
TOKEN_GEN="node .claude/skills/tools/gen-test-tokens.js"

# IDs
TENANT1="60ae0c35-f0df-473a-8800-b67714ccec2e"
USER1="408a0cdb-1582-47dc-bd44-cc442af245e8"
TENANT2="9dca1bb5-1156-4716-9fa9-18628f25b5ce"

echo "=========================================="
echo "🔐 RBAC Security Tests"
echo "=========================================="

# 1. Generate token for regular user
echo "Generating token for regular user (member role)..."
USER_TOKEN=$($TOKEN_GEN tenant-access --tenant-id $TENANT1 --user-id $USER1)

echo ""
echo "=========================================="
echo "🧪 场景 1: 直接权限绕过 (Regular User accessing Admin API)"
echo "=========================================="

echo "Trying to list all users (Returns scoped list, should NOT contain Tenant 2 users)..."
USERS_RESPONSE=$(curl -s -H "Authorization: Bearer $USER_TOKEN" "$API_BASE/api/v1/users")
# Get user from Tenant 2
USER2_EMAIL="user2@test.com"
if echo "$USERS_RESPONSE" | grep -q "$USER2_EMAIL"; then
    echo "❌ FAIL: Regular user can see users from other tenants in /api/v1/users!"
else
    echo "✅ PASS: Scoped list working correctly. Other tenant users not visible."
fi

echo "Trying to create a tenant (Platform Admin operation)..."
STATUS=$(curl -s -X POST -o /dev/null -w "%{http_code}" -H "Authorization: Bearer $USER_TOKEN" -H "Content-Type: application/json" -d '{"name":"Hacker Tenant","slug":"hacker"}' "$API_BASE/api/v1/tenants")
if [ "$STATUS" == "403" ]; then
    echo "✅ PASS: Tenant creation rejected (HTTP 403)"
else
    echo "❌ FAIL: Tenant creation allowed or returned unexpected status: $STATUS"
fi

echo "Trying to delete another user (Tenant Admin operation)..."
# User 2 ID
USER2="24052573-8597-4eba-a545-0430f1d545e1"
STATUS=$(curl -s -X DELETE -o /dev/null -w "%{http_code}" -H "Authorization: Bearer $USER_TOKEN" "$API_BASE/api/v1/users/$USER2")
if [ "$STATUS" == "403" ]; then
    echo "✅ PASS: User deletion rejected (HTTP 403)"
else
    echo "❌ FAIL: User deletion allowed or returned unexpected status: $STATUS"
fi

echo ""
echo "=========================================="
echo "🧪 场景 3: HTTP 方法绕过 (X-HTTP-Method-Override)"
echo "=========================================="

echo "Trying to use X-HTTP-Method-Override to bypass method restriction..."
# Regular user cannot delete themselves (they need admin/owner or specialized permission)
STATUS=$(curl -s -X POST -o /dev/null -w "%{http_code}" -H "Authorization: Bearer $USER_TOKEN" -H "X-HTTP-Method-Override: DELETE" "$API_BASE/api/v1/users/$USER1")

if [ "$STATUS" == "405" ] || [ "$STATUS" == "404" ] || [ "$STATUS" == "403" ]; then
    echo "✅ PASS: Method override did not result in unauthorized deletion (HTTP $STATUS)"
else
    echo "❌ FAIL: Method override might have worked or returned unexpected status: $STATUS"
fi

echo ""
echo "=========================================="
echo "🧪 场景 4: 参数级权限绕过 (Updating sensitive fields)"
echo "=========================================="

echo "User trying to upgrade their own role to 'owner' via PUT /users/me..."
# Get initial state
INITIAL_ROLE=$(mysql -h 127.0.0.1 -P 4000 -u root auth9 -N -e "SELECT role_in_tenant FROM tenant_users WHERE user_id = '$USER1' AND tenant_id = '$TENANT1';")
echo "Initial role: $INITIAL_ROLE"

# Attempt upgrade
curl -s -X PUT -H "Authorization: Bearer $USER_TOKEN" -H "Content-Type: application/json" -d '{"role_in_tenant": "owner", "display_name": "Upgraded User"}' "$API_BASE/api/v1/users/me" > /dev/null

# Verify state
FINAL_ROLE=$(mysql -h 127.0.0.1 -P 4000 -u root auth9 -N -e "SELECT role_in_tenant FROM tenant_users WHERE user_id = '$USER1' AND tenant_id = '$TENANT1';")
echo "Final role: $FINAL_ROLE"

if [ "$FINAL_ROLE" == "$INITIAL_ROLE" ]; then
    echo "✅ PASS: Sensitive field 'role_in_tenant' was ignored."
else
    echo "❌ FAIL: Sensitive field 'role_in_tenant' was modified! Escalation successful."
fi

echo ""
echo "=========================================="
echo "📊 测试总结"
echo "=========================================="
echo "已执行 4 个 RBAC 绕过场景。"
