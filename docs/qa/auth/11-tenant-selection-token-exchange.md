# 认证流程 - Tenant 选择与 Tenant Token Exchange

**模块**: 认证流程
**测试范围**: 登录后 tenant 选择页、切换 tenant 时 token exchange、Identity Token 权限收敛、gRPC 使用 tenant token
**场景数**: 5
**优先级**: 高

---

## 背景说明

本次改动将多租户登录后的入口从直接进入 Dashboard 调整为先进入 tenant 选择，并引入 HTTP 交换端点：

- `POST /api/v1/auth/tenant-token`

核心约束：
1. 多租户用户登录后必须在 `/tenant/select` 明确选择 tenant 并完成 token exchange
2. Dashboard 侧边栏切换 tenant 时必须再次调用 token exchange
3. Identity Token 仅允许最小白名单接口；tenant 业务接口需使用 Tenant Access Token
4. gRPC 调用应使用 tenant token（identity token 仅用于 exchange）

---

## 场景 1：tenant 选择入口可见性与多租户登录分流

### 初始状态
- 用户已属于 2 个或以上 active tenant
- 用户通过任一登录方式完成认证（OIDC/SSO/Passkey）

### 目的
验证登录后不会直接进入 Dashboard，而是强制用户显式选择 tenant

### 测试操作流程
1. 访问 `/login` 并完成登录
2. 观察登录回调后的跳转路径
3. 在 `/tenant/select` 页面选择一个 tenant，点击「Continue」
4. 观察页面跳转和组织上下文

### 预期结果
- 登录成功后跳转到 `/tenant/select`
- 页面展示用户可访问的 tenant 列表（单选）
- 点击「Continue」后调用 `POST /api/v1/auth/tenant-token`
- 成功后跳转到 `/dashboard`
- Dashboard 中组织切换器显示刚才选择的 tenant

---

## 场景 2：单租户用户自动 exchange 并进入 Dashboard

### 初始状态
- 用户仅属于 1 个 active tenant

### 目的
验证单租户场景不要求手工选择，但仍执行 token exchange

### 测试操作流程
1. 使用单租户账号登录
2. 观察登录后的跳转链路
3. 在浏览器 Network 面板确认是否调用 `POST /api/v1/auth/tenant-token`

### 预期结果
- 登录后短暂经过 `/tenant/select` loader 逻辑（可无可视停留）
- 自动调用 `POST /api/v1/auth/tenant-token`
- 最终进入 `/dashboard`
- Dashboard 的活跃 tenant 为该唯一 tenant

---

## 场景 3：Dashboard 切换 tenant 必须重新 exchange

### 初始状态
- 用户已登录且属于至少 2 个 tenant
- 当前位于 `/dashboard`

### 目的
验证组织切换时不仅更新 `activeTenantId`，还会重新交换 tenant token

### 测试操作流程
1. 在侧边栏组织切换器点击另一个 tenant
2. 在 Network 面板观察请求
3. 切换后刷新当前页面
4. 访问一个 tenant scoped 页面（例如 `/dashboard/tenants/{tenant_id}/users` 对应页面入口）

### 预期结果
- 切换动作触发 `POST /api/v1/auth/tenant-token`
- 成功后返回 `/dashboard`，组织显示为新 tenant
- 刷新后 tenant 上下文保持
- 后续 tenant 业务请求使用新 tenant token，权限与 tenant 对应

---

## 场景 4：Identity Token 访问 tenant 业务接口被拒绝

### 初始状态
- 已获取一个有效 Identity Token（登录后 `access_token`）
- 未进行 tenant token exchange

### 目的
验证 Identity Token 权限收窄生效，tenant 业务接口拒绝 identity token

### 测试操作流程
1. 使用 Identity Token 调用 tenant API

```bash
curl -i http://localhost:8080/api/v1/tenants \
  -H "Authorization: Bearer {identity_token}"
```

2. 使用同一 Identity Token 调用白名单接口

```bash
curl -i http://localhost:8080/api/v1/users/me/tenants \
  -H "Authorization: Bearer {identity_token}"
```

### 预期结果
- 调用 `/api/v1/tenants` 返回 `403 FORBIDDEN`
- 错误信息包含 identity token 仅用于 tenant 选择/交换的语义
- 调用 `/api/v1/users/me/tenants` 返回 `200 OK`

---

## 场景 5：gRPC 业务调用使用 tenant token（identity token 仅用于 exchange）

### 初始状态
- 已登录 auth9-demo，session 中有 `identityToken`
- 已知一个目标 tenant（`{tenant_id}`）

### 目的
验证 gRPC 交互链路中 identity token 只用于 exchange，业务鉴权使用 tenant token

### 测试操作流程
1. 在 Demo Dashboard 点击「Exchange Token」
2. 确认 `/demo/exchange-token` 返回 `accessToken`
3. 使用返回的 tenant token 请求受保护 API：

```bash
curl -i http://localhost:3002/api/resources \
  -H "Authorization: Bearer {tenant_access_token}"
```

4. 使用 identity token 请求同一接口：

```bash
curl -i http://localhost:3002/api/resources \
  -H "Authorization: Bearer {identity_token}"
```

### 预期结果
- 第 1 步成功后，Demo 页面显示当前 `Tenant Access Token`
- 第 3 步返回 `200 OK`
- 第 4 步返回 `401/403`（不允许使用 identity token 直接访问 tenant 业务）

---

## 检查清单

| # | 场景 | 状态 | 测试日期 | 测试人员 | 备注 |
|---|------|------|----------|----------|------|
| 1 | 多租户用户登录后强制进入 tenant 选择页 | ☐ | | | |
| 2 | 单租户用户自动 exchange 并进入 Dashboard | ☐ | | | |
| 3 | Dashboard 切换 tenant 必须重新 exchange | ☐ | | | |
| 4 | Identity Token 访问 tenant 业务接口被拒绝 | ☐ | | | |
| 5 | gRPC 业务调用使用 tenant token（identity token 仅用于 exchange） | ☐ | | | |
