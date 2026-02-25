#!/bin/bash
# security_ssrf_test.sh - Automated SSRF Security Tests

set -euo pipefail

API_BASE="http://localhost:8080"
TOKEN_GEN="node .claude/skills/tools/gen-test-tokens.js"
PLATFORM_TENANT_ID="a4ccb751-f62c-44c9-8f45-19f5d2fd6491"

echo "=========================================="
echo "🔐 SSRF Security Tests"
echo "=========================================="

# 1. Generate Platform Admin Token
echo "Generating platform admin token..."
TOKEN=$($TOKEN_GEN tenant-owner --tenant-id $PLATFORM_TENANT_ID --user-id 51d09a19-0eb8-4d2c-89fb-43bfa82c5381)

echo ""
echo "=========================================="
echo "🧪 场景 1: Webhook URL 内网探测 (127.0.0.1, internal IP)"
echo "=========================================="

# Loop through dangerous targets
TARGETS=("http://127.0.0.1:4000" "http://localhost:8080" "http://192.168.1.1" "http://169.254.169.254/latest/meta-data/")

for target in "${TARGETS[@]}"; do
    echo "Testing target: $target"
    STATUS=$(curl -s -X POST -o /dev/null -w "%{http_code}" -H "Authorization: Bearer $TOKEN" -H "Content-Type: application/json" "$API_BASE/api/v1/tenants/$PLATFORM_TENANT_ID/webhooks" -d "{\"url\": \"$target\", \"events\": [\"user.created\"]}")
    
    if [ "$STATUS" == "400" ] || [ "$STATUS" == "422" ]; then
        echo "✅ PASS: Internal address $target rejected (HTTP $STATUS)"
    else
        echo "❌ FAIL: Internal address $target accepted or returned unexpected status: $STATUS"
    fi
done

echo ""
echo "=========================================="
echo "🧪 场景 2: URL 协议滥用 (file://, gopher://, etc.)"
echo "=========================================="

PROTOCOLS=("file:///etc/passwd" "gopher://127.0.0.1:6379/_FLUSHALL" "dict://127.0.0.1:6379/INFO" "data:text/html,hack")

for target in "${PROTOCOLS[@]}"; do
    echo "Testing protocol: $target"
    STATUS=$(curl -s -X POST -o /dev/null -w "%{http_code}" -H "Authorization: Bearer $TOKEN" -H "Content-Type: application/json" "$API_BASE/api/v1/tenants/$PLATFORM_TENANT_ID/webhooks" -d "{\"url\": \"$target\", \"events\": [\"user.created\"]}")
    
    if [ "$STATUS" == "400" ] || [ "$STATUS" == "422" ]; then
        echo "✅ PASS: Protocol $target rejected (HTTP $STATUS)"
    else
        echo "❌ FAIL: Protocol $target accepted or returned unexpected status: $STATUS"
    fi
done

echo ""
echo "=========================================="
echo "📊 测试总结"
echo "=========================================="
echo "已执行 8 个 SSRF 场景。"
