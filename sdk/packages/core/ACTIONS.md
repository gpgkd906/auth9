# Actions API - Usage Guide

The `@auth9/core` SDK provides TypeScript types and HTTP client support for Auth9 Actions.

## Installation

```bash
npm install @auth9/core
# or
pnpm add @auth9/core
```

## Quick Start

```typescript
import { Auth9HttpClient, ActionTrigger } from '@auth9/core';
import type { Action, CreateActionInput } from '@auth9/core';

const client = new Auth9HttpClient({
  baseUrl: 'https://auth9.example.com',
  accessToken: 'your-api-token',
});

const tenantId = 'your-tenant-id';
```

## Creating Actions

### Basic Action

```typescript
const input: CreateActionInput = {
  name: 'Add department claim',
  triggerId: ActionTrigger.PostLogin,
  script: `
    // TypeScript code executed in V8 isolate
    context.claims = context.claims || {};
    context.claims.department = "engineering";
    context.claims.tier = "premium";
    context;
  `,
  enabled: true,
  executionOrder: 0,
  timeoutMs: 3000,
};

const { data: action } = await client.post<{ data: Action }>(
  `/api/v1/tenants/${tenantId}/actions`,
  input
);

console.log('Created action:', action.id);
```

### Block User Registration (Pre-trigger)

```typescript
const blockAction: CreateActionInput = {
  name: 'Block competitor domains',
  triggerId: ActionTrigger.PreUserRegistration,
  script: `
    const blockedDomains = ['@competitor.com', '@spam.com'];
    if (blockedDomains.some(domain => context.user.email.endsWith(domain))) {
      throw new Error('Email domain not allowed');
    }
    context;
  `,
  enabled: true,
};

await client.post(`/api/v1/tenants/${tenantId}/actions`, blockAction);
```

## Listing Actions

### All Actions

```typescript
const { data: actions } = await client.get<{ data: Action[] }>(
  `/api/v1/tenants/${tenantId}/actions`
);

console.log(`Found ${actions.length} actions`);
```

### Filter by Trigger

```typescript
const { data: loginActions } = await client.get<{ data: Action[] }>(
  `/api/v1/tenants/${tenantId}/actions`,
  { trigger_id: ActionTrigger.PostLogin }
);
```

## Updating Actions

```typescript
import type { UpdateActionInput } from '@auth9/core';

const update: UpdateActionInput = {
  enabled: false,
  executionOrder: 5,
};

const { data: updated } = await client.patch<{ data: Action }>(
  `/api/v1/tenants/${tenantId}/actions/${actionId}`,
  update
);

console.log('Updated action:', updated.name);
```

## Deleting Actions

```typescript
await client.delete(`/api/v1/tenants/${tenantId}/actions/${actionId}`);
console.log('Action deleted');
```

## Batch Operations (AI Agent Friendly)

Perfect for AI Agents managing multiple services:

```typescript
import type { UpsertActionInput, BatchUpsertResponse } from '@auth9/core';

const actions: UpsertActionInput[] = [
  {
    name: 'service-a-access-control',
    triggerId: ActionTrigger.PostLogin,
    script: `
      if (!context.user.email.endsWith('@company.com')) {
        throw new Error('Unauthorized domain');
      }
      context.claims = context.claims || {};
      context.claims.services = ['service-a'];
      context;
    `,
    enabled: true,
    executionOrder: 0,
    timeoutMs: 3000,
  },
  {
    name: 'service-b-access-control',
    triggerId: ActionTrigger.PostLogin,
    script: `
      context.claims = context.claims || {};
      context.claims.services = (context.claims.services || []).concat(['service-b']);
      context;
    `,
    enabled: true,
    executionOrder: 1,
    timeoutMs: 3000,
  },
  {
    id: 'existing-action-id', // Update existing
    name: 'updated-action',
    triggerId: ActionTrigger.PostLogin,
    script: 'context;',
    enabled: false,
    executionOrder: 2,
    timeoutMs: 5000,
  },
];

const { data: result } = await client.post<{ data: BatchUpsertResponse }>(
  `/api/v1/tenants/${tenantId}/actions/batch`,
  { actions }
);

console.log(`Created: ${result.created.length}`);
console.log(`Updated: ${result.updated.length}`);
console.log(`Errors: ${result.errors.length}`);

// Handle errors
result.errors.forEach(error => {
  console.error(`Action "${error.name}" failed: ${error.error}`);
});
```

