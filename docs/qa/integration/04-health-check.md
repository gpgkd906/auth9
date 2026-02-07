# 集成测试 - 健康检查与就绪探针测试

**模块**: 集成测试
**测试范围**: 健康检查端点、就绪探针、依赖服务检测
**场景数**: 5
**优先级**: 中

---

## 背景说明

Auth9 提供两个运维端点（无需认证）：

| 端点 | 用途 | 检查内容 |
|------|------|----------|
| `GET /health` | 存活探针（Liveness） | 应用进程是否运行 |
| `GET /ready` | 就绪探针（Readiness） | DB + Redis 是否可连接 |

### 响应格式
- `/health` 返回 JSON：`{"status": "healthy", "version": "x.y.z"}`
- `/ready` 返回文本：`ready` 或 `not_ready`

### Kubernetes 集成
这两个端点通常配置为 K8s 的 `livenessProbe` 和 `readinessProbe`。

---

## 场景 1：健康检查正常响应

### 初始状态
- Auth9 服务正在运行

### 目的
验证健康检查端点返回正确信息

### 测试操作流程
1. 调用健康检查端点：
   ```bash
   curl -v http://localhost:8080/health
   ```
2. 检查响应状态码和内容

### 预期结果
- 状态码 200
- 响应 JSON 包含 `status` = `"healthy"`
- 响应 JSON 包含 `version`（与 Cargo.toml 版本一致）
- 无需 Authorization 头

---

## 场景 2：就绪探针 - 所有依赖正常

### 初始状态
- Auth9 服务运行中
- TiDB 数据库可连接
- Redis 可连接

### 目的
验证就绪探针在所有依赖正常时返回 ready

### 测试操作流程
1. 确认 DB 和 Redis 运行正常
2. 调用就绪探针：
   ```bash
   curl -v http://localhost:8080/ready
   ```

### 预期结果
- 状态码 200
- 响应体：`ready`
- 无需 Authorization 头

---

## 场景 3：就绪探针 - 数据库不可用

### 初始状态
- Auth9 服务运行中
- TiDB 数据库停止或不可连接

### 目的
验证 DB 不可用时就绪探针返回 not_ready

### 测试操作流程
1. 停止 TiDB：
   ```bash
   docker stop auth9-tidb
   ```
2. 调用就绪探针：
   ```bash
   curl -v http://localhost:8080/ready
   ```
3. 恢复 TiDB：
   ```bash
   docker start auth9-tidb
   ```

### 预期结果
- 步骤 2：状态码 503 Service Unavailable
- 响应体：`not_ready`
- `/health` 仍返回 200（进程存活）
- 步骤 3 恢复后 `/ready` 恢复为 200

---

## 场景 4：就绪探针 - Redis 不可用

### 初始状态
- Auth9 服务运行中
- Redis 停止或不可连接

### 目的
验证 Redis 不可用时就绪探针返回 not_ready

### 测试操作流程
1. 停止 Redis：
   ```bash
   docker stop auth9-redis
   ```
2. 调用就绪探针：
   ```bash
   curl -v http://localhost:8080/ready
   ```
3. 同时检查健康端点：
   ```bash
   curl -v http://localhost:8080/health
   ```
4. 恢复 Redis：
   ```bash
   docker start auth9-redis
   ```

### 预期结果
- `/ready`：状态码 503，响应 `not_ready`
- `/health`：状态码 200，响应 `{"status": "healthy", ...}`
- 恢复后 `/ready` 恢复为 200

---

## 场景 5：健康检查无需认证

### 初始状态
- Auth9 服务运行中
- 不携带任何 Authorization 头

### 目的
验证健康检查和就绪探针是公开端点

### 测试操作流程
1. 不携带 Token 调用两个端点：
   ```bash
   curl http://localhost:8080/health
   curl http://localhost:8080/ready
   ```
2. 携带无效 Token 调用：
   ```bash
   curl -H "Authorization: Bearer invalid-token" http://localhost:8080/health
   curl -H "Authorization: Bearer invalid-token" http://localhost:8080/ready
   ```

### 预期结果
- 两种情况都正常返回（不被认证中间件拦截）
- 不返回 401 或 403

---

## 检查清单

| # | 场景 | 状态 | 测试日期 | 测试人员 | 备注 |
|---|------|------|----------|----------|------|
| 1 | 健康检查正常响应 | ☐ | | | |
| 2 | 就绪探针 - 正常 | ☐ | | | |
| 3 | 就绪探针 - DB 不可用 | ☐ | | | |
| 4 | 就绪探针 - Redis 不可用 | ☐ | | | |
| 5 | 无需认证 | ☐ | | | |
