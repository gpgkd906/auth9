# Auth9 Demo

A simple Express.js demo application that shows how to integrate Auth9 using the `@auth9/node` SDK.

## What This Demo Covers

| Scenario | SDK Feature | Endpoint |
|----------|------------|----------|
| Token Verification | `auth9Middleware` | `GET /api/me` |
| Role-based Access | `requireRole` | `GET /api/admin` |
| Permission-based Access | `requirePermission` | `GET /api/resources` |
| Token Exchange | `Auth9GrpcClient.exchangeToken` | `POST /api/exchange-token` |
| Token Introspection | `Auth9GrpcClient.introspectToken` | `POST /api/introspect` |
| Management API | `Auth9HttpClient` | `GET /api/tenants`, `GET /api/users` |

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
| `AUTH9_ADMIN_TOKEN` | *(empty)* | Admin token for Management API |

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
