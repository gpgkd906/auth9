# 会话与安全 - Identity Token 白名单与 Tenant Token 强制校验

**模块**: 会话与安全
**测试范围**: Identity Token 最小白名单、tenant 业务接口强制 Tenant Token、切租户后 token 生效边界
**场景数**: 5
**优先级**: 高

---

## 背景说明

本次会话安全收敛后，服务端对 Identity Token 的使用范围进行了限制：

1. Identity Token 仅允许访问最小白名单接口（如 `/api/v1/auth/*`、`/api/v1/users/me/tenants`）
2. tenant 业务接口（如 `/api/v1/tenants`、`/api/v1/tenants/{id}/*`）要求 Tenant Access Token
3. Portal 切换 tenant 时会触发 `POST /api/v1/auth/tenant-token` 重新交换 token

该文档用于验证“会话态 + token 类型 + 路由策略”三者一致性。

---

## 场景 1：Identity Token 访问白名单接口成功

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
1. 调用 `GET /api/v1/tenants`：

```bash
curl -i "http://localhost:8080/api/v1/tenants" \
  -H "Authorization: Bearer {identity_token}"
```

2. 调用 `GET /api/v1/tenants/{tenant_id}`：

```bash
curl -i "http://localhost:8080/api/v1/tenants/{tenant_id}" \
  -H "Authorization: Bearer {identity_token}"
```

### 预期结果
- 请求返回 `403 FORBIDDEN`
- 错误语义明确提示需使用 Tenant Token（或先完成 tenant token exchange）

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

---

## 检查清单

| # | 场景 | 状态 | 测试日期 | 测试人员 | 备注 |
|---|------|------|----------|----------|------|
| 1 | Identity Token 访问白名单接口成功 | ☐ | | | |
| 2 | Identity Token 访问 tenant 业务接口被拒绝 | ☐ | | | |
| 3 | 使用 Tenant Token 访问 tenant 接口成功 | ☐ | | | |
| 4 | tenant 与 service 不匹配时 exchange 被拒绝 | ☐ | | | |
| 5 | Portal 切换 tenant 后旧 token 不应继续用于新 tenant 资源 | ☐ | | | |
