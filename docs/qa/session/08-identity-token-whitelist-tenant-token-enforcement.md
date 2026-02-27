# 会话与安全 - Identity Token 白名单与 Tenant Token 强制校验

**模块**: 会话与安全
**测试范围**: Identity Token 最小白名单、tenant 业务接口强制 Tenant Token、切租户后 token 生效边界
**场景数**: 5
**优先级**: 高

---

## 背景说明

本次会话安全收敛后，服务端对 Identity Token 的使用范围进行了限制：

1. Identity Token 仅允许访问最小白名单接口（如 `/api/v1/auth/*`、`/api/v1/users/me/tenants`、`GET /api/v1/tenants`）
2. tenant 业务接口（如 `/api/v1/tenants/{id}`、`/api/v1/tenants/{id}/*`）要求 Tenant Access Token

> **注意**: `GET /api/v1/tenants`（租户列表）是白名单接口，Identity Token 可以访问。Handler 会根据 token 类型自动过滤结果：Identity Token 仅返回用户自己的租户成员关系（通过 `resolve_tenant_list_mode_with_state` 策略）。这是设计行为，用于支持登录后的租户选择流程。
3. Portal 切换 tenant 时会触发 `POST /api/v1/auth/tenant-token` 重新交换 token

该文档用于验证“会话态 + token 类型 + 路由策略”三者一致性。

---

## 场景 1：租户切换入口可见性与 Identity Token 访问白名单接口成功

### 初始状态
- 已完成登录并获得 `{identity_token}`

### 目的
验证 Identity Token 在允许路径上可用，避免过度收敛导致登录后基础能力不可用

### 测试操作流程
1. 调用 `GET /api/v1/users/me/tenants`：

```bash
curl -i "http://localhost:8080/api/v1/users/me/tenants" \
  -H "Authorization: Bearer {identity_token}"
```

2. 调用 `GET /api/v1/auth/userinfo`：

```bash
curl -i "http://localhost:8080/api/v1/auth/userinfo" \
  -H "Authorization: Bearer {identity_token}"
```

### 预期结果
- 两个请求均返回 `200 OK`
- 返回体包含当前用户信息/租户列表

---

## 场景 2：Identity Token 访问 tenant 业务接口被拒绝

### 初始状态
- 已完成登录并获得 `{identity_token}`

### 目的
验证非白名单 tenant 路由不再接受 Identity Token

### 测试操作流程
1. 调用 `GET /api/v1/tenants`（**白名单接口，预期成功**）：

```bash
curl -i "http://localhost:8080/api/v1/tenants" \
  -H "Authorization: Bearer {identity_token}"
```

2. 调用 `GET /api/v1/tenants/{tenant_id}`（**非白名单接口，预期拒绝**）：

```bash
curl -i "http://localhost:8080/api/v1/tenants/{tenant_id}" \
  -H "Authorization: Bearer {identity_token}"
```

3. 调用 `PUT /api/v1/tenants/{tenant_id}`（**非白名单接口，预期拒绝**）：

```bash
curl -i -X PUT "http://localhost:8080/api/v1/tenants/{tenant_id}" \
  -H "Authorization: Bearer {identity_token}" \
  -H "Content-Type: application/json" \
  -d '{"name":"test"}'
```

### 预期结果
- **步骤 1**: `GET /api/v1/tenants` 返回 `200 OK`，且仅返回当前用户的租户成员关系（非全量租户列表）
- **步骤 2**: `GET /api/v1/tenants/{tenant_id}` 返回 `403 FORBIDDEN`，提示 `"Identity token is only allowed for tenant selection and exchange"`
- **步骤 3**: `PUT /api/v1/tenants/{tenant_id}` 返回 `403 FORBIDDEN`

### 常见误报

| 症状 | 原因 | 结论 |
|------|------|------|
| `GET /api/v1/tenants` 返回 200 | 该接口是白名单接口，Identity Token 允许访问 | **非漏洞** — 设计行为，用于租户选择 |
| 平台管理员 `GET /api/v1/tenants` 返回全部租户 | 平台管理员拥有全局绕过 | 使用非平台管理员用户验证过滤效果 |

---

## 场景 3：使用 Tenant Token 访问 tenant 接口成功

