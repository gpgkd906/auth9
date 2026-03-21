# 集成测试 - 安全加固配置与运行时行为

**模块**: 集成测试
**测试范围**: 生产环境安全配置校验、gRPC 鉴权强制、Tenant Access Token aud 严格校验、HSTS 条件下发
**场景数**: 5
**优先级**: 高

---

## 背景说明

本次变更引入了三类安全收敛：

- `ENVIRONMENT=production` 下启动时执行 fail-fast 安全校验
- gRPC 不允许以无鉴权模式在生产环境启动
- REST 侧 Tenant Access Token 必须通过 audience allowlist 校验
- HSTS 改为按环境与 HTTPS 条件下发

关键配置项：

- `ENVIRONMENT`
- `GRPC_AUTH_MODE`
- `GRPC_API_KEYS`
- `JWT_TENANT_ACCESS_ALLOWED_AUDIENCES`
- `AUTH9_PORTAL_CLIENT_ID`
- `HSTS_ENABLED`
- `HSTS_HTTPS_ONLY`
- `HSTS_TRUST_X_FORWARDED_PROTO`

---

## 场景 1：production 下 `GRPC_AUTH_MODE=none` 启动失败

### 初始状态
- 本地代码已更新到包含安全校验逻辑的版本
- 可执行文件可通过 `cargo run --bin auth9-core -- serve` 启动

### 目的
验证生产环境禁止 gRPC 无鉴权启动

### 测试操作流程
1. 执行命令（仅用于验证启动前校验）：
   ```bash
   cd auth9-core
   ENVIRONMENT=production \
   DATABASE_URL='mysql://{user}:{password}@{host}:{port}/{db}' \
   JWT_SECRET='{jwt_secret}' \
   GRPC_AUTH_MODE=none \
   cargo run --bin auth9-core -- serve
   ```
2. 观察进程退出码与错误日志

### 预期结果
- 进程启动失败并退出（非 0）
- 日志包含类似信息：`gRPC authentication is disabled (GRPC_AUTH_MODE=none) in production`

---

## 场景 2：production 下 `api_key` 但未配置 keys 启动失败

### 初始状态
- 与场景 1 相同

### 目的
验证 production 下 `GRPC_AUTH_MODE=api_key` 时必须配置 `GRPC_API_KEYS`

### 测试操作流程
1. 执行命令：
   ```bash
   cd auth9-core
   ENVIRONMENT=production \
   DATABASE_URL='mysql://{user}:{password}@{host}:{port}/{db}' \
   JWT_SECRET='{jwt_secret}' \
   GRPC_AUTH_MODE=api_key \
   GRPC_API_KEYS='' \
   cargo run --bin auth9-core -- serve
   ```
2. 观察启动日志

### 预期结果
- 进程启动失败并退出（非 0）
- 日志包含类似信息：`gRPC auth_mode is api_key but no keys configured`

---

## 场景 3：production 下 tenant token audience allowlist 缺失启动失败

### 初始状态
- 与场景 1 相同

### 目的
验证 production 必须配置 Tenant Access Token audience allowlist

> **注意：动态 audience 加载机制**
> `JWT_TENANT_ACCESS_ALLOWED_AUDIENCES` 环境变量现在是可选的。系统在启动时会从 `clients` 表动态加载 audience 到 Redis 缓存中（参见 `config/mod.rs` 注释："jwt_tenant_access_allowed_audiences is now optional — audiences are dynamically loaded from the clients table into Redis on startup."）。
> 因此，此场景仅在 `clients` 表也为空且 `AUTH9_PORTAL_CLIENT_ID` 也未设置时才会触发启动失败。如果数据库中存在 client 记录，即使 `JWT_TENANT_ACCESS_ALLOWED_AUDIENCES` 为空，系统仍可正常启动。

### 测试操作流程
1. 执行命令（同时清空显式 allowlist 与 portal client id，**且确保数据库 `clients` 表为空**）：
   ```bash
   cd auth9-core
   ENVIRONMENT=production \
   DATABASE_URL='mysql://{user}:{password}@{host}:{port}/{db}' \
   JWT_SECRET='{jwt_secret}' \
   GRPC_AUTH_MODE=api_key \
   GRPC_API_KEYS='{grpc_api_key}' \
   JWT_TENANT_ACCESS_ALLOWED_AUDIENCES='' \
   AUTH9_PORTAL_CLIENT_ID='' \
   cargo run --bin auth9-core -- serve
   ```
2. 观察启动日志

### 预期结果
- 若 `clients` 表为空：进程启动失败并退出（非 0），日志包含类似信息：`Tenant access token audience allowlist is empty in production`
- 若 `clients` 表中有记录：系统正常启动，audience 从 `clients` 表动态加载到 Redis

---

## 场景 4：REST 侧 Tenant Access Token aud 严格校验

### 初始状态
- 服务以 `ENVIRONMENT=production` 启动成功
- 已准备两个 Tenant Access Token：
  - `{tenant_token_aud_auth9_portal}`（`aud=auth9-portal`）
  - `{tenant_token_aud_other}`（`aud=other-service`，且 `other-service` 不在 `clients` 表中）

