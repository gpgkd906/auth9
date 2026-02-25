# 认证流程 - 企业 SSO 域名发现与登录路由测试

**模块**: 认证流程
**测试范围**: 企业邮箱域名发现、连接器路由、登录跳转与异常处理
**场景数**: 5
**优先级**: 高

---

## 背景说明

本功能新增企业 SSO 发现端点，通过用户邮箱域名匹配租户连接器并返回重定向地址。

**本文档测试的是 Portal `/login` 页面上「Continue with Enterprise SSO」路径**，即输入企业邮箱 → 域名发现 → 跳转到对应的企业 IdP。与「Sign in with password」（直接跳 Keycloak 密码登录页）是不同的认证路径。

端点：
- `POST /api/v1/enterprise-sso/discovery`
- `GET /api/v1/auth/authorize`（新增 `connector_alias` 参数，映射 `kc_idp_hint`）

---

## 数据库表结构参考

### `enterprise_sso_connectors` 表
| 字段 | 类型 | 说明 |
|------|------|------|
| id | CHAR(36) | 连接器主键 |
| tenant_id | CHAR(36) | 租户 ID |
| alias | VARCHAR(100) | 连接器别名 |
| provider_type | VARCHAR(20) | `saml` / `oidc` |
| enabled | BOOLEAN | 是否启用 |
| keycloak_alias | VARCHAR(140) | Keycloak IdP alias |
| config | JSON | 协议配置 |

### `enterprise_sso_domains` 表
| 字段 | 类型 | 说明 |
|------|------|------|
| id | CHAR(36) | 主键 |
| tenant_id | CHAR(36) | 租户 ID |
| connector_id | CHAR(36) | 连接器 ID |
| domain | VARCHAR(255) | 企业邮箱域名（全局唯一） |
| is_primary | BOOLEAN | 是否主域名 |

---

## 场景 1：企业 SSO 入口可见性与邮箱域名命中返回跳转地址

### 初始状态
- 已存在租户 `{tenant_id}`，slug 为 `{tenant_slug}`
- 该租户存在已启用连接器 `{connector_alias}`，绑定域名 `{corp_domain}`
- 测试邮箱 `qa-user@{corp_domain}`

### 目的
验证 discovery 能命中域名并返回可跳转的 `authorize_url`

### 测试操作流程
1. 打开 `http://localhost:3002`，在首页「Login with Enterprise SSO」输入 `qa-user@{corp_domain}` 并提交
2. 或直接调用 demo discovery 代理接口：
```bash
curl -X POST 'http://localhost:3002/demo/enterprise/discovery' \
  -H 'Content-Type: application/json' \
  -d '{"email":"qa-user@{corp_domain}"}'
```
3. 也可直连 core discovery 接口：
```bash
curl -X POST 'http://localhost:8080/api/v1/enterprise-sso/discovery?response_type=code&client_id=auth9-portal&redirect_uri=http%3A%2F%2Flocalhost%3A3000%2Fauth%2Fcallback&scope=openid%20email%20profile&state={state}&nonce={nonce}' \
  -H 'Content-Type: application/json' \
  -d '{"email":"qa-user@{corp_domain}"}'
```
4. 解析返回 JSON 中 `data.authorize_url`

### 预期结果
- HTTP 状态码为 `200`
- 返回 `data.tenant_id`、`data.tenant_slug`、`data.connector_alias`
- `authorize_url` 包含 `kc_idp_hint=` 且值为租户连接器对应的 `keycloak_alias`

### 预期数据状态
```sql
SELECT c.id, c.alias, c.keycloak_alias, c.enabled, d.domain
FROM enterprise_sso_connectors c
JOIN enterprise_sso_domains d ON d.connector_id = c.id
WHERE d.domain = '{corp_domain}';
-- 预期: 返回 1 条启用连接器记录，domain 与请求邮箱域名一致
```

---

## 场景 2：未配置域名时返回未找到错误

### 初始状态
- 系统中不存在域名 `{unknown_domain}` 的连接器绑定

### 目的
验证 discovery 对未命中域名返回明确失败

### 测试操作流程
1. 通过 demo 代理调用 discovery，邮箱使用 `user@{unknown_domain}`
2. 记录状态码与错误信息

### 预期结果
- HTTP 状态码为 `404`
- 响应 `message` 包含域名未配置连接器信息

### 预期数据状态
```sql
SELECT * FROM enterprise_sso_domains WHERE domain = '{unknown_domain}';
-- 预期: 0 行
```

---

## 场景 3：连接器禁用后 discovery 不可用

### 初始状态
- 域名 `{corp_domain}` 已绑定连接器 `{connector_alias}`
- 连接器初始为启用状态

### 目的
验证禁用连接器后不应继续参与登录路由