### 初始状态
- 已有 `{identity_token}`
- 用户属于 `{tenant_id}`，且知道 `{service_client_id}`（如 `auth9-portal`）

### 目的
验证通过 exchange 获取 Tenant Token 后 tenant API 可正常访问

### 测试操作流程
1. 交换 Tenant Token：

```bash
curl -s -X POST "http://localhost:8080/api/v1/auth/tenant-token" \
  -H "Authorization: Bearer {identity_token}" \
  -H "Content-Type: application/json" \
  -d '{"tenant_id":"{tenant_id}","service_id":"{service_client_id}"}'
```

2. 使用返回的 `{tenant_access_token}` 调用 tenant 接口：

```bash
curl -i "http://localhost:8080/api/v1/tenants/{tenant_id}" \
  -H "Authorization: Bearer {tenant_access_token}"
```

### 预期结果
- 第 1 步返回 `200` 且包含 `access_token`
- 第 2 步返回 `200 OK`
- 返回租户信息与 `{tenant_id}` 一致

---

## 场景 4：tenant 与 service 不匹配时 exchange 被拒绝

### 初始状态
- 已有 `{identity_token}`
- 准备一个与目标 tenant 不匹配的 `{service_client_id_other_tenant}`

### 目的
验证 token exchange 不允许跨 tenant 滥用 service/client

### 测试操作流程
1. 调用 exchange：

```bash
curl -i -X POST "http://localhost:8080/api/v1/auth/tenant-token" \
  -H "Authorization: Bearer {identity_token}" \
  -H "Content-Type: application/json" \
  -d '{"tenant_id":"{tenant_id}","service_id":"{service_client_id_other_tenant}"}'
```

### 预期结果
- 返回 `403 FORBIDDEN`
- 错误信息指示 service 不属于请求 tenant（或等价语义）

---

## 场景 5：Portal 切换 tenant 后旧 token 不应继续用于新 tenant 资源

### 初始状态
- **必须使用非平台管理员用户**（平台管理员 email 配置在 `PLATFORM_ADMIN_EMAILS` 中，其拥有跨租户访问特权，会绕过 tenant scope 校验）
- 用户属于 `{tenant_a}` 与 `{tenant_b}`
- 已在 Portal 中选中 `{tenant_a}` 并拿到 `{token_a}`

### 目的
验证切租户后必须使用新交换 token，旧 token 不可跨租户访问

### 测试操作流程
1. 在 Portal 侧边栏切换到 `{tenant_b}`
2. 记录网络请求中 `POST /api/v1/auth/tenant-token` 成功响应，得到 `{token_b}`
3. 使用 `{token_a}` 请求 `{tenant_b}` 资源：

```bash
curl -i "http://localhost:8080/api/v1/tenants/{tenant_b}" \
  -H "Authorization: Bearer {token_a}"
```

4. 使用 `{token_b}` 请求同一资源：

```bash
curl -i "http://localhost:8080/api/v1/tenants/{tenant_b}" \
  -H "Authorization: Bearer {token_b}"
```

### 预期结果
- 切换动作触发新的 tenant token exchange
- 第 3 步返回 `403 FORBIDDEN`
- 第 4 步返回 `200 OK`

### 故障排查

| 症状 | 原因 | 解决 |
|------|------|------|
| 第 3 步返回 200 而非 403 | 使用了平台管理员账号（平台管理员可跨租户访问） | 使用非平台管理员用户重新测试 |
| 两个请求都返回 403 | token 已过期或签名无效 | 确认 token exchange 正常并获取新 token |

---

## 检查清单

| # | 场景 | 状态 | 测试日期 | 测试人员 | 备注 |
|---|------|------|----------|----------|------|
| 1 | Identity Token 访问白名单接口成功 | ☐ | | | |
| 2 | Identity Token 访问 tenant 业务接口被拒绝 | ☐ | | | |
| 3 | 使用 Tenant Token 访问 tenant 接口成功 | ☐ | | | |
| 4 | tenant 与 service 不匹配时 exchange 被拒绝 | ☐ | | | |
| 5 | Portal 切换 tenant 后旧 token 不应继续用于新 tenant 资源 | ☐ | | | |
