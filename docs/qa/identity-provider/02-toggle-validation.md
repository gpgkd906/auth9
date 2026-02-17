# 身份提供商 - 启用/禁用与验证测试

**模块**: 身份提供商管理
**测试范围**: 启用/禁用切换、配置验证、登录集成
**场景数**: 5

---

## 场景 1：启用/禁用身份提供商

### 初始状态
- 管理员已登录
- 存在已配置的身份提供商（如 Google）

### 目的
验证快速切换身份提供商启用状态

### 测试操作流程
1. 进入「设置」→「身份提供商」
2. 找到目标提供商
3. 点击启用/禁用开关

### 预期结果
- 开关状态立即更新
- 禁用后，品牌化登录页不显示该提供商选项
- 启用后，登录页恢复显示

### 预期数据状态
```sql
-- Keycloak Admin API 验证
-- GET /admin/realms/auth9/identity-provider/instances/{alias}
-- enabled 字段应与开关状态一致
```

### Keycloak 登录页验证
- 通过 Auth9 登录入口触发到品牌化登录页（底层 Keycloak 渲染）
- 检查社交登录按钮是否根据启用状态显示/隐藏
- 如需排障，可直接访问 Keycloak 登录页面 URL 对比验证

---

## 场景 2：创建重复别名的提供商

### 初始状态
- 管理员已登录
- 已存在 alias 为 `google` 的提供商

### 目的
验证别名唯一性约束

### 测试操作流程
1. 进入「设置」→「身份提供商」
2. 点击「Add provider」
3. 选择任意类型
4. 填写 Alias：`google`（与已存在的相同）
5. 点击「Add provider」

### 预期结果
- 显示错误提示：「Identity provider with this alias already exists」
- 提供商未被创建

### 预期数据状态
```sql
-- Keycloak Admin API 验证
-- GET /admin/realms/auth9/identity-provider/instances
-- 预期: 只有一个 alias=google 的提供商
```

---

## 场景 3：验证必填字段

### 初始状态
- 管理员已登录

### 目的
验证创建提供商时的必填字段验证

### 测试操作流程
1. 进入「设置」→「身份提供商」
2. 点击「Add provider」
3. 选择「Google」类型
4. 不填写 Client ID 和 Client Secret
5. 点击「Add provider」

### 预期结果
- 显示验证错误
- Client ID 和 Client Secret 字段标记为必填
- 提供商未被创建

### 预期数据状态
无数据库变更

---

## 场景 4：使用社交登录

### 初始状态
- 已配置并启用 Google 身份提供商
- 存在有效的 Google OAuth 配置

### 目的
验证社交登录端到端流程

### 测试操作流程
1. 访问应用登录页
2. 点击「Sign in with Google」按钮
3. 在 Google 页面完成授权
4. 自动跳回应用

### 预期结果
- 成功重定向到 Google 授权页
- 授权后跳回应用
- 用户成功登录
- 如果是新用户，自动创建账户

### 预期数据状态
```sql
SELECT id, email, keycloak_id FROM users WHERE email = '{google_email}';
-- 预期: 存在用户记录

SELECT * FROM linked_identities WHERE user_id = '{user_id}';
-- 预期: 存在 provider=google 的关联记录
```

---

## 场景 5：查看用户关联的身份提供商

### 初始状态
- 用户已通过社交登录创建账户
- 用户已登录

### 目的
验证用户可以查看和管理关联的身份

### 测试操作流程
1. 用户登录后进入个人设置
2. 查看「Linked Accounts」或类似区域

### 预期结果
- 显示已关联的身份提供商列表
- 显示关联的账户信息（如 Google 邮箱）
- 可以解除关联（如果有其他登录方式）

### 预期数据状态
```sql
SELECT li.provider_type, li.provider_user_id, li.provider_username
FROM linked_identities li
WHERE li.user_id = '{user_id}';
-- 预期: 列出所有关联的身份提供商
```

---

## 通用场景：认证状态检查

### 初始状态
- 用户已登录管理后台
- 页面正常显示

### 目的
验证页面正确检查认证状态，未登录或 session 失效时重定向到登录页

### 测试操作流程
1. 关闭浏览器
2. 重新打开浏览器，访问本页面对应的 URL

### 预期结果
- 页面自动重定向到 `/login`
- 不显示 dashboard 内容
- 登录后可正常访问原页面

---

## 检查清单

| # | 场景 | 状态 | 测试日期 | 测试人员 | 备注 |
|---|------|------|----------|----------|------|
| 1 | 启用/禁用身份提供商 | ☐ | | | |
| 2 | 创建重复别名的提供商 | ☐ | | | |
| 3 | 验证必填字段 | ☐ | | | |
| 4 | 使用社交登录 | ☐ | | | |
| 5 | 查看用户关联的身份提供商 | ☐ | | | |
| 6 | 认证状态检查 | ☐ | | | |
