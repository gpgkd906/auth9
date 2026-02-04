#!/bin/bash

# Configuration
API_URL="http://localhost:8080"
GRPC_HOST="localhost:50051"
PORTAL_URL="http://localhost:3000"

USER_TOKEN=$(cat /tmp/user_token 2>/dev/null)
ADMIN_TOKEN=$(cat /tmp/admin_token 2>/dev/null)

echo "=== Loading Tokens ==="
if [ -z "$USER_TOKEN" ]; then
    echo "ERROR: User token not found. Please run setup_tokens.sh first."
    exit 1
fi

echo "User Token found."

# ==========================================
# 1. gRPC Security Test (02-grpc-api.md)
# ==========================================
echo -e "\n=== 1. gRPC Security Test ==="

# 1.1 List Services (Unauthenticated)
echo "--- 1.1 List Services (Unauthenticated) ---"
grpcurl -plaintext $GRPC_HOST list
if [ $? -eq 0 ]; then
    echo "[!] WARN: gRPC reflection is enabled and accessible without auth."
else
    echo "[+] PASS: gRPC reflection is restricted."
fi

# 1.2 Call ExchangeToken (Unauthenticated/Invalid)
echo "--- 1.2 Call ExchangeToken (Unauthenticated) ---"
# We expect this to fail with UNAUTHENTICATED if secured.
OUTPUT=$(grpcurl -plaintext -d '{"identity_token":"dummy"}' $GRPC_HOST auth9.TokenExchange/ExchangeToken 2>&1)
echo "Output: $OUTPUT"
if [[ "$OUTPUT" == *"UNAUTHENTICATED"* ]]; then
    echo "[+] PASS: ExchangeToken requires authentication."
else
    echo "[!] FAIL: ExchangeToken accessed without authentication (or different error returned)."
fi

# ==========================================
# 2. Rate Limiting Test (03-rate-limiting.md)
# ==========================================
echo -e "\n=== 2. Rate Limiting Test ==="
echo "--- 2.1 Rapid Request Test (30 requests) ---"

RATE_LIMIT_HIT=0
echo "Sending 30 requests to /api/v1/health..."
for i in {1..30}; do
    STATUS=$(curl -s -o /dev/null -w "%{http_code}" "$API_URL/health")
    if [ "$STATUS" == "429" ]; then
         RATE_LIMIT_HIT=1
         echo -n "429 "
    else
         echo -n "$STATUS "
    fi
done
echo ""

if [ $RATE_LIMIT_HIT -eq 1 ]; then
    echo "[+] PASS: Rate limiting triggered (429 received)."
else
    echo "[-] INFO: Rate limiting validation - No 429 received in 30 requests. Limit might be higher."
fi

echo "--- 2.2 Check Rate Limit Headers ---"
curl -I -s "$API_URL/health" | grep -i "X-RateLimit" || echo "No X-RateLimit headers found."

# ==========================================
# 3. CORS & Headers Test (04-cors-headers.md)
# ==========================================
echo -e "\n=== 3. CORS & Headers Test ==="

# 3.1 CORS Preflight
echo "--- 3.1 CORS Preflight (Valid Origin: $PORTAL_URL) ---"
OUTPUT_VALID=$(curl -I -s -X OPTIONS "$API_URL/api/v1/users" \
  -H "Origin: $PORTAL_URL" \
  -H "Access-Control-Request-Method: GET")

if echo "$OUTPUT_VALID" | grep -qi "Access-Control-Allow-Origin: $PORTAL_URL" || echo "$OUTPUT_VALID" | grep -q "access-control-allow-origin: *"; then
    echo "[+] PASS: Valid Origin allowed (or *)."
else
    echo "[!] FAIL: Valid Origin NOT allowed."
    echo "$OUTPUT_VALID"
fi

echo "--- 3.2 CORS Preflight (Invalid Origin: http://evil.com) ---"
OUTPUT_INVALID=$(curl -I -s -X OPTIONS "$API_URL/api/v1/users" \
  -H "Origin: http://evil.com" \
  -H "Access-Control-Request-Method: GET")

if echo "$OUTPUT_INVALID" | grep -qi "Access-Control-Allow-Origin"; then
    echo "[!] FAIL: Invalid Origin allowed!"
    echo "$OUTPUT_INVALID" | grep -i "Access-Control-Allow-Origin"
else
    echo "[+] PASS: Invalid Origin rejected (no ACAO header)."
fi

# 3.3 Security Headers
echo "--- 3.3 Security Headers Check ---"
HEADERS=$(curl -I -s "$API_URL/health")
echo "$HEADERS"
echo "Checking for recommended headers..."

REQUIRED_HEADERS=("X-Content-Type-Options" "X-Frame-Options")
for header in "${REQUIRED_HEADERS[@]}"; do
    if echo "$HEADERS" | grep -qi "$header"; then
        echo "[+] PASS: $header present."
    else
        echo "[!] FAIL: $header MISSING."
    fi
done
