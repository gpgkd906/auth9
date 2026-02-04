#!/bin/bash
set -e

# 1. Get Master Token
echo "Getting Master Token..."
MASTER_TOKEN=$(curl -s -d "client_id=admin-cli" -d "username=admin" -d "password=admin" -d "grant_type=password" "http://localhost:8081/realms/master/protocol/openid-connect/token" | jq -r .access_token)

if [ -z "$MASTER_TOKEN" ] || [ "$MASTER_TOKEN" == "null" ]; then
    echo "Failed to get master token"
    exit 1
fi

# 2. Check/Create Test User
echo "Creating testuser..."
# Create user (POST returns 201 or 409 if exists). We ignore 409.
curl -s -o /dev/null -X POST -H "Authorization: Bearer $MASTER_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"username": "testuser", "enabled": true, "email": "test@example.com", "firstName": "Test", "lastName": "User"}' \
  "http://localhost:8081/admin/realms/auth9/users"

# Get User ID
USER_ID=$(curl -s -H "Authorization: Bearer $MASTER_TOKEN" "http://localhost:8081/admin/realms/auth9/users?username=testuser" | jq -r '.[0].id')
echo "User ID: $USER_ID"

# Reset Password
echo "Setting password..."
curl -s -o /dev/null -X PUT -H "Authorization: Bearer $MASTER_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"type": "password", "value": "Test123!", "temporary": false}' \
  "http://localhost:8081/admin/realms/auth9/users/$USER_ID/reset-password"

# 3. Get Tokens
CLIENT_ID="auth9-portal"
CLIENT_SECRET="38V3Qd3y80aweTpmNASfsTWTegE5sbzI"

echo "Getting testuser token..."
USER_TOKEN=$(curl -s -d "client_id=$CLIENT_ID" --data-urlencode "client_secret=$CLIENT_SECRET" -d "username=testuser" -d "password=Test123!" -d "grant_type=password" "http://localhost:8081/realms/auth9/protocol/openid-connect/token" | jq -r .access_token)

echo "Getting admin token..."
ADMIN_TOKEN=$(curl -s -d "client_id=$CLIENT_ID" --data-urlencode "client_secret=$CLIENT_SECRET" -d "username=admin" -d "password=Admin123!" -d "grant_type=password" "http://localhost:8081/realms/auth9/protocol/openid-connect/token" | jq -r .access_token)

# Save tokens
echo "$USER_TOKEN" > /tmp/user_token
echo "$ADMIN_TOKEN" > /tmp/admin_token

echo "Tokens saved to /tmp/user_token and /tmp/admin_token"
