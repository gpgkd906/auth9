# 认证流程 - 企业 SSO UI 入口与回归测试

**模块**: 认证流程
**测试范围**: Portal `/login` 企业 SSO 入口可见性、异常回归
**场景数**: 2
**优先级**: 高

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


---

## 检查清单

| # | 场景 | 状态 | 测试日期 | 测试人员 | 备注 |
|---|------|------|----------|----------|------|
| 1 | Portal /login 页面通过 UI 输入企业邮箱触发 SSO 发现 | ☐ | | | UI 端到端 |
| 2 | Portal /login 页面输入未配置域名邮箱显示错误（UI 回归） | ☐ | | | 防止 auto-redirect 绕过 SSO 入口 |
