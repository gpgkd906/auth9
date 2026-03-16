# @auth9/node

Node.js SDK for Auth9 - JWT verification, gRPC client, and framework middleware.

## Installation

```bash
npm install @auth9/node @auth9/core
# or
pnpm add @auth9/node @auth9/core
```

## Features

- ✅ **JWT Token Verification** - JWKS-based verification with caching
- ✅ **gRPC Client** - Token exchange, validation, introspection, user roles
- ✅ **M2M Authentication** - Client credentials grant with auto-refresh
- ✅ **Express Middleware** - Authentication + permission/role guards
- ✅ **Fastify Plugin** - Token verification plugin
- ✅ **Next.js Middleware** - Edge-compatible auth with header forwarding
- ✅ **Testing Utilities** - Mock tokens and middleware for unit tests

## Quick Start

```typescript
import { Auth9 } from '@auth9/node';

const auth9 = new Auth9({
  domain: 'https://auth9.example.com',
  audience: 'my-service-client-id',
  clientId: 'service-client-id',
  clientSecret: 'service-secret', // pragma: allowlist secret
});

// Verify a JWT token
const claims = await auth9.verifyToken(token);
console.log(claims.sub, claims.email, claims.roles);

// Get M2M service token (auto-cached)
const serviceToken = await auth9.getServiceToken();

// Create gRPC client for token exchange
const grpc = auth9.grpc({ address: 'localhost:50051' });
const result = await grpc.exchangeToken({
  identityToken,
  tenantId: 'tenant-123',
  serviceId: 'my-app',
});
```

## Token Verification

```typescript
import { TokenVerifier } from '@auth9/node';

const verifier = new TokenVerifier({
  domain: 'https://auth9.example.com',
  audience: 'my-service',          // expected audience
  jwksCacheTtl: 3600,              // JWKS cache TTL in seconds (default: 3600)
  algorithms: ['RS256'],           // allowed algorithms (default: ["RS256"])
});

const { claims, tokenType } = await verifier.verify(token);
// tokenType: "identity" | "tenantAccess" | "serviceClient"

if (tokenType === 'tenantAccess') {
  console.log(claims.tenantId, claims.roles, claims.permissions);
}
```

## Client Credentials (M2M)

```typescript
import { ClientCredentials } from '@auth9/node';

const creds = new ClientCredentials({
  domain: 'https://auth9.example.com',
  clientId: 'my-service',
  clientSecret: 'my-secret', // pragma: allowlist secret
});

// Auto-cached, auto-refreshed (30s buffer before expiration)
const token = await creds.getToken();

// Force refresh
creds.clearCache();
```

## gRPC Client

```typescript
import { Auth9GrpcClient } from '@auth9/node';

const client = new Auth9GrpcClient({
  address: 'localhost:50051',
  tls: true,
  auth: {
    apiKey: 'your-api-key', // pragma: allowlist secret
    // or mTLS:
    // mtls: { cert: Buffer, key: Buffer, ca: Buffer },
  },
});

// Exchange identity token for tenant access token
const exchange = await client.exchangeToken({
  identityToken: 'id-token',
  tenantId: 'tenant-123',
  serviceId: 'my-app',
});
console.log(exchange.accessToken, exchange.expiresIn);

// Validate a token
const validation = await client.validateToken({
  accessToken: 'access-token',
  audience: 'my-service',
});
console.log(validation.valid, validation.userId);

// Get user roles and permissions
const roles = await client.getUserRoles({
  userId: 'user-123',
  tenantId: 'tenant-123',
  serviceId: 'my-app',
});
console.log(roles.roles, roles.permissions);

// Introspect token details
const introspection = await client.introspectToken({ token: 'access-token' });
console.log(introspection.active, introspection.roles);

// Clean up
client.close();
```

## Express Middleware

```typescript
import express from 'express';
import { auth9Middleware, requirePermission, requireRole } from '@auth9/node/middleware/express';

const app = express();

// Require authentication on all routes
app.use(auth9Middleware({
  domain: process.env.AUTH9_DOMAIN!,
  audience: process.env.AUTH9_AUDIENCE,
}));

// Access authenticated user info via req.auth
app.get('/api/profile', (req, res) => {
  res.json({
    userId: req.auth.userId,
    email: req.auth.email,
    roles: req.auth.roles,
  });
});

// Permission guard
app.post('/api/admin/users', requirePermission('user:write'), (req, res) => {
  res.json({ created: true });
});

// Role guard (require any)
app.get('/api/manage', requireRole(['admin', 'manager'], { mode: 'any' }), (req, res) => {
  res.json({ canManage: true });
});

// Inline permission checks
app.get('/api/dashboard', (req, res) => {
  if (req.auth.hasPermission('analytics:read')) {
    return res.json({ analytics: true });
  }
  res.status(403).json({ error: 'Forbidden' });
});
```

