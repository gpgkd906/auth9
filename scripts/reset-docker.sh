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
echo "[1/5] Stopping and removing containers..."
docker-compose down --remove-orphans 2>/dev/null || true

# Step 2: Remove project images (force rebuild)
echo "[2/5] Removing project images..."
docker rmi auth9-auth9-core auth9-auth9-portal 2>/dev/null || true
docker rmi $(docker images -q 'auth9-*' 2>/dev/null) 2>/dev/null || true

# Step 3: Remove volumes (clean data)
echo "[3/5] Removing volumes..."
docker volume rm auth9_tidb-data auth9_redis-data 2>/dev/null || true

# Step 4: Build all images
echo "[4/5] Building images..."
docker-compose build --no-cache

# Step 5: Start all services
echo "[5/5] Starting services..."
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
echo "Admin Portal: http://localhost:3000"
echo "  Email:    admin@auth9.local"
echo "  Password: Admin123!"
echo ""
echo "Keycloak Admin: http://localhost:8081"
echo "  Username: admin"
echo "  Password: admin"
echo ""
echo "=========================================="
echo "Reset complete!"
echo "=========================================="
