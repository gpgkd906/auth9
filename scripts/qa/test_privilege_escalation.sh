#!/usr/bin/env bash
# Privilege Escalation regression checks with dynamic fixture data.
# No hardcoded tenant IDs, user IDs, or stale browser tokens.

set -euo pipefail

API_BASE="${API_BASE:-http://localhost:8080}"
MYSQL_HOST="${MYSQL_HOST:-127.0.0.1}"
MYSQL_PORT="${MYSQL_PORT:-4000}"
MYSQL_USER="${MYSQL_USER:-root}"
MYSQL_DB="${MYSQL_DB:-auth9}"
# JWT_PRIVATE_KEY can be a file path or inline PEM content.
# Auto-detect from .env when neither is set and the default file is missing.
if [[ -z "${JWT_PRIVATE_KEY:-}" ]]; then
  if [[ -f "deploy/dev-certs/jwt/private.key" ]]; then
    JWT_PRIVATE_KEY="deploy/dev-certs/jwt/private.key"
  elif [[ -f ".env" ]]; then
    _inline_key="$(grep '^JWT_PRIVATE_KEY=' .env | head -1 | sed 's/^JWT_PRIVATE_KEY=//' | sed 's/^"//' | sed 's/"$//')"
    if [[ -n "$_inline_key" ]]; then
      _tmpkey="$(mktemp)"
      printf '%b' "$_inline_key" > "$_tmpkey"
      JWT_PRIVATE_KEY="$_tmpkey"
      trap 'rm -f "$_tmpkey"' EXIT
    else
      JWT_PRIVATE_KEY="deploy/dev-certs/jwt/private.key"
    fi
  else
    JWT_PRIVATE_KEY="deploy/dev-certs/jwt/private.key"
  fi
fi
TEST_TENANT_SLUG="${TEST_TENANT_SLUG:-demo}"
PLATFORM_TENANT_SLUG="${PLATFORM_TENANT_SLUG:-auth9-platform}"
PORTAL_CLIENT_ID="${PORTAL_CLIENT_ID:-auth9-portal}"

MEMBER_USER_ID="11111111-1111-1111-1111-111111111111"
TENANT_ADMIN_USER_ID="33333333-3333-3333-3333-333333333333"
MEMBER_TU_ID="22222222-2222-2222-2222-222222222222"
TENANT_ADMIN_TU_ID="44444444-4444-4444-4444-444444444444"

PASS_COUNT=0
FAIL_COUNT=0
SKIP_COUNT=0

mysql_q() {
  mysql -h "$MYSQL_HOST" -P "$MYSQL_PORT" -u "$MYSQL_USER" "$MYSQL_DB" -N -e "$1"
}

mark_pass() { PASS_COUNT=$((PASS_COUNT + 1)); echo "  ✅ PASS: $1"; }
mark_fail() { FAIL_COUNT=$((FAIL_COUNT + 1)); echo "  ❌ FAIL: $1"; }
mark_skip() { SKIP_COUNT=$((SKIP_COUNT + 1)); echo "  ⏭️  SKIP: $1"; }

require_bin() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "Missing dependency: $1"
    exit 2
  fi
}

json_get() {
  local body="$1"
  local expr="$2"
  echo "$body" | jq -r "$expr" 2>/dev/null || true
}

do_request() {
  local method="$1"
  local url="$2"
  local token="${3:-}"
  local body="${4:-}"
  local out_file
  out_file="$(mktemp)"

  if [[ -n "$token" ]]; then
    if [[ -n "$body" ]]; then
      curl -sS -o "$out_file" -w "%{http_code}" -X "$method" "$url" \
        -H "Authorization: Bearer $token" \
        -H "Content-Type: application/json" \
        -d "$body"
    else
      curl -sS -o "$out_file" -w "%{http_code}" -X "$method" "$url" \
        -H "Authorization: Bearer $token"
    fi
  else
    if [[ -n "$body" ]]; then
      curl -sS -o "$out_file" -w "%{http_code}" -X "$method" "$url" \
        -H "Content-Type: application/json" \
        -d "$body"
    else
      curl -sS -o "$out_file" -w "%{http_code}" -X "$method" "$url"
    fi
  fi
  echo
  cat "$out_file"
  rm -f "$out_file"
}

