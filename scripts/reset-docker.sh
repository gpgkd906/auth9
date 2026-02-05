#!/bin/bash
# Reset Docker environment for Auth9 local development
# This script ensures a clean state by removing all containers, images, and volumes

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"

cd "$PROJECT_DIR"

echo "=========================================="
echo "Auth9 Docker Environment Reset"
echo "=========================================="
echo ""

# Step 1: Stop and remove all containers
echo "[1/6] Stopping and removing containers..."
docker-compose down --remove-orphans 2>/dev/null || true

# Step 2: Remove project images (force rebuild)
echo "[2/6] Removing project images..."
docker rmi auth9-auth9-core auth9-auth9-portal auth9-auth9-theme-builder 2>/dev/null || true
docker rmi $(docker images -q 'auth9-*' 2>/dev/null) 2>/dev/null || true

# Step 3: Remove volumes (clean data)
echo "[3/6] Removing volumes..."
docker volume rm auth9_tidb-data auth9_redis-data auth9_keycloak-theme auth9_keycloak-events 2>/dev/null || true

# Step 4: Build Keycloak theme and events plugin
echo "[4/6] Building Keycloak theme and events plugin..."
docker-compose --profile build up --build auth9-theme-builder
docker-compose --profile build up --build auth9-keycloak-events-builder

# Step 5: Build all images
echo "[5/6] Building images..."
docker-compose build --no-cache

# Step 6: Start all services
echo "[6/6] Starting services..."
docker-compose up -d

# Wait for services to be healthy
echo ""
echo "Waiting for services to be healthy..."
sleep 30

# Show status
echo ""
echo "=========================================="
echo "Service Status"
echo "=========================================="
docker-compose ps

echo ""
echo "=========================================="
echo "Initial Credentials"
echo "=========================================="
echo "Auth9 Admin Portal: http://localhost:3000"
echo "  Username: admin"
echo "  Password: Admin123!"
echo ""
echo "Keycloak Admin: http://localhost:8081"
echo "  Username: admin"
echo "  Password: admin"
echo ""
echo "Mailpit (Email Testing): http://localhost:8025"
echo "  All outgoing emails are captured here"
echo ""
echo "NOTE: To use the Auth9 login theme, go to:"
echo "  Keycloak Admin > Realm Settings > Themes > Login Theme > auth9"
echo ""
echo "=========================================="
echo "Reset complete!"
echo "=========================================="
