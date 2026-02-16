# SDK 集成指南

## 概述

Auth9 提供官方 TypeScript SDK (`@auth9/core`)，简化了与 Auth9 服务的集成。SDK 提供类型安全的 API 客户端、完整的 TypeScript 类型定义，以及便捷的辅助函数。

### SDK 特性

- ✅ **类型安全**: 完整的 TypeScript 类型定义
- ✅ **HTTP 客户端**: 封装 REST API 调用
- ✅ **gRPC 支持**: （规划中）高性能 gRPC 客户端
- ✅ **自动重试**: 内置请求重试机制
- ✅ **错误处理**: 统一的错误处理和类型
- ✅ **轻量级**: 最小化依赖，体积小
- ✅ **文档完善**: 详细的 API 文档和示例

## 快速开始

### 安装

使用 npm、yarn 或 pnpm 安装：

```bash
# npm
npm install @auth9/core

# yarn
yarn add @auth9/core

# pnpm
pnpm add @auth9/core
```

### 初始化客户端

```typescript
import { Auth9HttpClient } from '@auth9/core';

// 创建客户端实例
const client = new Auth9HttpClient({
  baseUrl: 'https://auth9.example.com',  // Auth9 API 地址
  accessToken: 'your-access-token'        // 访问令牌
});
```

### 基础用法

```typescript
// 获取租户列表
const tenants = await client.get('/api/v1/tenants');

// 创建新用户
const newUser = await client.post('/api/v1/users', {
  body: {
    email: 'user@example.com',
    name: 'John Doe',
    tenant_id: 'tenant_123'
  }
});

// 更新服务
await client.put('/api/v1/services/service_123', {
  body: {
    name: 'Updated Service Name'
  }
});

// 删除角色
await client.delete('/api/v1/roles/role_456');
```

## 客户端 API

### Auth9HttpClient

HTTP 客户端类，封装所有 REST API 调用。

#### 构造函数

```typescript
constructor(config: {
  baseUrl: string;      // Auth9 API 基础 URL
  accessToken: string;  // 访问令牌
  timeout?: number;     // 请求超时（毫秒，默认 30000）
})
```

#### 方法

##### get<T>(path, options?)

发送 GET 请求。

```typescript
const tenants = await client.get<{ data: Tenant[] }>('/api/v1/tenants', {
  params: { page: 1, per_page: 20 }
});
```

**参数**:
- `path`: API 路径
- `options.params`: 查询参数对象
- `options.headers`: 额外的请求头

**返回**: Promise<T>

##### post<T>(path, options?)

发送 POST 请求。

```typescript
const newTenant = await client.post<Tenant>('/api/v1/tenants', {
  body: {
    name: 'Acme Corp',
    slug: 'acme'
  }
});
```

**参数**:
- `path`: API 路径
- `options.body`: 请求体（自动序列化为 JSON）
- `options.headers`: 额外的请求头

**返回**: Promise<T>

##### put<T>(path, options?)

发送 PUT 请求（完整更新）。

```typescript
await client.put<Tenant>('/api/v1/tenants/tenant_123', {
  body: {
    name: 'Updated Name',
    slug: 'acme'
  }
});
```

##### patch<T>(path, options?)

发送 PATCH 请求（部分更新）。

```typescript
await client.patch<Tenant>('/api/v1/tenants/tenant_123', {
  body: {
    name: 'New Name'  // 只更新名称
  }
});
```

##### delete<T>(path, options?)

发送 DELETE 请求。

```typescript
await client.delete('/api/v1/tenants/tenant_123');
```

## TypeScript 类型

SDK 提供完整的 TypeScript 类型定义：

### Action 类型

