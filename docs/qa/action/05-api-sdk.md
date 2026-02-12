# Action API & SDK 集成测试

**模块**: Action REST API & TypeScript SDK
**测试范围**: API 功能、SDK 客户端、批量操作、错误处理、集成模式
**场景数**: 10

---

## 前置条件

### 环境准备

1. **auth9-core 运行**: `http://localhost:8080`
2. **生成 API Token**:
```bash
TOKEN=$(.claude/skills/tools/gen-admin-token.sh)
```

3. **安装 @auth9/core**:
```bash
cd /path/to/test-project
npm install @auth9/core
```

---

## 场景 1：REST API - 完整 CRUD 流程

### 初始状态
- auth9-core 运行正常
- 存在测试租户

### 目的
验证 REST API 的完整 CRUD 生命周期

### 测试操作流程

#### 1.1 创建 Action
```bash
TENANT_ID=$(mysql -h 127.0.0.1 -P 4000 -u root auth9 -N -e "SELECT id FROM tenants LIMIT 1;")

curl -X POST http://localhost:8080/api/v1/tenants/$TENANT_ID/actions \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "API Test Action",
    "description": "Created via REST API",
    "trigger_id": "post-login",
    "script": "context.claims = context.claims || {}; context.claims.api_test = true; context;",
    "enabled": true,
    "execution_order": 0,
    "timeout_ms": 3000
  }' | jq '.'
```

**预期响应**:
```json
{
  "success": true,
  "data": {
    "id": "action-uuid",
    "tenant_id": "tenant-uuid",
    "name": "API Test Action",
    "trigger_id": "post-login",
    "enabled": true,
    "created_at": "2026-02-12T10:00:00Z"
  }
}
```

#### 1.2 获取 Action 列表
```bash
curl http://localhost:8080/api/v1/tenants/$TENANT_ID/actions \
  -H "Authorization: Bearer $TOKEN" | jq '.'
```

**预期**: 返回包含刚创建的 Action 的数组

#### 1.3 获取单个 Action
```bash
ACTION_ID="<from_create_response>"
curl http://localhost:8080/api/v1/tenants/$TENANT_ID/actions/$ACTION_ID \
  -H "Authorization: Bearer $TOKEN" | jq '.'
```

**预期**: 返回完整的 Action 对象

#### 1.4 更新 Action
```bash
curl -X PATCH http://localhost:8080/api/v1/tenants/$TENANT_ID/actions/$ACTION_ID \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "description": "Updated description",
    "enabled": false
  }' | jq '.'
```

**预期**: 返回更新后的 Action，`enabled = false`

#### 1.5 删除 Action
```bash
curl -X DELETE http://localhost:8080/api/v1/tenants/$TENANT_ID/actions/$ACTION_ID \
  -H "Authorization: Bearer $TOKEN"
```

**预期**: HTTP 200，返回成功消息

#### 1.6 验证删除
```bash
curl http://localhost:8080/api/v1/tenants/$TENANT_ID/actions/$ACTION_ID \
  -H "Authorization: Bearer $TOKEN"
```

**预期**: HTTP 404 Not Found

---

## 场景 2：REST API - 触发器筛选

### 测试操作流程
```bash
# 创建多个不同触发器的 Actions
for trigger in post-login pre-user-registration post-user-registration; do
  curl -X POST http://localhost:8080/api/v1/tenants/$TENANT_ID/actions \
    -H "Authorization: Bearer $TOKEN" \
    -H "Content-Type: application/json" \
    -d "{
      \"name\": \"Test $trigger\",
      \"trigger_id\": \"$trigger\",
      \"script\": \"context;\"
    }"
done

# 筛选 post-login 触发器
curl "http://localhost:8080/api/v1/tenants/$TENANT_ID/actions?trigger_id=post-login" \
  -H "Authorization: Bearer $TOKEN" | jq '.data | length'
```

**预期**: 仅返回 `trigger_id = post-login` 的 Actions

---

## 场景 3：TypeScript SDK - 基础 CRUD

### 测试代码
```typescript
import { Auth9Client } from '@auth9/core';

const client = new Auth9Client({
  baseUrl: 'http://localhost:8080',
  apiKey: process.env.AUTH9_API_KEY!, // 从环境变量读取 Token
  tenantId: 'tenant-uuid',
});

async function testCRUD() {
  // 1. 创建 Action
  const action = await client.actions.create({
    name: 'SDK Test Action',
    trigger_id: 'post-login',
    script: 'context.claims = context.claims || {}; context.claims.sdk_test = true; context;',
    enabled: true,
  });
  console.log('Created:', action.id);

  // 2. 获取列表
  const actions = await client.actions.list();
  console.log('Total actions:', actions.length);

  // 3. 获取单个
  const retrieved = await client.actions.get(action.id);
  console.log('Retrieved:', retrieved.name);

  // 4. 更新
  const updated = await client.actions.update(action.id, {
    description: 'Updated via SDK',
  });
  console.log('Updated description:', updated.description);

  // 5. 删除
  await client.actions.delete(action.id);
  console.log('Deleted successfully');
}

testCRUD().catch(console.error);
```

### 执行方式
```bash
export AUTH9_API_KEY=$(.claude/skills/tools/gen-admin-token.sh)
ts-node test-sdk-crud.ts
```

