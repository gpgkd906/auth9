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
- ✅ **26 domain sub-clients** - Tenants, Users, Services, Roles, Permissions, RBAC, Sessions, Invitations, Webhooks, Identity Providers, SSO, SAML, ABAC, SCIM, Organizations, Password, Passkeys, Email OTP, Auth, Tenant Services, Audit Logs, Analytics, Security Alerts, System Settings, Email Templates, Branding
- ✅ **100% auth9-core REST API coverage** - Every endpoint mapped to a typed SDK method
- ✅ Claims validation and token type detection
- ✅ Full type safety

## Quick Start

```typescript
import { Auth9Client } from '@auth9/core';

const client = new Auth9Client({
  baseUrl: 'https://auth9.example.com',
  apiKey: 'your-api-key', // pragma: allowlist secret // pragma: allowlist secret
  serviceId: 'your-service-id',  // required for actions API
});

// Domain sub-clients with full type safety
const tenants = await client.tenants.list();
const users = await client.users.list();
const roles = await client.roles.list('service-id');

// Actions API
const triggers = await client.actions.getTriggers();
const actions = await client.actions.list();
const logs = await client.actions.logs({ success: false, limit: 10 });
const logDetail = await client.actions.getLog('log-id');
```

### Low-level HTTP Client

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
import { Auth9Client, ActionTrigger } from '@auth9/core';

const client = new Auth9Client({
  baseUrl: 'https://auth9.example.com',
  apiKey: 'your-api-key', // pragma: allowlist secret
  serviceId: 'your-service-id',
});

// List available triggers
const triggers = await client.actions.getTriggers();
// => ["post-login", "pre-user-registration", ...]

// Create an action
const action = await client.actions.create({
  name: 'Add department claim',
  triggerId: ActionTrigger.PostLogin,
  script: `
    context.claims = context.claims || {};
    context.claims.department = "engineering";
    context;
  `,
  enabled: true,
});

// Query execution logs (paginated, with full filter)
const logs = await client.actions.logs({
  actionId: action.id,
  success: false,
  from: '2026-01-01T00:00:00Z',
  limit: 50,
});

// Get single log detail
const logDetail = await client.actions.getLog(logs.data[0].id);
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

  // Actions
  Action,
  ActionContext,
  ActionExecution,
  ActionStats,
  LogQueryFilter,

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
  ActionTrigger,  // post-login, pre-user-registration, etc.
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

## Sub-Clients Reference

All sub-clients are lazy-loaded and accessed as properties on `Auth9Client`.

| Sub-Client | Methods | Description |
|------------|---------|-------------|
| `client.tenants` | `list` `get` `create` `update` `delete` `listUsers` `getMaliciousIpBlacklist` `updateMaliciousIpBlacklist` | Tenant management |
| `client.users` | `list` `get` `getMe` `updateMe` `create` `update` `delete` `enableMfa` `disableMfa` `getTenants` `addToTenant` `removeFromTenant` `updateRoleInTenant` | User management |
| `client.services` | `list` `get` `create` `update` `delete` `getIntegrationInfo` `listClients` `createClient` `deleteClient` `regenerateClientSecret` | Service & client management |
| `client.roles` | `list` `get` `create` `update` `delete` `assignPermission` `removePermission` | Role management |
| `client.permissions` | `list` `create` `delete` | Permission management |
| `client.rbac` | `assignRoles` `getUserRoles` `getUserAssignedRoles` `unassignRole` | Role-based access control |
| `client.invitations` | `list` `get` `create` `delete` `revoke` `resend` `validate` `accept` | Tenant invitations |
| `client.identityProviders` | `list` `get` `create` `update` `delete` `getTemplates` `listMyLinkedIdentities` `unlinkIdentity` | Social/Enterprise IdP |
| `client.sso` | `listConnectors` `createConnector` `updateConnector` `deleteConnector` `testConnector` | Enterprise SSO |
| `client.saml` | `list` `get` `create` `update` `delete` `getMetadata` `getCertificate` `getCertificateInfo` | SAML applications |
| `client.abac` | `listPolicies` `createPolicy` `updatePolicy` `publishPolicy` `rollbackPolicy` `simulate` | Attribute-based access control |
| `client.sessions` | `listMy` `revoke` `revokeAllOther` `forceLogout` | Session management |
| `client.webhooks` | `list` `get` `create` `update` `delete` `test` `regenerateSecret` | Webhook management |
| `client.scim` | `listTokens` `createToken` `revokeToken` `listLogs` `listGroupMappings` `updateGroupMappings` | SCIM provisioning admin |
| `client.tenantServices` | `list` `toggle` `getEnabled` | Tenant-service associations |
| `client.password` | `forgotPassword` `resetPassword` `changeMyPassword` `adminSetPassword` `getPolicy` `updatePolicy` | Password flows & policy |
| `client.passkeys` | `list` `delete` `startRegistration` `completeRegistration` `startAuthentication` `completeAuthentication` | WebAuthn/Passkey |
| `client.emailOtp` | `send` `verify` | Email OTP authentication |
| `client.auth` | `getAuthorizeUrl` `getLogoutUrl` `exchangeTenantToken` `getUserInfo` `discoverEnterpriseSso` | OAuth/OIDC flows |
| `client.organizations` | `create` `getMyTenants` | Organization management |
| `client.auditLogs` | `list` | Audit log queries |
| `client.analytics` | `getLoginStats` `listLoginEvents` `getDailyTrend` | Login analytics |
| `client.securityAlerts` | `list` `resolve` | Security alert management |
| `client.system` | `getEmailSettings` `updateEmailSettings` `testEmailConnection` `sendTestEmail` `getMaliciousIpBlacklist` `updateMaliciousIpBlacklist` | System configuration |
| `client.emailTemplates` | `list` `get` `update` `reset` `preview` `sendTest` | Email template management |
| `client.branding` | `get` `update` `getPublic` `getForService` `updateForService` `deleteForService` | Branding configuration |
| `client.actions` | `list` `get` `create` `update` `delete` `test` `batchUpsert` `logs` `getLog` `stats` `getTriggers` | Actions (requires `serviceId`) |

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
await client.patch<T>(path, body?);
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
import { Auth9Client, ActionTrigger } from '@auth9/core';
import type { UpsertActionInput } from '@auth9/core';

const client = new Auth9Client({
  baseUrl: process.env.AUTH9_URL!,
  apiKey: process.env.AUTH9_API_KEY!,
  serviceId: process.env.AUTH9_SERVICE_ID!,
});

// Deploy access control rules via batch upsert
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
  strictMode: false,
  executionOrder: index,
  timeoutMs: 3000,
}));

const result = await client.actions.batchUpsert(actions);
console.log(`Deployed ${result.created.length} rules`);

// Monitor execution health
for (const action of result.created) {
  const stats = await client.actions.stats(action.id);
  console.log(`${action.name}: ${stats.errorCount} errors / ${stats.executionCount} runs`);
}
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
- [GitHub Repository](https://github.com/gpgkd906/auth9)