```typescript
import type {
  Action,
  CreateActionInput,
  UpdateActionInput,
  ActionContext,
  TestActionResponse,
  ActionExecution,
  ActionStats
} from '@auth9/core';

// Action 实体
interface Action {
  id: string;
  tenant_id: string;
  name: string;
  trigger: string;
  code: string;
  enabled: boolean;
  created_at: string;
  updated_at: string;
}

// 创建 Action 输入
interface CreateActionInput {
  name: string;
  trigger: string;
  code: string;
  enabled?: boolean;
}

// 更新 Action 输入
interface UpdateActionInput {
  name?: string;
  code?: string;
  enabled?: boolean;
}

// Action 执行上下文
interface ActionContext {
  trigger: string;
  user: {
    id: string;
    email: string;
    name: string;
  };
  tenant: {
    id: string;
    name: string;
    slug: string;
  };
  event?: Record<string, any>;
  secrets?: Record<string, string>;
}

// 测试响应
interface TestActionResponse {
  success: boolean;
  result?: any;
  logs: string[];
  duration_ms: number;
  error?: string;
}

// Action 执行记录
interface ActionExecution {
  id: string;
  action_id: string;
  status: 'success' | 'failure';
  duration_ms: number;
  logs: string;
  error?: string;
  executed_at: string;
}

// Action 统计
interface ActionStats {
  total_executions: number;
  success_count: number;
  failure_count: number;
  avg_duration_ms: number;
  last_executed_at?: string;
}
```

### 其他类型

```typescript
// 租户
interface Tenant {
  id: string;
  name: string;
  slug: string;
  logo_url?: string;
  created_at: string;
  updated_at: string;
}

// 用户
interface User {
  id: string;
  email: string;
  name: string;
  first_name?: string;
  last_name?: string;
  created_at: string;
  updated_at: string;
}

// 服务
interface Service {
  id: string;
  tenant_id: string;
  name: string;
  client_id: string;
  base_url: string;
  redirect_uris: string[];
  logout_uris: string[];
  created_at: string;
  updated_at: string;
}

// 角色
interface Role {
  id: string;
  service_id: string;
  name: string;
  description?: string;
  created_at: string;
  updated_at: string;
}
```

## 实际应用示例

### 在 React 应用中使用

#### 1. 创建客户端实例

```typescript
// lib/auth9-client.ts
import { Auth9HttpClient } from '@auth9/core';

export function getAuth9Client(accessToken?: string) {
  const baseUrl = process.env.NEXT_PUBLIC_AUTH9_URL || 'http://localhost:8080';
  return new Auth9HttpClient({
    baseUrl,
    accessToken: accessToken || ''
  });
}
```

#### 2. 在组件中使用

```typescript
// components/TenantList.tsx
import { useEffect, useState } from 'react';
import { getAuth9Client } from '@/lib/auth9-client';
import type { Tenant } from '@auth9/core';

export function TenantList() {
  const [tenants, setTenants] = useState<Tenant[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    async function fetchTenants() {
      try {
        const client = getAuth9Client(/* 从认证状态获取 token */);
        const response = await client.get<{ data: Tenant[] }>('/api/v1/tenants');
        setTenants(response.data);
      } catch (error) {
        console.error('Failed to fetch tenants:', error);
      } finally {
        setLoading(false);
      }
    }

    fetchTenants();
  }, []);

  if (loading) return <div>Loading...</div>;

  return (
    <ul>
      {tenants.map(tenant => (
        <li key={tenant.id}>{tenant.name}</li>
      ))}
    </ul>
  );
}
```

### 在 Next.js API Routes 中使用

```typescript
// app/api/tenants/route.ts
import { NextRequest, NextResponse } from 'next/server';
import { getAuth9Client } from '@/lib/auth9-client';

export async function GET(request: NextRequest) {
  try {
    // 从请求头获取 token
    const token = request.headers.get('Authorization')?.replace('Bearer ', '');
    
    if (!token) {
      return NextResponse.json({ error: 'Unauthorized' }, { status: 401 });
    }

    const client = getAuth9Client(token);
    const tenants = await client.get('/api/v1/tenants');
    
    return NextResponse.json(tenants);
  } catch (error) {
    return NextResponse.json(
      { error: 'Failed to fetch tenants' },
      { status: 500 }
    );
  }
}

export async function POST(request: NextRequest) {
  try {
    const token = request.headers.get('Authorization')?.replace('Bearer ', '');
    const body = await request.json();
    
    const client = getAuth9Client(token);
    const newTenant = await client.post('/api/v1/tenants', { body });
    
    return NextResponse.json(newTenant, { status: 201 });
  } catch (error) {
    return NextResponse.json(
      { error: 'Failed to create tenant' },
      { status: 500 }
    );
  }
}
```

