# 认证流程 - B2B 首次入驻与租户路由

**模块**: 认证流程
**测试范围**: 新用户首次登录 Onboarding、Dashboard 租户路由分发、组织切换器、Demo 租户自动加入移除
**场景数**: 5
**优先级**: 高

---

## 背景说明

B2B 模式下，用户首次 OAuth 登录后不再自动加入 demo 租户。改为：

1. OAuth 登录成功 → 重定向到 `/tenant/select`
2. `tenant/select` 路由根据租户数量分流：
   - **0 个租户** → 重定向到 `/onboard`（创建组织向导）
   - **1 个租户** → 自动完成 token exchange 后进入 `/dashboard`
   - **N 个租户** → 用户手动选择 tenant，并在 token exchange 成功后进入 `/dashboard`
3. `/onboard` 页面引导用户创建组织
4. 创建成功（Active）→ 进入 Dashboard；创建成功（Pending）→ `/onboard/pending` 等待审批

Portal 路由：
- `/tenant/select` — 登录后 tenant 选择与 token exchange
- `/onboard` — 创建组织向导
- `/onboard/pending` — 等待审批页
- `/dashboard` — 主控制台（含组织切换器）

---

## 场景 1：新用户首次登录 — 无租户重定向到 Onboard

### 初始状态
- 用户通过 Keycloak OAuth 首次登录
- 用户不属于任何租户
- 不存在 demo 租户自动加入（已移除）

### 目的
验证新用户首次登录被引导到组织创建页面

### 测试操作流程
1. 访问 `/login`，使用以下任一方式登录：
   - 点击「**Sign in with password**」→ 在 Keycloak 页面输入用户名密码
   - 点击「**Continue with Enterprise SSO**」→ 输入企业邮箱完成 SSO 认证
   - 点击「**Sign in with passkey**」→ 使用设备 Passkey 认证
2. 完成认证后，观察页面跳转
3. 确认到达 `/onboard` 页面

### 预期结果
- 认证成功后首先进入 `/tenant/select`
- `tenant/select` 检测到 0 个租户后自动重定向到 `/onboard`
- `/onboard` 页面显示：
  - 标题「Create your organization」
  - 表单字段：Organization name、Slug（自动生成）、Domain（从邮箱自动提取）
  - 「Create Organization」按钮
  - 「I'm waiting for an invitation」链接
- **不**自动加入 demo 租户

### 预期数据状态
```sql
SELECT COUNT(*) FROM tenant_users WHERE user_id = '{user_id}';
-- 预期: 0（新用户无租户关联）

-- 验证 demo 自动加入已移除
SELECT COUNT(*) FROM tenant_users tu
JOIN tenants t ON t.id = tu.tenant_id
WHERE tu.user_id = '{user_id}' AND t.slug = 'demo';
-- 预期: 0
```

---

## 场景 2：Onboard 创建组织 — 域名匹配直接进入 Dashboard

### 初始状态
- 用户在 `/onboard` 页面
- 用户邮箱为 `user@acme.com`

### 目的
验证在 Onboard 页面成功创建组织后直接跳转到 Dashboard

### 测试操作流程
1. 在 `/onboard` 页面，填写：
   - Organization name: `Acme Corp`
   - Slug: `acme-corp`（自动生成或手动输入）
   - Domain: `acme.com`（从邮箱自动提取）
2. 点击「Create Organization」
3. 观察页面跳转

### 预期结果
- 创建成功，组织状态为 `active`（邮箱域名匹配）
- 自动重定向到 `/dashboard`
- Dashboard 侧边栏显示 `Acme Corp` 组织名称
- 用户为该组织的 `owner`

### 预期数据状态
```sql
SELECT name, slug, domain, status FROM tenants WHERE slug = 'acme-corp';
-- 预期: status = 'active', domain = 'acme.com'

SELECT tu.role_in_tenant FROM tenant_users tu
JOIN tenants t ON t.id = tu.tenant_id
WHERE tu.user_id = '{user_id}' AND t.slug = 'acme-corp';
-- 预期: role_in_tenant = 'owner'
```

