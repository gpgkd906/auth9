# Auth9 - Identity & RBAC Powerhouse

A self-hosted identity and access management service, designed to replace expensive solutions like Auth0.

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                        Client Layer                              │
├─────────────────┬─────────────────────┬─────────────────────────┤
│  auth9-portal   │  Business Services  │      auth9-sdk          │
│ (React Router 7)│                     │      (Optional)         │
└────────┬────────┴──────────┬──────────┴────────────┬────────────┘
         │ REST API          │ gRPC                   │ gRPC
         ▼                   ▼                        ▼
┌─────────────────────────────────────────────────────────────────┐
│                       auth9-core (Rust)                          │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐           │
│  │  REST API    │  │ gRPC Server  │  │  JWT Engine  │           │
│  └──────────────┘  └──────────────┘  └──────────────┘           │
└────────┬────────────────────┬────────────────────────────────────┘
         │                    │
    ┌────┴────┐          ┌────┴────┐
    │  TiDB   │          │  Redis  │
    │ (MySQL) │          │ (Cache) │
    └─────────┘          └─────────┘
```

## Components

| Component | Technology | Description |
|-----------|------------|-------------|
| **auth9-core** | Rust (axum, tonic, sqlx) | Backend API & gRPC services |
| **auth9-portal** | React Router 7 + TypeScript + Vite | Admin dashboard UI |
| **Database** | TiDB (MySQL compatible) | Tenant, user, RBAC data |
| **Cache** | Redis | Session, token caching |
| **Auth Engine** | Keycloak | OIDC provider (optional) |

## Features

- **Multi-tenant**: Isolated tenants with custom settings
- **SSO**: Single Sign-On via OIDC
- **Dynamic RBAC**: Roles, permissions, inheritance
- **Token Exchange**: Service-to-service authentication
- **Audit Logs**: Track all administrative actions
- **Modern UI**: React Router 7-based design system
- **Action Engine**: Event-driven automation workflows with JavaScript/TypeScript
- **TypeScript SDK**: Official SDK for seamless integration
- **Invitation System**: Email-based user onboarding with automated workflows
- **Brand Customization**: Custom logos, colors, themes for tenant branding
- **Email Templates**: Flexible email template system with multi-language support
- **Password Management**: Password policies, reset, and change
- **Session Management**: View and revoke active sessions
- **WebAuthn/Passkey**: Passwordless authentication
- **Social Login**: Google, GitHub, OIDC, SAML support
- **Security Alerts**: Real-time threat detection
- **Login Analytics**: Detailed login statistics and events
- **Webhooks**: Real-time event notifications

## Quick Start

### Local Development

```bash
# Start dependencies (TiDB, Redis, Keycloak)
docker-compose -f docker-compose.yml -f docker-compose.dev.yml up -d

# Run auth9-core
cd auth9-core
cp .env.example .env
cargo run

# Run auth9-portal
cd auth9-portal
cp .env.example .env
npm install
npm run dev
```

### Full Stack with Docker

```bash
docker-compose up -d
```

- Portal: http://localhost:3000
- API: http://localhost:8080
- Keycloak: http://localhost:8081

## API Endpoints

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/api/v1/tenants` | GET, POST | List/create tenants |
| `/api/v1/users` | GET, POST | List/create users |
| `/api/v1/services` | GET, POST | List/register services |
| `/api/v1/roles` | GET, POST | List/create roles |
| `/api/v1/rbac/assign` | POST | Assign roles to users |
| `/api/v1/audit-logs` | GET | Query audit logs |

## gRPC Services

```protobuf
service TokenExchange {
  rpc ExchangeToken(ExchangeTokenRequest) returns (ExchangeTokenResponse);
  rpc ValidateToken(ValidateTokenRequest) returns (ValidateTokenResponse);
  rpc GetUserRoles(GetUserRolesRequest) returns (GetUserRolesResponse);
}
```

## Deployment

### Kubernetes

```bash
# Create secrets first
kubectl create secret generic auth9-secrets \
  --from-literal=DATABASE_URL='mysql://...' \
  --from-literal=JWT_SECRET='...' \
  -n auth9

# Deploy
./deploy/deploy.sh
```

### Docker Images

Images are automatically built and pushed to GHCR on merge to main:

```
ghcr.io/gpgkd906/auth9-core:latest
ghcr.io/gpgkd906/auth9-portal:latest
```

## Configuration

### Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `DATABASE_URL` | TiDB/MySQL connection string | Required |
| `REDIS_URL` | Redis connection string | `redis://localhost:6379` |
| `JWT_SECRET` | JWT signing secret | Required |
| `JWT_ISSUER` | JWT issuer URL | `https://auth9.example.com` |
| `KEYCLOAK_URL` | Keycloak server URL | `http://localhost:8081` |

## Documentation

- **[Wiki 主页](wiki/Home.md)** - 完整的中文文档
- **[用户操作指南](userguide/USER_GUIDE.md)** - 详细的操作手册
- **[Architecture](docs/architecture.md)** - System design and architecture overview
- **[Design System](docs/design-system.md)** - Liquid Glass UI design language
- **[Action Engine](wiki/操作引擎-Action-Engine.md)** - 自动化工作流系统
- **[SDK Integration](wiki/SDK集成指南.md)** - TypeScript SDK 使用指南
- **[QA Test Cases](docs/qa/README.md)** - Functional testing scenarios (185 scenarios)
- **[UI/UX Test Cases](docs/uiux/README.md)** - UI/UX testing scenarios (27 scenarios)
- **[Security Test Cases](docs/security/README.md)** - Security testing scenarios (177 scenarios)
- **[Keycloak Theme](docs/keycloak-theme.md)** - Customizing Keycloak login pages

## Development

### Running Tests

```bash
# auth9-core
cd auth9-core
cargo test --lib           # Unit tests
cargo test --test '*'      # Integration tests

# auth9-portal
cd auth9-portal
npm run test               # Unit tests
npm run lint               # Linting
npm run typecheck          # Type checking
```

### CI/CD

- **CI**: Runs on every PR to `main`
  - Rust: fmt, clippy, tests
  - Node: lint, typecheck, tests, build
  - Docker: build test

- **CD**: Runs on push to `main`
  - Builds and pushes Docker images to GHCR
  - Generates deployment summary with image tags

## License

MIT
