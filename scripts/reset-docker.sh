#!/bin/bash
# Reset Docker environment for Auth9 local development
# This script ensures a clean state by removing all containers, images, and volumes
#
# Smart rebuild: uses content hashing to skip rebuilding unchanged components.
# Only components whose source files have changed will be rebuilt.
#
# Usage:
#   ./scripts/reset-docker.sh          # Smart reset (skips unchanged components)
#   ./scripts/reset-docker.sh --purge  # Full purge (rebuild everything, clear all caches)

set -eo pipefail

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

# ==================== Smart Build: Content Hash ====================
CACHE_DIR="$PROJECT_DIR/.build-cache"
mkdir -p "$CACHE_DIR"

# Compute a content hash for a component's source files.
# Usage: compute_hash <component_name>
# Outputs the sha256 hash to stdout.
compute_hash() {
  local component="$1"
  local hash_input=""

  # Disable pipefail/errexit here: find may return non-zero for missing dirs, which is expected
  case "$component" in
    auth9-core)
      hash_input=$(set +eo pipefail; find auth9-core/src auth9-core/migrations auth9-core/proto -type f 2>/dev/null | sort | xargs cat 2>/dev/null; cat auth9-core/Cargo.toml auth9-core/Cargo.lock auth9-core/Dockerfile 2>/dev/null; true)
      ;;
    auth9-portal)
      hash_input=$(set +eo pipefail; find auth9-portal/app sdk/packages/core/src -type f 2>/dev/null | sort | xargs cat 2>/dev/null; cat auth9-portal/package.json auth9-portal/package-lock.json auth9-portal/Dockerfile sdk/packages/core/package.json 2>/dev/null; true)
      ;;
    auth9-demo)
      hash_input=$(set +eo pipefail; find auth9-demo/src sdk/packages/core/src sdk/packages/node/src -type f 2>/dev/null | sort | xargs cat 2>/dev/null; cat auth9-demo/package.json auth9-demo/package-lock.json auth9-demo/tsconfig.json auth9-demo/Dockerfile sdk/packages/core/package.json sdk/packages/node/package.json 2>/dev/null; true)
      ;;
    auth9-theme-builder)
      hash_input=$(set +eo pipefail; find auth9-keycloak-theme/src -type f 2>/dev/null | sort | xargs cat 2>/dev/null; cat auth9-keycloak-theme/package.json auth9-keycloak-theme/package-lock.json auth9-keycloak-theme/Dockerfile 2>/dev/null; true)
      ;;
    auth9-keycloak-events-builder)
      hash_input=$(cat auth9-keycloak-events/Dockerfile 2>/dev/null || true)
      ;;
  esac

  echo "$hash_input" | shasum -a 256 | cut -d' ' -f1
}

# Check whether a component needs rebuilding.
# Returns 0 (true) if rebuild needed, 1 (false) if skip.
needs_rebuild() {
  local component="$1"
  local docker_image="auth9-${component}"
  local hash_file="$CACHE_DIR/${component}.hash"

  # --purge always rebuilds
  if [ "$PURGE" = true ]; then
    return 0
  fi

  # Check if Docker image exists
  if ! docker image inspect "$docker_image" &>/dev/null; then
    return 0
  fi

  # Compute current hash and compare with stored hash
  local current_hash
  current_hash=$(compute_hash "$component")

  if [ -f "$hash_file" ] && [ "$(cat "$hash_file")" = "$current_hash" ]; then
    return 1  # no rebuild needed
  fi

  return 0
}

# Save the current hash after a successful build.
save_hash() {
  local component="$1"
  compute_hash "$component" > "$CACHE_DIR/${component}.hash"
}

# ==================== Determine what needs rebuilding ====================
REBUILD_CORE=false
REBUILD_PORTAL=false
REBUILD_DEMO=false
REBUILD_THEME=false
REBUILD_EVENTS=false

for comp in auth9-core auth9-portal auth9-demo auth9-theme-builder auth9-keycloak-events-builder; do
  if needs_rebuild "$comp"; then
    case "$comp" in
      auth9-core)                    REBUILD_CORE=true ;;
      auth9-portal)                  REBUILD_PORTAL=true ;;
      auth9-demo)                    REBUILD_DEMO=true ;;
      auth9-theme-builder)           REBUILD_THEME=true ;;
      auth9-keycloak-events-builder) REBUILD_EVENTS=true ;;
    esac
  fi
done

echo "Auth9 Docker Environment Reset"
echo "==============================="
if [ "$PURGE" = true ]; then
  echo "Mode: FULL PURGE (clearing all caches)"
else
  echo "Mode: Smart (skipping unchanged components)"
  echo ""
  echo "  Build plan:"
  [ "$REBUILD_CORE" = true ]   && echo "    auth9-core                    → REBUILD" || echo "    auth9-core                    → skip (unchanged)"
  [ "$REBUILD_PORTAL" = true ] && echo "    auth9-portal                  → REBUILD" || echo "    auth9-portal                  → skip (unchanged)"
  [ "$REBUILD_DEMO" = true ]   && echo "    auth9-demo                    → REBUILD" || echo "    auth9-demo                    → skip (unchanged)"
  [ "$REBUILD_THEME" = true ]  && echo "    auth9-theme-builder           → REBUILD" || echo "    auth9-theme-builder           → skip (unchanged)"
  [ "$REBUILD_EVENTS" = true ] && echo "    auth9-keycloak-events-builder → REBUILD" || echo "    auth9-keycloak-events-builder → skip (unchanged)"
