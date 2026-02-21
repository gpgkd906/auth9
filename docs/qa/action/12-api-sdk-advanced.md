# Action API & SDK 集成测试（进阶）

**模块**: Action REST API & TypeScript SDK
**测试范围**: 日志、统计、错误处理、并发与 AI Agent 集成
**场景数**: 5

---
## 场景 6：SDK - 日志查询

### 测试代码
```typescript
import { Auth9Client } from '@auth9/core';

const client = new Auth9Client({
  baseUrl: 'http://localhost:8080',
  apiKey: process.env.AUTH9_API_KEY!,
  serviceId: 'service-uuid',
});

async function testLogs() {
  const action = await client.actions.create({
    name: 'Logging Test',
    trigger_id: 'post-login',
    script: 'context;',
  });

  // 触发执行（通过登录或测试）
  await client.actions.test(action.id, {
    user: { id: '1', email: 'test@example.com', mfa_enabled: false },
    tenant: { id: 'tenant-id', slug: 'test', name: 'Test' },
    request: { timestamp: new Date().toISOString() },
  });

  // 查询日志
  const logs = await client.actions.logs({
    action_id: action.id,
    limit: 10,
  });

  console.log('Total logs:', logs.length);
  console.log('Latest log:', logs[0]);
}

testLogs().catch(console.error);
```

### 预期输出
```
Total logs: 1
Latest log: {
  id: "log-uuid",
  action_id: "action-uuid",
  success: true,
  duration_ms: 12,
  executed_at: "2026-02-12T10:30:00Z"
}
```

---

## 场景 7：SDK - 统计信息查询

### 测试代码
```typescript
import { Auth9Client } from '@auth9/core';

const client = new Auth9Client({
  baseUrl: 'http://localhost:8080',
  apiKey: process.env.AUTH9_API_KEY!,
  serviceId: 'service-uuid',
});

async function testStats() {
  const action = await client.actions.create({
    name: 'Stats Test',
    trigger_id: 'post-login',
    script: 'context;',
  });

  // 触发多次执行
  for (let i = 0; i < 10; i++) {
    await client.actions.test(action.id, {
      user: { id: '1', email: 'test@example.com', mfa_enabled: false },
      tenant: { id: 'tenant-id', slug: 'test', name: 'Test' },
      request: { timestamp: new Date().toISOString() },
    });
  }

  // 查询统计
  const stats = await client.actions.stats(action.id);
  console.log('Execution count:', stats.execution_count);
  console.log('Success rate:', stats.success_rate);
  console.log('Avg duration:', stats.avg_duration_ms);
  console.log('Last 24h:', stats.last_24h_count);
}

testStats().catch(console.error);
```

### 预期输出
```
Execution count: 10
Success rate: 100.0
Avg duration: 15
Last 24h: 10
```

---

## 场景 8：错误处理 - SDK

### 测试代码
```typescript
import { Auth9Client, NotFoundError, ValidationError, AuthenticationError } from '@auth9/core';

const client = new Auth9Client({
  baseUrl: 'http://localhost:8080',
  apiKey: 'invalid-token',
  serviceId: 'service-uuid',
});

async function testErrors() {
  try {
    // 测试认证失败
    await client.actions.list();
  } catch (error) {
    if (error instanceof AuthenticationError) {
      console.log('Authentication failed:', error.message);
    }
  }

  const validClient = new Auth9Client({
    baseUrl: 'http://localhost:8080',
    apiKey: process.env.AUTH9_API_KEY!,
    serviceId: 'service-uuid',
  });

  try {
    // 测试 404
    await validClient.actions.get('non-existent-id');
  } catch (error) {
    if (error instanceof NotFoundError) {
      console.log('Action not found:', error.message);
    }
  }

  try {
    // 测试验证错误
    await validClient.actions.create({
      name: '',
      trigger_id: 'invalid-trigger',
      script: '',
    } as any);
  } catch (error) {
    if (error instanceof ValidationError) {
      console.log('Validation failed:', error.message);
    }
  }
}

testErrors().catch(console.error);
```

### 预期输出
```
Authentication failed: Unauthorized
Action not found: Action not found
Validation failed: Invalid input
```

---

## 场景 9：并发请求测试

