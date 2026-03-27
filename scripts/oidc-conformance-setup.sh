#!/usr/bin/env bash
# OIDC Conformance Suite - Client Setup Script
# Creates a dedicated OIDC test client in Auth9 for Conformance Suite testing.
#
# Usage: ./scripts/oidc-conformance-setup.sh
# Prerequisites: ./scripts/reset-docker.sh --conformance

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

API_BASE="${API_BASE:-http://localhost:8080}"
CONFORMANCE_CALLBACK="https://localhost:9443/test/a/auth9-oidc-test/callback"

# Colors
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[0;33m'
NC='\033[0m'

echo "OIDC Conformance Suite - Client Setup"
echo "======================================"
echo ""

# Step 1: Wait for auth9-core to be healthy
echo -n "Waiting for auth9-core..."
for i in $(seq 1 30); do
    if curl -sf "${API_BASE}/health" > /dev/null 2>&1; then
        echo -e " ${GREEN}OK${NC}"
        break
    fi
    if [[ $i -eq 30 ]]; then
        echo -e " ${RED}FAILED${NC}"
        echo "auth9-core is not responding at ${API_BASE}/health"
        echo "Run ./scripts/reset-docker.sh --conformance first."
        exit 1
    fi
    sleep 2
done

# Step 2: Wait for Conformance Suite
echo -n "Waiting for Conformance Suite..."
for i in $(seq 1 30); do
    if curl -skf "https://localhost:9443" > /dev/null 2>&1; then
        echo -e " ${GREEN}OK${NC}"
        break
    fi
    if [[ $i -eq 30 ]]; then
        echo -e " ${YELLOW}NOT AVAILABLE${NC} (continuing without — Suite may still be starting)"
    fi
    sleep 2
done

# Step 3: Generate admin tenant access token
echo -n "Generating admin token..."
# Use known seed admin user ID
ADMIN_USER_ID="${ADMIN_USER_ID:-746ceba8-3ddf-4a8b-b021-a1337b7a1a35}"
TOKEN=$(node "$PROJECT_ROOT/.claude/skills/tools/gen_tenant_access_token.js" \
    "$ADMIN_USER_ID" "" "admin" "rbac:*,user:*,service:*,action:*,tenant:*" "admin@auth9.local" 2>/dev/null)
if [[ -z "$TOKEN" ]]; then
    echo -e " ${RED}FAILED${NC}"
    echo "Could not generate admin token. Check JWT private key and node/jsonwebtoken."
    exit 1
fi
echo -e " ${GREEN}OK${NC}"

# Step 4: Check if OIDC conformance client already exists
echo -n "Checking for existing conformance client..."
existing=$(curl -s -H "Authorization: Bearer $TOKEN" "${API_BASE}/api/v1/services" 2>/dev/null || echo "{}")
if echo "$existing" | jq -e '.data[]? | select(.name == "OIDC Conformance Test")' > /dev/null 2>&1; then
    service_id=$(echo "$existing" | jq -r '.data[] | select(.name == "OIDC Conformance Test") | .id')
    echo -e " ${YELLOW}EXISTS${NC} (service_id: $service_id)"

    # Fetch integration info for client details
    integration=$(curl -s -H "Authorization: Bearer $TOKEN" "${API_BASE}/api/v1/services/${service_id}/integration" 2>/dev/null || echo "{}")
    client_id=$(echo "$integration" | jq -r '.data.clients[0].client_id // empty')
    client_secret=$(echo "$integration" | jq -r '.data.clients[0].client_secret // "(hashed - regenerate if needed)"')

    echo ""
    echo -e "${GREEN}OIDC Conformance Client${NC}"
    echo "  Service ID:    $service_id"
    echo "  Client ID:     ${client_id:-unknown}"
    echo "  Client Secret: $client_secret"
    echo "  Redirect URI:  $CONFORMANCE_CALLBACK"
    exit 0
fi
echo -e " ${GREEN}not found, creating...${NC}"

# Step 5: Create OIDC conformance service + client
echo -n "Creating OIDC Conformance Test service..."
create_resp=$(curl -s -w '\n%{http_code}' \
    -X POST "${API_BASE}/api/v1/services" \
    -H "Authorization: Bearer $TOKEN" \
    -H "Content-Type: application/json" \
    -d "{
        \"name\": \"OIDC Conformance Test\",
        \"redirect_uris\": [\"${CONFORMANCE_CALLBACK}\", \"https://localhost:9443/test/a/auth9-oidc-test/post\"]
    }")
status=$(echo "$create_resp" | tail -1)
body=$(echo "$create_resp" | sed '$d')

if [[ "$status" != "200" && "$status" != "201" ]]; then
    echo -e " ${RED}FAILED${NC} (HTTP $status)"
    echo "$body" | jq . 2>/dev/null || echo "$body"
    exit 1
fi
echo -e " ${GREEN}OK${NC}"

service_id=$(echo "$body" | jq -r '.data.id // .id // empty')
if [[ -z "$service_id" ]]; then
    echo -e "${RED}Could not extract service_id from response${NC}"
    echo "$body" | jq . 2>/dev/null || echo "$body"
    exit 1
fi

# Step 6: Fetch integration info (includes client_id and client_secret)
echo -n "Fetching client credentials..."
integration=$(curl -s -H "Authorization: Bearer $TOKEN" "${API_BASE}/api/v1/services/${service_id}/integration" 2>/dev/null || echo "{}")
client_id=$(echo "$integration" | jq -r '.data.clients[0].client_id // empty')
client_secret=$(echo "$integration" | jq -r '.data.clients[0].client_secret // empty')
echo -e " ${GREEN}OK${NC}"

echo ""
echo "======================================"
echo -e "${GREEN}OIDC Conformance Client Created${NC}"
echo "======================================"
echo ""
echo "  Service ID:    $service_id"
echo "  Client ID:     $client_id"
echo "  Client Secret: $client_secret"
echo "  Redirect URIs:"
echo "    - $CONFORMANCE_CALLBACK"
echo "    - https://localhost:9443/test/a/auth9-oidc-test/post"
echo ""
echo "Discovery URL (for Conformance Suite):"
echo "  http://auth9-core:8080/.well-known/openid-configuration"
echo ""
echo "Conformance Suite UI:"
echo "  https://localhost:9443"
echo ""
echo -e "${YELLOW}Next steps:${NC}"
echo "  1. Open https://localhost:9443 in browser"
echo "  2. Create a new test plan (e.g., 'oidcc-basic-certification-test-plan')"
echo "  3. Enter the discovery URL and client credentials above"
echo "  4. Run the test plan"
