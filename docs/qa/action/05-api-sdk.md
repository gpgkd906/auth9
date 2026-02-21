# Action API & SDK 集成测试

**模块**: Action REST API & TypeScript SDK
**测试范围**: API 功能、SDK 客户端、批量操作、错误处理、集成模式
**场景数**: 5

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
SERVICE_ID=$(mysql -h 127.0.0.1 -P 4000 -u root auth9 -N -e "SELECT id FROM services LIMIT 1;")

curl -X POST http://localhost:8080/api/v1/services/$SERVICE_ID/actions \
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
    "service_id": "service-uuid",
    "name": "API Test Action",
    "trigger_id": "post-login",
    "enabled": true,
    "created_at": "2026-02-12T10:00:00Z"
  }
}
```

#### 1.2 获取 Action 列表
```bash
curl http://localhost:8080/api/v1/services/$SERVICE_ID/actions \
  -H "Authorization: Bearer $TOKEN" | jq '.'
```

**预期**: 返回包含刚创建的 Action 的数组

#### 1.3 获取单个 Action
```bash
ACTION_ID="<from_create_response>"
curl http://localhost:8080/api/v1/services/$SERVICE_ID/actions/$ACTION_ID \
  -H "Authorization: Bearer $TOKEN" | jq '.'
```

**预期**: 返回完整的 Action 对象

#### 1.4 更新 Action
```bash
curl -X PATCH http://localhost:8080/api/v1/services/$SERVICE_ID/actions/$ACTION_ID \
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
curl -X DELETE http://localhost:8080/api/v1/services/$SERVICE_ID/actions/$ACTION_ID \
  -H "Authorization: Bearer $TOKEN"
```

**预期**: HTTP 200，返回成功消息

#### 1.6 验证删除
```bash
curl http://localhost:8080/api/v1/services/$SERVICE_ID/actions/$ACTION_ID \
  -H "Authorization: Bearer $TOKEN"
```

**预期**: HTTP 404 Not Found

---

## 场景 2：REST API - 触发器筛选

### 测试操作流程
```bash
# 创建多个不同触发器的 Actions
for trigger in post-login pre-user-registration post-user-registration; do
  curl -X POST http://localhost:8080/api/v1/services/$SERVICE_ID/actions \
    -H "Authorization: Bearer $TOKEN" \
    -H "Content-Type: application/json" \
    -d "{
      \"name\": \"Test $trigger\",
      \"trigger_id\": \"$trigger\",
      \"script\": \"context;\"
    }"
done

# 筛选 post-login 触发器
curl "http://localhost:8080/api/v1/services/$SERVICE_ID/actions?trigger_id=post-login" \
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
  serviceId: 'service-uuid',
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
  serviceId: 'service-uuid',
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
  serviceId: 'service-uuid',
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


---

## 说明

场景 6-10（日志/统计/错误处理/并发/AI Agent）已拆分到 `docs/qa/action/12-api-sdk-advanced.md`。

---

## 回归测试检查清单

| # | 场景 | 状态 | 测试日期 | 测试人员 | 备注 |
|---|------|------|----------|----------|------|
| 1 | REST API - 完整 CRUD 流程 | ☐ | | | |
| 2 | REST API - 触发器筛选 | ☐ | | | |
| 3 | TypeScript SDK - 基础 CRUD | ☐ | | | |
| 4 | SDK - 批量操作（Batch Upsert） | ☐ | | | |
| 5 | SDK - 测试 Action（Test Endpoint） | ☐ | | | |