fi

# Step 0: Ensure dev gRPC TLS certificate exists
echo ""
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
$DC --profile build down -v --remove-orphans > /dev/null 2>&1 || true

# Step 2: Remove project images (only those that need rebuilding)
echo "[2/7] Removing images..."
if [ "$PURGE" = true ]; then
  docker rmi auth9-auth9-core auth9-auth9-portal auth9-auth9-demo auth9-auth9-theme-builder auth9-auth9-keycloak-events-builder 2>/dev/null || true
  docker rmi $(docker images -q 'auth9-*' 2>/dev/null) 2>/dev/null || true
else
  [ "$REBUILD_CORE" = true ]   && { docker rmi auth9-auth9-core auth9-auth9-init 2>/dev/null || true; }
  [ "$REBUILD_PORTAL" = true ] && { docker rmi auth9-auth9-portal 2>/dev/null || true; }
  [ "$REBUILD_DEMO" = true ]   && { docker rmi auth9-auth9-demo 2>/dev/null || true; }
  [ "$REBUILD_THEME" = true ]  && { docker rmi auth9-auth9-theme-builder 2>/dev/null || true; }
  [ "$REBUILD_EVENTS" = true ] && { docker rmi auth9-auth9-keycloak-events-builder 2>/dev/null || true; }
fi

# Step 3: Remove any remaining volumes
echo "[3/7] Removing volumes..."
docker volume rm auth9_tidb-data auth9_redis-data auth9_keycloak-theme auth9_keycloak-events auth9_prometheus-data auth9_grafana-data auth9_loki-data auth9_tempo-data 2>/dev/null || true

