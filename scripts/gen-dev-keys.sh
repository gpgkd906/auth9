#!/bin/bash
# Generate development JWT keys for local development and QA tools.
#
# Generates a fresh RSA 2048 key pair and outputs:
#   1. deploy/dev-certs/jwt/private.key  (for QA tools)
#   2. Escaped PEM strings suitable for .env (printed to stdout)
#
# If .env exists and JWT_PRIVATE_KEY is empty, the keys are written into .env automatically.
#
# Idempotent: skips generation if the key file already exists.
# Use --force to regenerate.

set -eo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"

FORCE=false
for arg in "$@"; do
  case $arg in
    --force) FORCE=true ;;
  esac
done

JWT_DIR="$PROJECT_DIR/deploy/dev-certs/jwt"
PRIVATE_KEY="$JWT_DIR/private.key"

if [ -f "$PRIVATE_KEY" ] && [ "$FORCE" = false ]; then
  echo "JWT dev key already exists: $PRIVATE_KEY (use --force to regenerate)"
  exit 0
fi

mkdir -p "$JWT_DIR"

# Generate a fresh RSA 2048 key pair
echo "Generating RSA 2048 key pair..."
openssl genrsa 2048 2>/dev/null | openssl pkcs8 -topk8 -nocrypt -out "$PRIVATE_KEY" 2>/dev/null
chmod 600 "$PRIVATE_KEY"

# Verify the key is valid
if ! openssl rsa -in "$PRIVATE_KEY" -check -noout 2>/dev/null; then
  echo "ERROR: Generated key failed validation"
  rm -f "$PRIVATE_KEY"
  exit 1
fi

# Extract public key
PUBLIC_KEY_FILE="$JWT_DIR/public.key"
openssl rsa -in "$PRIVATE_KEY" -pubout -out "$PUBLIC_KEY_FILE" 2>/dev/null

# Create escaped versions for .env (replace newlines with literal \n)
ESCAPED_PRIVATE=$(awk '{printf "%s\\n", $0}' "$PRIVATE_KEY" | sed 's/\\n$//')
ESCAPED_PUBLIC=$(awk '{printf "%s\\n", $0}' "$PUBLIC_KEY_FILE" | sed 's/\\n$//')

echo "JWT dev key generated: $PRIVATE_KEY"
echo "JWT public key generated: $PUBLIC_KEY_FILE"

# Auto-populate .env if JWT_PRIVATE_KEY is empty
ENV_FILE="$PROJECT_DIR/.env"
if [ -f "$ENV_FILE" ]; then
  CURRENT_PRIVATE=$(grep '^JWT_PRIVATE_KEY=' "$ENV_FILE" | cut -d= -f2- || true)
  if [ -z "$CURRENT_PRIVATE" ] || [ "$FORCE" = true ]; then
    if grep -q '^JWT_PRIVATE_KEY=' "$ENV_FILE"; then
      # Use | as sed delimiter since keys contain /
      sed -i.bak "s|^JWT_PRIVATE_KEY=.*|JWT_PRIVATE_KEY=$ESCAPED_PRIVATE|" "$ENV_FILE"
      rm -f "$ENV_FILE.bak"
    else
      echo "JWT_PRIVATE_KEY=$ESCAPED_PRIVATE" >> "$ENV_FILE"
    fi
    if grep -q '^JWT_PUBLIC_KEY=' "$ENV_FILE"; then
      sed -i.bak "s|^JWT_PUBLIC_KEY=.*|JWT_PUBLIC_KEY=$ESCAPED_PUBLIC|" "$ENV_FILE"
      rm -f "$ENV_FILE.bak"
    else
      echo "JWT_PUBLIC_KEY=$ESCAPED_PUBLIC" >> "$ENV_FILE"
    fi
    echo "Updated .env with new JWT keys"
  else
    echo "Skipping .env update (JWT_PRIVATE_KEY already set; use --force to overwrite)"
  fi
fi