### 预期输出
```
Created: action-uuid
Total actions: 5
Retrieved: SDK Test Action
Updated description: Updated via SDK
Deleted successfully
```

---

## 场景 4：SDK - 批量操作（Batch Upsert）

### 测试代码
```typescript
import { Auth9Client } from '@auth9/core';

const client = new Auth9Client({
  baseUrl: 'http://localhost:8080',
  apiKey: process.env.AUTH9_API_KEY!,
  tenantId: 'tenant-uuid',
});

async function testBatchUpsert() {
  // 批量创建/更新 Actions
  const result = await client.actions.batchUpsert([
    {
      name: 'service-a-rule',
      trigger_id: 'post-login',
      script: 'context.claims = context.claims || {}; context.claims.service_a = true; context;',
    },
    {
      name: 'service-b-rule',
      trigger_id: 'post-login',
      script: 'context.claims = context.claims || {}; context.claims.service_b = true; context;',
    },
    {
      id: 'existing-action-id', // 更新现有 Action
      name: 'updated-service-c',
      trigger_id: 'post-login',
      script: 'context.claims = context.claims || {}; context.claims.service_c = "updated"; context;',
    },
  ]);

  console.log('Created:', result.created.length);
  console.log('Updated:', result.updated.length);
  console.log('Errors:', result.errors.length);
}

testBatchUpsert().catch(console.error);
```

### 预期结果
- `created.length = 2` (service-a 和 service-b)
- `updated.length = 1` (service-c)
- `errors.length = 0`

---

## 场景 5：SDK - 测试 Action（Test Endpoint）

### 测试代码
```typescript
import { Auth9Client } from '@auth9/core';

const client = new Auth9Client({
  baseUrl: 'http://localhost:8080',
  apiKey: process.env.AUTH9_API_KEY!,
  tenantId: 'tenant-uuid',
});

async function testAction() {
  // 创建 Action
  const action = await client.actions.create({
    name: 'Test Action',
    trigger_id: 'post-login',
    script: `
      if (context.user.email.endsWith("@blocked.com")) {
        throw new Error("Blocked domain");
      }
      context.claims = context.claims || {};
      context.claims.tested = true;
      context;
    `,
  });

  // 测试：允许的邮箱域
  const successResult = await client.actions.test(action.id, {
    user: {
      id: 'test-user-id',
      email: 'user@allowed.com',
      mfa_enabled: false,
    },
    tenant: {
      id: 'tenant-id',
      slug: 'test-tenant',
      name: 'Test Tenant',
    },
    request: {
      ip: '1.2.3.4',
      user_agent: 'Mozilla/5.0',
      timestamp: new Date().toISOString(),
    },
  });
  console.log('Success test:', successResult.success); // true
  console.log('Modified claims:', successResult.modified_context?.claims);

  // 测试：阻止的邮箱域
  const failResult = await client.actions.test(action.id, {
    user: {
      id: 'test-user-id',
      email: 'user@blocked.com',
      mfa_enabled: false,
    },
    tenant: {
      id: 'tenant-id',
      slug: 'test-tenant',
      name: 'Test Tenant',
    },
    request: {
      ip: '1.2.3.4',
      timestamp: new Date().toISOString(),
    },
  });
  console.log('Fail test:', failResult.success); // false
  console.log('Error:', failResult.error_message); // "Blocked domain"
}

testAction().catch(console.error);
```

### 预期输出
```
Success test: true
Modified claims: { tested: true }
Fail test: false
Error: Blocked domain
```

---

## 场景 6：SDK - 日志查询

### 测试代码
```typescript
import { Auth9Client } from '@auth9/core';

const client = new Auth9Client({
  baseUrl: 'http://localhost:8080',
  apiKey: process.env.AUTH9_API_KEY!,
  tenantId: 'tenant-uuid',
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
  tenantId: 'tenant-uuid',
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
  tenantId: 'tenant-uuid',
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
    tenantId: 'tenant-uuid',
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
  tenantId: 'tenant-uuid',
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
  tenantId: 'tenant-uuid',
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
time curl -X POST http://localhost:8080/api/v1/tenants/$TENANT_ID/actions \
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

## 回归测试检查清单

### REST API
- [ ] 创建 Action 返回完整对象
- [ ] 列表查询支持筛选
- [ ] 获取单个 Action 返回详情
- [ ] 更新 Action 部分更新生效
- [ ] 删除 Action 成功
- [ ] 触发器筛选正确
- [ ] 权限检查生效

### TypeScript SDK
- [ ] CRUD 操作正常
- [ ] 批量操作正常
- [ ] 测试端点正常
- [ ] 日志查询正常
- [ ] 统计查询正常
- [ ] 错误类型正确
- [ ] 并发请求稳定
- [ ] AI Agent 集成模式可用

### 性能
- [ ] API 响应时间 < 200ms
- [ ] 批量操作 < 2s（100 actions）
- [ ] 日志查询 < 500ms（1000 logs）
- [ ] 并发请求稳定

### 错误处理
- [ ] 认证失败返回 401
- [ ] 未找到返回 404
- [ ] 验证失败返回 400
- [ ] 权限不足返回 403
- [ ] SDK 抛出正确错误类型
