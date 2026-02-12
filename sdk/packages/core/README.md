# @auth9/core

Core TypeScript SDK for Auth9 - Self-hosted identity and access management.

## Installation

```bash
npm install @auth9/core
# or
pnpm add @auth9/core
```

## Features

- ✅ TypeScript types for all Auth9 resources
- ✅ HTTP client with automatic snake_case/camelCase conversion
- ✅ Error handling with typed exceptions
- ✅ **Actions API** - Manage Auth9 Actions (TypeScript scripts in authentication flow)
- ✅ Claims validation and token type detection
- ✅ Full type safety

## Quick Start

```typescript
import { Auth9HttpClient } from '@auth9/core';
import type { Tenant } from '@auth9/core';

const client = new Auth9HttpClient({
  baseUrl: 'https://auth9.example.com',
  accessToken: 'your-api-token',
});

// Make API requests
const { data: tenants } = await client.get<{ data: Tenant[] }>('/api/v1/tenants');
```

## Actions API

Auth9 Actions allow you to run custom TypeScript code at key points in the authentication flow.

```typescript
import { Auth9HttpClient, ActionTrigger } from '@auth9/core';
import type { Action, CreateActionInput } from '@auth9/core';

const client = new Auth9HttpClient({
  baseUrl: 'https://auth9.example.com',
  accessToken: 'your-api-token',
});

// Create an action to add custom claims
const input: CreateActionInput = {
  name: 'Add department claim',
  triggerId: ActionTrigger.PostLogin,
  script: `
    context.claims = context.claims || {};
    context.claims.department = "engineering";
    context;
  `,
  enabled: true,
};

const { data: action } = await client.post<{ data: Action }>(
  '/api/v1/tenants/your-tenant-id/actions',
  input
);
```

👉 **[Full Actions API Guide](./ACTIONS.md)** - Comprehensive documentation and examples

## Available Types

### Domain Models

```typescript
import type {
  // Core resources
  Tenant,
  User,
  Service,
  Role,
  Permission,
  Session,
  Invitation,
  Webhook,

  // Actions (NEW)
  Action,
  ActionContext,
  ActionExecution,
  ActionStats,

  // Input types
  CreateTenantInput,
  CreateUserInput,
  CreateActionInput,
  UpdateActionInput,

  // Response types
  DataResponse,
  PaginatedResponse,
  BatchUpsertResponse,
  TestActionResponse,
} from '@auth9/core';
```

### Enums

```typescript
import {
  ActionTrigger,  // NEW: post-login, pre-user-registration, etc.
} from '@auth9/core';
```

### Claims & Tokens

```typescript
import type {
  IdentityClaims,
  TenantAccessClaims,
  ServiceClientClaims,
  TokenType,
} from '@auth9/core';

import { getTokenType } from '@auth9/core';

const tokenType = getTokenType(claims);  // "identity" | "tenant_access" | "service_client"
```

### Errors

```typescript
import {
  Auth9Error,
  NotFoundError,
  UnauthorizedError,
  ForbiddenError,
  ValidationError,
  ConflictError,
  RateLimitError,
  BadRequestError,
  createErrorFromStatus,
} from '@auth9/core';

try {
  await client.get('/api/v1/tenants/not-found');
} catch (error) {
  if (error instanceof NotFoundError) {
    console.error('Resource not found');
  }
}
```

## HTTP Client

The HTTP client automatically handles:

- ✅ Bearer token authentication
- ✅ Request body conversion: camelCase → snake_case
- ✅ Response body conversion: snake_case → camelCase
- ✅ Error handling with typed exceptions
- ✅ Request timeout (default: 10s)
- ✅ Automatic retry on 5xx errors (configurable)

```typescript
const client = new Auth9HttpClient({
  baseUrl: 'https://auth9.example.com',
  accessToken: 'token', // or async function
  timeout: 10_000,      // 10 seconds
  retries: 3,           // Retry 5xx errors
});

// Supported methods
await client.get<T>(path, params?);
await client.post<T>(path, body?);
await client.put<T>(path, body?);
await client.patch<T>(path, body?);  // NEW: For Actions updates
await client.delete(path);
```

### Dynamic Access Token

```typescript
const client = new Auth9HttpClient({
  baseUrl: 'https://auth9.example.com',
  accessToken: async () => {
    // Fetch fresh token (e.g., from OAuth flow)
    return await getAccessToken();
  },
});
```

## Utils

```typescript
import { toSnakeCase, toCamelCase } from '@auth9/core';

toSnakeCase({ userId: '123', displayName: 'John' });
// { user_id: '123', display_name: 'John' }

toCamelCase({ user_id: '123', display_name: 'John' });
// { userId: '123', displayName: 'John' }
```

## Use Cases

### AI Agents Managing Services

Perfect for AI Agents that dynamically create and manage services:

```typescript
import { Auth9HttpClient, ActionTrigger } from '@auth9/core';
import type { UpsertActionInput, BatchUpsertResponse } from '@auth9/core';

const client = new Auth9HttpClient({
  baseUrl: process.env.AUTH9_URL,
  accessToken: process.env.AUTH9_API_KEY,
});

// Deploy access control rules for multiple services
const actions: UpsertActionInput[] = services.map((service, index) => ({
  name: `${service}-access-control`,
  triggerId: ActionTrigger.PostLogin,
  script: `
    if (!context.user.email.endsWith('@company.com')) {
      throw new Error('Unauthorized domain');
    }
    context.claims.services = context.claims.services || [];
    context.claims.services.push('${service}');
    context;
  `,
  enabled: true,
  executionOrder: index,
  timeoutMs: 3000,
}));

const { data: result } = await client.post<{ data: BatchUpsertResponse }>(
  `/api/v1/tenants/${tenantId}/actions/batch`,
  { actions }
);

console.log(`Deployed ${result.created.length} rules`);
```

See [ACTIONS.md](./ACTIONS.md) for complete examples.

## Testing

```bash
pnpm test              # Run all tests
pnpm test:coverage     # With coverage
```

All tests use vitest with fetch mocking (no external dependencies).

## Related Packages

- **@auth9/node** - Node.js specific utilities (JWT verification, gRPC client)

## License

MIT

## Links

- [Auth9 Documentation](../../docs/)
- [Actions API Guide](./ACTIONS.md)
- [GitHub Repository](https://github.com/auth9/auth9)
