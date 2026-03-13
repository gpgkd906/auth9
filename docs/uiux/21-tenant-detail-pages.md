# UI/UX 测试 - Tenant 租户详情子页面

**模块**: 页面专项
**测试范围**: 租户概览详情、Webhooks 配置、SSO 连接器管理、Invitations 邀请管理
**场景数**: 5

---

## 场景 1：Tenant Detail 概览页网格布局与快速链接

### 目的
验证租户详情页的多列网格布局、统计卡片和快速链接导航的响应式表现。

### 测试操作流程
1. 访问 `/dashboard/tenants/:tenantId`。
2. 检查页面顶部返回按钮（ArrowLeftIcon）和租户名标题。
3. 验证概览统计卡片的网格排列。
4. 检查右侧快速链接（Services / Invitations / Webhooks / SSO）的 Badge 计数。
5. 缩小视口至移动端，验证网格从三列变为单列。

### 预期视觉效果
- **返回按钮**: ArrowLeftIcon + "Back to Tenants" 文字，`variant="ghost"`，`--text-secondary` 色。
- **租户名**: 24px `font-weight: 600`，`--text-primary`。
- **网格布局**: `grid-cols-1 lg:grid-cols-3`，左侧 2 列（`col-span-2`）配置区 + 右侧 1 列快速链接。
- **配置卡片**:
  - Card 标题 16-17px `font-weight: 600`。
  - Status Select（Active / Inactive / Suspended）使用标准 Select 组件，`border-radius: 12px`。
  - MFA Switch 组件与标签水平排列，Switch 垂直居中。
- **快速链接**:
  - Card 内垂直堆叠的按钮列表，`divide-y` 分隔。
  - 每项: 图标 + 名称 + Badge（数字计数），`variant="ghost"` 样式。
  - Badge: `variant="secondary"` pill，显示关联资源数量。
- **移动端**: 三列变单列，快速链接卡片在配置卡片下方。
- **useFetcher**: Status/MFA 更改使用 `useFetcher` 提交，不触发整页刷新。

---

## 场景 2：Webhooks 配置列表与创建 Dialog

### 目的
验证 Webhooks 页面的列表展示、事件复选框网格和密钥显示 Dialog。

### 测试操作流程
1. 访问 `/dashboard/tenants/:tenantId/webhooks`。
2. 检查 Webhook 列表项的状态指示（绿点/灰点）。
3. 点击"Create Webhook"打开 Dialog，验证事件复选框网格。
4. 创建后验证 Secret 显示 Dialog 的安全展示。
5. 点击"Test"按钮验证测试请求反馈。

### 预期视觉效果
- **Webhook 列表**: `divide-y` 分隔，每项 flex 布局：
  - 左侧: 状态指示点（绿色 = active，灰色 = inactive，`w-2 h-2 rounded-full`）。
  - 中间: URL（`font-mono text-sm`）+ 事件 Badge 列表。
  - 右侧: 操作按钮组（Test / Regenerate / Edit / Delete）。
- **事件 Badge**: 小型 pill（`text-xs`），`variant="secondary"`，多个横向排列 `flex-wrap gap-1`。
- **创建 Dialog**:
  - URL Input + 事件复选框网格（`grid-cols-2` 或 `grid-cols-3`）。
  - 复选框: 标准 Checkbox 组件，label 14px。
  - Active Switch: 默认开启。
- **Secret Dialog**:
  - 创建成功后弹出，一次性显示密钥。
  - 密钥: `font-mono` 代码块背景（`--bg-tertiary`），`border-radius: 10px`。
  - 复制按钮: 点击后图标变为 CheckIcon 反馈（1.5s 后恢复）。
  - 警告文字: `--accent-orange` 色 "This secret will only be shown once"。
- **Test 按钮**: 点击后显示 loading 状态，成功/失败后内联反馈（绿色/红色文字）。

---

## 场景 3：SSO 配置页条件字段渲染

### 目的
验证 SSO 页面的 SAML / OIDC 提供商类型切换时字段的条件渲染和布局。

