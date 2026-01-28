# REST API

Auth9 提供完整的 REST API，用于管理租户、用户、服务和权限。

## API 基础

### Base URL

```
http://localhost:8080/api/v1
```

生产环境：

```
https://api.auth9.yourdomain.com/api/v1
```

### 认证方式

所有 API 请求需要在 Header 中携带 JWT Token：

```http
Authorization: Bearer <your-jwt-token>
```

### 请求格式

- Content-Type: `application/json`
- Accept: `application/json`

### 响应格式

成功响应：

```json
{
  "data": { ... },
  "meta": {
    "page": 1,
    "per_page": 20,
    "total": 100
  }
}
```

错误响应：

```json
{
  "error": {
    "code": "INVALID_INPUT",
    "message": "租户名称不能为空",
    "details": {
      "field": "name",
      "reason": "required"
    }
  }
}
```

### HTTP 状态码

| 状态码 | 说明 |
|-------|------|
| 200 | 成功 |
| 201 | 创建成功 |
| 204 | 成功（无内容） |
| 400 | 请求参数错误 |
| 401 | 未认证 |
| 403 | 无权限 |
| 404 | 资源不存在 |
| 409 | 冲突（如重复创建） |
| 422 | 验证失败 |
| 500 | 服务器错误 |

## 认证 API

### 登录

```http
POST /api/v1/auth/login
Content-Type: application/json

{
  "email": "user@example.com",
  "password": "password123"
}
```

响应：

```json
{
  "data": {
    "access_token": "eyJhbGciOiJIUzI1NiIs...",
    "token_type": "Bearer",
    "expires_in": 3600,
    "refresh_token": "refresh_token_here"
  }
}
```

### 刷新 Token

```http
POST /api/v1/auth/refresh
Content-Type: application/json

{
  "refresh_token": "refresh_token_here"
}
```

### 登出

```http
POST /api/v1/auth/logout
Authorization: Bearer <token>
```

### 获取当前用户信息

```http
GET /api/v1/auth/me
Authorization: Bearer <token>
```

响应：

```json
{
  "data": {
    "id": "user-uuid",
    "email": "user@example.com",
    "display_name": "User Name",
    "avatar_url": "https://...",
    "tenants": [
      {
        "id": "tenant-uuid",
        "name": "My Company",
        "role": "admin"
      }
    ]
  }
}
```

## 租户 API

### 获取租户列表

```http
GET /api/v1/tenants?page=1&per_page=20&status=active
Authorization: Bearer <token>
```

查询参数：

| 参数 | 类型 | 说明 |
|-----|------|------|
| page | integer | 页码，默认 1 |
| per_page | integer | 每页数量，默认 20 |
| status | string | 状态筛选：active, disabled |
| search | string | 搜索关键词 |

响应：

```json
{
  "data": [
    {
      "id": "tenant-uuid",
      "name": "公司名称",
      "slug": "company-slug",
      "logo_url": "https://...",
      "status": "active",
      "settings": {
        "require_mfa": false,
        "password_policy": "strong"
      },
      "created_at": "2024-01-01T00:00:00Z",
      "updated_at": "2024-01-01T00:00:00Z"
    }
  ],
  "meta": {
    "page": 1,
    "per_page": 20,
    "total": 100,
    "total_pages": 5
  }
}
```

### 创建租户

```http
POST /api/v1/tenants
Authorization: Bearer <token>
Content-Type: application/json

{
  "name": "公司名称",
  "slug": "company-slug",
  "logo_url": "https://...",
  "settings": {
    "require_mfa": false,
    "password_policy": "strong"
  }
}
```

响应：

```json
{
  "data": {
    "id": "tenant-uuid",
    "name": "公司名称",
    "slug": "company-slug",
    "status": "active",
    "created_at": "2024-01-01T00:00:00Z"
  }
}
```

### 获取租户详情

```http
GET /api/v1/tenants/{tenant_id}
Authorization: Bearer <token>
```