> **重要：动态 audience 验证机制**
> REST 侧的 audience 校验通过 `require_auth` 中间件执行，从 Redis 缓存查找有效 audience。
> Audience 来源有两个（合并生效）：
> 1. `JWT_TENANT_ACCESS_ALLOWED_AUDIENCES` 环境变量（静态配置）
> 2. `clients` 表中的 `client_id`（启动时动态加载到 Redis）
>
> 因此，如果某个 `client_id`（如 `auth9-demo`）存在于 `clients` 表中，它会被自动加载为有效 audience，即使未出现在 `JWT_TENANT_ACCESS_ALLOWED_AUDIENCES` 中。测试 audience 拒绝场景时，必须使用一个**不在 `clients` 表中**的 audience 值。
>
> **测试本场景必须**：
> 1. 在 docker-compose.yml 的 auth9-core 环境变量中添加 `ENVIRONMENT: production`
> 2. （可选）添加 `JWT_TENANT_ACCESS_ALLOWED_AUDIENCES: auth9-portal`
> 3. 重启 auth9-core 容器使配置生效
> 4. Token 必须通过 gRPC TokenExchange 生成（`service_id` 对应的 client 的 `client_id` 即为 token 的 `aud` 值）
> 5. 确认用于"拒绝"测试的 audience 值不在 `clients` 表中：`SELECT client_id FROM clients WHERE client_id = 'other-service';` 应返回空

### 目的
验证 REST 认证链路只接受有效 audience（静态 allowlist + 动态 clients 表）内的 aud

### 测试操作流程
1. 使用 allowlist 内 token 访问受保护接口：
   ```bash
   curl -i 'http://localhost:8080/api/v1/auth/userinfo' \
     -H 'Authorization: Bearer {tenant_token_aud_auth9_portal}'
   ```
2. 使用 allowlist 外 token 访问同一接口：
   ```bash
   curl -i 'http://localhost:8080/api/v1/auth/userinfo' \
     -H 'Authorization: Bearer {tenant_token_aud_other}'
   ```

### 预期结果
- 第 1 步返回 200
- 第 2 步返回 401（或统一未授权错误）
- 响应体包含未授权语义（如 `{"error": "unauthorized", "message": "..."}` / `Invalid or expired token`）

### 常见误报排查

| 症状 | 原因 | 解决方法 |
|------|------|----------|
| 两个 token 都返回 401 (InvalidAudience) | `JWT_TENANT_ACCESS_ALLOWED_AUDIENCES` 未生效或值与 token aud 不匹配 | 确认 docker-compose 中配置正确并重启容器；用 `jwt.io` 解码 token 验证 aud 值 |
| 两个 token 都返回 200 | 未设置 `ENVIRONMENT=production`（兼容模式），或"拒绝"测试的 audience 值存在于 `clients` 表中（被动态加载为有效 audience） | 确认 `ENVIRONMENT: production` 已设置；用 `SELECT client_id FROM clients` 检查 audience 是否被动态加载 |
| Token 无法生成 | TokenExchange 的 `service_id` 与数据库中的 client `client_id` 不匹配 | 查询数据库 `SELECT client_id FROM clients` 确认正确的 service_id |

---

## 场景 5：HSTS 按 HTTPS 条件下发 + gRPC validate_token audience 必填

### 初始状态
- 服务以 `ENVIRONMENT=production` 启动成功
- `HSTS_ENABLED=true`，`HSTS_HTTPS_ONLY=true`，`HSTS_TRUST_X_FORWARDED_PROTO=true`
- gRPC 服务可通过 `grpcurl` 访问（已配置认证头）

### 目的
验证 HSTS 仅在 HTTPS（forwarded proto）条件下下发；验证 gRPC `validate_token` 在 production 下必须提供 audience

### 测试操作流程
1. 模拟 HTTPS 代理头访问健康接口：
   ```bash
   curl -i 'http://localhost:8080/health' \
     -H 'x-forwarded-proto: https'
   ```
2. 不带代理头访问健康接口：
   ```bash
   curl -i 'http://localhost:8080/health'
   ```
3. gRPC `validate_token` 传空 audience：
   ```bash
   grpcurl -plaintext \
     -H 'x-api-key: {grpc_api_key}' \
     -d '{"access_token":"{tenant_access_token}","audience":""}' \
     localhost:50051 auth9.grpc.TokenExchange/ValidateToken
   ```

### 预期结果
- 第 1 步响应头包含 `Strict-Transport-Security`
- 第 2 步响应头不包含 `Strict-Transport-Security`
- 第 3 步返回 gRPC 错误：`FailedPrecondition`，错误信息包含 `audience is required in production`

---

## 检查清单

| # | 场景 | 状态 | 测试日期 | 测试人员 | 备注 |
|---|------|------|----------|----------|------|
| 1 | production 下 GRPC_AUTH_MODE=none 启动失败 | ✅ PASS | 2026-03-10 | opencode | 启动失败并显示错误信息: "gRPC authentication is disabled (GRPC_AUTH_MODE=none) in production" |
| 2 | production 下 api_key 无 keys 启动失败 | ✅ PASS | 2026-03-10 | opencode | 启动失败并显示错误信息: "gRPC auth_mode is api_key but no keys configured (GRPC_API_KEYS) in production" |
| 3 | production 下 audience allowlist 缺失启动失败 | ✅ PASS | 2026-03-10 | opencode | 启动失败并显示错误信息: "Tenant access token audience allowlist is empty in production" |
| 4 | REST Tenant Token aud 严格校验 | ✅ PASS | 2026-03-10 | opencode | aud=auth9-portal (在allowlist中) 返回200; aud=auth9-demo (不在allowlist中) 返回401 |
| 5 | HSTS 条件下发 + gRPC validate_token audience 必填 | ✅ PASS | 2026-03-10 | opencode | 带x-forwarded-proto:https有HSTS头; 不带无HSTS头; gRPC空audience返回FailedPrecondition |
