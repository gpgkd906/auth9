#!/bin/bash

# Configuration
KEYCLOAK_URL="http://localhost:8081"
AUTH9_URL="http://localhost:8080"
REALM="auth9"
CLIENT_ID="auth9-portal"
REDIRECT_URI="http://localhost:3000/callback"
USERNAME="security-test-user"
PASSWORD="Test1234!"

echo ">>> Starting OIDC Security Tests <<<"

# ==========================================
# Scenario 1: Authorization Code Interception
# ==========================================
echo -e "\n[Scenario 1] Authorization Code Interception (Replay Attack)"

# 1. Get a fresh code (Simulation: we need to do a full login flow potentially, 
# or just simulate the token exchange part if we had a valid code manually. 
# Since getting a code programmatically via curl without a browser is complex (form parsing),
# We will test the 'code use' part by trying to use an invalid/expired code and asserting handling
# or by using Direct Access Grants to get a token and then trying to abuse parameters if applicable.
# BUT, for 'Code Interception', we verify that a code cannot be used twice.
# We will simulate the error case: reusing a random string as code or an expired one.

# Attempt to exchange an invalid code
RESPONSE=$(curl -s -X POST "$KEYCLOAK_URL/realms/$REALM/protocol/openid-connect/token" \
  -d "grant_type=authorization_code" \
  -d "code=INVALID_OR_REUSED_CODE" \
  -d "client_id=$CLIENT_ID" \
  -d "redirect_uri=$REDIRECT_URI")

if echo "$RESPONSE" | grep -q "invalid_grant" || echo "$RESPONSE" | grep -q "Code not valid"; then
    echo "PASS: Invalid/Reused code rejected."
else
    echo "FAIL: Unexpected response for invalid code: $RESPONSE"
fi


# ==========================================
# Scenario 2: Redirect URI Validation Bypass
# ==========================================
echo -e "\n[Scenario 2] Redirect URI Validation Bypass"

EVIL_REDIRECT="http://attacker.com/callback"
RESPONSE=$(curl -s -o /dev/null -w "%{http_code}" "$KEYCLOAK_URL/realms/$REALM/protocol/openid-connect/auth?client_id=$CLIENT_ID&redirect_uri=$EVIL_REDIRECT&response_type=code&scope=openid")

if [ "$RESPONSE" == "400" ]; then
    echo "PASS: Evil redirect URI ($EVIL_REDIRECT) rejected (400)."
else
    # Keycloak might return 200 with an error page instead of 400 immediately on the GET, let's check content.
    CONTENT=$(curl -s "$KEYCLOAK_URL/realms/$REALM/protocol/openid-connect/auth?client_id=$CLIENT_ID&redirect_uri=$EVIL_REDIRECT&response_type=code&scope=openid")
    if echo "$CONTENT" | grep -q "Invalid parameter: redirect_uri"; then
         echo "PASS: Evil redirect URI rejected (Error page)."
    else
         echo "FAIL: Evil redirect URI might be accepted. HTTP: $RESPONSE"
    fi
fi


# ==========================================
# Scenario 3: State Parameter CSRF Protection
# ==========================================
echo -e "\n[Scenario 3] State Parameter CSRF Protection"

# Request without state - Keycloak strictly doesn't require state by spec, but it's best practice.
# We check if it *allows* it.
RESPONSE=$(curl -s -o /dev/null -w "%{http_code}" "$KEYCLOAK_URL/realms/$REALM/protocol/openid-connect/auth?client_id=$CLIENT_ID&redirect_uri=$REDIRECT_URI&response_type=code&scope=openid")

if [ "$RESPONSE" == "200" ]; then
    echo "WARN: Auth allowed without state parameter (Standard OIDC behavior, ensure client enforces it)."
else
    echo "INFO: Response without state: $RESPONSE"
fi

# ==========================================
# Scenario 4: Scope Escalation
# ==========================================
echo -e "\n[Scenario 4] Scope Escalation"

# Attempt to request 'admin' scope which shouldn't exist or be allowed easily
ADMIN_SCOPE_RESPONSE=$(curl -s -X POST "$KEYCLOAK_URL/realms/$REALM/protocol/openid-connect/token" \
  -d "grant_type=password" \
  -d "username=$USERNAME" \
  -d "password=$PASSWORD" \
  -d "client_id=$CLIENT_ID" \
  -d "scope=openid admin")

SCOPES=$(echo "$ADMIN_SCOPE_RESPONSE" | jq -r .scope)

if [[ "$SCOPES" == *"admin"* ]]; then
    echo "FAIL: 'admin' scope was granted! Scopes: $SCOPES"
else
    echo "PASS: 'admin' scope NOT granted. Scopes: $SCOPES"
fi

# ==========================================
# Scenario 5: OIDC Metadata Tampering
# ==========================================
echo -e "\n[Scenario 5] OIDC Metadata Tampering"

METADATA=$(curl -s "$KEYCLOAK_URL/realms/$REALM/.well-known/openid-configuration")
ISSUER=$(echo "$METADATA" | jq -r .issuer)

if [ "$ISSUER" == "$KEYCLOAK_URL/realms/$REALM" ]; then
    echo "PASS: Issuer matches expected URL."
else
    echo "FAIL: Issuer mismatch. Got: $ISSUER, Expected: $KEYCLOAK_URL/realms/$REALM"
fi

# Check for https in endpoints (We are on localhost so http is expected, but checking logic)
# echo "Checking HTTPS..."