### 测试操作流程
1. 访问 `/dashboard/tenants/:tenantId/sso`。
2. 观察 Provider Type 控件外观，确认其为项目统一 Select 组件。
3. 选择 Provider Type = SAML，验证展示的字段。
4. 切换为 OIDC，验证字段变化。
5. 检查已配置连接器的列表展示。

### 预期视觉效果
- **Provider Type Select**: 顶部 Select 组件（SAML / OIDC），Trigger 高度 40px、圆角 10px，样式与 Portal 其他选择器一致。
- **SAML 字段**: `SAML Entity ID` + `SAML SSO URL` + `SAML 签名证书`，布局为 `grid-cols-1 md:grid-cols-2`，证书输入与其他 Input 保持统一表单风格。
- **OIDC 字段**: `OIDC Client ID` + `OIDC Client Secret` + `OIDC Authorization URL` + `OIDC Token URL`，`grid-cols-1 md:grid-cols-2`。
- **字段切换**: 类型更改时字段区域平滑过渡（无布局跳动）。
- **下拉面板**: glass/popover 风格，选中项带勾选反馈，不出现浏览器原生 `<option>` 样式。
- **连接器列表**:
  - 每项: Card 样式，左侧提供商图标/名称 + 类型 Badge。
  - Switch: 启用/禁用切换。
  - 操作: Test Connection + Delete 按钮。
- **Test Connection**: 点击后 loading 状态 → 成功（绿色 "Connected"）/ 失败（红色错误详情）。

---

## 场景 4：Tenant Invitations 邀请管理列表

### 目的
验证邀请列表的状态 Badge 颜色编码和批量操作布局。

### 测试操作流程
1. 访问 `/dashboard/tenants/:tenantId/invitations`。
2. 检查邀请列表的状态 Badge（Pending / Accepted / Expired / Revoked）。
3. 创建新邀请验证 Dialog 表单。
4. 撤销邀请验证确认弹窗。

### 预期视觉效果
- **邀请列表**: Table 或 Card list，每项显示:
  - Email 地址（`--text-primary`，14px）。
  - 状态 Badge:
    - Pending: `variant="warning"` 橙色。
    - Accepted: `variant="success"` 绿色。
    - Expired: `--text-tertiary` 灰色。
    - Revoked: `variant="danger"` 红色。
  - 邀请时间: `FormattedDate`，`--text-secondary`。
  - 操作: Revoke 按钮（仅 Pending 状态可见），`variant="destructive"` 或 `variant="outline"` 红色文字。
- **创建 Dialog**: Email Input + Role Select + 到期时间 Input，标准 Dialog 布局。
- **确认弹窗**: AlertDialog 组件，"Are you sure?" 确认文字 + Cancel / Revoke 按钮。
- **空状态**: "No invitations" + "Send Invitation" CTA 按钮，居中展示。

---

## 场景 5：Tenant 子页面间导航一致性

### 目的
验证从 Tenant Detail 通过快速链接导航至各子页面后的面包屑和返回路径一致性。

### 测试操作流程
1. 从 Tenant Detail 点击 "Services" 快速链接。
2. 检查页面顶部返回按钮指向正确的 Tenant Detail 页。
3. 依次通过快速链接访问 Invitations → Webhooks → SSO。
4. 验证每个子页面的返回按钮和页面标题。

### 预期视觉效果
- **返回按钮**: 所有子页面顶部统一使用 ArrowLeftIcon + "Back to [Tenant Name]"，`variant="ghost"`。
- **页面标题**: 24px `font-weight: 600`，紧跟在返回按钮下方，间距 `mb-6`（24px）。
- **内容区**: 与主 Dashboard 页面共享侧边栏，内容区宽度一致。
- **面包屑**: 可选。若有面包屑则使用 "/" 分隔，当前页面 `--text-primary`，父级 `--accent-blue` 可点击。
- **加载状态**: 页面切换时无白屏闪烁（React Router 7 的 loader 先加载数据）。
- **主题持续**: 子页面之间切换不触发主题重置。
