# 身份提供商 - CRUD 操作测试

**模块**: 身份提供商管理
**测试范围**: 创建、更新、删除身份提供商
**场景数**: 5

---

## 功能概述

身份提供商（Identity Provider）允许用户通过第三方服务（如 Google、GitHub、Microsoft）进行登录。支持的类型包括：
- **Google** - Google OAuth 2.0
- **GitHub** - GitHub OAuth
- **Microsoft** - Microsoft/Azure AD
- **OpenID Connect** - 通用 OIDC 提供商
- **SAML 2.0** - 企业 SAML 集成

---

## 场景 1：查看身份提供商列表

### 初始状态
- 管理员已登录
- 可能存在已配置的身份提供商

### 目的
验证身份提供商列表页面正确显示

### 测试操作流程
1. 进入「设置」→「身份提供商」

### 预期结果
- 显示已配置的身份提供商列表
- 每个提供商显示：图标、名称、别名、类型、启用状态开关
- 显示「Add provider」按钮
- 无提供商时显示空状态提示

### 预期数据状态
```sql
-- Keycloak Admin API 验证
-- GET /admin/realms/auth9/identity-provider/instances
```

---

## 场景 2：添加 Google 身份提供商

### 初始状态
- 管理员已登录
- 未配置 Google 身份提供商

### 目的
验证添加 Google OAuth 提供商功能

### 测试操作流程
1. 进入「设置」→「身份提供商」
2. 点击「Add provider」
3. **点击「Google」类型卡片**（卡片应高亮为蓝色边框，表示已选中）
4. **确认配置表单出现**（Alias、Display Name、Client ID、Client Secret 字段应在选中后立即显示）
5. 填写配置：
   - Alias：`google`（选中 Google 后自动填充）
   - Display Name：`Sign in with Google`
   - Client ID：`your-google-client-id`
   - Client Secret：`your-google-client-secret`
   - 启用开关：开启
6. 点击「Add provider」

> **故障排除**: 如果选中 Google 后配置表单未出现，请尝试：
> 1. 关闭对话框后重新打开（确保状态被重置）
> 2. 刷新页面后重试
> 3. 检查浏览器控制台是否有 JavaScript 错误

### 预期结果
- 显示「Identity provider created」提示
- 列表中出现 Google 提供商
- 状态显示为启用

### Troubleshooting

| 现象 | 原因 | 解决方法 |
|------|------|----------|
| "Add provider" 按钮无反应 | 未选择 Provider 类型或必填字段未填写时按钮处于 disabled 状态 | 确认已点击 Provider 类型卡片（应显示蓝色高亮边框），并填写 Client ID、Client Secret 等必填字段 |
| API 返回 403 "Identity token is only allowed..." | 使用了 Identity Token 调用 API | Portal UI 会自动使用正确的 Token；若直接调用 API，需使用 Tenant Access Token |
| 对话框无任何错误提示 | 后端返回错误但前端可能未正确展示 | 检查浏览器 Network 面板查看 API 响应状态码和内容 |

### 预期数据状态
```sql
-- Keycloak Admin API 验证
-- GET /admin/realms/auth9/identity-provider/instances/google
-- 预期: 存在 alias=google, providerId=google, enabled=true
```

---

## 场景 3：添加 OIDC 身份提供商

### 初始状态
- 管理员已登录

### 目的
验证添加自定义 OIDC 提供商功能

### 测试操作流程
1. 进入「设置」→「身份提供商」
2. 点击「Add provider」
3. 选择「OpenID Connect」类型
4. 填写配置：
   - Alias：`custom-oidc`
   - Display Name：`Enterprise SSO`
   - Client ID：`client-id`
   - Client Secret：`client-secret`
   - Authorization URL：`https://idp.example.com/oauth/authorize`
   - Token URL：`https://idp.example.com/oauth/token`
   - 启用开关：开启
5. 点击「Add provider」

### 预期结果
- 显示创建成功提示
- 列表中出现 OIDC 提供商
- 显示自定义名称「Enterprise SSO」

### 预期数据状态
```sql
-- Keycloak Admin API 验证
-- GET /admin/realms/auth9/identity-provider/instances/custom-oidc
-- 预期: 存在配置，config 包含 authorizationUrl 和 tokenUrl
```

---

## 场景 4：更新身份提供商配置

### 初始状态
- 管理员已登录
- 存在已配置的身份提供商

### 目的
验证更新身份提供商配置功能

### 测试操作流程
1. 进入「设置」→「身份提供商」
2. 找到目标提供商，点击编辑图标
3. 修改配置：
   - Display Name：`Updated Name`
   - 更新 Client Secret（可选）
4. 点击「Save changes」

### 预期结果
- 显示「Identity provider updated」提示
- 列表中显示新名称
- 配置更新生效

### 预期数据状态
```sql
-- Keycloak Admin API 验证
-- GET /admin/realms/auth9/identity-provider/instances/{alias}
-- 预期: displayName 已更新
```

---

## 场景 5：删除身份提供商

### 初始状态
- 管理员已登录
- 存在已配置的身份提供商

### 目的
验证删除身份提供商功能

### 测试操作流程
1. 进入「设置」→「身份提供商」
2. 找到目标提供商，点击删除图标
3. 确认删除

### 预期结果
- 显示「Identity provider deleted」提示
- 提供商从列表中消失
- 使用该提供商的登录选项不再可用

### 预期数据状态
```sql
-- Keycloak Admin API 验证
-- GET /admin/realms/auth9/identity-provider/instances/{alias}
-- 预期: 404 Not Found
```

---

## 通用场景：认证状态检查

### 初始状态
- 用户已登录管理后台
- 页面正常显示

### 目的
验证页面正确检查认证状态，未登录或 session 失效时重定向到登录页

### 测试操作流程
1. 通过以下任一方式构造未认证状态：
   - 使用浏览器无痕/隐私窗口访问
   - 手动清除 auth9_session cookie
   - 在当前会话点击「Sign out」退出登录
2. 访问本页面对应的 URL

### 预期结果
- 页面自动重定向到 `/login`
- 不显示 dashboard 内容
- 登录后可正常访问原页面

---

## 检查清单

| # | 场景 | 状态 | 测试日期 | 测试人员 | 备注 |
|---|------|------|----------|----------|------|
| 1 | 查看身份提供商列表 | ☐ | | | |
| 2 | 添加 Google 身份提供商 | ☐ | | | |
| 3 | 添加 OIDC 身份提供商 | ☐ | | | |
| 4 | 更新身份提供商配置 | ☐ | | | |
| 5 | 删除身份提供商 | ☐ | | | |
| 6 | 认证状态检查 | ☐ | | | |
