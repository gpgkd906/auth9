# 认证流程 - Dark Mode 认证页对比度回归

**模块**: 认证流程
**测试范围**: Portal 忘记密码/重置密码页面与 Auth9 品牌认证页在 Dark Mode 下的对比度、层级和非回归检查
**场景数**: 5

---

## 背景说明

本用例用于验证 2026-03-13 的 Dark Mode 对比度修正：

- `auth9-portal` 的 `/forgot-password` 与 `/reset-password`
- Auth9 品牌认证页承载的登录/忘记密码/信息页

## 入口可见性说明

本文件要求优先从 Portal `/login` 或其他可见认证入口进入目标页面，禁止仅依赖手输 URL 跳过入口可见性验证。

本回归只验证视觉层级与可读性，不替代密码功能本身的主流程验收。密码能力正确性仍以 [03-password.md](./03-password.md) 为主。

---

## 场景 1：从 Portal 可见入口进入忘记密码页，Dark Mode 控件布局正确

### 初始状态
- `auth9-portal` 已启动
- 浏览器可访问 `http://localhost:3000/login`
- 当前浏览器已切换为 Dark Mode，或右上角主题切换已设为 Dark

### 目的
验证用户可以从可见入口进入忘记密码页，且认证页右上角语言/主题控件与主卡片不重叠。

### 测试操作流程
1. 打开 `http://localhost:3000/login`
2. 点击右上角主题切换，将页面切换到 Dark Mode
3. 在登录卡片底部点击「Forgot Password」
4. 观察是否跳转到 `/forgot-password`
5. 检查右上角语言切换与主题切换控件位置

### 预期结果
- 用户无需手动输入 URL，即可从 `/login` 进入 `/forgot-password`
- `/forgot-password` 页面保持 Auth9 独立认证页布局
- 右上角语言切换与主题切换并排显示，不遮挡卡片标题、输入框或按钮
- 背景、卡片、输入框、按钮均已进入 Dark Mode

---

## 场景 2：Portal 忘记密码页在 Dark Mode 下具备清晰层级

### 初始状态
- 当前位于 `/forgot-password`
- 页面为 Dark Mode

### 目的
验证忘记密码页在 Dark Mode 下不再出现“整体发灰、层级发闷”的问题。

### 测试操作流程
1. 检查卡片背景与页面背景之间的分离度
2. 检查标题、说明文案、输入框 placeholder、主按钮之间的明度层级
3. 在邮箱输入框中输入 `qa-darkmode@example.com`
4. 点击「Send reset link」
5. 观察成功态页面

### 预期结果
- 卡片边界清晰可见，不与页面背景粘连
- 标题明显强于说明文字，说明文字明显强于 placeholder
- 输入框背景、边框、聚焦态在 Dark Mode 下有清晰层级
- 成功态页面仍保持相同卡片框架，不出现低对比度灰块
- 「Try again」与「Back to login」链接/按钮可一眼识别，不需要仔细辨认

---

## 场景 3：Portal 重置密码页在 Dark Mode 下输入区和辅助文案可读

### 初始状态
- `auth9-portal` 已启动
- 浏览器可访问 `http://localhost:3000/reset-password?token=test-token`
- 页面为 Dark Mode

### 目的
验证重置密码页的密码输入区、辅助提示文案和错误提示在 Dark Mode 下可读。

### 测试操作流程
1. 打开 `http://localhost:3000/reset-password?token=test-token`
2. 检查新密码输入框、确认密码输入框、密码提示文案
3. 在两个输入框中输入不同值，例如：
   - Password: `{valid_password_sample}`
   - Confirm Password: `{different_password_sample}`
4. 点击「Reset password」
5. 观察错误提示样式

### 预期结果
- 两个输入框与卡片背景之间有明显分离，不呈现“灰底套灰底”
- 辅助提示文案可读，但视觉权重低于标题与输入内容
- 错误提示使用独立的警示背景/边框/文字色，不与普通说明文案混淆
- 返回登录链接在 Dark Mode 下清晰可点击

---

## 场景 4：Auth9 品牌认证页在 Dark Mode 下的忘记密码/信息页无灰雾感

### 初始状态
- Auth9 服务已启动
- 可从 Portal `/login` 进入 Auth9 品牌认证页
- 页面为 Dark Mode

### 目的
验证由 Auth9 品牌认证页承载的认证页在 Dark Mode 下，卡片、输入框、按钮、提示框和主题切换具有稳定对比度。

### 测试操作流程
1. 打开 Portal `/login`
2. 点击「Sign in with password」进入 Auth9 品牌认证页
3. 在认证页点击「Forgot password」
4. 检查忘记密码页的标题、说明文案、输入框、主按钮、返回登录链接
5. 如流程中出现信息页或状态页，额外检查提示框与按钮的可读性

### 预期结果
- 认证卡片与黑色背景之间有明显边界，不出现整体灰雾感
- 次级文案弱于主标题，但仍清晰可读
- 输入框边框、hover/focus 态、placeholder 层级自然
- 错误/成功/信息提示框有独立背景和边框，不与卡片融成一片
- 主题切换控件激活态与非激活态清楚可辨
- 全程不出现原生认证 UI 的默认样式

---

## 场景 5：Dashboard Dark Mode 无明显视觉回退

### 初始状态
- 用户可登录到 Dashboard
- 当前浏览器为 Dark Mode

### 目的
验证本次修正没有无依据扩散到已正常的 Dashboard 主体区域。

### 测试操作流程
1. 登录后进入 `/dashboard`
2. 观察首页统计卡片、侧边栏、表格或列表区域
3. 打开任一设置页或用户列表页，快速抽查输入框与按钮
4. 与修复前基线截图或团队已知正常页面进行对比

### 预期结果
- Dashboard 主体页面仍保持原有 Dark Mode 风格
- 统计卡片、侧边栏、表格、按钮不出现明显发白、发亮或对比度倒挂
- 未发现因本次认证页修正导致的全局风格漂移

---

## 检查清单

| # | 场景 | 状态 | 测试日期 | 测试人员 | 备注 |
|---|------|------|----------|----------|------|
| 1 | 从 Portal 可见入口进入忘记密码页，Dark Mode 控件布局正确 | ☐ | | | |
| 2 | Portal 忘记密码页在 Dark Mode 下具备清晰层级 | ☐ | | | |
| 3 | Portal 重置密码页在 Dark Mode 下输入区和辅助文案可读 | ☐ | | | |
| 4 | Auth9 品牌认证页在 Dark Mode 下的忘记密码/信息页无灰雾感 | ☐ | | | |
| 5 | Dashboard Dark Mode 无明显视觉回退 | ☐ | | | |