### 更新租户

```http
PUT /api/v1/tenants/{tenant_id}
Authorization: Bearer <token>
Content-Type: application/json

{
  "name": "新的公司名称",
  "logo_url": "https://...",
  "settings": {
    "require_mfa": true
  }
}
```

### 禁用租户

```http
DELETE /api/v1/tenants/{tenant_id}
Authorization: Bearer <token>
```

## 用户 API

### 获取用户列表

```http
GET /api/v1/users?page=1&per_page=20&tenant_id=xxx
Authorization: Bearer <token>
```

查询参数：

| 参数 | 类型 | 说明 |
|-----|------|------|
| page | integer | 页码 |
| per_page | integer | 每页数量 |
| tenant_id | uuid | 租户 ID 筛选 |
| search | string | 搜索（邮箱、姓名） |
| mfa_enabled | boolean | MFA 状态筛选 |

响应：

```json
{
  "data": [
    {
      "id": "user-uuid",
      "email": "user@example.com",
      "display_name": "张三",
      "avatar_url": "https://...",
      "mfa_enabled": true,
      "created_at": "2024-01-01T00:00:00Z",
      "tenants": [
        {
          "tenant_id": "tenant-uuid",
          "tenant_name": "公司名称",
          "role": "editor"
        }
      ]
    }
  ],
  "meta": {
    "page": 1,
    "per_page": 20,
    "total": 50
  }
}
```

### 创建用户

```http
POST /api/v1/users
Authorization: Bearer <token>
Content-Type: application/json

{
  "email": "newuser@example.com",
  "display_name": "新用户",
  "password": "SecurePassword123!",
  "tenant_id": "tenant-uuid",
  "send_welcome_email": true
}
```

### 获取用户详情

```http
GET /api/v1/users/{user_id}
Authorization: Bearer <token>
```

响应：

```json
{
  "data": {
    "id": "user-uuid",
    "email": "user@example.com",
    "display_name": "张三",
    "avatar_url": "https://...",
    "mfa_enabled": true,
    "created_at": "2024-01-01T00:00:00Z",
    "updated_at": "2024-01-01T00:00:00Z",
    "tenants": [
      {
        "tenant_id": "tenant-uuid",
        "tenant_name": "公司名称",
        "joined_at": "2024-01-01T00:00:00Z",
        "roles": ["editor", "viewer"]
      }
    ]
  }
}
```

### 更新用户

```http
PUT /api/v1/users/{user_id}
Authorization: Bearer <token>
Content-Type: application/json

{
  "display_name": "新名称",
  "avatar_url": "https://..."
}
```

### 重置用户密码

```http
POST /api/v1/users/{user_id}/reset-password
Authorization: Bearer <token>
Content-Type: application/json

{
  "new_password": "NewPassword123!",
  "send_email": true
}
```

### 启用/禁用用户 MFA

```http
POST /api/v1/users/{user_id}/mfa
Authorization: Bearer <token>
Content-Type: application/json

{
  "enabled": true
}
```

### 添加用户到租户

```http
POST /api/v1/users/{user_id}/tenants
Authorization: Bearer <token>
Content-Type: application/json

{
  "tenant_id": "tenant-uuid"
}
```

### 从租户移除用户

```http
DELETE /api/v1/users/{user_id}/tenants/{tenant_id}
Authorization: Bearer <token>
```

## 服务 API

### 获取服务列表

```http
GET /api/v1/services?tenant_id=xxx
Authorization: Bearer <token>
```

响应：

```json
{
  "data": [
    {
      "id": "service-uuid",
      "tenant_id": "tenant-uuid",
      "name": "我的应用",
      "client_id": "app-client-id",
      "base_url": "https://app.example.com",
      "redirect_uris": [
        "https://app.example.com/callback"
      ],
      "logout_uris": [
        "https://app.example.com/logout"
      ],
      "status": "active",
      "created_at": "2024-01-01T00:00:00Z"
    }
  ]
}
```

