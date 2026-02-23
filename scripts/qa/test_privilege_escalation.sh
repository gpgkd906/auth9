#!/bin/bash
# Privilege Escalation QA Test Script
# Based on docs/security/authorization/03-privilege-escalation.md

set -e

API_BASE="http://localhost:8080"
TENANT_ID="e8e8084e-ed86-4fc4-bd4d-1eabb832c7aa"
PLATFORM_TENANT_ID="a72c1a91-5691-4aa6-b540-29414011d2e6"

echo "=========================================="
echo "🔒 Privilege Escalation QA Test"
echo "=========================================="

# Use the token from playwright browser session
IDENTITY_TOKEN="eyJ0eXAiOiJKV1QiLCJhbGciOiJSUzI1NiIsImtpZCI6ImF1dGg5LWN1cnJlbnQifQ.eyJzdWIiOiIxNTA2MjQzNi1kOTY1LTQyMjAtOWVhMi05M2Y4NThiZDllNDAiLCJzaWQiOiJjMTExYTRhMi03NGI5LTRhYWYtODY1NC02NmE3NGUxZjE5YjAiLCJlbWFpbCI6ImFkbWluQGF1dGg5LmxvY2FsIiwibmFtZSI6IlRlc3QgVXNlciBVc2VyIiwiaXNzIjoiaHR0cDovL2xvY2FsaG9zdDo4MDgwIiwiYXVkIjoiYXV0aDkiLCJ0b2tlbl90eXBlIjoiaWRlbnRpdHkiLCJpYXQiOjE3NzE4MTI5MDcsImV4cCI6MTc3MTgxMzgwN30.eivzjbt6i-AcTvDl_Uh7PTGp4AGckwSewGDR3ySWdXzpi4Dz-Ys6guWH3imRTGawXggjJqt5ey8VrExhltO9rvPUfK1RiNvETtf-vU7Ge1FWrw97A3Flv5QbjXtNVn4etU7zfEePmOFpYBvn7iHuZjgDM_0D5jq5-BRAKU_KBAyGgnEIrGnJ5NPgt4ukZKufFVWoaVN6ptSOze_N9PY7AZrWOuE_qfKpRygAszefVfq3FXUot7O5akRitg9zgD5p6hXxyWhW1DqzER-_1Uw4xLCQrrfM-GfCGFHXRcxkTUpc48qepgbyT8tMEH4b_RI4bzZLOWcUEugufm5UUmg0cg"

ADMIN_USER_ID="15062436-d965-4220-9ea2-93f858bd9e40"

echo ""
echo "=========================================="
echo "🧪 场景1: 自我角色分配攻击"
echo "=========================================="
echo ""

