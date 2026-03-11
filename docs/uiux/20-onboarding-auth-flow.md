# UI/UX 测试 - Onboarding 引导流程与认证页面

**模块**: 页面专项
**测试范围**: 首次引导创建组织、Pending 等待页、登录页多方式切换、注册页、忘记密码、邀请接受、租户选择
**场景数**: 6

---

## 场景 1：Onboarding 首次引导页 Liquid Glass 居中布局

### 目的
验证 Onboarding 页面的全屏居中卡片布局、品牌标识和表单自动填充逻辑。

### 测试操作流程
1. 以新用户身份访问系统（无已创建组织），自动重定向至 `/onboard`。
2. 检查页面居中卡片的玻璃效果和品牌 Logo（"A9"）。
3. 输入组织名称，验证 Slug 自动生成。
4. 验证 Domain 字段从用户邮箱自动填充。
5. 缩小至移动端验证布局。

### 预期视觉效果
- **页面布局**: 全屏居中（`flex items-center justify-center min-h-screen`），动态背景 `page-backdrop`。
- **品牌 Logo**: "A9" 文字图标，`logo-icon` class，居中置顶。
- **Card**: `liquid-glass` 效果，`max-w-md`（448px），`border-radius: 20px`。
- **入场动画**: `animate-fade-in-up`，500ms，`cubic-bezier(0.34, 1.56, 0.64, 1)`。
- **表单字段**: Organization Name → Slug → Domain，`space-y-4` 间距。
- **Slug 自动生成**: 实时将 Name 转为小写 + 连字符格式，手动编辑后停止自动同步。
- **Domain 预填**: 从用户邮箱的 `@` 后部分提取，可手动修改。
- **ThemeToggle**: 固定定位 `top: 24px; right: 24px`，不被卡片遮挡。
- **提交按钮**: `variant="default"` 蓝色，`w-full` 全宽。

---

## 场景 2：Onboarding Pending 等待页与操作引导

### 目的
验证组织创建后等待审批的 Pending 页面信息展示和操作按钮。

### 测试操作流程
1. 创建组织后重定向至 `/onboard/pending`。
2. 检查等待状态信息卡片。
3. 验证"Try another domain"和"Sign out"按钮。

### 预期视觉效果
- **卡片**: 居中 Card，与 Onboarding 页保持相同 `max-w-md` 宽度和玻璃效果。
- **状态图标**: 钟表或等待图标，`h-12 w-12`，`--accent-orange` 色（警告/等待语义）。
- **消息**: 标题 "Pending Approval"（或 i18n 等效），描述文字 `--text-secondary`。
- **按钮组**:
  - "Try another domain": `variant="outline"`，全宽。
  - "Sign out": `variant="ghost"` 或 `variant="destructive"`，全宽。
  - 按钮间距 `gap-3`（12px），垂直堆叠。
- **入场动画**: 与 Onboarding 页一致的 `animate-fade-in-up`。

---

## 场景 3：Login 页面多认证方式切换布局

### 目的
验证登录页面 SSO / 密码 / Passkey 三种认证方式的切换体验和视觉一致性。

### 测试操作流程
1. 访问 `/login`。
2. 检查默认展示状态（SSO Email 输入或密码登录）。
3. 切换至 Passkey 登录方式。
4. 验证错误消息（OAuth 错误参数）的友好展示。
5. 检查底部链接（Forgot Password / Register）的样式。

### 预期视觉效果
- **页面布局**: 全屏居中，`page-backdrop` 动态背景，Card `max-w-md`。
- **Card 玻璃效果**: 标准 `liquid-glass`，`border-radius: 20px`。
- **认证方式分隔**: "or" 文字分隔线，使用 `--glass-border-subtle` 横线 + 居中文字。
- **SSO 输入**: Email Input + "Continue with SSO" Button（`variant="default"`）。
- **密码登录**: 指向 Keycloak 的 Button（`variant="glass"` 或 `variant="default"`）。
- **Passkey 登录**: 按钮触发 WebAuthn，提交中显示 loading 状态。
- **错误消息**: 映射自 `mapOAuthError()`，红色背景 + `--accent-red` 文字，`border-radius: 12px`。
- **底部链接**: `--accent-blue` 色，`text-sm`（14px），hover 时 `underline`。
- **入场动画**: `animate-fade-in-up`，卡片从底部渐入。
- **主题/语言控件**: 位于页面右上角，不遮挡卡片内容。