### 在 Remix Loader/Action 中使用

```typescript
// app/routes/dashboard.tenants.tsx
import { json, type LoaderFunctionArgs, type ActionFunctionArgs } from '@remix-run/node';
import { useLoaderData, Form } from '@remix-run/react';
import { getAuth9Client } from '~/lib/auth9-client';

// Loader - 获取数据
export async function loader({ request }: LoaderFunctionArgs) {
  const token = /* 从 session 获取 */;
  const client = getAuth9Client(token);
  
  const tenants = await client.get('/api/v1/tenants');
  return json({ tenants });
}

// Action - 处理表单提交
export async function action({ request }: ActionFunctionArgs) {
  const token = /* 从 session 获取 */;
  const formData = await request.formData();
  
  const client = getAuth9Client(token);
  
  if (request.method === 'POST') {
    const newTenant = await client.post('/api/v1/tenants', {
      body: {
        name: formData.get('name'),
        slug: formData.get('slug')
      }
    });
    return json({ success: true, tenant: newTenant });
  }
  
  return json({ error: 'Invalid method' }, { status: 400 });
}

// 组件
export default function TenantsPage() {
  const { tenants } = useLoaderData<typeof loader>();
  
  return (
    <div>
      <h1>Tenants</h1>
      <ul>
        {tenants.data.map(tenant => (
          <li key={tenant.id}>{tenant.name}</li>
        ))}
      </ul>
      
      <Form method="post">
        <input name="name" placeholder="Name" required />
        <input name="slug" placeholder="Slug" required />
        <button type="submit">Create Tenant</button>
      </Form>
    </div>
  );
}
```

### Node.js 服务中使用

```typescript
// services/user-service.ts
import { Auth9HttpClient } from '@auth9/core';

class UserService {
  private client: Auth9HttpClient;

  constructor(accessToken: string) {
    this.client = new Auth9HttpClient({
      baseUrl: process.env.AUTH9_URL!,
      accessToken
    });
  }

  async getAllUsers(tenantId: string) {
    return this.client.get(`/api/v1/tenants/${tenantId}/users`);
  }

  async createUser(tenantId: string, userData: {
    email: string;
    name: string;
  }) {
    return this.client.post(`/api/v1/users`, {
      body: {
        ...userData,
        tenant_id: tenantId
      }
    });
  }

  async updateUser(userId: string, updates: Partial<{
    name: string;
    email: string;
  }>) {
    return this.client.patch(`/api/v1/users/${userId}`, {
      body: updates
    });
  }

  async deleteUser(userId: string) {
    return this.client.delete(`/api/v1/users/${userId}`);
  }
}

export default UserService;
```

## Action 辅助函数

SDK 提供了针对 Action API 的便捷辅助函数：