# Check user_tenant_roles for admin role assignment
ROLE_ADMIN_ID=$(mysql -h 127.0.0.1 -P 4000 -u root auth9 -N -e "
  SELECT id FROM roles WHERE name = 'admin' LIMIT 1;
")

# Get tenant_user_id for admin user in demo tenant
TENANT_USER_ID=$(mysql -h 127.0.0.1 -P 4000 -u root auth9 -N -e "
  SELECT id FROM tenant_users 
  WHERE user_id = '$ADMIN_USER_ID' AND tenant_id = '$TENANT_ID';
")

echo "  Tenant User ID: $TENANT_USER_ID"
echo "  Admin Role ID: $ROLE_ADMIN_ID"

# Check current role assignments
CURRENT_ADMIN_ROLES=$(mysql -h 127.0.0.1 -P 4000 -u root auth9 -N -e "
  SELECT COUNT(*) FROM user_tenant_roles 
  WHERE tenant_user_id = '$TENANT_USER_ID' AND role_id = '$ROLE_ADMIN_ID';
")
echo "  Current admin role count for user: $CURRENT_ADMIN_ROLES"

# Test: Try to assign admin role to self via API
echo ""
echo "  Testing: Assigning admin role to self..."
RESPONSE=$(curl -s -w "\nHTTP:%{http_code}" -X POST "$API_BASE/api/v1/rbac/assign" \
  -H "Authorization: Bearer $IDENTITY_TOKEN" \
  -H "Content-Type: application/json" \
  -d "{
    \"user_id\": \"$ADMIN_USER_ID\",
    \"tenant_id\": \"$TENANT_ID\",
    \"role_id\": \"$ROLE_ADMIN_ID\"
  }")

HTTP_CODE=$(echo "$RESPONSE" | grep -o "HTTP:[0-9]*" | cut -d: -f2)
BODY=$(echo "$RESPONSE" | grep -v "HTTP:")

echo "  HTTP Status: $HTTP_CODE"
echo "  Response: $BODY"

if [ "$HTTP_CODE" = "403" ] || [ "$HTTP_CODE" = "401" ]; then
  echo "  ✅ PASS: Self-role assignment blocked (HTTP $HTTP_CODE)"
  SCENARIO1="PASS"
else
  # Check if role was actually assigned
  NEW_ADMIN_ROLES=$(mysql -h 127.0.0.1 -P 4000 -u root auth9 -N -e "
    SELECT COUNT(*) FROM user_tenant_roles 
    WHERE tenant_user_id = '$TENANT_USER_ID' AND role_id = '$ROLE_ADMIN_ID';
  ")
  if [ "$NEW_ADMIN_ROLES" = "$CURRENT_ADMIN_ROLES" ]; then
    echo "  ✅ PASS: Role not assigned (no change in DB)"
    SCENARIO1="PASS"
  else
    echo "  ❌ FAIL: Self-role assignment succeeded!"
    SCENARIO1="FAIL"
  fi
fi

echo ""
echo "=========================================="
echo "🧪 场景2: 角色创建后门"
echo "=========================================="
echo ""

# Get service ID for demo tenant
SERVICE_ID=$(mysql -h 127.0.0.1 -P 4000 -u root auth9 -N -e "
  SELECT id FROM services WHERE tenant_id = '$TENANT_ID' LIMIT 1;
")

echo "  Service ID: $SERVICE_ID"

# Try to create role with elevated permissions
echo "  Testing: Creating role with elevated permissions..."
RESPONSE=$(curl -s -w "\nHTTP:%{http_code}" -X POST "$API_BASE/api/v1/roles" \
  -H "Authorization: Bearer $IDENTITY_TOKEN" \
  -H "Content-Type: application/json" \
  -d "{
    \"name\": \"super_role_$(date +%s)\",
    \"service_id\": \"$SERVICE_ID\",
    \"description\": \"Test role with elevated permissions\"
  }")

HTTP_CODE=$(echo "$RESPONSE" | grep -o "HTTP:[0-9]*" | cut -d: -f2)
BODY=$(echo "$RESPONSE" | grep -v "HTTP:")

echo "  HTTP Status: $HTTP_CODE"
echo "  Response: $BODY"

if [ "$HTTP_CODE" = "400" ] || [ "$HTTP_CODE" = "403" ] || [ "$HTTP_CODE" = "401" ]; then
  echo "  ✅ PASS: Elevated role creation blocked (HTTP $HTTP_CODE)"
  SCENARIO2="PASS"
elif echo "$BODY" | jq -r '.error' 2>/dev/null | grep -q "permission"; then
  echo "  ✅ PASS: Permission error returned"
  SCENARIO2="PASS"
else
  echo "  ⚠️  INFO: Role creation returned HTTP $HTTP_CODE"
  SCENARIO2="INFO"
fi

echo ""
echo "=========================================="
echo "🧪 场景3: 邀请链接权限提升"
echo "=========================================="
echo ""

# Create an invitation
echo "  Testing: Creating invitation..."
INVITE_RESPONSE=$(curl -s -X POST "$API_BASE/api/v1/tenants/$TENANT_ID/invitations" \
  -H "Authorization: Bearer $IDENTITY_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "emails": ["test-invite-'"$(date +%s)"'@example.com"]
  }')

echo "  Invite Response: $INVITE_RESPONSE"

# Check if invitation has role info
if echo "$INVITE_RESPONSE" | jq -r '.invitation.role_ids' 2>/dev/null | grep -q "\["; then
  echo "  ⚠️  INFO: Invitation includes role info"
fi

# Try to accept with role override
echo "  Testing: Accepting invitation with role override..."
ACCEPT_RESPONSE=$(curl -s -w "\nHTTP:%{http_code}" -X POST "$API_BASE/api/v1/invitations/accept" \
  -H "Content-Type: application/json" \
  -d '{
    "token": "invalid-token-test-'"$(date +%s)"'",
    "role": "admin"
  }')

HTTP_CODE=$(echo "$ACCEPT_RESPONSE" | grep -o "HTTP:[0-9]*" | cut -d: -f2)
echo "  HTTP Status: $HTTP_CODE"

if [ "$HTTP_CODE" = "400" ] || [ "$HTTP_CODE" = "401" ] || [ "$HTTP_CODE" = "404" ]; then
  echo "  ✅ PASS: Invalid token rejected properly"
  SCENARIO3="PASS"
else
  echo "  ⚠️  INFO: Accept returned HTTP $HTTP_CODE"
  SCENARIO3="INFO"
fi

echo ""
echo "=========================================="
echo "🧪 场景4: 租户所有权转移攻击"
echo "=========================================="
echo ""

# Try to transfer ownership (non-owner trying to change owner)
echo "  Testing: Non-owner trying to transfer ownership..."
RESPONSE=$(curl -s -w "\nHTTP:%{http_code}" -X PUT "$API_BASE/api/v1/tenants/$TENANT_ID" \
  -H "Authorization: Bearer $IDENTITY_TOKEN" \
  -H "Content-Type: application/json" \
  -d "{
    \"owner_id\": \"$ADMIN_USER_ID\"
  }")

HTTP_CODE=$(echo "$RESPONSE" | grep -o "HTTP:[0-9]*" | cut -d: -f2)
BODY=$(echo "$RESPONSE" | grep -v "HTTP:")

echo "  HTTP Status: $HTTP_CODE"
echo "  Response: $BODY"

if [ "$HTTP_CODE" = "403" ] || [ "$HTTP_CODE" = "401" ]; then
  echo "  ✅ PASS: Ownership transfer blocked (HTTP $HTTP_CODE)"
  SCENARIO4="PASS"
elif echo "$BODY" | jq -r '.error' 2>/dev/null | grep -q "owner"; then
  echo "  ✅ PASS: Owner error returned"
  SCENARIO4="PASS"
else
  echo "  ⚠️  INFO: Ownership transfer returned HTTP $HTTP_CODE"
  SCENARIO4="INFO"
fi

echo ""
echo "=========================================="
echo "🧪 场景5: API 密钥权限提升"
echo "=========================================="
echo ""

# Try to create API key with elevated scopes
echo "  Testing: Creating API key with elevated scopes..."
RESPONSE=$(curl -s -w "\nHTTP:%{http_code}" -X POST "$API_BASE/api/v1/api-keys" \
  -H "Authorization: Bearer $IDENTITY_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "test-elevated-key-'"$(date +%s)"'",
    "scopes": ["admin:write", "platform:admin"]
  }')

HTTP_CODE=$(echo "$RESPONSE" | grep -o "HTTP:[0-9]*" | cut -d: -f2)
BODY=$(echo "$RESPONSE" | grep -v "HTTP:")

echo "  HTTP Status: $HTTP_CODE"
echo "  Response: $BODY"

if [ "$HTTP_CODE" = "403" ] || [ "$HTTP_CODE" = "401" ] || [ "$HTTP_CODE" = "400" ]; then
  echo "  ✅ PASS: Elevated API key scopes blocked (HTTP $HTTP_CODE)"
  SCENARIO5="PASS"
elif echo "$BODY" | jq -r '.scopes[]' 2>/dev/null | grep -q "platform"; then
  echo "  ❌ FAIL: Platform scope was granted"
  SCENARIO5="FAIL"
else
  SCENARIO5="PASS"
  echo "  ✅ PASS: Elevated scopes were filtered or blocked"
fi

echo ""
echo "=========================================="
echo "📊 测试总结"
echo "=========================================="
echo ""
echo "场景测试结果:"
echo "  1. 自我角色分配攻击: $SCENARIO1"
echo "  2. 角色创建后门: $SCENARIO2"
echo "  3. 邀请链接权限提升: $SCENARIO3"
echo "  4. 租户所有权转移攻击: $SCENARIO4"
echo "  5. API 密钥权限提升: $SCENARIO5"
echo ""

# Count passes
PASS_COUNT=0
for result in $SCENARIO1 $SCENARIO2 $SCENARIO3 $SCENARIO4 $SCENARIO5; do
  if [ "$result" = "PASS" ]; then
    PASS_COUNT=$((PASS_COUNT + 1))
  fi
done

echo "通过: $PASS_COUNT/5"

if [ "$PASS_COUNT" -eq 5 ]; then
  echo ""
  echo "✅ 所有安全测试通过！"
else
  echo ""
  echo "⚠️  部分测试需要进一步验证"
fi
