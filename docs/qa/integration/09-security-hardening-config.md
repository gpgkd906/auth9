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

### 测试操作流程
1. 执行命令（同时清空显式 allowlist 与 portal client id）：
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
- 进程启动失败并退出（非 0）
- 日志包含类似信息：`Tenant access token audience allowlist is empty in production`

---

## 场景 4：REST 侧 Tenant Access Token aud 严格校验

### 初始状态
- 服务以 `ENVIRONMENT=production` 启动成功
- 已配置 `JWT_TENANT_ACCESS_ALLOWED_AUDIENCES='auth9-portal'`
- 已准备两个 Tenant Access Token：
  - `{tenant_token_aud_auth9_portal}`（`aud=auth9-portal`）
  - `{tenant_token_aud_other}`（`aud=other-service`）

### 目的
验证 REST 认证链路只接受 allowlist 内 aud

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
- 响应体包含未授权语义（如 `UNAUTHORIZED` / `Invalid or expired token`）

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
| 1 | production 下 GRPC_AUTH_MODE=none 启动失败 | ☐ | | | |
| 2 | production 下 api_key 无 keys 启动失败 | ☐ | | | |
| 3 | production 下 audience allowlist 缺失启动失败 | ☐ | | | |
| 4 | REST Tenant Token aud 严格校验 | ☐ | | | |
| 5 | HSTS 条件下发 + gRPC validate_token audience 必填 | ☐ | | | |