```typescript
import { getAuth9Client } from '@/lib/auth9-client';
import type { CreateActionInput, UpdateActionInput } from '@auth9/core';

// 创建租户作用域的 Action 客户端
function withTenant(client: Auth9HttpClient, tenantId: string) {
  return {
    actions: {
      // 列出所有 Actions
      async list(trigger?: string) {
        const params = trigger ? { trigger } : {};
        return client.get(`/api/v1/tenants/${tenantId}/actions`, { params });
      },

      // 获取单个 Action
      async get(id: string) {
        return client.get(`/api/v1/actions/${id}`);
      },

      // 创建 Action
      async create(input: CreateActionInput) {
        return client.post(`/api/v1/tenants/${tenantId}/actions`, {
          body: input
        });
      },

      // 更新 Action
      async update(id: string, input: UpdateActionInput) {
        return client.put(`/api/v1/actions/${id}`, {
          body: input
        });
      },

      // 删除 Action
      async delete(id: string) {
        return client.delete(`/api/v1/actions/${id}`);
      },

      // 测试 Action
      async test(id: string, context: ActionContext) {
        return client.post(`/api/v1/actions/${id}/test`, {
          body: { context }
        });
      },

      // 获取执行日志
      async logs(actionId?: string) {
        const path = actionId
          ? `/api/v1/actions/${actionId}/executions`
          : `/api/v1/tenants/${tenantId}/actions/executions`;
        return client.get(path);
      },

      // 获取统计信息
      async stats(id: string) {
        return client.get(`/api/v1/actions/${id}/stats`);
      }
    }
  };
}

// 使用示例
const client = getAuth9Client(token);
const tenantClient = withTenant(client, 'tenant_123');

// 列出所有 Actions
const actions = await tenantClient.actions.list();

// 创建新 Action
const newAction = await tenantClient.actions.create({
  name: 'Send Welcome Email',
  trigger: 'user.created',
  code: `
    async function handler(context) {
      console.log('User created:', context.user.email);
      return { success: true };
    }
  `,
  enabled: true
});

// 测试 Action
const testResult = await tenantClient.actions.test(newAction.id, {
  trigger: 'user.created',
  user: {
    id: 'user_123',
    email: 'test@example.com',
    name: 'Test User'
  },
  tenant: {
    id: 'tenant_123',
    name: 'Acme Corp',
    slug: 'acme'
  }
});

console.log('Test result:', testResult);
```

## 错误处理

SDK 使用标准的 HTTP 错误响应：

```typescript
import { Auth9HttpClient } from '@auth9/core';

const client = new Auth9HttpClient({ baseUrl, accessToken });

try {
  const tenant = await client.get('/api/v1/tenants/invalid_id');
} catch (error) {
  if (error.response) {
    // HTTP 错误响应
    console.error('Status:', error.response.status);
    console.error('Message:', error.response.data.message);
    
    switch (error.response.status) {
      case 401:
        // 未授权 - Token 无效或过期
        console.error('Authentication failed');
        break;
      case 403:
        // 禁止访问 - 权限不足
        console.error('Access denied');
        break;
      case 404:
        // 资源不存在
        console.error('Resource not found');
        break;
      case 500:
        // 服务器错误
        console.error('Server error');
        break;
    }
  } else if (error.request) {
    // 请求已发送但未收到响应
    console.error('Network error:', error.message);
  } else {
    // 请求配置错误
    console.error('Error:', error.message);
  }
}
```

### 封装错误处理

```typescript
// lib/api-client.ts
import { Auth9HttpClient } from '@auth9/core';

class ApiClient {
  private client: Auth9HttpClient;

  constructor(accessToken: string) {
    this.client = new Auth9HttpClient({
      baseUrl: process.env.AUTH9_URL!,
      accessToken
    });
  }

  async request<T>(
    method: 'get' | 'post' | 'put' | 'patch' | 'delete',
    path: string,
    options?: any
  ): Promise<{ data?: T; error?: string }> {
    try {
      const data = await this.client[method](path, options);
      return { data };
    } catch (error: any) {
      const message = error.response?.data?.message || error.message || 'Unknown error';
      console.error(`API Error [${method.toUpperCase()} ${path}]:`, message);
      return { error: message };
    }
  }

  async get<T>(path: string, options?: any) {
    return this.request<T>('get', path, options);
  }

  async post<T>(path: string, options?: any) {
    return this.request<T>('post', path, options);
  }

  async put<T>(path: string, options?: any) {
    return this.request<T>('put', path, options);
  }

  async patch<T>(path: string, options?: any) {
    return this.request<T>('patch', path, options);
  }

  async delete<T>(path: string, options?: any) {
    return this.request<T>('delete', path, options);
  }
}

export default ApiClient;
```

## 高级用法

### 自定义请求拦截器

