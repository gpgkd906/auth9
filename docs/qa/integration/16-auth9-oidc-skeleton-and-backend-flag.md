# 集成测试 - auth9-oidc 唯一后端验证

**模块**: 集成测试
**测试范围**: `auth9-oidc` 作为唯一后端的正确性验证、健康检查
**场景数**: 2
**优先级**: 高

---

## 背景说明

> **迁移已完成**: `IDENTITY_BACKEND` 开关已移除，auth9-oidc 现在是唯一后端。原场景 1（默认 keycloak backend）、场景 2（切换 backend）、场景 4（非法 backend 校验）已归档，因功能已不存在。

本用例验证：

- auth9-core 固定使用 `auth9_oidc` 后端
- auth9-oidc 独立服务健康检查正常
- State wiring、Session/Federation 注入链完整

---

## 场景 1：auth9-core 使用 auth9_oidc 后端启动成功

### 初始状态
- `auth9-core`、`auth9-oidc`、`auth9-redis`、`auth9-tidb` 已启动

### 目的
验证 auth9-core 固定使用 auth9_oidc 后端，启动正常

### 测试操作流程
1. 查看 `auth9-core` 最近日志：
   ```bash
   docker logs auth9-core --tail 200
   ```
2. 调用健康检查：
   ```bash
   curl -sS http://localhost:8080/health
   ```

### 预期结果
- 日志中出现 `Identity backend: auth9_oidc`
- `/health` 返回 `200 OK`
- 不出现 `missing identity engine`、`panic`

---

## 场景 2：`auth9-oidc` 独立服务 `/health` 返回成功

### 初始状态
- `auth9-oidc` 已启动
- TiDB 可访问

### 目的
验证独立服务骨架可启动、可连 DB、可对外提供 health probe。

### 测试操作流程
1. 启动服务：
   ```bash
   cargo run --manifest-path auth9-oidc/Cargo.toml 2>&1 | tee /tmp/auth9-oidc.log
   ```
2. 调用 health 端点：
   ```bash
   curl -sS http://localhost:8090/health
   ```

### 预期结果
- 返回 `200 OK`
- 响应 JSON 包含 `service = "auth9-oidc"`
- 响应 JSON 包含 `identity_backend = "auth9_oidc"`
- 响应 JSON 包含 `database = "up"`

---

## 检查清单

| # | 场景 | 状态 | 测试日期 | 测试人员 | 备注 |
|---|------|------|----------|----------|------|
| 1 | auth9-core 使用 auth9_oidc 后端启动成功 | ☐ | | | |
| 2 | `auth9-oidc` 独立服务 `/health` 返回成功 | ☐ | | | |
