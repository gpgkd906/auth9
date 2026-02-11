#!/bin/bash
# Reset Docker environment for Auth9 local development
# This script ensures a clean state by removing all containers, images, and volumes
#
# Usage:
#   ./scripts/reset-docker.sh          # Normal reset (uses mount cache for fast rebuilds)
#   ./scripts/reset-docker.sh --purge  # Full purge (also clears BuildKit mount caches)

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"

cd "$PROJECT_DIR"

# Parse arguments
PURGE=false
for arg in "$@"; do
  case $arg in
    --purge) PURGE=true ;;
  esac
done

# Compose files: base + observability overlay
DC="docker-compose -f docker-compose.yml -f docker-compose.observability.yml"

echo "Auth9 Docker Environment Reset"
echo "==============================="
if [ "$PURGE" = true ]; then
  echo "Mode: FULL PURGE (clearing all caches)"
else
  echo "Mode: Normal (preserving BuildKit mount caches for fast rebuilds)"
fi

# Step 0: Ensure dev gRPC TLS certificate exists
echo "[0/7] gRPC TLS certificate..."
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
echo "[1/7] Stopping containers..."
$DC --profile build down -v --remove-orphans 2>&1 | tail -1 || true

# Step 2: Remove project images
echo "[2/7] Removing images..."
docker rmi auth9-auth9-core auth9-auth9-portal auth9-auth9-theme-builder auth9-auth9-keycloak-events-builder 2>/dev/null || true
docker rmi $(docker images -q 'auth9-*' 2>/dev/null) 2>/dev/null || true

# Step 3: Remove any remaining volumes
echo "[3/7] Removing volumes..."
docker volume rm auth9_tidb-data auth9_redis-data auth9_keycloak-theme auth9_keycloak-events auth9_prometheus-data auth9_grafana-data auth9_loki-data auth9_tempo-data 2>/dev/null || true

# Step 4: Prune builder cache (only in --purge mode)
if [ "$PURGE" = true ]; then
  echo "[4/7] Pruning ALL builder cache (--purge)..."
  docker builder prune -af 2>/dev/null | tail -1 || true
else
  echo "[4/7] Skipping builder cache prune (mount caches preserved for fast rebuilds)"
fi

# Step 5: Build all images in parallel
echo "[5/7] Building all images (parallel)..."

# Build Keycloak plugins and app images in parallel
$DC --profile build build --no-cache --parallel auth9-theme-builder auth9-keycloak-events-builder 2>&1 | grep -E '(Built|ERROR|built)' || true &
BUILD_PLUGINS_PID=$!

$DC build --no-cache --parallel 2>&1 | grep -E '(Built|ERROR|built)' || true &
BUILD_APP_PID=$!

# Wait for all builds to complete
wait $BUILD_PLUGINS_PID || { echo "ERROR: Plugin build failed"; exit 1; }
wait $BUILD_APP_PID || { echo "ERROR: App build failed"; exit 1; }

# Run plugin builders in parallel to copy JARs to volumes
echo "  Copying Keycloak plugin JARs..."
$DC --profile build up auth9-theme-builder 2>&1 | tail -1 &
$DC --profile build up auth9-keycloak-events-builder 2>&1 | tail -1 &
wait

# Step 6: Start all services and wait for health checks
echo "[6/7] Starting services..."
$DC up -d 2>&1 | tail -1

echo "  Waiting for services to become healthy..."
TIMEOUT=120
ELAPSED=0
while [ $ELAPSED -lt $TIMEOUT ]; do
  # Count services that are still starting or unhealthy
  NOT_READY=$($DC ps --format "{{.Status}}" 2>/dev/null | grep -ciE "starting|unhealthy" || true)
  if [ "$NOT_READY" -eq 0 ]; then
    echo "  All services healthy! (${ELAPSED}s)"
    break
  fi
  sleep 5
  ELAPSED=$((ELAPSED + 5))
  echo "  Still waiting... ($NOT_READY services not ready, ${ELAPSED}s elapsed)"
done

if [ $ELAPSED -ge $TIMEOUT ]; then
  echo "  WARNING: Timed out after ${TIMEOUT}s, some services may not be healthy"
fi

# Step 7: Verify
echo "[7/7] Verifying..."
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