### 注册服务

```http
POST /api/v1/services
Authorization: Bearer <token>
Content-Type: application/json

{
  "tenant_id": "tenant-uuid",
  "name": "我的应用",
  "base_url": "https://app.example.com",
  "redirect_uris": [
    "https://app.example.com/callback"
  ],
  "logout_uris": [
    "https://app.example.com/logout"
  ]
}
```

响应：

```json
{
  "data": {
    "id": "service-uuid",
    "client_id": "generated-client-id",
    "client_secret": "generated-client-secret",
    "name": "我的应用"
  }
}
```

### 获取服务详情

```http
GET /api/v1/services/{service_id}
Authorization: Bearer <token>
```

### 更新服务

```http
PUT /api/v1/services/{service_id}
Authorization: Bearer <token>
Content-Type: application/json

{
  "name": "新的应用名称",
  "redirect_uris": [
    "https://app.example.com/callback",
    "https://app.example.com/another-callback"
  ]
}
```

### 重新生成服务密钥

```http
POST /api/v1/services/{service_id}/regenerate-secret
Authorization: Bearer <token>
```

响应：

```json
{
  "data": {
    "client_secret": "new-generated-secret"
  }
}
```

### 删除服务

```http
DELETE /api/v1/services/{service_id}
Authorization: Bearer <token>
```

## 角色与权限 API

### 获取服务的角色列表

```http
GET /api/v1/services/{service_id}/roles
Authorization: Bearer <token>
```

响应：

```json
{
  "data": [
    {
      "id": "role-uuid",
      "service_id": "service-uuid",
      "name": "editor",
      "display_name": "编辑者",
      "description": "可以编辑内容",
      "parent_role_id": null,
      "permissions": [
        {
          "id": "perm-uuid",
          "code": "content:write",
          "name": "编辑内容"
        }
      ]
    }
  ]
}
```

### 创建角色

```http
POST /api/v1/services/{service_id}/roles
Authorization: Bearer <token>
Content-Type: application/json

{
  "name": "editor",
  "display_name": "编辑者",
  "description": "可以编辑内容",
  "parent_role_id": null
}
```

### 获取服务的权限列表

```http
GET /api/v1/services/{service_id}/permissions
Authorization: Bearer <token>
```

响应：

```json
{
  "data": [
    {
      "id": "perm-uuid",
      "service_id": "service-uuid",
      "code": "content:write",
      "name": "编辑内容",
      "description": "允许创建和修改内容"
    }
  ]
}
```

### 创建权限

```http
POST /api/v1/services/{service_id}/permissions
Authorization: Bearer <token>
Content-Type: application/json

{
  "code": "content:write",
  "name": "编辑内容",
  "description": "允许创建和修改内容"
}
```

### 为角色分配权限

```http
POST /api/v1/roles/{role_id}/permissions
Authorization: Bearer <token>
Content-Type: application/json

{
  "permission_ids": [
    "perm-uuid-1",
    "perm-uuid-2"
  ]
}
```

### 移除角色的权限

```http
DELETE /api/v1/roles/{role_id}/permissions/{permission_id}
Authorization: Bearer <token>
```

## RBAC API

### 为用户分配角色

```http
POST /api/v1/rbac/assign
Authorization: Bearer <token>
Content-Type: application/json

{
  "user_id": "user-uuid",
  "tenant_id": "tenant-uuid",
  "role_ids": [
    "role-uuid-1",
    "role-uuid-2"
  ]
}
```

### 获取用户在租户中的角色

```http
GET /api/v1/rbac/user-roles?user_id=xxx&tenant_id=xxx
Authorization: Bearer <token>
```

响应：

```json
{
  "data": {
    "user_id": "user-uuid",
    "tenant_id": "tenant-uuid",
    "roles": [
      {
        "id": "role-uuid",
        "name": "editor",
        "service_name": "我的应用",
        "granted_at": "2024-01-01T00:00:00Z",
        "granted_by": "admin-uuid"
      }
    ]
  }
}
```

