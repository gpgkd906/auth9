# Auth9 Demo

A simple Express.js demo application that shows how to integrate Auth9 using the `@auth9/node` SDK.

## What This Demo Covers

| Scenario | SDK Feature | Endpoint |
|----------|------------|----------|
| Token Verification | `auth9Middleware` | `GET /api/me` |
| Role-based Access | `requireRole` | `GET /api/admin` |
| Permission-based Access | `requirePermission` | `GET /api/resources` |
| Token Exchange | `Auth9GrpcClient.exchangeToken` | `POST /demo/exchange-token` |
| Token Introspection | `Auth9GrpcClient.introspectToken` | `POST /demo/introspect` |
| Management API | `Auth9HttpClient` | `GET /demo/tenants`, `GET /demo/users` |
| Enterprise SSO Discovery | Discovery + authorize redirect | `POST /enterprise/login`, `POST /demo/enterprise/discovery` |
| Enterprise Connector QA | Tenant-level SSO connector APIs | `GET/POST/PUT/DELETE /demo/enterprise/connectors*` |

## Run with Docker Compose

```bash
# Start the full Auth9 environment (includes this demo)
docker-compose up -d

# View demo logs
docker-compose logs -f auth9-demo

# Open in browser
open http://localhost:3002
```

## Run Locally (Development)

```bash
# Prerequisites: Auth9 dependencies running (docker-compose -f docker-compose.yml -f docker-compose.dev.yml up -d)

cd auth9-demo
npm install
npm run dev
```

## Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `PORT` | `3002` | Server port |
| `AUTH9_DOMAIN` | `http://localhost:8080` | Auth9 Core URL |
| `AUTH9_GRPC_ADDRESS` | `localhost:50051` | Auth9 gRPC address |
| `AUTH9_GRPC_API_KEY` | `dev-grpc-api-key` | gRPC API key |
| `AUTH9_AUDIENCE` | `demo-service` | Expected JWT audience |
| `AUTH9_ADMIN_TOKEN` | *(empty)* | Optional default admin token for enterprise connector APIs |
| `AUTH9_DEFAULT_TENANT_ID` | `demo` | Default tenant used by enterprise SSO demo APIs |

## SDK Packages

- **`@auth9/node`** — Main SDK entry point (`Auth9` class, `TokenVerifier`, `Auth9GrpcClient`)
- **`@auth9/node/middleware/express`** — Express middleware (`auth9Middleware`, `requireRole`, `requirePermission`)
- **`@auth9/core`** — HTTP client & types (`Auth9HttpClient`, `Tenant`, `User`, `Role`, etc.)

## Auth Flow

```
User Login (Keycloak) → Identity Token
        ↓
Token Exchange (gRPC) → Tenant Access Token (roles + permissions)
        ↓
Access Protected API  → auth9Middleware verifies token
        ↓
RBAC Check            → requireRole / requirePermission
```

## Enterprise SSO Test Flow

1. Open `http://localhost:3002`.
2. Use **Login with Enterprise SSO** and input enterprise email (for example `qa-user@corp.example.com`).
3. Demo calls `/enterprise/login` -> Auth9 Core `/api/v1/enterprise-sso/discovery` and redirects to the matched IdP.
4. After callback, open Dashboard and use **Enterprise SSO QA Panel** for connector CRUD/test/discovery.

## Enterprise SSO QA API Examples

```bash
# 1) Domain discovery
curl -X POST http://localhost:3002/demo/enterprise/discovery \
  -H "Content-Type: application/json" \
  -d '{"email":"qa-user@corp.example.com"}'

# 2) List connectors (pass admin token by header if env not set)
curl "http://localhost:3002/demo/enterprise/connectors?tenantId=demo" \
  -H "x-admin-token: $ADMIN_TOKEN"

# 3) Create SAML connector (requires AUTH9_ADMIN_TOKEN in demo env)
curl -X POST http://localhost:3002/demo/enterprise/connectors \
  -H "Content-Type: application/json" \
  -H "x-admin-token: $ADMIN_TOKEN" \
  -d '{
    "tenantId":"demo",
    "alias":"corp-saml",
    "provider_type":"saml",
    "enabled":true,
    "priority":100,
    "domains":["corp.example.com"],
    "keycloak_alias":"corp-saml",
    "config":{
      "entityId":"urn:demo:corp-saml",
      "singleSignOnServiceUrl":"https://idp.example.com/corp-saml/sso",
      "signingCertificate":"-----BEGIN CERTIFICATE-----\nMIID...demo...\n-----END CERTIFICATE-----"
    }
  }'

# 4) Test connector
curl -X POST http://localhost:3002/demo/enterprise/connectors/{connector_id}/test \
  -H "Content-Type: application/json" \
  -H "x-admin-token: $ADMIN_TOKEN" \
  -d '{"tenantId":"demo"}'

# 5) Delete connector
curl -X DELETE "http://localhost:3002/demo/enterprise/connectors/{connector_id}?tenantId=demo" \
  -H "x-admin-token: $ADMIN_TOKEN"
```
