# OIDC 环境搭建 - Conformance Suite 与客户端预置

**模块**: 环境搭建
**测试范围**: OIDC Conformance Suite 部署与 OIDC 客户端注册
**场景数**: 3

---

## 架构说明

Auth9 OIDC Conformance 测试依赖以下组件：

| 组件 | 地址 | 用途 |
|------|------|------|
| auth9-core | `http://auth9-core:8080` (Docker) / `http://localhost:8080` (Host) | OIDC Provider |
| Conformance Suite | `https://localhost:9443` | OpenID Foundation 测试引擎 |
| MongoDB | `conformance-mongo:27017` | Conformance Suite 数据存储 |

**注意**: 启用 `--conformance` 后，`JWT_ISSUER` 被覆盖为 `http://auth9-core:8080`（Docker 内部地址），使 Conformance Suite 能通过网络访问 Discovery 端点。

---

## 场景 1：环境重置与服务启动

### 初始状态
- 无运行中的 Docker 容器（或需重置）

### 目的
验证 `--conformance` 参数能正确启动包含 Conformance Suite 的完整环境

### 测试操作流程
1. 执行环境重置：
   ```bash
   ./scripts/reset-docker.sh --conformance
   ```
2. 等待脚本输出 `Reset complete!`
3. 确认输出包含 `Conformance: https://localhost:9443`

### 预期结果
- 所有核心服务正常启动（auth9-core、auth9-portal、tidb、redis）
- Conformance Suite 容器 `auth9-conformance` 运行中
- MongoDB 容器 `auth9-conformance-mongo` 运行中

### 验证命令
```bash
# 确认容器状态
docker ps --format "table {{.Names}}\t{{.Status}}" | grep -E "auth9-(core|conformance)"

# 确认 auth9-core 健康
curl -sf http://localhost:8080/health && echo "Core OK"

# 确认 Conformance Suite 可访问（自签证书，忽略 TLS）
curl -skf https://localhost:9443 > /dev/null && echo "Conformance Suite OK"
```

---

## 场景 2：Discovery 端点 Docker 内部可达性

### 初始状态
- 场景 1 完成，所有服务运行中

### 目的
验证 Conformance Suite 能通过 Docker 网络访问 auth9-core 的 OIDC Discovery 端点

### 测试操作流程
1. 从 Conformance Suite 容器内部访问 Discovery 端点：
   ```bash
   docker exec auth9-conformance curl -sf http://auth9-core:8080/.well-known/openid-configuration | jq .
   ```
2. 验证返回的 `issuer` 字段

### 预期结果
- 返回有效的 JSON
- `issuer` 值为 `http://auth9-core:8080`
- 所有端点 URL 以 `http://auth9-core:8080` 为前缀

---

## 场景 3：OIDC 测试客户端预置

### 初始状态
- 场景 1 完成，所有服务运行中

### 目的
验证 `oidc-conformance-setup.sh` 能成功创建测试用 OIDC 客户端

### 测试操作流程
1. 执行预置脚本：
   ```bash
   ./scripts/oidc-conformance-setup.sh
   ```
2. 脚本应输出 client_id 和 redirect_uri 信息

### 预期结果
- 脚本成功执行（exit code 0）
- 输出包含已注册的 `client_id`
- 输出包含 Conformance Suite callback 地址作为 `redirect_uri`

### 预期数据状态
```sql
SELECT c.client_id, s.name FROM clients c
JOIN services s ON c.service_id = s.id
WHERE s.name LIKE '%conformance%' OR s.name LIKE '%oidc-test%'
LIMIT 1;
-- 预期: 存在一条记录
```

---

## 检查清单

| # | 场景 | 状态 | 测试日期 | 测试人员 | 备注 |
|---|------|------|----------|----------|------|
| 1 | 环境重置与服务启动 | ☐ | | | |
| 2 | Discovery 端点 Docker 内部可达性 | ☐ | | | |
| 3 | OIDC 测试客户端预置 | ☐ | | | |