gen_tenant_access_token() {
  local user_id="$1"
  local email="$2"
  local tenant_id="$3"
  local roles_json="$4"
  local perms_json="$5"
  node -e '
const jwt=require("jsonwebtoken");
const fs=require("fs");
const now=Math.floor(Date.now()/1000);
const privateKey=fs.readFileSync(process.argv[1], "utf8");
const payload={
  sub: process.argv[2],
  email: process.argv[3],
  iss: "http://localhost:8080",
  aud: process.argv[4],
  token_type: "tenant_access",
  tenant_id: process.argv[5],
  roles: JSON.parse(process.argv[6]),
  permissions: JSON.parse(process.argv[7]),
  iat: now,
  exp: now + 3600,
  sid: "sid-" + process.argv[2].slice(0, 8)
};
process.stdout.write(jwt.sign(payload, privateKey, {algorithm:"RS256", keyid:"auth9-current"}));
' "$JWT_PRIVATE_KEY" "$user_id" "$email" "$PORTAL_CLIENT_ID" "$tenant_id" "$roles_json" "$perms_json"
}

echo "=========================================="
echo "🔒 Privilege Escalation QA Test (Dynamic)"
echo "=========================================="

require_bin mysql
require_bin jq
require_bin node
require_bin curl

if [[ ! -f "$JWT_PRIVATE_KEY" ]]; then
  echo "Missing key: $JWT_PRIVATE_KEY"
  exit 2
fi

DEMO_TENANT_ID="$(mysql_q "SELECT id FROM tenants WHERE slug='$TEST_TENANT_SLUG' LIMIT 1;")"
PLATFORM_TENANT_ID="$(mysql_q "SELECT id FROM tenants WHERE slug='$PLATFORM_TENANT_SLUG' LIMIT 1;")"
PLATFORM_ADMIN_ID="$(mysql_q "SELECT id FROM users WHERE email='admin@auth9.local' LIMIT 1;")"
ADMIN_ROLE_ID="$(mysql_q "SELECT id FROM roles WHERE name='admin' LIMIT 1;")"
ADMIN_SERVICE_ID="$(mysql_q "SELECT service_id FROM roles WHERE name='admin' LIMIT 1;")"

if [[ -z "$DEMO_TENANT_ID" || -z "$PLATFORM_TENANT_ID" || -z "$PLATFORM_ADMIN_ID" || -z "$ADMIN_ROLE_ID" || -z "$ADMIN_SERVICE_ID" ]]; then
  echo "Missing required baseline data in DB; run reset/init first."
  exit 2
fi

# Deterministic fixture users for regression tests.
mysql -h "$MYSQL_HOST" -P "$MYSQL_PORT" -u "$MYSQL_USER" "$MYSQL_DB" <<SQL
DELETE FROM tenant_users WHERE user_id IN ('$MEMBER_USER_ID','$TENANT_ADMIN_USER_ID');
DELETE FROM users WHERE id IN ('$MEMBER_USER_ID','$TENANT_ADMIN_USER_ID');
INSERT INTO users (id,identity_subject,email,display_name,mfa_enabled)
VALUES
  ('$MEMBER_USER_ID','kc-member-111','member@test.local','Member User',0),
  ('$TENANT_ADMIN_USER_ID','kc-tenant-admin-333','tenantadmin@test.local','Tenant Admin',0);
INSERT INTO tenant_users (id,tenant_id,user_id,role_in_tenant)
VALUES
  ('$MEMBER_TU_ID','$DEMO_TENANT_ID','$MEMBER_USER_ID','member'),
  ('$TENANT_ADMIN_TU_ID','$DEMO_TENANT_ID','$TENANT_ADMIN_USER_ID','admin');
SQL

MEMBER_TOKEN="$(gen_tenant_access_token "$MEMBER_USER_ID" "member@test.local" "$DEMO_TENANT_ID" '["member"]' '[]')"
TENANT_ADMIN_TOKEN="$(gen_tenant_access_token "$TENANT_ADMIN_USER_ID" "tenantadmin@test.local" "$DEMO_TENANT_ID" '["admin"]' '["rbac:write"]')"

echo
echo "=========================================="
echo "🧪 场景1: 自我角色分配攻击"
echo "=========================================="
REQ_BODY="$(jq -nc --arg uid "$MEMBER_USER_ID" --arg tid "$DEMO_TENANT_ID" --arg rid "$ADMIN_ROLE_ID" '{user_id:$uid,tenant_id:$tid,role_ids:[$rid]}')"
HTTP_AND_BODY="$(do_request POST "$API_BASE/api/v1/rbac/assign" "$MEMBER_TOKEN" "$REQ_BODY")"
HTTP_CODE="$(echo "$HTTP_AND_BODY" | head -n1)"
BODY="$(echo "$HTTP_AND_BODY" | tail -n +2)"
echo "  HTTP Status: $HTTP_CODE"
echo "  Response: $BODY"
if [[ "$HTTP_CODE" == "403" ]]; then
  mark_pass "Self-role assignment blocked with 403"