## Testing Actions

```typescript
import type { ActionContext, TestActionResponse } from '@auth9/core';

const testContext: ActionContext = {
  user: {
    id: 'user-123',
    email: 'test@example.com',
    displayName: 'Test User',
    mfaEnabled: false,
  },
  tenant: {
    id: tenantId,
    slug: 'test-tenant',
    name: 'Test Tenant',
  },
  request: {
    ip: '127.0.0.1',
    userAgent: 'Mozilla/5.0',
    timestamp: new Date().toISOString(),
  },
};

const { data: testResult } = await client.post<{ data: TestActionResponse }>(
  `/api/v1/tenants/${tenantId}/actions/${actionId}/test`,
  { context: testContext }
);

if (testResult.success) {
  console.log(`Test passed in ${testResult.durationMs}ms`);
  console.log('Modified claims:', testResult.modifiedContext?.claims);
} else {
  console.error('Test failed:', testResult.errorMessage);
}

// View console logs from script
testResult.consoleLogs.forEach(log => console.log('[Action]', log));
```

## Querying Execution Logs

```typescript
import type { ActionExecution, LogQueryFilter } from '@auth9/core';

// Query all logs for a tenant
const { data: allLogs } = await client.get<{ data: ActionExecution[] }>(
  `/api/v1/tenants/${tenantId}/actions/logs`
);

// Filter by action
const filter: LogQueryFilter = {
  actionId: 'action-123',
  success: false, // Only failures
  limit: 100,
  offset: 0,
};

const params = new URLSearchParams(
  Object.entries(filter)
    .filter(([, v]) => v !== undefined)
    .map(([k, v]) => [k, String(v)])
);

const { data: filteredLogs } = await client.get<{ data: ActionExecution[] }>(
  `/api/v1/tenants/${tenantId}/actions/logs?${params.toString()}`
);

filteredLogs.forEach(log => {
  console.log(`${log.executedAt}: ${log.success ? '✓' : '✗'} ${log.durationMs}ms`);
  if (!log.success) {
    console.error('  Error:', log.errorMessage);
  }
});
```

## Getting Action Statistics

```typescript
import type { ActionStats } from '@auth9/core';

const { data: stats } = await client.get<{ data: ActionStats }>(
  `/api/v1/tenants/${tenantId}/actions/${actionId}/stats`
);

console.log('Execution count:', stats.executionCount);
console.log('Error count:', stats.errorCount);
console.log('Error rate:', (stats.errorCount / stats.executionCount * 100).toFixed(2) + '%');
console.log('Avg duration:', stats.avgDurationMs + 'ms');
console.log('Last 24h executions:', stats.last24hCount);
```

## Listing Available Triggers

```typescript
const { data: triggers } = await client.get('/api/v1/triggers');

triggers.forEach(trigger => {
  console.log(`${trigger.id}: ${trigger.name}`);
  console.log(`  ${trigger.description}`);
});
```

## AI Agent Complete Example

Full workflow for an AI Agent managing authentication for multiple services:

