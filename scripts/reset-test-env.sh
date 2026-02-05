#!/bin/bash
# Reset Auth9 Test Environment
# 重置 Auth9 测试环境

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

echo "=========================================="
echo "Auth9 Test Environment Reset"
echo "=========================================="
echo ""

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Confirmation
read -p "⚠️  This will delete all test data. Continue? (y/N) " -n 1 -r
echo ""
if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    echo "❌ Aborted"
    exit 1
fi

echo ""
echo "Step 1: Cleaning database test data..."
echo "----------------------------------------"

# Check if database is accessible
if ! mysql -h 127.0.0.1 -P 4000 -u root -e "SELECT 1;" > /dev/null 2>&1; then
    echo "❌ Database not accessible. Please start services first:"
    echo "   docker-compose -f docker-compose.yml -f docker-compose.dev.yml up -d"
    exit 1
fi

# Clean test data from database
mysql -h 127.0.0.1 -P 4000 -u root -D auth9 <<EOF
-- Clean test data (prefixed with qa- or sec-)
DELETE FROM audit_logs WHERE created_at < DATE_SUB(NOW(), INTERVAL 1 HOUR);
DELETE FROM user_tenant_roles WHERE tenant_user_id IN (
    SELECT id FROM tenant_users WHERE tenant_id IN (
        SELECT id FROM tenants WHERE slug LIKE 'qa-%' OR slug LIKE 'sec-%'
    )
);
DELETE FROM tenant_users WHERE tenant_id IN (
    SELECT id FROM tenants WHERE slug LIKE 'qa-%' OR slug LIKE 'sec-%'
);
DELETE FROM invitations WHERE tenant_id IN (
    SELECT id FROM tenants WHERE slug LIKE 'qa-%' OR slug LIKE 'sec-%'
);
DELETE FROM webhooks WHERE tenant_id IN (
    SELECT id FROM tenants WHERE slug LIKE 'qa-%' OR slug LIKE 'sec-%'
);
DELETE FROM role_permissions WHERE role_id IN (
    SELECT id FROM roles WHERE service_id IN (
        SELECT id FROM services WHERE slug LIKE 'qa-%' OR slug LIKE 'sec-%'
    )
);
DELETE FROM roles WHERE service_id IN (
    SELECT id FROM services WHERE slug LIKE 'qa-%' OR slug LIKE 'sec-%'
);
DELETE FROM permissions WHERE service_id IN (
    SELECT id FROM services WHERE slug LIKE 'qa-%' OR slug LIKE 'sec-%'
);
DELETE FROM clients WHERE service_id IN (
    SELECT id FROM services WHERE slug LIKE 'qa-%' OR slug LIKE 'sec-%'
);
DELETE FROM services WHERE slug LIKE 'qa-%' OR slug LIKE 'sec-%';
DELETE FROM tenants WHERE slug LIKE 'qa-%' OR slug LIKE 'sec-%';
DELETE FROM linked_identities WHERE user_id IN (
    SELECT id FROM users WHERE email LIKE '%@qa-test.local' OR email LIKE '%@security.local'
);
DELETE FROM sessions WHERE user_id IN (
    SELECT id FROM users WHERE email LIKE '%@qa-test.local' OR email LIKE '%@security.local'
);
DELETE FROM password_reset_tokens WHERE user_id IN (
    SELECT id FROM users WHERE email LIKE '%@qa-test.local' OR email LIKE '%@security.local'
);
DELETE FROM users WHERE email LIKE '%@qa-test.local' OR email LIKE '%@security.local';
EOF

echo "✅ Database cleaned"
echo ""

# Step 2: Clean Keycloak test users (optional - requires Keycloak Admin API)
echo "Step 2: Cleaning Keycloak test users..."
echo "----------------------------------------"
echo "⚠️  Manual step: Please clean test users in Keycloak Admin Console"
echo "    URL: http://localhost:8081/admin"
echo "    Realm: auth9"
echo "    Filter by email: @qa-test.local, @security.local"
echo ""

# Step 3: Clean Redis cache
echo "Step 3: Cleaning Redis cache..."
echo "----------------------------------------"
if command -v redis-cli &> /dev/null; then
    redis-cli FLUSHDB > /dev/null 2>&1 && echo "✅ Redis cache cleared" || echo "⚠️  Redis not accessible (skip)"
else
    echo "⚠️  redis-cli not installed (skip)"
fi
echo ""

# Step 4: Load seed data
echo "Step 4: Load seed data?"
echo "----------------------------------------"
echo "Available datasets:"
echo "  1) qa-basic        - Basic QA test data (recommended)"
echo "  2) qa-complex      - Complex QA scenarios"
echo "  3) security        - Security test data (vulnerable configs)"
echo "  4) skip            - Skip seed data loading"
echo ""
read -p "Select dataset (1-4): " -n 1 -r DATASET_CHOICE
echo ""

case $DATASET_CHOICE in
    1)
        DATASET="qa-basic"
        ;;
    2)
        DATASET="qa-complex"
        ;;
    3)
        DATASET="security-vulnerable"
        ;;
    4)
        echo "⏭️  Skipping seed data"
        echo ""
        echo "=========================================="
        echo "✅ Test environment reset complete"
        echo "=========================================="
        exit 0
        ;;
    *)
        echo "❌ Invalid choice. Exiting."
        exit 1
        ;;
esac

echo ""
echo "Loading seed data: $DATASET..."
echo "----------------------------------------"

cd "$ROOT_DIR/auth9-core"

if [ -f "Cargo.toml" ]; then
    # TODO: Implement seed-data binary
    echo "⚠️  seed-data binary not yet implemented"
    echo "    Manual step: Load data from scripts/seed-data/$DATASET.yaml"
    echo ""
else
    echo "❌ auth9-core not found"
    exit 1
fi

echo "=========================================="
echo "✅ Test environment reset complete"
echo "=========================================="
echo ""
echo "Next steps:"
echo "  1. Start backend: cd auth9-core && cargo run"
echo "  2. Start frontend: cd auth9-portal && npm run dev"
echo "  3. Access portal: http://localhost:3000"
echo ""

case $DATASET in
    qa-basic)
        echo "Test accounts:"
        echo "  - admin@qa-acme-corp.local / QaAcmeAdmin123!"
        echo "  - user1@qa-acme-corp.local / QaUser123!"
        ;;
    security-vulnerable)
        echo "⚠️  Security test data loaded (contains vulnerable configurations)"
        echo "Test accounts:"
        echo "  - sqli-test@security.local / SecTest123!"
        echo "  - weak@security.local / 1 (weak password)"
        ;;
esac
echo ""
