> **本文档已归档** — Keycloak 解耦已完成，Auth9 已完全迁移至 auth9-oidc。此文档仅供历史参考。

---

# 集成测试: Config 重构 — KeycloakConfig 退役

**模块**: integration / config
**关联 FR**: Phase5 FR4 — 重构 Config: KeycloakConfig → Config 顶层字段
**前置条件**: auth9-core 已构建并部署（Docker 环境已 reset）

---

## 场景 1: 服务启动 — 无 Keycloak 环境变量仍正常启动

**目的**: 确认移除 `KEYCLOAK_URL`、`KEYCLOAK_REALM` 等环境变量后，服务不再依赖它们。

### 步骤

1. 确认 docker-compose 中 auth9-core 不再需要 `KEYCLOAK_URL` 等变量即可启动
2. 验证健康检查：

```bash
curl -sf http://localhost:8080/health
```

### 预期结果

- 返回 HTTP 200
- 服务正常运行，无外部身份引擎连接错误日志

---

## 场景 2: Webhook 签名验证仍正常工作

**目的**: 确认 `webhook_secret` 从 `config.keycloak.webhook_secret` 提升到 `config.webhook_secret` 后，webhook 签名验证逻辑不受影响。

### 步骤 0 — Gate Check

```bash
TOKEN=$(.claude/skills/tools/gen-admin-token.sh)
# 确认 webhook 端点可达
curl -sf http://localhost:8080/api/v1/keycloak/events -X POST -H "Content-Type: application/json" -d '{}' -w '%{http_code}' -o /dev/null
```

预期: 返回 401（签名缺失）或 204（无签名配置时）

### 步骤 1 — 无签名请求被拒绝

```bash
curl -s -o /dev/null -w '%{http_code}' \
  -X POST http://localhost:8080/api/v1/keycloak/events \
  -H "Content-Type: application/json" \
  -d '{"type":"LOGIN","userId":"test-user","time":1700000000000,"details":{"email":"test@example.com"}}'
```

### 预期结果

- 如果 `KEYCLOAK_WEBHOOK_SECRET` 已配置: 返回 `401`（Missing webhook signature）
- 如果未配置: 返回 `204`（事件被接受）

---

## 场景 3: SAML 元数据端点返回正确 URL

**目的**: 确认 `Auth9OidcClientStore` 的 SAML 方法在 `core_public_url` 配置后返回正确的 SSO URL 和 IdP descriptor。

### 步骤

```bash
TOKEN=$(.claude/skills/tools/gen-admin-token.sh)

# 创建 SAML 应用（需要已有 tenant 和 service）
# 通过 integration info 端点间接验证 core_public_url 生效
curl -s http://localhost:8080/health | jq .
```

### 预期结果

- 健康检查正常，表明配置加载无误
- `AUTH9_CORE_PUBLIC_URL` 和 `AUTH9_PORTAL_URL` 环境变量正确映射到 `config.core_public_url` 和 `config.portal_url`

---

## 场景 4: 单元测试全量通过

**目的**: 确认重构后所有 620+ 个单元测试通过。

### 步骤

```bash
cd auth9-core && cargo test 2>&1 | tail -5
```

### 预期结果

- `test result: ok. 620 passed; 0 failed`（数字可能因后续开发略有变化）
- 无编译错误

---

## 场景 5: 无 KeycloakConfig 残留

**目的**: 确认源码中不再存在 `KeycloakConfig` 类型或 `.keycloak.` 字段访问。

### 步骤

```bash
# 检查 KeycloakConfig 类型引用
rg "KeycloakConfig" auth9-core/src/ --type rust
# 期望: 0 结果

# 检查 .keycloak. 字段访问（排除注释中的 Java 类名引用）
rg "\.keycloak\." auth9-core/src/ --type rust | grep -v "^.*//.*org\.keycloak"
# 期望: 0 结果
```

### 预期结果

- 两个命令均返回空结果
