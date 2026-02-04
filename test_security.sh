#!/bin/bash

BASE_URL="http://localhost:8080"
USER_TOKEN=$(cat /tmp/user_token)
ADMIN_TOKEN=$(cat /tmp/admin_token)

echo "=== Scenario 1: Unauthenticated Endpoint Access ==="
ENDPOINTS=(
  "/api/v1/tenants"
  "/api/v1/users"
  "/api/v1/services"
  "/api/v1/roles"
  "/api/v1/audit-logs"
  "/api/v1/system/email"
)

for endpoint in "${ENDPOINTS[@]}"; do
  CODE=$(curl -s -o /dev/null -w "%{http_code}" "$BASE_URL$endpoint")
  if [ "$CODE" == "401" ]; then
    echo "PASS: $endpoint -> 401"
  else
    echo "FAIL: $endpoint -> $CODE (Expected 401)"
  fi
done

echo "--- Public Endpoints ---"
PUBLIC_ENDPOINTS=(
  "/health"
  "/.well-known/openid-configuration"
)
for endpoint in "${PUBLIC_ENDPOINTS[@]}"; do
  CODE=$(curl -s -o /dev/null -w "%{http_code}" "$BASE_URL$endpoint")
  if [ "$CODE" == "200" ]; then
    echo "PASS: $endpoint -> 200"
  else
    echo "FAIL: $endpoint -> $CODE (Expected 200)"
  fi
done


echo -e "\n=== Scenario 2: Token Validation Bypass ==="
# Invalid format
CODE=$(curl -s -o /dev/null -w "%{http_code}" -H "Authorization: Bearer not.a.jwt" "$BASE_URL/api/v1/users")
if [ "$CODE" == "401" ]; then
    echo "PASS: Invalid Token Format -> 401"
else
    echo "FAIL: Invalid Token Format -> $CODE"
fi

# Query Param (should be rejected)
CODE=$(curl -s -o /dev/null -w "%{http_code}" "$BASE_URL/api/v1/users?access_token=$ADMIN_TOKEN")
if [ "$CODE" == "401" ]; then
    echo "PASS: Query Param Token -> 401"
else
    echo "FAIL: Query Param Token -> $CODE (Should be 401)"
fi

# Basic Auth
CODE=$(curl -s -o /dev/null -w "%{http_code}" -u "admin:password" "$BASE_URL/api/v1/users")
if [ "$CODE" == "401" ]; then
    echo "PASS: Basic Auth -> 401"
else
    echo "FAIL: Basic Auth -> $CODE"
fi


echo -e "\n=== Scenario 3: API Version & Deprecated ==="
# Old version
CODE=$(curl -s -o /dev/null -w "%{http_code}" "$BASE_URL/api/v0/users")
if [ "$CODE" == "404" ]; then
    echo "PASS: /api/v0/users -> 404"
else
    echo "FAIL: /api/v0/users -> $CODE"
fi

# Internal
CODE=$(curl -s -o /dev/null -w "%{http_code}" "$BASE_URL/api/v1/internal/config")
if [ "$CODE" == "404" ]; then
    echo "PASS: Internal Config -> 404"
else
    echo "FAIL: Internal Config -> $CODE"
fi


echo -e "\n=== Scenario 4: Bulk Data Extraction ==="
# Limit check
# We need to assume some data exists or at least the endpoint works.
# /api/v1/users might list users.
COUNT=$(curl -s -H "Authorization: Bearer $ADMIN_TOKEN" "$BASE_URL/api/v1/users?limit=1000" | jq '.data | length')
if [ "$COUNT" == "null" ] || [ -z "$COUNT" ]; then
     # Maybe format is different or error
     echo "WARN: Could not parse response for limit check"
     curl -s -H "Authorization: Bearer $ADMIN_TOKEN" "$BASE_URL/api/v1/users?limit=1000" | head -n 5
else
    if [ "$COUNT" -le 100 ]; then
        echo "PASS: Limit enforced (Count: $COUNT)"
    else
        echo "FAIL: Limit ignored (Count: $COUNT)"
    fi
fi


echo -e "\n=== Scenario 5: Sensitive Endpoint Protection ==="
# System configuration access
# User
CODE=$(curl -s -o /dev/null -w "%{http_code}" -H "Authorization: Bearer $USER_TOKEN" "$BASE_URL/api/v1/system/email")
if [ "$CODE" == "403" ] || [ "$CODE" == "401" ]; then
    echo "PASS: User access to system config -> $CODE (Expected 403)"
else
    echo "FAIL: User access to system config -> $CODE"
fi

# Admin
CODE=$(curl -s -o /dev/null -w "%{http_code}" -H "Authorization: Bearer $ADMIN_TOKEN" "$BASE_URL/api/v1/system/email")
# Expected 200 or 404 if not implemented, but definitely not 403/401 for admin if it exists
if [ "$CODE" == "200" ]; then
    echo "PASS: Admin access to system config -> 200"
else
    echo "WARN: Admin access to system config -> $CODE (Maybe endpoint doesn't exist?)"
fi