```typescript
import { Auth9HttpClient, ActionTrigger } from '@auth9/core';
import type {
  Action,
  UpsertActionInput,
  BatchUpsertResponse,
} from '@auth9/core';

class Auth9ActionsManager {
  private client: Auth9HttpClient;
  private tenantId: string;

  constructor(baseUrl: string, apiKey: string, tenantId: string) {
    this.client = new Auth9HttpClient({
      baseUrl,
      accessToken: apiKey,
    });
    this.tenantId = tenantId;
  }

  /**
   * Deploy access control rules for multiple services
   */
  async deployServiceRules(services: string[]): Promise<void> {
    const actions: UpsertActionInput[] = services.map((service, index) => ({
      name: `${service}-access-control`,
      triggerId: ActionTrigger.PostLogin,
      script: `
        // Allow only company email domain
        if (!context.user.email.endsWith('@company.com')) {
          throw new Error('Unauthorized domain');
        }

        // Add service to allowed services list
        context.claims = context.claims || {};
        context.claims.services = context.claims.services || [];
        if (!context.claims.services.includes('${service}')) {
          context.claims.services.push('${service}');
        }

        context;
      `,
      enabled: true,
      executionOrder: index,
      timeoutMs: 3000,
    }));

    const { data: result } = await this.client.post<{ data: BatchUpsertResponse }>(
      `/api/v1/tenants/${this.tenantId}/actions/batch`,
      { actions }
    );

    console.log(`Deployed ${result.created.length} new rules`);

    if (result.errors.length > 0) {
      throw new Error(`Failed to deploy ${result.errors.length} rules`);
    }
  }

  /**
   * Monitor action health
   */
  async checkHealth(): Promise<void> {
    const { data: actions } = await this.client.get<{ data: Action[] }>(
      `/api/v1/tenants/${this.tenantId}/actions`
    );

    for (const action of actions) {
      const { data: stats } = await this.client.get<{ data: ActionStats }>(
        `/api/v1/tenants/${this.tenantId}/actions/${action.id}/stats`
      );

      const errorRate = stats.executionCount > 0
        ? (stats.errorCount / stats.executionCount) * 100
        : 0;

      if (errorRate > 5) {
        console.warn(`⚠️  Action "${action.name}" has ${errorRate.toFixed(2)}% error rate`);

        // Auto-disable failing actions
        if (errorRate > 20) {
          await this.client.patch(
            `/api/v1/tenants/${this.tenantId}/actions/${action.id}`,
            { enabled: false }
          );
          console.log(`🛑 Disabled action "${action.name}" due to high error rate`);
        }
      }
    }
  }
}

// Usage
const manager = new Auth9ActionsManager(
  'https://auth9.example.com',
  process.env.AUTH9_API_KEY!,
  'tenant-123'
);

await manager.deployServiceRules(['service-x', 'service-y', 'service-z']);
await manager.checkHealth();
```

## Available Triggers

```typescript
import { ActionTrigger } from '@auth9/core';

ActionTrigger.PostLogin              // After successful login
ActionTrigger.PreUserRegistration    // Before creating user (can block)
ActionTrigger.PostUserRegistration   // After user created
ActionTrigger.PostChangePassword     // After password changed
ActionTrigger.PostEmailVerification  // After email verified
ActionTrigger.PreTokenRefresh        // Before token refresh (can block)
```

## Script Context

Your TypeScript scripts receive a `context` object:

```typescript
interface ActionContext {
  user: {
    id: string;
    email: string;
    displayName?: string;
    mfaEnabled: boolean;
  };
  tenant: {
    id: string;
    slug: string;
    name: string;
  };
  request: {
    ip?: string;
    userAgent?: string;
    timestamp: string;
  };
  claims?: Record<string, unknown>;
}
```

**Important**: Always return `context` at the end of your script!

## Best Practices

1. **Keep scripts simple** - Complex logic should live in external services
2. **Use Pre-triggers for blocking** - Throw errors to prevent operations
3. **Use Post-triggers for enrichment** - Add claims, send notifications
4. **Set appropriate timeouts** - Default 3s, max 10s recommended
5. **Monitor error rates** - Use stats API to track failures
6. **Test before deploying** - Use the test endpoint
7. **Handle errors gracefully** - Always include error messages
8. **Use batch operations** - For AI Agents managing multiple services

## TypeScript Support

All types are fully typed for TypeScript projects:

```typescript
import type {
  Action,
  CreateActionInput,
  UpdateActionInput,
  ActionContext,
  TestActionResponse,
  ActionExecution,
  ActionStats,
  UpsertActionInput,
  BatchUpsertResponse,
  BatchError,
  LogQueryFilter,
} from '@auth9/core';

import { ActionTrigger } from '@auth9/core';
```

## Error Handling

```typescript
import {
  Auth9Error,
  ValidationError,
  ConflictError,
  NotFoundError
} from '@auth9/core';

try {
  await client.post(`/api/v1/tenants/${tenantId}/actions`, invalidInput);
} catch (error) {
  if (error instanceof ValidationError) {
    console.error('Validation failed:', error.message);
  } else if (error instanceof ConflictError) {
    console.error('Action name already exists');
  } else if (error instanceof Auth9Error) {
    console.error('API error:', error.status, error.message);
  }
}
```

## Next Steps

- Read the [Actions System Plan](../../docs/plans/actions-system.md)
- Check [API Documentation](../../docs/actions-implementation-status.md)
- See [Technical Debt](../../docs/debt/001-action-test-endpoint-axum-tonic-conflict.md) for known limitations
