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

# Step 0: Ensure dev gRPC TLS certificate exists (used by docker-compose.yml)
echo "[0/8] Ensuring gRPC dev TLS certificate..."
CERT_DIR="$PROJECT_DIR/deploy/dev-certs/grpc"
mkdir -p "$CERT_DIR"
if [ ! -f "$CERT_DIR/server.crt" ] || [ ! -f "$CERT_DIR/server.key" ]; then
  openssl req -x509 -newkey rsa:2048 \
    -keyout "$CERT_DIR/server.key" \
    -out "$CERT_DIR/server.crt" \
    -days 3650 -nodes \
    -subj "/CN=localhost" >/dev/null 2>&1
  chmod 600 "$CERT_DIR/server.key" || true
  echo "Generated $CERT_DIR/server.crt and server.key"
else
  echo "Found existing $CERT_DIR/server.crt and server.key"
fi

# Step 1: Stop and remove all containers (including build profile) and volumes
echo "[1/8] Stopping and removing containers and volumes..."
docker-compose --profile build down -v --remove-orphans 2>/dev/null || true

# Step 2: Remove project images (force rebuild)
echo "[2/8] Removing project images..."
docker rmi auth9-auth9-core auth9-auth9-portal auth9-auth9-theme-builder auth9-auth9-keycloak-events-builder 2>/dev/null || true
docker rmi $(docker images -q 'auth9-*' 2>/dev/null) 2>/dev/null || true

# Step 3: Remove any remaining volumes (safety net)
echo "[3/8] Removing remaining volumes..."
docker volume rm auth9_tidb-data auth9_redis-data auth9_keycloak-theme auth9_keycloak-events 2>/dev/null || true

# Step 4: Prune Docker builder cache to avoid stale layers
echo "[4/8] Pruning Docker builder cache..."
docker builder prune -af 2>/dev/null || true

# Step 5: Build Keycloak theme and events plugin (no cache)
echo "[5/8] Building Keycloak theme and events plugin..."
docker-compose --profile build build --no-cache auth9-theme-builder auth9-keycloak-events-builder
docker-compose --profile build up auth9-theme-builder
docker-compose --profile build up auth9-keycloak-events-builder

# Step 6: Build all images (no cache)
echo "[6/8] Building images..."
docker-compose build --no-cache

# Step 7: Start all services
echo "[7/8] Starting services..."
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
