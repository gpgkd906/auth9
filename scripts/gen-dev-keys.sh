#!/bin/bash
# Generate development JWT keys for local development and QA tools.
#
# Extracts the JWT private key from docker-compose.yml so that QA tools
# and the running services use the same key material.
#
# Output: deploy/dev-certs/jwt/private.key
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

# Extract the JWT_PRIVATE_KEY value from docker-compose.yml.
# The key is stored as an inline escaped PEM string with \n literal sequences.
# We use python3 (available on macOS and most Linux) to parse YAML reliably.
COMPOSE_FILE="$PROJECT_DIR/docker-compose.yml"
if [ ! -f "$COMPOSE_FILE" ]; then
  echo "ERROR: docker-compose.yml not found at $COMPOSE_FILE"
  exit 1
fi

# Extract using grep + sed: find JWT_PRIVATE_KEY line, unescape \n to real newlines
RAW_KEY=$(grep 'JWT_PRIVATE_KEY:' "$COMPOSE_FILE" | head -1 | sed 's/.*JWT_PRIVATE_KEY: *//' | sed 's/^"//' | sed 's/"$//')

if [ -z "$RAW_KEY" ]; then
  echo "ERROR: Could not extract JWT_PRIVATE_KEY from docker-compose.yml"
  exit 1
fi

# Convert escaped \n to real newlines
printf '%b' "$RAW_KEY" > "$PRIVATE_KEY"

chmod 600 "$PRIVATE_KEY"

# Verify the key is valid
if openssl rsa -in "$PRIVATE_KEY" -check -noout 2>/dev/null; then
  echo "JWT dev key generated: $PRIVATE_KEY"
else
  echo "ERROR: Generated key failed validation"
  rm -f "$PRIVATE_KEY"
  exit 1
fi