# Step 4: Prune builder cache (only in --purge mode)
if [ "$PURGE" = true ]; then
  echo "[4/7] Pruning ALL builder cache (--purge)..."
  docker builder prune -af > /dev/null 2>&1 || true
  # Clear all stored hashes
  rm -f "$CACHE_DIR"/*.hash
else
  echo "[4/7] Skipping builder cache prune (mount caches preserved for fast rebuilds)"
fi

# Step 5: Build images (only changed components)
echo "[5/7] Building images..."

# Build Keycloak plugins (only if needed)
PLUGIN_BUILD_TARGETS=""
[ "$REBUILD_THEME" = true ]  && PLUGIN_BUILD_TARGETS="$PLUGIN_BUILD_TARGETS auth9-theme-builder"
[ "$REBUILD_EVENTS" = true ] && PLUGIN_BUILD_TARGETS="$PLUGIN_BUILD_TARGETS auth9-keycloak-events-builder"

# Build app images (only if needed)
APP_BUILD_TARGETS=""
[ "$REBUILD_CORE" = true ]   && APP_BUILD_TARGETS="$APP_BUILD_TARGETS auth9-core"
[ "$REBUILD_PORTAL" = true ] && APP_BUILD_TARGETS="$APP_BUILD_TARGETS auth9-portal"
[ "$REBUILD_DEMO" = true ]   && APP_BUILD_TARGETS="$APP_BUILD_TARGETS auth9-demo"

# auth9-init shares the same Dockerfile as auth9-core; tag it after build instead of building twice

BUILD_PLUGINS_PID=""
BUILD_APP_PID=""
BUILD_LOG_DIR=$(mktemp -d)

if [ -n "$PLUGIN_BUILD_TARGETS" ]; then
  echo "  Building plugins:$PLUGIN_BUILD_TARGETS"
  ( $DC --profile build build --no-cache --parallel $PLUGIN_BUILD_TARGETS > "$BUILD_LOG_DIR/plugins.log" 2>&1 ) &
  BUILD_PLUGINS_PID=$!
else
  echo "  Plugins: all unchanged, skipping"
fi

if [ -n "$APP_BUILD_TARGETS" ]; then
  echo "  Building apps:$APP_BUILD_TARGETS"
  ( $DC build --no-cache --parallel $APP_BUILD_TARGETS > "$BUILD_LOG_DIR/apps.log" 2>&1 ) &
  BUILD_APP_PID=$!
else
  echo "  Apps: all unchanged, skipping"
fi

# Wait for builds to complete
if [ -n "$BUILD_PLUGINS_PID" ]; then
  if ! wait $BUILD_PLUGINS_PID; then
    echo "ERROR: Plugin build failed. Log:"
    tail -20 "$BUILD_LOG_DIR/plugins.log" 2>/dev/null
    rm -rf "$BUILD_LOG_DIR"
    exit 1
  fi
fi
if [ -n "$BUILD_APP_PID" ]; then
  if ! wait $BUILD_APP_PID; then
    echo "ERROR: App build failed. Log:"
    tail -20 "$BUILD_LOG_DIR/apps.log" 2>/dev/null
    rm -rf "$BUILD_LOG_DIR"
    exit 1
  fi
fi
rm -rf "$BUILD_LOG_DIR"

# Tag auth9-core image as auth9-init (same binary, different command at runtime)
if [ "$REBUILD_CORE" = true ]; then
  docker tag auth9-auth9-core auth9-auth9-init
fi

# Save hashes for successfully built components
[ "$REBUILD_CORE" = true ]   && save_hash "auth9-core"
[ "$REBUILD_PORTAL" = true ] && save_hash "auth9-portal"
[ "$REBUILD_DEMO" = true ]   && save_hash "auth9-demo"
[ "$REBUILD_THEME" = true ]  && save_hash "auth9-theme-builder"
[ "$REBUILD_EVENTS" = true ] && save_hash "auth9-keycloak-events-builder"

# Run plugin builders to copy JARs to volumes (always needed since volumes are recreated)
echo "  Copying Keycloak plugin JARs..."
$DC --profile build up auth9-theme-builder > /dev/null 2>&1 &
THEME_PID=$!
$DC --profile build up auth9-keycloak-events-builder > /dev/null 2>&1 &
EVENTS_PID=$!
wait $THEME_PID || { echo "WARNING: Theme builder failed (non-fatal)"; }
wait $EVENTS_PID || { echo "WARNING: Events builder failed (non-fatal)"; }

# Step 6: Start all services and wait for health checks
echo "[6/7] Starting services..."
$DC up -d

echo "  Waiting for services to become healthy..."
CRITICAL_SERVICES="auth9-core auth9-portal"
TIMEOUT=180
ELAPSED=0
INIT_CHECKED=false

while [ $ELAPSED -lt $TIMEOUT ]; do
  # Check if auth9-init failed (only need to check once after it exits)
  if [ "$INIT_CHECKED" = false ]; then
    INIT_STATUS=$($DC ps auth9-init --format "{{.Status}}" 2>/dev/null || true)
    if echo "$INIT_STATUS" | grep -qiE "exited"; then
      if echo "$INIT_STATUS" | grep -qiE "exited \(0\)"; then
        INIT_CHECKED=true
      else
        echo ""
        echo "  ERROR: auth9-init failed! Dependent services (auth9-core, auth9-portal) cannot start."
        echo "  auth9-init logs:"
        $DC logs --tail=30 auth9-init 2>/dev/null || true
        echo ""
        echo "  All service statuses:"
        $DC ps --format "table {{.Name}}\t{{.Status}}" 2>/dev/null || $DC ps
        exit 1
      fi
    fi
  fi

  # Count services that are NOT ready (starting, unhealthy, created but not running, or exited with error)
  NOT_READY=$($DC ps --format "{{.Status}}" 2>/dev/null | grep -ciE "starting|unhealthy|created|exited \([^0]" || true)
  if [ "$NOT_READY" -eq 0 ]; then
    # Double-check: verify critical services are actually running
    ALL_CRITICAL_UP=true
    for svc in $CRITICAL_SERVICES; do
      SVC_STATUS=$($DC ps "$svc" --format "{{.Status}}" 2>/dev/null || true)
      if ! echo "$SVC_STATUS" | grep -qiE "^up|running"; then
        ALL_CRITICAL_UP=false
        break
      fi
    done

    if [ "$ALL_CRITICAL_UP" = true ]; then
      echo "  All services healthy! (${ELAPSED}s)"
      break
    fi
  fi

  sleep 5
  ELAPSED=$((ELAPSED + 5))
  if [ $((ELAPSED % 15)) -eq 0 ]; then
    echo "  Still waiting... ($NOT_READY services not ready, ${ELAPSED}s elapsed)"
  fi
done

if [ $ELAPSED -ge $TIMEOUT ]; then
  echo ""
  echo "  ERROR: Timed out after ${TIMEOUT}s. Service statuses:"
  $DC ps --format "table {{.Name}}\t{{.Status}}" 2>/dev/null || $DC ps
  echo ""
  # Show logs of critical services that aren't running
  for svc in $CRITICAL_SERVICES; do
    SVC_STATUS=$($DC ps "$svc" --format "{{.Status}}" 2>/dev/null || true)
    if ! echo "$SVC_STATUS" | grep -qiE "^up|running"; then
      echo "  $svc is not running (status: $SVC_STATUS). Logs:"
      $DC logs --tail=15 "$svc" 2>/dev/null || true
      echo ""
    fi
  done
  exit 1
fi

# Step 7: Verify
echo "[7/7] Verifying..."
$DC ps --format "table {{.Name}}\t{{.Status}}" 2>/dev/null || $DC ps

echo ""
echo "URLs:"
echo "  Portal:     http://localhost:3000  (admin / SecurePass123!)"
echo "  Demo:       http://localhost:3002  (SDK integration guide)"
echo "  Keycloak:   http://localhost:8081  (admin / admin)"
echo "  Mailpit:    http://localhost:8025"
echo "  Grafana:    http://localhost:3001"
echo "  Prometheus: http://localhost:9090"
echo ""
echo "Reset complete!"