---

## 场景 3：Onboard 创建组织 — 域名不匹配进入 Pending 页

### 初始状态
- 用户在 `/onboard` 页面
- 用户邮箱为 `user@gmail.com`

### 目的
验证域名不匹配时组织进入 Pending 状态并显示等待页

### 测试操作流程
1. 在 `/onboard` 页面，填写：
   - Organization name: `My Startup`
   - Slug: `my-startup`
   - Domain: `mystartup.io`（与 gmail.com 不匹配）
2. 点击「Create Organization」
3. 观察页面跳转

### 预期结果
- 创建成功，组织状态为 `pending`
- 自动重定向到 `/onboard/pending`
- Pending 页面显示：
  - 标题「Your organization is pending activation」
  - 说明文字表明邮箱域名不匹配
  - 「Sign out」按钮
  - 返回链接

### 预期数据状态
```sql
SELECT name, slug, domain, status FROM tenants WHERE slug = 'my-startup';
-- 预期: status = 'pending', domain = 'mystartup.io'
```

---

## 场景 4：多租户用户 — 组织切换器

### 初始状态
- 用户属于 2 个或以上 active 租户
- 用户已登录

### 目的
验证 Dashboard 组织切换器正确显示并能切换活跃租户，并触发 token exchange

### 测试操作流程
1. 访问 `/dashboard`
2. 查看侧边栏顶部组织切换器区域
3. 点击组织切换器，展开下拉菜单
4. 点击第二个组织
5. 观察 Dashboard 变化

### 预期结果
- 组织切换器显示当前活跃组织名称和 logo（或首字母）
- 下拉菜单列出所有用户所属组织
- 当前活跃组织有选中标识（如 checkmark）
- 点击其他组织后：
  - 组织切换器更新为新选中的组织名称
  - Session 中的 `activeTenantId` 更新
  - 调用 `POST /api/v1/auth/tenant-token` 完成 tenant token 交换
  - 下拉菜单底部显示「Create new organization」链接
- 刷新页面后，切换保持（`activeTenantId` 持久化在 session 中）

---

## 场景 5：已有租户用户重新访问 /onboard — 自动重定向到 Dashboard

### 初始状态
- 用户已属于至少 1 个租户
- 用户已登录

### 目的
验证已有租户的用户不会停留在 /onboard 页面

### 测试操作流程
1. 在浏览器地址栏手动输入 `/onboard`
2. 观察页面行为
3. 手动输入 `/dashboard`
4. 观察页面行为

### 预期结果
- 访问 `/onboard` → 自动重定向到 `/dashboard`（onboard loader 检测到已有租户）
- 访问 `/dashboard` → 正常显示 Dashboard（不重定向到 /onboard）
- Dashboard 显示用户的活跃租户信息

---

## 通用场景：Demo 租户自动加入已移除

### 测试操作流程
1. 确保系统中存在 slug 为 `demo` 的租户
2. 创建一个全新的 Keycloak 用户
3. 使用该用户 OAuth 登录
4. 检查该用户的租户关联

### 预期结果
- 用户**不**自动加入 `demo` 租户
- 用户被重定向到 `/onboard` 页面
- `tenant_users` 表中无该用户与 demo 租户的关联记录

---

## 检查清单

| # | 场景 | 状态 | 测试日期 | 测试人员 | 备注 |
|---|------|------|----------|----------|------|
| 1 | 新用户首次登录 — 重定向到 Onboard | ☐ | | | |
| 2 | Onboard 创建组织 — 域名匹配进入 Dashboard | ☐ | | | |
| 3 | Onboard 创建组织 — 域名不匹配进入 Pending | ☐ | | | |
| 4 | 多租户用户 — 组织切换器 | ☐ | | | |
| 5 | 已有租户用户重访 /onboard — 重定向 | ☐ | | | |
| 6 | Demo 租户自动加入已移除（通用） | ☐ | | | |
