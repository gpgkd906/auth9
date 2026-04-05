# QA API Testing Cookbook

Quick-reference recipes for common QA testing tasks. Avoids trial-and-error on known pitfalls.

---

## 1. gRPC Token Exchange Testing

### Key Facts

- **Service name**: `auth9.TokenExchange` (NOT `auth9.token_exchange.TokenExchange`)
- **Reflection is disabled**: Must provide `-import-path /proto -proto auth9.proto`
- **mTLS required on `auth9-grpc-tls:50051`**: Must provide `-cacert`, `-cert`, and `-key`
- **API key required**: Header `x-api-key: dev-grpc-api-key`
- **Host port 50051 is often blocked**: Use `grpcurl-docker.sh` helper or `docker run` inside the Docker network
- **`service_id` expects OAuth client_id**: e.g. `"auth9-portal"`, NOT a service UUID
- **Portal service belongs to `auth9-platform` tenant**: Token Exchange for `demo` tenant with `auth9-portal` will fail with "Service does not belong to the requested tenant"

### Working Recipe

```bash
TOKEN=$(.claude/skills/tools/gen-admin-token.sh)
PLATFORM_TENANT_ID=$(mysql -u root -h 127.0.0.1 -P 4000 auth9 -N -e \
  "SELECT id FROM tenants WHERE slug = 'auth9-platform';")

# Via grpcurl-docker.sh (preferred)
.claude/skills/tools/grpcurl-docker.sh \
  -cacert /certs/ca.crt \
  -cert /certs/client.crt \
  -key /certs/client.key \
  -import-path /proto -proto auth9.proto \
  -H "x-api-key: dev-grpc-api-key" \
  -d "{\"identity_token\": \"$TOKEN\", \"tenant_id\": \"$PLATFORM_TENANT_ID\", \"service_id\": \"auth9-portal\"}" \
  auth9-grpc-tls:50051 auth9.TokenExchange/ExchangeToken

# Via plain docker run (if helper not available)
docker run --rm \
  --network auth9_auth9-network \
  -v "$(pwd)/auth9-core/proto:/proto:ro" \
  -v "$(pwd)/deploy/dev-certs/grpc:/certs:ro" \
  fullstorydev/grpcurl:latest \
  -cacert /certs/ca.crt \
  -cert /certs/client.crt \
  -key /certs/client.key \
  -import-path /proto -proto auth9.proto \
  -H "x-api-key: dev-grpc-api-key" \
  -d "{\"identity_token\": \"$TOKEN\", \"tenant_id\": \"$PLATFORM_TENANT_ID\", \"service_id\": \"auth9-portal\"}" \
  auth9-grpc-tls:50051 auth9.TokenExchange/ExchangeToken
```

### Decode JWT to Verify Roles/Permissions

```bash
echo "$ACCESS_TOKEN" | cut -d. -f2 | (cat; echo '==') | base64 -d 2>/dev/null | python3 -m json.tool
```

Expected output for admin user:
```json
{
  "roles": ["admin"],
  "permissions": ["admin:full"],
  "tenant_id": "...",
  "email": "admin@auth9.local"
}
```

### Common Errors

| Error | Cause | Fix |
|-------|-------|-----|
| `server does not support the reflection API` | Missing proto file | Add `-import-path /proto -proto auth9.proto` |
| `Missing API key` | No auth header | Add `-H "x-api-key: dev-grpc-api-key"` |
| `Client not found for service_id ''` | Empty service_id | Add `"service_id": "auth9-portal"` |
| `Service does not belong to the requested tenant` | Wrong tenant for service | Use `auth9-platform` tenant for `auth9-portal` service |
| `context deadline exceeded` on localhost:50051 | Host port blocked | Use `grpcurl-docker.sh` or `docker run --network` |
| `x509: certificate signed by unknown authority` | Missing CA trust | Add `-cacert /certs/ca.crt` |
| `unexpected HTTP status code ... 400 (Bad Request)` | mTLS client cert missing | Add `-cert /certs/client.crt -key /certs/client.key` |
| `target server does not expose service "auth9.token_exchange.TokenExchange"` | Wrong service name | Use `auth9.TokenExchange` (not `auth9.token_exchange.TokenExchange`) |

---

## 2. Identity Webhook Event Testing

### Key Facts

- **Endpoint**: `POST http://localhost:8080/api/v1/identity/events`
- **Webhook secret**: `dev-webhook-secret-change-in-production` (from `docker-compose.yml` env `IDENTITY_WEBHOOK_SECRET`)
- **Signature header**: `x-webhook-signature: sha256=<hex>` (legacy `x-keycloak-signature` also accepted)
- **HMAC algorithm**: HMAC-SHA256
- **Field names in JSON**: camelCase (e.g. `credentialType`, `authMethod`, `ipAddress`)

