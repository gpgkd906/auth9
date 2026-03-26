#!/bin/bash
# Initialize the .env file with all required secrets for local development.
#
# This script:
#   1. Copies .env.example to .env (if .env doesn't exist)
#   2. Generates random values for JWT_SECRET, SESSION_SECRET, PASSWORD_RESET_HMAC_KEY, SETTINGS_ENCRYPTION_KEY
#   3. Generates RSA 2048 key pair for JWT_PRIVATE_KEY / JWT_PUBLIC_KEY
#   4. Calls gen-dev-keys.sh to also create deploy/dev-certs/jwt/private.key
#
# Safe to run multiple times — only fills in empty values unless --force is used.
#
# Usage:
#   ./scripts/init-dev-env.sh          # First-time setup
#   ./scripts/init-dev-env.sh --force  # Regenerate all secrets

set -eo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"

FORCE=false
for arg in "$@"; do
  case $arg in
    --force) FORCE=true ;;
  esac
done

ENV_FILE="$PROJECT_DIR/.env"
ENV_EXAMPLE="$PROJECT_DIR/.env.example"

echo "=== Auth9 Dev Environment Setup ==="
echo ""

# Step 1: Create .env from template if it doesn't exist
if [ ! -f "$ENV_FILE" ]; then
  if [ ! -f "$ENV_EXAMPLE" ]; then
    echo "ERROR: .env.example not found at $ENV_EXAMPLE"
    exit 1
  fi
  cp "$ENV_EXAMPLE" "$ENV_FILE"
  echo "[1/3] Created .env from .env.example"
else
  echo "[1/3] .env already exists"
fi

# Step 2: Generate random secrets for empty fields
set_if_empty() {
  local key="$1"
  local value="$2"
  local current
  current=$(grep "^${key}=" "$ENV_FILE" | cut -d= -f2- || true)

  # Treat placeholder values as empty
  if [ -z "$current" ] || [ "$current" = "your-super-secret-jwt-key" ] || \
     [ "$current" = "your-session-secret" ] || \
     [ "$current" = "your-password-reset-hmac-key-change-in-production" ] || \
     [ "$FORCE" = true ]; then
    if grep -q "^${key}=" "$ENV_FILE"; then
      sed -i.bak "s|^${key}=.*|${key}=${value}|" "$ENV_FILE"
      rm -f "$ENV_FILE.bak"
    else
      echo "${key}=${value}" >> "$ENV_FILE"
    fi
    echo "  Generated $key"
  else
    echo "  $key already set (skipping)"
  fi
}

echo "[2/3] Generating random secrets..."
set_if_empty "JWT_SECRET" "$(openssl rand -hex 32)"
set_if_empty "SESSION_SECRET" "$(openssl rand -hex 32)"
set_if_empty "PASSWORD_RESET_HMAC_KEY" "$(openssl rand -hex 32)"
set_if_empty "SETTINGS_ENCRYPTION_KEY" "$(openssl rand -base64 32)"

# Step 3: Generate RSA key pair (delegates to gen-dev-keys.sh)
echo "[3/3] Generating JWT RSA key pair..."
FORCE_FLAG=""
[ "$FORCE" = true ] && FORCE_FLAG="--force"
"$SCRIPT_DIR/gen-dev-keys.sh" $FORCE_FLAG

echo ""
echo "=== Setup complete ==="
echo "You can now run: docker-compose up -d"