### 测试代码
```typescript
import { Auth9Client } from '@auth9/core';

const client = new Auth9Client({
  baseUrl: 'http://localhost:8080',
  apiKey: process.env.AUTH9_API_KEY!,
  serviceId: 'service-uuid',
});

async function testConcurrency() {
  // 并发创建 20 个 Actions
  const start = Date.now();
  const promises = Array.from({ length: 20 }, (_, i) =>
    client.actions.create({
      name: `Concurrent Action ${i}`,
      trigger_id: 'post-login',
      script: 'context;',
    })
  );

  const results = await Promise.all(promises);
  const duration = Date.now() - start;

  console.log('Created:', results.length);
  console.log('Total time:', duration, 'ms');
  console.log('Avg time per request:', duration / results.length, 'ms');
}

testConcurrency().catch(console.error);
```

### 预期结果
- 所有 20 个 Actions 创建成功
- 总耗时 < 2000ms（并发执行）
- 平均每个请求 < 100ms

---

## 场景 10：AI Agent 集成模式

### 测试代码（模拟 AI Agent 自动配置）
```typescript
import { Auth9Client } from '@auth9/core';

const client = new Auth9Client({
  baseUrl: 'http://localhost:8080',
  apiKey: process.env.AUTH9_API_KEY!,
  serviceId: 'service-uuid',
});

/**
 * AI Agent 场景：
 * - Agent 启动新服务 "service-x"
 * - 自动配置访问控制规则
 */
async function aiAgentConfigureService() {
  const serviceName = 'service-x';

  // 1. 检查是否已有规则
  const existingActions = await client.actions.list('post-login');
  const existingRule = existingActions.find(a => a.name === `${serviceName}-access-control`);

  if (existingRule) {
    console.log('Rule already exists, updating...');
    await client.actions.update(existingRule.id, {
      script: generateAccessControlScript(serviceName),
    });
  } else {
    console.log('Creating new rule...');
    await client.actions.create({
      name: `${serviceName}-access-control`,
      description: `Auto-generated by AI Agent for ${serviceName}`,
      trigger_id: 'post-login',
      script: generateAccessControlScript(serviceName),
      enabled: true,
    });
  }

  // 2. 测试规则
  const action = (await client.actions.list('post-login')).find(
    a => a.name === `${serviceName}-access-control`
  )!;

  const testResult = await client.actions.test(action.id, {
    user: {
      id: 'test-user',
      email: 'admin@company.com',
      mfa_enabled: false,
    },
    tenant: {
      id: 'tenant-id',
      slug: 'company',
      name: 'Company',
    },
    request: {
      timestamp: new Date().toISOString(),
    },
    claims: {
      roles: ['developer'],
    },
  });

  console.log('Test result:', testResult.success);
  console.log('Service access granted:', testResult.modified_context?.claims?.service_access);
}

function generateAccessControlScript(serviceName: string): string {
  return `
// Auto-generated access control for ${serviceName}
const allowedRoles = ["admin", "developer"];
const userRoles = (context.claims?.roles as string[]) || [];

const hasAccess = allowedRoles.some(role => userRoles.includes(role));

if (!hasAccess) {
  throw new Error("Insufficient permissions for ${serviceName}");
}

context.claims = context.claims || {};
context.claims.service_access = context.claims.service_access || [];
(context.claims.service_access as string[]).push("${serviceName}");

context;
  `.trim();
}

aiAgentConfigureService().catch(console.error);
```

### 预期输出
```
Creating new rule...
Test result: true
Service access granted: ["service-x"]
```

---

## 性能测试

### 1. API 响应时间
```bash
# 创建 Action
time curl -X POST http://localhost:8080/api/v1/services/$SERVICE_ID/actions \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"name":"Perf Test","trigger_id":"post-login","script":"context;"}' \
  > /dev/null
```
**预期**: < 200ms

### 2. 批量操作性能
```typescript
// 批量创建 100 个 Actions
const start = Date.now();
await client.actions.batchUpsert(
  Array.from({ length: 100 }, (_, i) => ({
    name: `Batch ${i}`,
    trigger_id: 'post-login',
    script: 'context;',
  }))
);
const duration = Date.now() - start;
console.log('Duration:', duration, 'ms');
```
**预期**: < 2000ms

### 3. 日志查询性能
```typescript
// 查询 1000 条日志
const start = Date.now();
const logs = await client.actions.logs({ limit: 1000 });
const duration = Date.now() - start;
console.log('Duration:', duration, 'ms');
```
**预期**: < 500ms

---


---

## 回归测试检查清单

| # | 场景 | 状态 | 测试日期 | 测试人员 | 备注 |
|---|------|------|----------|----------|------|
| 1 | SDK - 日志查询 | ☐ | | | |
| 2 | SDK - 统计信息查询 | ☐ | | | |
| 3 | 错误处理 - SDK | ☐ | | | |
| 4 | 并发请求测试 | ☐ | | | |
| 5 | AI Agent 集成模式 | ☐ | | | |