### Optional Authentication

```typescript
app.use(auth9Middleware({
  domain: process.env.AUTH9_DOMAIN!,
  optional: true,  // allows unauthenticated requests
}));

app.get('/api/public', (req, res) => {
  if (req.auth) {
    res.json({ greeting: `Hello ${req.auth.email}` });
  } else {
    res.json({ greeting: 'Hello guest' });
  }
});
```

### `req.auth` API

```typescript
req.auth.userId                          // string
req.auth.email                           // string
req.auth.tokenType                       // "identity" | "tenantAccess" | "serviceClient"
req.auth.tenantId                        // string | undefined
req.auth.roles                           // string[]
req.auth.permissions                     // string[]
req.auth.raw                             // Auth9Claims (full decoded claims)
req.auth.hasPermission('user:read')      // boolean
req.auth.hasRole('admin')                // boolean
req.auth.hasAnyPermission(['a', 'b'])    // boolean
req.auth.hasAllPermissions(['a', 'b'])   // boolean
```

## Fastify Plugin

```typescript
import fastify from 'fastify';
import { auth9Plugin } from '@auth9/node/middleware/fastify';

const app = fastify();

await app.register(auth9Plugin, {
  domain: process.env.AUTH9_DOMAIN!,
  audience: process.env.AUTH9_AUDIENCE,
});

app.get('/api/profile', async (request, reply) => {
  if (!request.auth9) {
    return reply.status(401).send({ error: 'Unauthorized' });
  }
  reply.send({
    userId: request.auth9.userId,
    isAdmin: request.auth9.hasRole('admin'),
  });
});
```

## Next.js Middleware

```typescript
// middleware.ts
import { auth9Middleware } from '@auth9/node/middleware/next';

export default auth9Middleware({
  domain: process.env.AUTH9_DOMAIN!,
  audience: process.env.AUTH9_AUDIENCE,
  publicPaths: ['/', '/login', '/api/health'],
});

export const config = {
  matcher: ['/((?!_next/static|_next/image|favicon.ico).*)'],
};
```

Auth info is forwarded via headers to route handlers:

```typescript
// app/api/profile/route.ts
export async function GET(request: Request) {
  const userId = request.headers.get('x-auth9-user-id');
  const roles = JSON.parse(request.headers.get('x-auth9-roles') || '[]');

  if (!userId) {
    return Response.json({ error: 'Unauthorized' }, { status: 401 });
  }

  return Response.json({ userId, roles });
}
```

**Forwarded Headers:**
| Header | Content |
|--------|---------|
| `x-auth9-user-id` | User ID (sub claim) |
| `x-auth9-email` | User email |
| `x-auth9-token-type` | `identity` / `tenantAccess` / `serviceClient` |
| `x-auth9-tenant-id` | Tenant ID (if present) |
| `x-auth9-roles` | JSON array of roles |
| `x-auth9-permissions` | JSON array of permissions |

## Testing Utilities

```typescript
import { createMockToken, createMockAuth9 } from '@auth9/node/testing';

// Create mock JWT (not cryptographically signed)
const token = createMockToken({
  sub: 'test-user',
  roles: ['admin'],
  permissions: ['user:read', 'user:write'],
});

// Create mock Auth9 instance
const mock = createMockAuth9({
  defaultUser: {
    sub: 'test-user',
    email: 'test@example.com',
    roles: ['user'],
  },
});

// Use mock middleware in Express tests
const app = express();
app.use(mock.middleware());
app.get('/api/profile', (req, res) => res.json({ userId: req.auth.userId }));

// Verify mock tokens
const claims = mock.verifyToken(token);
expect(claims.sub).toBe('test-user');
```

## Testing

```bash
pnpm test              # Run all tests
pnpm test:coverage     # With coverage
```

## Related Packages

- **@auth9/core** - Framework-agnostic HTTP client and TypeScript types

## License

MIT

## Links

- [Auth9 Documentation](../../docs/)
- [@auth9/core README](../core/README.md)
- [GitHub Repository](https://github.com/gpgkd906/auth9)
