# 认证流程 - 企业 SSO 域名发现与登录路由测试

**模块**: 认证流程
**测试范围**: 企业邮箱域名发现、连接器路由、登录跳转与异常处理
**场景数**: 7
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

## 场景 1：邮箱域名命中后返回企业 SSO 跳转地址

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
1. 调用 `POST /demo/enterprise/discovery`，请求体传空邮箱：`{"email":""}`
2. 调用 `POST /demo/enterprise/discovery`，请求体传非法邮箱：`{"email":"not-an-email"}`

### 预期结果
- HTTP 状态码为 `422`
- 错误类型为验证错误（`validation`）

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

## 场景 6：Portal `/login` 页面通过 UI 输入企业邮箱触发 SSO 发现

### 初始状态
- 已存在租户，该租户存在已启用连接器，绑定域名 `{corp_domain}`
- 用户已登出

### 目的
验证用户在 Portal `/login` 页面通过 UI 输入企业邮箱后，能触发 SSO 发现并跳转到对应的企业 IdP。此场景是场景 1 的 UI 版本——场景 1 通过 curl/API 验证，本场景通过浏览器 UI 操作验证完整端到端流程。

### 测试操作流程
1. 在浏览器中访问 `http://localhost:3000/login`
2. 确认页面正常渲染，显示 Enterprise SSO 邮箱输入框
3. 在邮箱输入框中输入 `qa-user@{corp_domain}`
4. 点击「Continue with Enterprise SSO」按钮
5. 按钮变为「Finding your SSO...」状态（禁用）
6. 等待页面跳转

### 预期结果
- 页面跳转到 Keycloak 授权端点（URL 包含 `/realms/auth9`）
- 跳转 URL 包含 `kc_idp_hint=` 参数，值为该连接器的 `keycloak_alias`
- 用户进入企业 IdP 的登录页面（非 Keycloak 默认用户名/密码表单）

### 预期数据状态
```sql
SELECT c.keycloak_alias, d.domain
FROM enterprise_sso_connectors c
JOIN enterprise_sso_domains d ON d.connector_id = c.id
WHERE d.domain = '{corp_domain}' AND c.enabled = 1;
-- 预期: 返回 1 条记录，keycloak_alias 与跳转 URL 中的 kc_idp_hint 一致
```

---

## 场景 7：Portal `/login` 页面输入未配置域名邮箱显示错误（UI 回归）

### 初始状态
- 系统中不存在域名 `unknown-corp.com` 的连接器绑定
- 用户已登出

### 目的
**回归验证**：确认用户在 Portal `/login` 页面输入未配置域名的企业邮箱后，页面停留在 `/login` 并显示错误信息，而不是发生意外跳转或白屏。

> **回归背景**：commit `25ea411` 曾引入 loader auto-redirect，导致用户根本无法到达 `/login` 页面的 Enterprise SSO 输入框——页面在 loader 阶段就被重定向到 Keycloak。修复后 `/login` 始终渲染，本场景验证 Enterprise SSO 的错误路径在 UI 层面工作正常。

### 测试操作流程
1. 在浏览器中访问 `http://localhost:3000/login`
2. 在邮箱输入框中输入 `user@unknown-corp.com`
3. 点击「Continue with Enterprise SSO」按钮
4. 等待响应

### 预期结果
- 页面停留在 `/login`（不发生跳转）
- 显示红色错误提示，包含域名未配置连接器相关信息
- 用户可以重新输入其他邮箱，或改用「Sign in with password」/「Sign in with passkey」

### 回归失败的表现（若 auto-redirect bug 复发）
- 用户根本无法看到 Enterprise SSO 邮箱输入框
- 访问 `/login` 后立即被 302 重定向到 Keycloak 密码登录页
- Enterprise SSO 功能完全不可用

---

## Agent 自动化测试：Playwright MCP 工具

场景 6、7 可由 AI Agent 通过 Playwright MCP 工具执行。

> **前提条件**: 全栈环境运行中（Docker + auth9-core on :8080 + auth9-portal on :3000），且已存在至少一个绑定域名的企业 SSO 连接器。

### 步骤 1：场景 7 — 未配置域名错误提示

1. 调用 **`browser_navigate`**: `http://localhost:3000/login`
2. 调用 **`browser_snapshot`** 确认页面渲染（未发生 auto-redirect）
3. 调用 **`browser_fill_form`**: 在邮箱输入框填入 `user@unknown-corp.com`
4. 调用 **`browser_click`**: 点击「Continue with Enterprise SSO」按钮
5. 等待页面响应后调用 **`browser_snapshot`**
6. **验证**：
   - 页面 URL 仍为 `/login`
   - 页面显示错误提示信息
   - 邮箱输入框和三种认证按钮仍可用

### 步骤 2：场景 6 — 企业邮箱命中后跳转 IdP

> 此步骤需要环境中存在绑定域名的连接器。若无可用连接器，跳过。

1. 调用 **`browser_navigate`**: `http://localhost:3000/login`
2. 调用 **`browser_fill_form`**: 在邮箱输入框填入 `qa-user@{corp_domain}`
3. 调用 **`browser_click`**: 点击「Continue with Enterprise SSO」按钮
4. 等待跳转后调用 **`browser_snapshot`**
5. **验证**：
   - 页面 URL 包含 `/realms/auth9` 或企业 IdP 域名
   - URL 包含 `kc_idp_hint=` 参数

---

## 检查清单

| # | 场景 | 状态 | 测试日期 | 测试人员 | 备注 |
|---|------|------|----------|----------|------|
| 1 | 邮箱域名命中后返回企业 SSO 跳转地址 | ☐ | | | API 层 |
| 2 | 未配置域名时返回未找到错误 | ☐ | | | API 层 |
| 3 | 连接器禁用后 discovery 不可用 | ☐ | | | API 层 |
| 4 | 缺失或非法邮箱参数返回验证错误 | ☐ | | | API 层 |
| 5 | `/api/v1/auth/authorize` 透传 `connector_alias` | ☐ | | | API 层 |
| 6 | Portal UI 输入企业邮箱触发 SSO 发现 | ☐ | | | UI 端到端 |
| 7 | Portal UI 未配置域名显示错误（回归） | ☐ | | | 防止 auto-redirect 绕过 SSO 入口 |