### 撤销用户角色

```http
DELETE /api/v1/rbac/revoke
Authorization: Bearer <token>
Content-Type: application/json

{
  "user_id": "user-uuid",
  "tenant_id": "tenant-uuid",
  "role_id": "role-uuid"
}
```

## 审计日志 API

### 查询审计日志

```http
GET /api/v1/audit-logs?page=1&per_page=50&tenant_id=xxx
Authorization: Bearer <token>
```

查询参数：

| 参数 | 类型 | 说明 |
|-----|------|------|
| page | integer | 页码 |
| per_page | integer | 每页数量（最大 100） |
| tenant_id | uuid | 租户筛选 |
| actor_id | uuid | 操作者筛选 |
| action | string | 操作类型筛选 |
| resource_type | string | 资源类型筛选 |
| start_date | datetime | 开始时间 |
| end_date | datetime | 结束时间 |

响应：

```json
{
  "data": [
    {
      "id": 12345,
      "actor_id": "user-uuid",
      "actor_name": "张三",
      "action": "user.created",
      "resource_type": "user",
      "resource_id": "new-user-uuid",
      "old_value": null,
      "new_value": {
        "email": "newuser@example.com",
        "display_name": "新用户"
      },
      "ip_address": "192.168.1.100",
      "created_at": "2024-01-01T12:00:00Z"
    }
  ],
  "meta": {
    "page": 1,
    "per_page": 50,
    "total": 1000
  }
}
```

## 健康检查 API

### 健康检查

```http
GET /health
```

响应：

```json
{
  "status": "healthy",
  "version": "0.1.0",
  "checks": {
    "database": "ok",
    "redis": "ok",
    "keycloak": "ok"
  }
}
```

### Readiness 检查

```http
GET /ready
```

## OpenID Connect

### 获取 OIDC 配置

```http
GET /.well-known/openid-configuration
```

响应：

```json
{
  "issuer": "https://auth9.yourdomain.com",
  "authorization_endpoint": "https://auth9.yourdomain.com/auth",
  "token_endpoint": "https://auth9.yourdomain.com/token",
  "userinfo_endpoint": "https://auth9.yourdomain.com/userinfo",
  "jwks_uri": "https://auth9.yourdomain.com/.well-known/jwks.json",
  "response_types_supported": ["code", "token", "id_token"],
  "subject_types_supported": ["public"],
  "id_token_signing_alg_values_supported": ["RS256", "HS256"]
}
```

### 获取 JWKS

```http
GET /.well-known/jwks.json
```

## 错误代码

| 错误代码 | 说明 |
|---------|------|
| `INVALID_INPUT` | 输入参数无效 |
| `UNAUTHORIZED` | 未认证 |
| `FORBIDDEN` | 无权限 |
| `NOT_FOUND` | 资源不存在 |
| `CONFLICT` | 资源冲突 |
| `TENANT_NOT_FOUND` | 租户不存在 |
| `USER_NOT_FOUND` | 用户不存在 |
| `SERVICE_NOT_FOUND` | 服务不存在 |
| `ROLE_NOT_FOUND` | 角色不存在 |
| `INVALID_CREDENTIALS` | 凭证无效 |
| `TOKEN_EXPIRED` | Token 过期 |
| `TOKEN_INVALID` | Token 无效 |
| `INTERNAL_ERROR` | 服务器内部错误 |

## 限流

API 实施限流策略：

- 匿名请求：100 req/min
- 认证请求：1000 req/min
- 管理员：10000 req/min

响应头：

```http
X-RateLimit-Limit: 1000
X-RateLimit-Remaining: 995
X-RateLimit-Reset: 1640995200
```

## 相关文档

- [gRPC API](gRPC-API.md)
- [Token 规范](Token规范.md)
- [认证流程](认证流程.md)
