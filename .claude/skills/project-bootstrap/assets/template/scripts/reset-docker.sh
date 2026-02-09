#!/usr/bin/env bash
# Project Docker Environment Reset
#
# Generic reset script. Optionally enable project-specific extras.
#
# Usage:
#   ./scripts/reset-docker.sh
#
# Env:
#   PROJECT_NAME (default: {{project_name}})
#   ENABLE_AUTH9_EXTRAS=true  # enable Auth9-specific optional steps

set -e

PROJECT_NAME="${PROJECT_NAME:-{{project_name}}}"

# Step 1: Stop and remove containers and volumes
printf "[1/4] Stopping and removing containers and volumes...\n"
docker-compose down -v --remove-orphans 2>/dev/null || true

# Step 2: Remove project images (force rebuild)
printf "[2/4] Removing project images...\n"
docker rmi ${PROJECT_NAME}-core ${PROJECT_NAME}-portal 2>/dev/null || true

# Step 3: Prune Docker builder cache (optional safety net)
printf "[3/4] Pruning Docker builder cache...\n"
docker builder prune -af 2>/dev/null || true

# Step 4: Build and start services
printf "[4/4] Building and starting services...\n"
docker-compose build --no-cache

docker-compose up -d

if [ "${ENABLE_AUTH9_EXTRAS:-}" = "true" ]; then
  echo "[Auth9 extras] Add Keycloak theme build steps here if needed."
fi

printf "Reset complete.\n"