```typescript
import { Auth9HttpClient } from '@auth9/core';

class CustomAuth9Client extends Auth9HttpClient {
  constructor(config) {
    super(config);
  }

  // 重写请求方法添加自定义逻辑
  async get(path: string, options?: any) {
    console.log(`[GET] ${path}`);
    const startTime = Date.now();
    
    try {
      const result = await super.get(path, options);
      console.log(`[GET] ${path} - ${Date.now() - startTime}ms`);
      return result;
    } catch (error) {
      console.error(`[GET] ${path} - Failed after ${Date.now() - startTime}ms`);
      throw error;
    }
  }
}
```

### 请求重试

```typescript
async function requestWithRetry<T>(
  client: Auth9HttpClient,
  method: 'get' | 'post' | 'put' | 'delete',
  path: string,
  options?: any,
  maxRetries = 3
): Promise<T> {
  let lastError: any;
  
  for (let i = 0; i < maxRetries; i++) {
    try {
      return await client[method](path, options);
    } catch (error: any) {
      lastError = error;
      
      // 只重试网络错误和 5xx 错误
      if (error.response?.status >= 500 || !error.response) {
        const delay = Math.pow(2, i) * 1000; // 指数退避
        console.log(`Retry ${i + 1}/${maxRetries} after ${delay}ms...`);
        await new Promise(resolve => setTimeout(resolve, delay));
        continue;
      }
      
      // 其他错误直接抛出
      throw error;
    }
  }
  
  throw lastError;
}

// 使用
const client = getAuth9Client(token);
const tenants = await requestWithRetry(client, 'get', '/api/v1/tenants');
```

### 批量操作

```typescript
async function batchCreateUsers(
  client: Auth9HttpClient,
  tenantId: string,
  users: Array<{ email: string; name: string }>
) {
  const results = await Promise.allSettled(
    users.map(user =>
      client.post('/api/v1/users', {
        body: {
          ...user,
          tenant_id: tenantId
        }
      })
    )
  );

  const succeeded = results.filter(r => r.status === 'fulfilled').length;
  const failed = results.filter(r => r.status === 'rejected').length;

  console.log(`Batch create: ${succeeded} succeeded, ${failed} failed`);

  return {
    succeeded,
    failed,
    results
  };
}
```

## 本地开发

如果您需要修改或扩展 SDK：

### 克隆仓库

```bash
git clone https://github.com/gpgkd906/auth9.git
cd auth9/sdk
```

### 安装依赖

```bash
pnpm install
```

### 构建 SDK

```bash
cd packages/core
pnpm build
```

### 运行测试

```bash
pnpm test
```

### 链接到本地项目

```bash
# 在 SDK 目录
cd packages/core
pnpm link --global

# 在您的项目目录
pnpm link --global @auth9/core
```

## 更新日志

### v0.1.0 (2026-02-12)

- ✨ 初始发布
- ✅ HTTP 客户端实现
- ✅ Action API 完整支持
- ✅ TypeScript 类型定义
- ✅ 基础错误处理
- 📚 完整文档和示例

### 未来计划

- 🚀 gRPC 客户端支持
- 🔄 自动重试和断路器
- 📊 请求追踪和监控
- 🔐 Token 自动刷新
- 📦 更多辅助函数和类型

## 相关资源

- **NPM 包**: [@auth9/core](https://www.npmjs.com/package/@auth9/core)
- **源代码**: [GitHub - auth9/sdk](https://github.com/gpgkd906/auth9/tree/main/sdk)
- **示例项目**: [auth9/sdk/examples](https://github.com/gpgkd906/auth9/tree/main/sdk/examples)
- **API 文档**: [REST API](REST-API.md)

## 获取帮助

- **GitHub Issues**: [提交问题](https://github.com/gpgkd906/auth9/issues)
- **GitHub Discussions**: [参与讨论](https://github.com/gpgkd906/auth9/discussions)

---

**最后更新**: 2026-02-16
**适用版本**: Auth9 v0.1.0+
