# Auth9 用户操作指南

本指南详细说明了 Auth9 管理系统的主要操作流程，包括租户管理、服务注册、角色权限管理、用户关联配置，以及安全相关的功能配置。

## 目录

1. [租户管理](#1-租户管理-tenant-management)
2. [服务注册](#2-服务注册-service-registration)
3. [角色与权限管理](#3-角色与权限管理-rbac)
4. [用户与权限关联](#4-用户与权限关联-user-provisioning)
5. [邀请系统](#5-邀请系统-invitation-system)
6. [品牌定制](#6-品牌定制-branding)
7. [邮件模板](#7-邮件模板-email-templates)
8. [密码管理](#8-密码管理-password-management)
9. [会话管理](#9-会话管理-session-management)
10. [Passkey 管理](#10-passkey-管理-webauthn)
11. [社交登录与企业 SSO](#11-社交登录与企业-sso-identity-providers)
12. [分析与监控](#12-分析与监控-analytics)
13. [安全告警](#13-安全告警-security-alerts)
14. [Webhook 配置](#14-webhook-配置)
15. [常见问题](#15-常见问题)
16. [常见工作流程](#常见工作流程)

## 快速开始

首次使用 Auth9？按照以下步骤快速上手：

1. **登录系统** - 使用管理员账号登录 Auth9 Portal
2. **创建租户** - 创建第一个租户组织
3. **注册服务** - 注册您的应用程序
4. **创建角色** - 为服务定义角色和权限
5. **邀请用户** - 添加用户并分配角色

详细操作请参考下面的各个章节。

---

## 1. 租户管理 (Tenant Management)

租户是系统的核心隔离单位。

### 创建租户
1. 点击左侧导航栏的 **Tenants**。
2. 点击右上角的 **+ Create Tenant** 按钮。
3. 填写以下信息：
   - **Name**: 租户显示名称 (例如 "Acme Corp")。
   - **Slug**: 租户唯一标识符，用于 URL (例如 "acme")。必须唯一。
   - **Logo URL**: (可选) 租户 Logo 图片地址。
4. 点击 **Create** 完成创建。

### 设置租户
当前租户设置主要通过 "Edit" 功能进行：
1. 在租户列表中找到目标租户。
2. 点击右侧的 `...` 菜单，选择 **Edit**。
3. 修改名称、Slug 或 Logo URL。
4. 点击 **Save Changes**。
*(注：更高级的租户特定设置将在后续版本中通过专门的 Settings 页面提供)*

---

## 2. 服务注册 (Service Registration)

服务 (Service) 代表接入 Auth9 的应用程序 (OIDC Client)。

### 注册新服务
1. 点击左侧导航栏的 **Services**。
2. 点击右上角的 **+ Register Service** 按钮。
3. 填写信息：
   - **Service Name**: 应用名称。
   - **Client ID**: (可选) 自定义 Client ID，留空则自动生成。
   - **Base URL**: 应用的主 URL (例如 `https://myapp.com`)。
   - **Redirect URIs**: OIDC 回调地址 (例如 `https://myapp.com/callback`)。多个地址用逗号分隔。
   - **Logout URIs**: 登出回调地址。
4. 点击 **Register**。

### 获取 Client ID 和 Client Secret
1. 在 Services 列表中，找到刚创建的服务。
2. 点击右侧菜单的 **Edit**。
3. 在弹出的对话框底部 "Client Credentials" 区域：
   - **Client ID**: 直接显示在界面上。
   - **Client Secret**: 点击 **Regenerate Client Secret** 按钮。
     - **警告**: 此操作会使旧的 Secret 失效。
     - 系统会弹出一个对话框显示新的 Secret。**请立即复制保存**，关闭对话框后将无法再次查看 Secret。

---

## 3. 角色与权限管理 (RBAC)

角色 (Role) 是基于服务 (Service) 定义的。租户本身不定义角色，而是使用服务定义的角色。

### 定义角色
1. 点击左侧导航栏的 **Roles**。
2. 页面会列出所有已注册的服务。
3. 找到目标服务，点击该服务区块右上角的 **+ Add Role**。
4. 输入：
   - **Role Name**: 角色标识 (例如 `admin`, `editor`).
   - **Description**: 角色描述。
5. 点击 **Create**。

*(注：权限 Permission 目前主要由开发人员在后端预设，管理员主要负责组合 Role)*

---

## 4. 用户与权限关联 (User Provisioning)

将用户加入租户并分配角色。

### 邀请/创建用户
1. 点击左侧导航栏的 **Users**。
2. *(当前版本用户通过 OIDC 登录自动创建，或通过管理员 API 创建，暂无前端创建按钮，后续添加)*
3. 在用户列表中找到目标用户。

### 关联用户到租户 (Assign Tenant)
1. 点击用户行右侧的 `...` 菜单。
2. 选择 **Manage Tenants**。
3. 在 "Add to Tenant" 区域：
   - 选择 **Tenant**。
   - 选择初始 **Role** (如 `member`)。
   - 点击 **Add**。
4. 用户现在已加入该租户列表。

### 分配服务角色 (Assign Roles)
在 **Manage Tenants** 对话框中，针对已加入的租户：
1. 点击 **Roles** 按钮。
2. 在弹出框中：
   - 选择 **Service** (即该角色所属的应用)。
   - 系统会列出该服务下可用的所有角色。
   - 勾选通过复选框 (Checkbox) 为用户分配的角色。
3. 点击 **Save Roles**。
4. 用户在此租户下即拥有了该服务对应的角色权限。

---

## 5. 邀请系统 (Invitation System)

Auth9 提供邮件邀请功能，让您可以快速邀请用户加入租户。

### 发送邀请

1. 点击左侧导航栏的 **Users** > **Invitations**。
2. 点击右上角的 **Send Invitation** 按钮。
3. 填写邀请信息：
   - **Email**: 受邀用户的邮箱地址
   - **Tenant**: 选择目标租户
   - **Role**: 为用户分配的初始角色
   - **Expiry**: 邀请过期时间（默认 7 天）
   - **Custom Message**: 可选的欢迎消息
4. 点击 **Send** 发送邀请。

系统会自动发送邀请邮件到用户邮箱，包含注册链接。

### 查看邀请列表

在 Invitations 页面可以看到：
- **Pending**: 已发送但未接受的邀请
- **Accepted**: 已接受的邀请
- **Expired**: 已过期的邀请
- **Revoked**: 已撤销的邀请

### 管理邀请

对于每个邀请，您可以：
- **Resend**: 重新发送邀请邮件（生成新的链接）
- **Revoke**: 撤销邀请（使链接失效）
- **View Details**: 查看邀请详情

### 用户接受邀请

1. 用户收到邮件后，点击邮件中的邀请链接
2. 如果已有账号，登录即可加入租户
3. 如果是新用户，需要完成注册：
   - 设置密码
   - 填写个人信息
   - 确认邮箱
4. 注册完成后自动加入租户并获得指定角色

### 批量邀请

对于需要邀请多个用户的场景：
1. 准备 CSV 文件，包含邮箱和角色信息
2. 在 Invitations 页面点击 **Bulk Import**
3. 上传 CSV 文件
4. 预览并确认邀请列表
5. 点击 **Send All** 批量发送

---

## 6. 品牌定制 (Branding)

自定义租户的视觉外观，打造专属品牌体验。

### 配置品牌元素

1. 导航到 **Settings** > **Branding**。
2. 选择要配置的租户。
3. 配置以下元素：

#### 上传 Logo
- **主 Logo**: 用于 Portal 顶部导航栏（推荐 200x60 px）
- **小 Logo**: 用于移动端和 Favicon（推荐 40x40 px）
- **登录页 Logo**: 用于 Keycloak 登录页面（推荐 300x100 px）

上传步骤：
1. 点击对应的 **Upload** 按钮
2. 选择图片文件（PNG、SVG、JPEG，最大 2MB）
3. 系统自动裁剪和优化
4. 点击 **Save**

**提示**: 建议使用透明背景的 PNG 或 SVG 格式

#### 配置颜色主题
1. 点击颜色选择器
2. 选择或输入 HEX 颜色码
3. 可配置的颜色：
   - **Primary Color**: 主要按钮、链接（默认 #007AFF）
   - **Secondary Color**: 次要按钮、辅助元素
   - **Accent Color**: 强调色、提示
   - **Background Color**: 页面背景
   - **Text Color**: 主要文字
   - **Border Color**: 边框、分隔线
4. 查看右侧实时预览
5. 点击 **Apply Theme** 应用

**预设主题**：
- 💙 Auth9 Blue (默认)
- 🟣 Purple Dream
- 🟢 Nature Green
- 🔴 Passionate Red
- ⚫ Dark Mode

#### 自定义字体
选择字体系列：
- Inter (默认)
- SF Pro (苹果风格)
- Roboto (Material Design)
- 思源黑体 (Noto Sans CJK)

### 品牌应用范围

品牌配置会自动应用到：
- ✅ Auth9 Portal 管理界面
- ✅ Keycloak 登录页面
- ✅ 系统邮件模板
- ✅ 移动端应用（未来支持）

### 重置品牌配置

如需恢复默认设置：
1. 在 Branding 页面点击 **Reset to Default**
2. 确认操作
3. 所有品牌配置将恢复为系统默认值

---

## 7. 邮件模板 (Email Templates)

自定义系统发送的所有邮件的外观和内容。

### 邮件类型

Auth9 支持自定义以下邮件模板：

| 类型 | 触发时机 |
|------|---------|
| **Welcome Email** | 用户注册成功 |
| **Invitation Email** | 发送邀请 |
| **Password Reset** | 请求重置密码 |
| **Password Changed** | 密码修改成功 |
| **Email Verification** | 注册或更改邮箱 |
| **MFA Setup** | 启用多因素认证 |
| **Login Alert** | 异常登录检测 |
| **Session Revoked** | 会话被撤销 |
| **Account Locked** | 账户被锁定 |

### 编辑邮件模板

1. 导航到 **Settings** > **Email Templates**。
2. 选择要编辑的邮件类型。
3. 编辑模板内容：
   - **Subject**: 邮件主题
   - **From Name**: 发件人名称
   - **From Email**: 发件人邮箱
   - **Reply To**: 回复邮箱（可选）
   - **HTML Body**: HTML 格式邮件正文
   - **Text Body**: 纯文本格式邮件正文
4. 使用右侧预览查看效果。

### 动态变量

邮件模板支持动态变量，用 `{{variable}}` 语法：

**全局变量**（所有邮件可用）：
- `{{tenant_name}}` - 租户名称
- `{{tenant_logo_url}}` - 租户 Logo URL
- `{{portal_url}}` - Portal 访问地址
- `{{support_email}}` - 支持邮箱
- `{{current_year}}` - 当前年份
- `{{primary_color}}` - 品牌主色

**用户变量**：
- `{{user_name}}` - 用户姓名
- `{{user_email}}` - 用户邮箱
- `{{user_first_name}}` - 名
- `{{user_last_name}}` - 姓

**特定邮件变量**（根据邮件类型不同）：
- 邀请邮件：`{{invitation_url}}`, `{{sender_name}}`, `{{role_name}}`
- 密码重置：`{{reset_url}}`, `{{expires_in_minutes}}`
- 登录告警：`{{login_ip}}`, `{{login_location}}`, `{{login_device}}`

### 测试邮件模板

在保存前测试邮件效果：
1. 在模板编辑页面点击 **Send Test Email**
2. 输入测试邮箱地址
3. 系统发送测试邮件到指定邮箱
4. 检查邮件显示效果

### 多语言支持

如果需要支持多语言：
1. 在模板编辑页面选择 **Add Translation**
2. 选择目标语言（英文、中文、日文等）
3. 为每种语言编辑独立的内容
4. 系统会根据用户语言偏好自动选择

### 重置模板

恢复为系统默认模板：
1. 在模板编辑页面点击 **Reset to Default**
2. 确认操作
3. 模板内容将恢复为系统默认版本

---

## 8. 密码管理 (Password Management)

Auth9 提供完整的密码管理功能，包括密码重置、密码修改和密码策略配置。

### 密码重置流程

#### 用户忘记密码
1. 在登录页面点击 **Forgot Password** 链接。
2. 输入注册邮箱地址。
3. 点击 **Send Reset Link**。
4. 检查邮箱，点击重置链接。
5. 在重置页面输入新密码并确认。
6. 点击 **Reset Password** 完成。

#### 管理员触发密码重置
1. 导航到 **Users** 页面。
2. 找到目标用户，点击 `...` 菜单。
3. 选择 **Reset Password**。
4. 系统会发送重置邮件到用户邮箱。

### 修改密码 (当前用户)
1. 点击右上角用户头像。
2. 选择 **Settings** > **Security**。
3. 在 "Change Password" 区域：
   - 输入 **Current Password**（当前密码）
   - 输入 **New Password**（新密码，至少8位）
   - 输入 **Confirm Password**（确认新密码）
4. 点击 **Change Password**。

### 配置密码策略 (管理员)
1. 导航到 **Settings** > **Security**。
2. 选择目标租户（如果有多个）。
3. 在 "Password Policy" 区域配置：
   - **Minimum Length**: 最小密码长度（默认8）
   - **Maximum Length**: 最大密码长度（默认128）
   - **Password Expiry**: 密码过期天数（0表示永不过期）
   - **Password History**: 记住的历史密码数量（防止重复使用）
   - **Lockout Threshold**: 锁定前允许的失败次数
   - **Lockout Duration**: 锁定持续时间（分钟）
   - **Require Uppercase**: 要求大写字母
   - **Require Lowercase**: 要求小写字母
   - **Require Numbers**: 要求数字
   - **Require Symbols**: 要求特殊符号
4. 点击 **Save Policy**。

---

## 9. 会话管理 (Session Management)

管理用户的活跃会话，支持查看和撤销会话。

### 查看我的会话
1. 点击右上角用户头像。
2. 选择 **Settings** > **Sessions**。
3. 页面显示：
   - **Current Session**: 当前会话信息（设备、IP、最后活跃时间）
   - **Other Sessions**: 其他设备上的活跃会话列表

### 撤销单个会话
1. 在 "Other Sessions" 列表中找到要撤销的会话。
2. 点击该会话右侧的 **Revoke** 按钮。
3. 确认撤销操作。

### 撤销所有其他会话
1. 在 Sessions 页面找到 **Sign out all other sessions** 按钮。
2. 点击确认。
3. 系统会登出除当前会话外的所有会话。

### 管理员强制登出用户
1. 导航到 **Users** 页面。
2. 找到目标用户，点击 `...` 菜单。
3. 选择 **Force Logout**。
4. 用户的所有活跃会话将被终止。

### 会话信息说明
每个会话显示以下信息：
- **Device**: 设备类型和浏览器（如 "Desktop - Chrome"）
- **IP Address**: 登录IP地址
- **Location**: 地理位置（基于IP）
- **Last Active**: 最后活跃时间
- **Created**: 会话创建时间

---

## 10. Passkey 管理 (WebAuthn)

Passkey 是一种更安全、更便捷的无密码认证方式，基于 WebAuthn 标准。

### 添加 Passkey
1. 导航到 **Settings** > **Passkeys**。
2. 点击 **Add Passkey** 按钮。
3. 系统会跳转到 Keycloak 的 WebAuthn 注册页面。
4. 按照浏览器提示完成注册：
   - 使用指纹、面容识别或安全密钥
   - 为 Passkey 命名（可选）
5. 注册成功后自动返回 Auth9。

### 查看已注册的 Passkey
在 **Settings** > **Passkeys** 页面可以看到所有已注册的 Passkey：
- **Label**: Passkey 名称
- **Type**: 类型（Passwordless 或 Two-Factor）
- **Created**: 创建时间

### 删除 Passkey
1. 在 Passkey 列表中找到要删除的凭据。
2. 点击右侧的 **Delete** 按钮。
3. 确认删除操作。

**注意**:
- 删除 Passkey 后，该设备/密钥将无法用于登录
- 建议至少保留一种认证方式

### Passkey 的优势
- **更安全**: 抵抗钓鱼攻击，私钥不离开设备
- **更便捷**: 使用生物识别（指纹/面容）快速登录
- **跨设备**: 支持云同步，可在多设备使用

---

## 11. 社交登录与企业 SSO (Identity Providers)

配置第三方身份提供商，支持社交登录和企业 SSO。

### 支持的身份提供商
- **Google**: Google 账号登录
- **GitHub**: GitHub 账号登录
- **Microsoft**: Microsoft/Azure AD 登录
- **OpenID Connect**: 通用 OIDC 提供商
- **SAML 2.0**: 企业级 SAML 集成

### 添加身份提供商
1. 导航到 **Settings** > **Identity Providers**。
2. 点击 **Add Provider** 按钮。
3. 选择提供商类型（Google/GitHub/OIDC/SAML 等）。
4. 填写配置信息：

**Google/GitHub/Microsoft**:
- **Alias**: 唯一标识符
- **Display Name**: 显示名称
- **Client ID**: 从提供商控制台获取
- **Client Secret**: 从提供商控制台获取

**OIDC (通用)**:
- **Alias**: 唯一标识符
- **Display Name**: 显示名称
- **Client ID**: OIDC Client ID
- **Client Secret**: OIDC Client Secret
- **Authorization URL**: 授权端点
- **Token URL**: Token 端点

**SAML 2.0**:
- **Alias**: 唯一标识符
- **Display Name**: 显示名称
- **Entity ID**: IdP Entity ID
- **SSO URL**: 单点登录 URL
- **Certificate**: IdP 公钥证书

5. 点击 **Create** 保存配置。

### 启用/禁用身份提供商
1. 在身份提供商列表中找到目标提供商。
2. 使用 **Enabled** 开关切换状态。

### 查看用户关联账户
1. 导航到 **Settings** > **Linked Accounts**。
2. 页面显示当前用户已关联的第三方账户。
3. 可以关联新账户或解绑已有关联。

### 解绑第三方账户
1. 在 Linked Accounts 页面找到要解绑的账户。
2. 点击 **Unlink** 按钮。
3. 确认解绑操作。

**注意**: 确保解绑后仍有其他登录方式（如密码或其他关联账户）。

---

## 12. 分析与监控 (Analytics)

查看登录统计和用户活动分析。

### 查看分析仪表板
1. 点击左侧导航栏的 **Analytics**。
2. 使用时间范围选择器筛选数据（7天/14天/30天/90天）。
3. 查看关键指标：
   - **Total Logins**: 总登录次数
   - **Successful Logins**: 成功登录次数和成功率
   - **Failed Logins**: 失败登录次数
   - **Unique Users**: 独立用户数

### 查看事件类型分布
分析仪表板显示以下分布图表：
- **By Event Type**: 按事件类型分布（登录成功、密码错误、MFA失败等）
- **By Device Type**: 按设备类型分布（Desktop/Mobile/Tablet）

### 查看登录事件日志
1. 在 Analytics 页面点击 **View Login Events**。
2. 或直接导航到 **Analytics** > **Events**。
3. 事件日志包含：
   - **Time**: 事件时间
   - **Event Type**: 事件类型
     - ✅ Login Success - 登录成功
     - ✅ Social Login - 社交登录成功
     - ❌ Wrong Password - 密码错误
     - ❌ MFA Failed - MFA 验证失败
     - 🔒 Account Locked - 账户被锁定
   - **User**: 用户邮箱或ID
   - **IP Address**: 来源IP
   - **Device**: 设备类型
   - **Details**: 详细信息（失败原因、地理位置等）

### 分页和筛选
- 使用页码导航浏览更多事件
- 事件按时间倒序排列（最新在前）

---

## 13. 安全告警 (Security Alerts)

监控和响应安全威胁。

### 查看安全告警
1. 导航到 **Security** > **Alerts**。
2. 默认显示所有告警，可切换筛选：
   - **All**: 所有告警
   - **Unresolved**: 仅未解决的告警（显示数量）

### 告警类型
系统自动检测以下安全威胁：
- **Brute Force Attack**: 暴力破解尝试（短时间内多次失败登录）
- **New Device Login**: 新设备登录
- **Impossible Travel**: 不可能的旅行（短时间内从不同地理位置登录）
- **Suspicious IP**: 可疑IP地址

### 告警严重级别
- 🔴 **Critical**: 严重 - 需要立即处理
- 🟠 **High**: 高 - 需要尽快处理
- 🟡 **Medium**: 中等 - 需要关注
- 🟢 **Low**: 低 - 仅供参考

### 处理告警
1. 查看告警详情（用户ID、时间、JSON详情）。
2. 评估威胁严重性。
3. 采取必要措施：
   - 强制用户登出
   - 重置用户密码
   - 启用 MFA
4. 点击 **Resolve** 标记为已解决。

### 安全建议
- 24小时内审查所有 Critical 告警
- 为管理员账户启用 MFA
- 配置速率限制防止暴力破解
- 设置 Webhook 实时通知安全事件
- 定期审查活跃会话

---

## 14. Webhook 配置

配置 Webhook 接收实时事件通知。

### 支持的事件类型
- `login.success` - 登录成功
- `login.failed` - 登录失败
- `user.created` - 用户创建
- `user.updated` - 用户更新
- `user.deleted` - 用户删除
- `password.changed` - 密码修改
- `mfa.enabled` - MFA 启用
- `mfa.disabled` - MFA 禁用
- `session.revoked` - 会话撤销
- `security.alert` - 安全告警

### 创建 Webhook
1. 导航到 **Tenants** 页面。
2. 点击目标租户进入详情。
3. 选择 **Webhooks** 标签页。
4. 点击 **Add Webhook** 按钮。
5. 填写配置：
   - **Name**: Webhook 名称
   - **Endpoint URL**: 接收通知的 HTTPS 端点
   - **Secret**: (可选) 用于签名验证的密钥
   - **Events**: 勾选要订阅的事件类型
   - **Enabled**: 是否启用
6. 点击 **Create**。

### 测试 Webhook
1. 在 Webhook 列表中找到目标 Webhook。
2. 点击 **Test** 按钮。
3. 系统发送测试请求并显示结果：
   - **Status Code**: HTTP 响应状态码
   - **Response Time**: 响应时间（毫秒）

### Webhook 签名验证
如果配置了 Secret，Auth9 会在请求头中包含 HMAC 签名：
```
X-Auth9-Signature: sha256=<signature>
```

验证示例（Node.js）：
```javascript
const crypto = require('crypto');

function verifySignature(payload, signature, secret) {
  const expected = 'sha256=' + crypto
    .createHmac('sha256', secret)
    .update(payload)
    .digest('hex');
  return crypto.timingSafeEqual(
    Buffer.from(signature),
    Buffer.from(expected)
  );
}
```

### 编辑/删除 Webhook
- **Edit**: 点击 Webhook 右侧的编辑按钮修改配置
- **Delete**: 点击删除按钮移除 Webhook

### 监控 Webhook 状态
Webhook 列表显示：
- **Status**: 启用状态（绿色/灰色指示点）
- **Failure Count**: 失败次数（如有）
- **Last Triggered**: 最后触发时间

---

## 15. 常见问题

### 认证问题
- **无法登录？** 检查 Redirect URI 是否配置正确。
- **Secret 丢失？** 使用 **Regenerate Client Secret** 生成新的。
- **MFA 设备丢失？** 联系管理员重置 MFA。

### 密码问题
- **忘记密码？** 使用登录页的 "Forgot Password" 功能。
- **密码不符合策略？** 检查密码策略要求（长度、复杂度等）。
- **账户被锁定？** 等待锁定时间过期或联系管理员解锁。

### 会话问题
- **看不到其他会话？** 可能只有当前设备登录。
- **会话被意外终止？** 检查是否有管理员强制登出或密码被修改。

### Passkey 问题
- **无法注册 Passkey？** 确保浏览器支持 WebAuthn（Chrome、Firefox、Safari 最新版）。
- **Passkey 登录失败？** 检查设备是否支持，或尝试删除重新注册。

### SSO 问题
- **第三方登录失败？** 检查身份提供商配置（Client ID、Secret、回调URL）。
- **SAML 登录循环？** 检查 Entity ID 和证书配置是否正确。

### Webhook 问题
- **Webhook 未触发？** 检查是否启用，以及订阅的事件类型是否正确。
- **签名验证失败？** 确保使用正确的 Secret 和签名算法。

---

## 相关文档

详细的技术文档请参考 Wiki：
- [多租户管理](../wiki/多租户管理.md)
- [RBAC权限系统](../wiki/RBAC权限系统.md)
- [认证流程](../wiki/认证流程.md)
- [REST API](../wiki/REST-API.md)
- [邀请系统](../wiki/邀请系统.md)
- [品牌定制](../wiki/品牌定制.md)
- [邮件模板](../wiki/邮件模板.md)
- [密码管理](../wiki/密码管理.md)
- [会话管理](../wiki/会话管理.md)
- [WebAuthn与Passkey](../wiki/WebAuthn与Passkey.md)
- [社交登录与SSO](../wiki/社交登录与SSO.md)
- [分析与安全告警](../wiki/分析与安全告警.md)

## 常见工作流程

### 新租户上线流程

1. 创建租户（设置名称、标识符、Logo）
2. 配置租户品牌（Settings > Branding）
3. 配置密码策略（Settings > Security）
4. 添加身份提供商（Settings > Identity Providers）- 可选
5. 注册应用服务（Services）
6. 为服务创建角色（Roles）
7. 邀请用户加入租户
8. 为用户分配角色

### 新服务接入流程

1. 注册服务（填写名称、回调地址等）
2. 获取 Client ID 和 Client Secret
3. 为服务创建角色和权限
4. 配置服务的 Webhook（可选）
5. 在服务端集成 OIDC 认证
6. 使用 Token Exchange 获取租户访问令牌
7. 根据角色和权限进行授权判断

### 安全加固流程

1. 配置强密码策略（最小长度、复杂度要求）
2. 启用账户锁定机制（失败次数、锁定时长）
3. 为管理员账户启用 Passkey 或 MFA
4. 配置社交登录（减少密码使用）
5. 设置 Webhook 接收安全告警
6. 定期查看安全告警（Security > Alerts）
7. 定期审查活跃会话
8. 查看登录分析，识别异常模式

### 用户管理流程

1. 发送邀请邮件给新用户
2. 用户通过邮件链接注册账号
3. 为用户分配初始角色
4. 根据需要调整用户角色
5. 监控用户登录活动（Analytics）
6. 必要时撤销用户会话或重置密码
7. 用户离职时移除租户关联

---

**文档版本**: 1.3.0  
**最后更新**: 2026-02-04  
**适用版本**: Auth9 v0.1.0+

**更新内容**:
- 新增邀请系统操作指南
- 新增品牌定制操作指南
- 新增邮件模板操作指南
- 更新章节编号和目录结构
