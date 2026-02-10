#!/bin/bash
# Reset Docker environment for Auth9 local development
# This script ensures a clean state by removing all containers, images, and volumes

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"

cd "$PROJECT_DIR"

# Compose files: base + observability overlay
DC="docker-compose -f docker-compose.yml -f docker-compose.observability.yml"

echo "Auth9 Docker Environment Reset"
echo "==============================="

# Step 0: Ensure dev gRPC TLS certificate exists
echo "[0/8] gRPC TLS certificate..."
CERT_DIR="$PROJECT_DIR/deploy/dev-certs/grpc"
mkdir -p "$CERT_DIR"
if [ ! -f "$CERT_DIR/server.crt" ] || [ ! -f "$CERT_DIR/server.key" ]; then
  openssl req -x509 -newkey rsa:2048 \
    -keyout "$CERT_DIR/server.key" \
    -out "$CERT_DIR/server.crt" \
    -days 3650 -nodes \
    -subj "/CN=localhost" >/dev/null 2>&1
  chmod 600 "$CERT_DIR/server.key" || true
  echo "  Generated new certificate"
else
  echo "  Found existing certificate"
fi

# Step 1: Stop and remove all containers and volumes
echo "[1/8] Stopping containers..."
$DC --profile build down -v --remove-orphans 2>&1 | tail -1 || true

# Step 2: Remove project images
echo "[2/8] Removing images..."
docker rmi auth9-auth9-core auth9-auth9-portal auth9-auth9-theme-builder auth9-auth9-keycloak-events-builder 2>/dev/null || true
docker rmi $(docker images -q 'auth9-*' 2>/dev/null) 2>/dev/null || true

# Step 3: Remove any remaining volumes
echo "[3/8] Removing volumes..."
docker volume rm auth9_tidb-data auth9_redis-data auth9_keycloak-theme auth9_keycloak-events auth9_prometheus-data auth9_grafana-data auth9_loki-data auth9_tempo-data 2>/dev/null || true

# Step 4: Prune Docker builder cache
echo "[4/8] Pruning builder cache..."
docker builder prune -af 2>/dev/null | tail -1 || true

# Step 5: Build Keycloak theme and events plugin
echo "[5/8] Building Keycloak plugins..."
$DC --profile build build --no-cache auth9-theme-builder auth9-keycloak-events-builder 2>&1 | grep -E '(Built|ERROR)' || true
$DC --profile build up auth9-theme-builder 2>&1 | tail -1
$DC --profile build up auth9-keycloak-events-builder 2>&1 | tail -1

# Step 6: Build all images
echo "[6/8] Building images..."
$DC build --no-cache 2>&1 | grep -E '(Built|ERROR)' || true

# Step 7: Start all services
echo "[7/8] Starting services..."
$DC up -d 2>&1 | tail -1

# Wait for services to be healthy
echo "     Waiting 30s for services..."
sleep 30

# Step 8: Verify
echo "[8/8] Verifying..."
$DC ps --format "table {{.Name}}\t{{.Status}}" 2>/dev/null || $DC ps

echo ""
echo "URLs:"
echo "  Portal:     http://localhost:3000  (admin / Admin123!)"
echo "  Keycloak:   http://localhost:8081  (admin / admin)"
echo "  Mailpit:    http://localhost:8025"
echo "  Grafana:    http://localhost:3001"
echo "  Prometheus: http://localhost:9090"
echo ""
echo "Reset complete!"
