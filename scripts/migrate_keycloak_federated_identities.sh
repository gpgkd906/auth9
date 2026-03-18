#!/usr/bin/env bash
#
# Migrate Keycloak federated identities to Auth9 linked_identities table.
#
# This script reads federated identity records from Keycloak Admin API
# and inserts them into the Auth9 linked_identities table (idempotent).
#
# Prerequisites:
#   - Keycloak must be running and accessible
#   - MySQL/TiDB must be running and accessible
#   - jq and mysql CLI must be installed
#
# Usage:
#   KEYCLOAK_URL=http://localhost:8081 \
#   KEYCLOAK_REALM=auth9 \
#   KEYCLOAK_ADMIN_USER=admin \
#   KEYCLOAK_ADMIN_PASSWORD=admin \
#   MYSQL_HOST=127.0.0.1 \
#   MYSQL_PORT=4000 \
#   MYSQL_USER=root \
#   MYSQL_PASSWORD="" \
#   MYSQL_DATABASE=auth9 \
#   ./scripts/migrate_keycloak_federated_identities.sh

set -euo pipefail

# ── Configuration ──
KEYCLOAK_URL="${KEYCLOAK_URL:-http://localhost:8081}"
KEYCLOAK_REALM="${KEYCLOAK_REALM:-auth9}"
KEYCLOAK_ADMIN_USER="${KEYCLOAK_ADMIN_USER:-admin}"
KEYCLOAK_ADMIN_PASSWORD="${KEYCLOAK_ADMIN_PASSWORD:-admin}"

MYSQL_HOST="${MYSQL_HOST:-127.0.0.1}"
MYSQL_PORT="${MYSQL_PORT:-4000}"
MYSQL_USER="${MYSQL_USER:-root}"
MYSQL_PASSWORD="${MYSQL_PASSWORD:-}"
MYSQL_DATABASE="${MYSQL_DATABASE:-auth9}"

# ── Helpers ──
total=0
inserted=0
skipped=0
errors=0

mysql_cmd() {
  mysql -h "$MYSQL_HOST" -P "$MYSQL_PORT" -u "$MYSQL_USER" \
    ${MYSQL_PASSWORD:+-p"$MYSQL_PASSWORD"} \
    "$MYSQL_DATABASE" -N -B -e "$1"
}

log() { echo "[$(date '+%Y-%m-%d %H:%M:%S')] $*"; }

# ── Step 1: Authenticate to Keycloak ──
log "Authenticating to Keycloak at $KEYCLOAK_URL..."
KC_TOKEN=$(curl -sf -X POST \
  "$KEYCLOAK_URL/realms/master/protocol/openid-connect/token" \
  -d "grant_type=client_credentials" \
  -d "client_id=admin-cli" \
  -d "username=$KEYCLOAK_ADMIN_USER" \
  -d "password=$KEYCLOAK_ADMIN_PASSWORD" \
  -d "grant_type=password" \
  | jq -r '.access_token')

if [ -z "$KC_TOKEN" ] || [ "$KC_TOKEN" = "null" ]; then
  echo "ERROR: Failed to authenticate to Keycloak" >&2
  exit 1
fi
log "Authenticated successfully."

# ── Step 2: List all users ──
log "Fetching users from realm '$KEYCLOAK_REALM'..."
FIRST=0
MAX=100
ALL_USERS="[]"

while true; do
  BATCH=$(curl -sf \
    -H "Authorization: Bearer $KC_TOKEN" \
    "$KEYCLOAK_URL/admin/realms/$KEYCLOAK_REALM/users?first=$FIRST&max=$MAX")

  COUNT=$(echo "$BATCH" | jq 'length')
  if [ "$COUNT" -eq 0 ]; then
    break
  fi

  ALL_USERS=$(echo "$ALL_USERS $BATCH" | jq -s 'add')
  FIRST=$((FIRST + MAX))
done

USER_COUNT=$(echo "$ALL_USERS" | jq 'length')
log "Found $USER_COUNT users."

# ── Step 3: For each user, fetch and migrate federated identities ──
echo "$ALL_USERS" | jq -c '.[]' | while read -r USER_JSON; do
  KC_USER_ID=$(echo "$USER_JSON" | jq -r '.id')
  KC_EMAIL=$(echo "$USER_JSON" | jq -r '.email // empty')

  # Find matching Auth9 user by identity_subject (which equals KC user ID)
  AUTH9_USER_ID=$(mysql_cmd "SELECT id FROM users WHERE identity_subject = '$KC_USER_ID' LIMIT 1" 2>/dev/null || echo "")

  if [ -z "$AUTH9_USER_ID" ]; then
    # Try by email
    if [ -n "$KC_EMAIL" ]; then
      AUTH9_USER_ID=$(mysql_cmd "SELECT id FROM users WHERE email = '$KC_EMAIL' LIMIT 1" 2>/dev/null || echo "")
    fi
  fi

  if [ -z "$AUTH9_USER_ID" ]; then
    continue  # No matching Auth9 user
  fi

  # Fetch federated identities from Keycloak
  FED_IDS=$(curl -sf \
    -H "Authorization: Bearer $KC_TOKEN" \
    "$KEYCLOAK_URL/admin/realms/$KEYCLOAK_REALM/users/$KC_USER_ID/federated-identity" 2>/dev/null || echo "[]")

  FED_COUNT=$(echo "$FED_IDS" | jq 'length')
  if [ "$FED_COUNT" -eq 0 ]; then
    continue
  fi

  echo "$FED_IDS" | jq -c '.[]' | while read -r FED_JSON; do
    PROVIDER_ALIAS=$(echo "$FED_JSON" | jq -r '.identityProvider')
    EXTERNAL_USER_ID=$(echo "$FED_JSON" | jq -r '.userId')
    EXTERNAL_EMAIL=$(echo "$FED_JSON" | jq -r '.userName // empty')

    total=$((total + 1))

    # Determine provider_type from alias
    PROVIDER_TYPE="$PROVIDER_ALIAS"
    case "$PROVIDER_ALIAS" in
      google*) PROVIDER_TYPE="google" ;;
      github*) PROVIDER_TYPE="github" ;;
      microsoft*) PROVIDER_TYPE="microsoft" ;;
      facebook*) PROVIDER_TYPE="facebook" ;;
      *-saml*|*saml*) PROVIDER_TYPE="saml" ;;
      *-oidc*|*oidc*) PROVIDER_TYPE="oidc" ;;
    esac

    NEW_ID=$(uuidgen | tr '[:upper:]' '[:lower:]')

    # Insert with ON DUPLICATE KEY UPDATE (idempotent)
    SQL="INSERT INTO linked_identities (id, user_id, provider_type, provider_alias, external_user_id, external_email, linked_at)
         VALUES ('$NEW_ID', '$AUTH9_USER_ID', '$PROVIDER_TYPE', '$PROVIDER_ALIAS', '$EXTERNAL_USER_ID', $([ -n "$EXTERNAL_EMAIL" ] && echo "'$EXTERNAL_EMAIL'" || echo "NULL"), NOW())
         ON DUPLICATE KEY UPDATE external_email = VALUES(external_email)"

    if mysql_cmd "$SQL" 2>/dev/null; then
      inserted=$((inserted + 1))
    else
      errors=$((errors + 1))
      echo "  WARN: Failed to insert identity for user=$AUTH9_USER_ID provider=$PROVIDER_ALIAS" >&2
    fi
  done
done

# ── Step 4: Report ──
log "Migration complete."
log "  Total federated identities processed: $total"
log "  Inserted/updated: $inserted"
log "  Errors: $errors"