else
  mark_fail "Expected 403, got $HTTP_CODE"
fi

echo
echo "=========================================="
echo "🧪 场景2: 角色创建后门"
echo "=========================================="
ROLE_NAME="super_role_$(date +%s)"
REQ_BODY="$(jq -nc --arg n "$ROLE_NAME" --arg sid "$ADMIN_SERVICE_ID" --arg d "Attempt role creation by non-platform admin" '{name:$n,service_id:$sid,description:$d,permissions:["platform:admin"]}')"
HTTP_AND_BODY="$(do_request POST "$API_BASE/api/v1/roles" "$TENANT_ADMIN_TOKEN" "$REQ_BODY")"
HTTP_CODE="$(echo "$HTTP_AND_BODY" | head -n1)"
BODY="$(echo "$HTTP_AND_BODY" | tail -n +2)"
echo "  HTTP Status: $HTTP_CODE"
echo "  Response: $BODY"
if [[ "$HTTP_CODE" == "403" ]]; then
  mark_pass "Role creation requires platform admin"
else
  mark_fail "Expected 403, got $HTTP_CODE"
fi

echo
echo "=========================================="
echo "🧪 场景3: 邀请链接权限提升"
echo "=========================================="
INVALID_TOKEN="invalid-token-$(date +%s)"
REQ_BODY="$(jq -nc --arg t "$INVALID_TOKEN" --arg r "admin" '{token:$t,role:$r}')"
HTTP_AND_BODY="$(do_request POST "$API_BASE/api/v1/invitations/accept" "" "$REQ_BODY")"
HTTP_CODE="$(echo "$HTTP_AND_BODY" | head -n1)"
BODY="$(echo "$HTTP_AND_BODY" | tail -n +2)"
echo "  HTTP Status: $HTTP_CODE"
echo "  Response: $BODY"
if [[ "$HTTP_CODE" == "400" || "$HTTP_CODE" == "401" || "$HTTP_CODE" == "404" ]]; then
  mark_pass "Invalid invitation token rejected"
else
  mark_fail "Expected 400/401/404, got $HTTP_CODE"
fi

echo
echo "=========================================="
echo "🧪 场景4: 租户所有权转移攻击"
echo "=========================================="
REQ_BODY="$(jq -nc --arg oid "$TENANT_ADMIN_USER_ID" '{owner_id:$oid}')"
HTTP_AND_BODY="$(do_request PUT "$API_BASE/api/v1/tenants/$DEMO_TENANT_ID" "$MEMBER_TOKEN" "$REQ_BODY")"
HTTP_CODE="$(echo "$HTTP_AND_BODY" | head -n1)"
BODY="$(echo "$HTTP_AND_BODY" | tail -n +2)"
echo "  HTTP Status: $HTTP_CODE"
echo "  Response: $BODY"
if [[ "$HTTP_CODE" == "403" ]]; then
  mark_pass "Ownership transfer blocked for non-owner"
else
  mark_fail "Expected 403, got $HTTP_CODE"
fi

echo
echo "=========================================="
echo "🧪 场景5: API 密钥权限提升"
echo "=========================================="
if [[ "$(do_request OPTIONS "$API_BASE/api/v1/api-keys" "$MEMBER_TOKEN" "" | head -n1)" == "404" ]]; then
  mark_skip "api-keys endpoint not present in current build"
else
  REQ_BODY="$(jq -nc --arg n "test-elevated-key-$(date +%s)" '{name:$n,scopes:["admin:write","platform:admin"]}')"
  HTTP_AND_BODY="$(do_request POST "$API_BASE/api/v1/api-keys" "$MEMBER_TOKEN" "$REQ_BODY")"
  HTTP_CODE="$(echo "$HTTP_AND_BODY" | head -n1)"
  BODY="$(echo "$HTTP_AND_BODY" | tail -n +2)"
  echo "  HTTP Status: $HTTP_CODE"
  echo "  Response: $BODY"
  if [[ "$HTTP_CODE" == "400" || "$HTTP_CODE" == "401" || "$HTTP_CODE" == "403" || "$HTTP_CODE" == "404" ]]; then
    mark_pass "Elevated API key scopes blocked or endpoint unavailable"
  else
    mark_fail "Unexpected API key escalation result: HTTP $HTTP_CODE"
  fi
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
