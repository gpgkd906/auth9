#!/bin/bash
# Verify Docker environment is ready for QA testing

set -e

echo "==========================================
Auth9 QA Environment Check
=========================================="

# Check if Docker is running
if ! docker ps > /dev/null 2>&1; then
    echo "❌ Docker is not running"
    exit 1
fi

echo "✅ Docker is running"
echo ""

# Check required services
REQUIRED_SERVICES=("auth9-core" "auth9-portal" "auth9-keycloak" "auth9-tidb" "auth9-redis")

echo "Checking services..."
ALL_HEALTHY=true

for SERVICE in "${REQUIRED_SERVICES[@]}"; do
    if docker ps --format "table {{.Names}}\t{{.Status}}" | grep -q "$SERVICE.*healthy"; then
        echo "✅ $SERVICE: healthy"
    elif docker ps --format "table {{.Names}}\t{{.Status}}" | grep -q "$SERVICE"; then
        STATUS=$(docker ps --format "table {{.Names}}\t{{.Status}}" | grep "$SERVICE" | awk '{print $2}')
        echo "⚠️  $SERVICE: $STATUS (not healthy yet)"
        ALL_HEALTHY=false
    else
        echo "❌ $SERVICE: not running"
        ALL_HEALTHY=false
    fi
done

echo ""

# Check service URLs
echo "Checking service URLs..."

if curl -s http://localhost:3000 > /dev/null 2>&1; then
    echo "✅ Portal (http://localhost:3000): accessible"
else
    echo "❌ Portal (http://localhost:3000): not accessible"
    ALL_HEALTHY=false
fi

if curl -s http://localhost:8080/health > /dev/null 2>&1; then
    echo "✅ Auth9 Core (http://localhost:8080): accessible"
else
    echo "❌ Auth9 Core (http://localhost:8080): not accessible"
    ALL_HEALTHY=false
fi

if curl -s http://localhost:8081 > /dev/null 2>&1; then
    echo "✅ Keycloak (http://localhost:8081): accessible"
else
    echo "❌ Keycloak (http://localhost:8081): not accessible"
    ALL_HEALTHY=false
fi

echo ""

# Check database connection
echo "Checking database connection..."
if command -v mysql &> /dev/null; then
    if mysql -h 127.0.0.1 -P 4000 -u root -e "SELECT 1;" > /dev/null 2>&1; then
        echo "✅ TiDB: connection successful (host mysql client)"
    else
        echo "❌ TiDB: connection failed"
        ALL_HEALTHY=false
    fi
else
    echo "⚠️  TiDB: mysql client not found on host (install with: brew install mysql-client)"
    echo "    Skipping database connection test"
fi

echo ""
echo "=========================================="

if [ "$ALL_HEALTHY" = true ]; then
    echo "✅ Environment is ready for QA testing"
    exit 0
else
    echo "❌ Environment has issues - please fix before testing"
    echo ""
    echo "Suggestions:"
    echo "- Wait for services to become healthy (check 'docker ps')"
    echo "- Check logs: docker logs <service-name>"
    echo "- Restart services: docker-compose restart"
    echo "- Reset environment: use reset-local-env skill"
    exit 1
fi
