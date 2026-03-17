# 集成测试 - Keycloak Adapter 层回归

**模块**: 集成测试
**测试范围**: `KeycloakIdentityEngineAdapter`、`KeycloakSessionStoreAdapter`、`KeycloakFederationBrokerAdapter`
**场景数**: 4
**优先级**: 高

---

## 背景说明

本用例用于验证 Phase 1 FR2 完成后的关键回归点：

- `KeycloakClient` 已降级为底层 HTTP client
- 业务服务通过 `identity_engine/adapters/keycloak/` 中的 adapter 访问身份后端
- `SessionService`、`IdentityProviderService`、`KeycloakSyncService` 的注入链保持可用
- Keycloak 特有 DTO 与中性表示之间的映射被收敛在 adapter 层

该 FR 不改变外部 API 契约，因此 QA 重点是“适配层抽离后，现有行为未回退”。

本用例仅覆盖 `IDENTITY_BACKEND=keycloak` 分支；`auth9_oidc` backend flag 与骨架探活由 `integration/16-auth9-oidc-skeleton-and-backend-flag.md` 覆盖。

---

## 场景 1：服务启动后 adapter 注入链正常

### 初始状态
- `auth9-core`、`auth9-keycloak`、`auth9-redis`、`auth9-tidb` 已启动
- 启动日志可访问

### 目的
验证服务在启用 Keycloak adapter 后仍能正常完成依赖注入与启动。

### 测试操作流程
1. 调用健康检查端点：
   ```bash
   curl -v http://localhost:8080/health
   ```
2. 查看 `auth9-core` 最近日志：
   ```bash
   docker logs auth9-core --tail 200
   ```

### 预期结果
- `/health` 返回 `200 OK`
- 响应 JSON 包含 `status = "healthy"`
- 日志中不出现 `missing identity engine`、`missing federation broker`、`missing session store`、`panic` 等错误

---

## 场景 2：Session revoke 路径保持正常

### 初始状态
- 服务已启动
- 已存在一条可撤销的用户会话
- 已生成可访问 Account Session API 的 Identity Token

### 目的
验证 `SessionService` 通过 `KeycloakSessionStoreAdapter` 撤销单个会话时，API 路径和数据库状态均正常。

### 测试操作流程
1. 查询当前用户会话列表：
   ```bash
   curl -sS \
     -H "Authorization: Bearer $IDENTITY_TOKEN" \
     http://localhost:8080/api/v1/users/me/sessions
   ```
2. 选取一条非当前会话，执行撤销：
   ```bash
   curl -i -X DELETE \
     -H "Authorization: Bearer $IDENTITY_TOKEN" \
     http://localhost:8080/api/v1/users/me/sessions/{session_id}
   ```
3. 再次查询会话列表，确认该会话已消失或标记为 revoked。

### 预期结果
- 删除请求返回 `200 OK` 或 `204 No Content`
- 不返回 `500`、`missing state`、`KeycloakClient trait object` 一类错误
- 再次查询时，被撤销会话不再处于 active 状态

### 预期数据状态
```sql
SELECT id, revoked_at
FROM sessions
WHERE id = '{session_id}';
-- 预期: revoked_at IS NOT NULL
```

---

## 场景 3：Identity Provider CRUD 仍经 adapter 正常执行

### 初始状态
- 服务已启动
- 已生成具备身份提供商管理权限的 Access Token
- 测试数据使用 `corp.example.com` 相关 URL

### 目的
验证 `IdentityProviderService` 通过 `KeycloakFederationBrokerAdapter` 管理身份提供商时，CRUD 行为未回退。

### 测试操作流程
1. 创建一个 OIDC provider：
   ```bash
   curl -i -X POST \
     -H "Authorization: Bearer $TENANT_OWNER_TOKEN" \
     -H "Content-Type: application/json" \
     http://localhost:8080/api/v1/identity-providers \
     -d '{
       "alias": "corp-oidc",
       "display_name": "Corp OIDC",
       "provider_id": "oidc",
       "enabled": true,
       "trust_email": true,
       "config": {
         "clientId": "corp-client",
         "clientSecret": "corp-secret", // pragma: allowlist secret
         "authorizationUrl": "https://sso.corp.example.com/oauth2/authorize",
         "tokenUrl": "https://sso.corp.example.com/oauth2/token"
       }
     }'
   ```
2. 查询 provider 详情：
   ```bash
   curl -sS \
     -H "Authorization: Bearer $TENANT_OWNER_TOKEN" \
     http://localhost:8080/api/v1/identity-providers/corp-oidc
   ```
3. 更新显示名称：
   ```bash
   curl -i -X PUT \
     -H "Authorization: Bearer $TENANT_OWNER_TOKEN" \
     -H "Content-Type: application/json" \
     http://localhost:8080/api/v1/identity-providers/corp-oidc \
     -d '{
       "display_name": "Corp OIDC Updated"
     }'
   ```
4. 删除该 provider：
   ```bash
   curl -i -X DELETE \
     -H "Authorization: Bearer $TENANT_OWNER_TOKEN" \
     http://localhost:8080/api/v1/identity-providers/corp-oidc
   ```

### 预期结果
- 创建、查询、更新、删除均成功
- 查询结果中的字段结构与既有 API 保持一致
- 不出现 `KeycloakIdentityProvider` DTO 直接泄漏到响应层的异常结构

---

## 场景 4：Linked identity 读取与解除关联路径保持正常

### 初始状态
- 服务已启动
- 测试用户已存在一条 linked identity
- 已生成可访问 Account Identity API 的 Access Token，且 `sub` 对应数据库中的真实用户

### 目的
验证 federated identity 的读取与删除路径仍通过 adapter 正常工作。

### 测试操作流程
1. 查询当前用户已关联身份：
   ```bash
   curl -sS \
     -H "Authorization: Bearer $ACCESS_TOKEN" \
     http://localhost:8080/api/v1/users/me/linked-identities
   ```
2. 对其中一条身份执行解除关联：
   ```bash
   curl -i -X DELETE \
     -H "Authorization: Bearer $ACCESS_TOKEN" \
     http://localhost:8080/api/v1/users/me/linked-identities/{identity_id}
   ```
3. 再次查询关联身份列表。

### 预期结果
- 列表接口返回 `200 OK`
- 删除接口成功，不返回 `500`
- 再次查询时，被删除 identity 不再出现

### 预期数据状态
```sql
SELECT id
FROM linked_identities
WHERE id = '{identity_id}';
-- 预期: 0 行
```

---

## 检查清单

| # | 场景 | 状态 | 测试日期 | 测试人员 | 备注 |
|---|------|------|----------|----------|------|
| 1 | 服务启动后 adapter 注入链正常 | ☐ | | | |
| 2 | Session revoke 路径保持正常 | ☐ | | | |
| 3 | Identity Provider CRUD 仍经 adapter 正常执行 | ☐ | | | |
| 4 | Linked identity 读取与解除关联路径保持正常 | ☐ | | | |