### 测试操作流程
1. 管理端将连接器 `enabled` 置为 `false`
2. 通过 demo 代理再次调用 discovery（邮箱 `qa-user@{corp_domain}`）

### 预期结果
- HTTP 状态码为 `404`
- discovery 不返回 `authorize_url`

### 预期数据状态
```sql
SELECT enabled FROM enterprise_sso_connectors WHERE alias = '{connector_alias}' AND tenant_id = '{tenant_id}';
-- 预期: enabled = 0
```

---

## 场景 4：缺失或非法邮箱参数返回验证错误

### 初始状态
- 无特殊要求

### 目的
验证 discovery 的输入校验（邮箱必填且格式正确）

### 测试操作流程

> **注意**: Demo 代理 (`/demo/enterprise/discovery`) 有自身的空邮箱前置校验，返回 `400 missing_email`。
> 如需验证 Core 的 `validator` 校验行为（422），请直连 Core 接口。

**方式 A — 直连 Core（推荐）**:
1. 调用 Core 接口传空邮箱：
```bash
curl -i -X POST 'http://localhost:8080/api/v1/enterprise-sso/discovery?response_type=code&client_id=auth9-portal&redirect_uri=http%3A%2F%2Flocalhost%3A3000%2Fauth%2Fcallback&scope=openid%20email%20profile&state=test&nonce=test' \
  -H 'Content-Type: application/json' \
  -d '{"email":""}'
```
2. 调用 Core 接口传非法邮箱：
```bash
curl -i -X POST 'http://localhost:8080/api/v1/enterprise-sso/discovery?response_type=code&client_id=auth9-portal&redirect_uri=http%3A%2F%2Flocalhost%3A3000%2Fauth%2Fcallback&scope=openid%20email%20profile&state=test&nonce=test' \
  -H 'Content-Type: application/json' \
  -d '{"email":"not-an-email"}'
```

**方式 B — 通过 Demo 代理**:
1. 调用 `POST http://localhost:3002/demo/enterprise/discovery`，请求体传空邮箱：`{"email":""}`
2. 调用 `POST http://localhost:3002/demo/enterprise/discovery`，请求体传非法邮箱：`{"email":"not-an-email"}`

### 预期结果

| 方式 | 空邮箱 | 非法邮箱 |
|------|--------|----------|
| Core 直连 | `422` validation 错误 | `422` validation 错误 |
| Demo 代理 | `400` missing_email（代理前置校验） | `422` validation 错误（透传 Core 响应） |

### 常见误报

| 现象 | 原因 | 解决 |
|------|------|------|
| 空邮箱返回 400 而非 422 | 通过 Demo 代理测试，代理自身校验在前 | 直连 Core 接口验证 |

### 预期数据状态
```sql
SELECT COUNT(*) AS cnt FROM enterprise_sso_connectors;
-- 预期: 仅校验失败，不产生任何数据变更
```

---

## 场景 5：`/api/v1/auth/authorize` 透传 `connector_alias` 到 Keycloak

### 初始状态
- 已存在可用 OIDC client：`auth9-portal`

### 目的
验证授权端点支持 `connector_alias` 并向 Keycloak 透传 `kc_idp_hint`

### 测试操作流程
1. 访问：
```bash
curl -I 'http://localhost:8080/api/v1/auth/authorize?response_type=code&client_id=auth9-portal&redirect_uri=http%3A%2F%2Flocalhost%3A3000%2Fauth%2Fcallback&scope=openid%20email%20profile&state={state}&connector_alias={keycloak_alias}'
```
2. 查看 `Location` 响应头

### 预期结果
- HTTP 状态码为 `307` 或 `302`
- `Location` 指向 Keycloak auth endpoint
- `Location` 查询参数中存在 `kc_idp_hint={keycloak_alias}`

### 预期数据状态
```sql
SELECT COUNT(*) AS cnt FROM enterprise_sso_connectors WHERE keycloak_alias = '{keycloak_alias}';
-- 预期: cnt >= 1（仅路由透传，无新增写入）
```

---


---

## 说明

场景 6-7（Portal UI 入口与回归）已拆分到 `docs/qa/auth/12-enterprise-sso-ui-regression.md`。

---

## 检查清单

| # | 场景 | 状态 | 测试日期 | 测试人员 | 备注 |
|---|------|------|----------|----------|------|
| 1 | 邮箱域名命中后返回企业 SSO 跳转地址 | ☐ | | | API 层 |
| 2 | 未配置域名时返回未找到错误 | ☐ | | | API 层 |
| 3 | 连接器禁用后 discovery 不可用 | ☐ | | | API 层 |
| 4 | 缺失或非法邮箱参数返回验证错误 | ☐ | | | API 层 |
| 5 | /api/v1/auth/authorize 透传 connector_alias 到 Keycloak | ☐ | | | API 层 |