### Working Recipe: Simulate MFA Failure Event

```bash
BODY='{"type":"LOGIN_ERROR","realmId":"auth9","userId":"00000000-0000-0000-0000-000000000001","error":"invalid_user_credentials","time":1704067200000,"details":{"username":"testuser","email":"testuser@example.com","credentialType":"otp"}}'
SECRET="dev-webhook-secret-change-in-production"  # pragma: allowlist secret
SIGNATURE=$(echo -n "$BODY" | openssl dgst -sha256 -hmac "$SECRET" | cut -d' ' -f2)

curl -s -w "\nHTTP: %{http_code}" -X POST "http://localhost:8080/api/v1/identity/events" \
  -H "Content-Type: application/json" \
  -H "x-webhook-signature: sha256=$SIGNATURE" \
  -d "$BODY"
```

### Working Recipe: Simulate Password Failure Event

```bash
BODY='{"type":"LOGIN_ERROR","realmId":"auth9","userId":"00000000-0000-0000-0000-000000000002","error":"invalid_user_credentials","time":1704067200000,"details":{"username":"testuser","email":"testuser@example.com"}}'
SECRET="dev-webhook-secret-change-in-production"  # pragma: allowlist secret
SIGNATURE=$(echo -n "$BODY" | openssl dgst -sha256 -hmac "$SECRET" | cut -d' ' -f2)

curl -s -w "\nHTTP: %{http_code}" -X POST "http://localhost:8080/api/v1/identity/events" \
  -H "Content-Type: application/json" \
  -H "x-webhook-signature: sha256=$SIGNATURE" \
  -d "$BODY"
```

### Working Recipe: Simulate Successful Login Event

```bash
BODY='{"type":"LOGIN","realmId":"auth9","userId":"00000000-0000-0000-0000-000000000003","time":1704067200000,"details":{"username":"testuser","email":"testuser@example.com"}}'
SECRET="dev-webhook-secret-change-in-production"  # pragma: allowlist secret
SIGNATURE=$(echo -n "$BODY" | openssl dgst -sha256 -hmac "$SECRET" | cut -d' ' -f2)

curl -s -w "\nHTTP: %{http_code}" -X POST "http://localhost:8080/api/v1/identity/events" \
  -H "Content-Type: application/json" \
  -H "x-webhook-signature: sha256=$SIGNATURE" \
  -d "$BODY"
```

### Verify in Database

```bash
mysql -u root -h 127.0.0.1 -P 4000 auth9 -e "
SELECT email, event_type, failure_reason FROM login_events ORDER BY created_at DESC LIMIT 5;"
```

### Event Type Mapping Reference

| Identity event | error field | details | Auth9 event_type |
|---------------|-------------|---------|------------------|
| `LOGIN` | - | - | `success` |
| `LOGIN_ERROR` | `invalid_user_credentials` | (none) | `failed_password` |
| `LOGIN_ERROR` | `invalid_user_credentials` | `credentialType: "otp"` | `failed_mfa` |
| `LOGIN_ERROR` | `invalid_user_credentials` | `authMethod: "otp"` | `failed_mfa` |
| `LOGIN_ERROR` | `invalid_totp` | - | `failed_mfa` |
| `LOGIN_ERROR` | `user_disabled` | - | `locked` |
| `LOGIN_WITH_OTP` | - | - | `success` |
| `LOGIN_WITH_OTP_ERROR` | - | - | `failed_mfa` |
| `IDENTITY_PROVIDER_LOGIN` | - | - | `social` |

---

---

## 4. RBAC Seed Data Verification

### Check Roles and Permissions

```bash
mysql -u root -h 127.0.0.1 -P 4000 auth9 -e "
SELECT r.name AS role_name, p.code AS perm_code
FROM roles r
JOIN role_permissions rp ON r.id = rp.role_id
JOIN permissions p ON rp.permission_id = p.id;"
```

### Check User Role Assignments

```bash
mysql -u root -h 127.0.0.1 -P 4000 auth9 -e "
SELECT u.email, t.slug AS tenant, r.name AS role_name
FROM user_tenant_roles utr
JOIN tenant_users tu ON utr.tenant_user_id = tu.id
JOIN users u ON tu.user_id = u.id
JOIN tenants t ON tu.tenant_id = t.id
JOIN roles r ON utr.role_id = r.id;"
```

### Service-Tenant Mapping

```bash
mysql -u root -h 127.0.0.1 -P 4000 auth9 -e "
SELECT s.name, t.slug AS owner_tenant
FROM services s
LEFT JOIN tenants t ON s.tenant_id = t.id;"
```

Expected: `Auth9 Admin Portal` owned by `auth9-platform` tenant.