---

## 场景 4：Register 注册页与 Forgot Password 页一致性

### 目的
验证注册页和忘记密码页与登录页保持相同的布局框架和设计语言。

### 测试操作流程
1. 访问 `/register`，检查页面结构。
2. 访问 `/forgot-password`，检查表单和成功状态切换。
3. 对比三个页面（Login / Register / Forgot Password）的 Card 宽度、圆角、间距一致性。

### 预期视觉效果
- **注册页**:
  - 字段: Email + Display Name + Password，`space-y-4` 间距。
  - 提交按钮: "Create Account" `variant="default"` 蓝色全宽。
  - 底部: "Already have an account?" + `Link to /login`，`--accent-blue` 色。
  - 错误框: 红色背景 `--accent-red-light` + `--accent-red` 文字。
- **忘记密码页**:
  - 初始态: Email Input + "Send Reset Link" 按钮。
  - 成功态: 绿色背景消息 "Check your email" + "Try again" 链接。
  - 安全设计: 始终返回成功（不泄露邮箱是否注册）。
- **一致性检查**:
  - 三个页面 Card 宽度均为 `max-w-md`（448px）。
  - 均使用 `animate-fade-in-up` 入场动画。
  - 均有 `page-backdrop` 动态背景。
  - 品牌标识位置和样式一致。

---

## 场景 5：Invite Accept 邀请接受页多状态展示

### 目的
验证邀请接受页面在不同邀请状态下的视觉反馈正确性。

### 测试操作流程
1. 访问 `/invite/accept?token=<valid_token>`，验证 Pending 状态表单。
2. 模拟过期 Token，验证 Expired 状态展示。
3. 模拟已接受 Token，验证 Accepted 状态展示。
4. 模拟已撤销 Token，验证 Revoked 状态展示。
5. 模拟无效 Token，验证 Invalid 状态展示。

### 预期视觉效果
- **Pending（可接受）**: 标准注册表单（Email + Display Name + Password）+ 隐藏的 token 字段。
- **Expired**: 橙色图标 + "Invitation Expired" 标题 + 描述，无可操作按钮。
- **Accepted**: 绿色图标 + "Already Accepted" 标题 + 跳转登录链接。
- **Revoked**: 红色图标 + "Invitation Revoked" 标题 + 描述。
- **Invalid**: 灰色图标 + "Invalid Invitation" 标题。
- **通用样式**:
  - 所有状态使用相同 Card `max-w-md` 居中布局。
  - 图标尺寸 `h-12 w-12`，颜色与状态语义匹配。
  - 已有账户提示: "Already have an account?" + Link 到 `/login?invite_token=xxx`。

---

## 场景 6：Tenant Select 租户选择页搜索与选中状态

### 目的
验证多租户用户的租户选择页面搜索过滤、选中状态和自动跳转逻辑。

### 测试操作流程
1. 以多租户用户登录，访问 `/tenant/select`。
2. 检查租户列表的展示样式。
3. 使用搜索框过滤租户，验证实时过滤。
4. 点击某租户，验证选中高亮和加载状态。
5. 验证单租户用户的自动跳转（不显示选择页）。

### 预期视觉效果
- **页面布局**: 全屏居中 Card，`max-w-md`，标准玻璃效果。
- **搜索框**: 顶部 Input，`auto-focus`，`h-10`，`border-radius: 12px`，`placeholder="Search tenants..."`。
- **租户列表**: 垂直堆叠，每项为按钮样式 tile。
  - 左侧: 圆形头像（首字母，`border-radius: 50%`，`w-10 h-10`）。
  - 中间: 租户名（16px `font-medium`）+ Slug（13px `--text-secondary`）。
  - 右侧: 当前活跃租户显示蓝色指示点或 Badge。
- **选中状态**: 点击后 tile 使用 `--accent-blue-light` 背景 + `--accent-blue` 边框。
- **禁用状态**: 提交期间所有 tile `opacity: 0.5` + `pointer-events: none`。
- **滚动**: 20+ 租户时列表容器 `max-h-[400px] overflow-y-auto`。
- **计数文字**: 底部 "Showing X of Y tenants"，13px `--text-secondary`。
- **空搜索**: "No tenants found" 文字，`--text-tertiary`，居中。
