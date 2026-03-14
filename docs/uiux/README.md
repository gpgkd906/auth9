# Auth9 UI/UX 测试用例文档

本目录包含 Auth9 Portal 的 UI/UX 专项测试用例，基于 **Liquid Glass Design System** 确保界面美观、一致性和可用性。

## 设计系统参考

- **设计规范**: [design-system.md](../design-system.md) - Liquid Glass 设计语言
- **QA 测试**: [qa/README.md](../qa/README.md) - 功能测试用例

---

## 测试用例索引

### 视觉设计 (3 个文档, 13 个场景)
| 文档 | 描述 | 场景数 |
|------|------|--------|
| [01-design-consistency.md](./01-design-consistency.md) | 设计一致性、玻璃质感、颜色系统 | 5 |
| [02-theme-switching.md](./02-theme-switching.md) | 主题切换、明暗模式 | 4 |
| [03-visual-hierarchy.md](./03-visual-hierarchy.md) | 视觉层级、间距、布局 | 4 |

### 交互体验 (3 个文档, 14 个场景)
| 文档 | 描述 | 场景数 |
|------|------|--------|
| [04-animations.md](./04-animations.md) | 动画流畅度、过渡效果 | 5 |
| [05-responsive-layout.md](./05-responsive-layout.md) | 响应式布局、移动端适配 | 4 |
| [22-dialog-empty-state-patterns.md](./22-dialog-empty-state-patterns.md) | Dialog 玻璃效果一致性、焦点陷阱、Empty State 统一、表单提交状态 | 5 |

### 可访问性 (1 个文档, 5 个场景)
| 文档 | 描述 | 场景数 |
|------|------|--------|
| [06-accessibility.md](./06-accessibility.md) | WCAG 合规性、键盘导航、屏幕阅读器 | 5 |

### 页面专项 (16 个文档, 85 个场景)
| 文档 | 描述 | 场景数 |
|------|------|--------|
| [07-dashboard-page.md](./07-dashboard-page.md) | 概览页统计、动态列表 | 5 |
| [08-tenants-page.md](./08-tenants-page.md) | 租户列表、搜索栏适配 | 5 |
| [09-users-page.md](./09-users-page.md) | 用户目录、角色分配弹窗 | 6 |
| [10-services-page.md](./10-services-page.md) | 服务管理、密钥显示适配、Actions/Branding Tab 布局 | 6 |
| [11-settings-page.md](./11-settings-page.md) | 系统设置、子导航体验、品牌页底部操作栏、密码策略选择器/开关布局 | 5 |
| [12-i18n-localization.md](./12-i18n-localization.md) | 国际化切换、首屏协商、格式化一致性 | 5 |
| [13-landing-page-interactions.md](./13-landing-page-interactions.md) | Landing 页卡片悬停效果、Navbar 控件可点击性、入场动画 | 4 |
| [14-global-controls-placement.md](./14-global-controls-placement.md) | 主题/语言切换控件在 Landing 和 Dashboard 中的无重叠布局 | 5 |
| [15-error-message-ux.md](./15-error-message-ux.md) | 错误消息人类可读性、mapApiError 两层映射、内联错误本地化 | 5 |
| [16-keycloak-theme-i18n.md](./16-keycloak-theme-i18n.md) | Keycloak 认证页 i18n 文案覆盖、语言参数透传 | 4 |
| [17-account-pages.md](./17-account-pages.md) | 账户管理（Profile、Security、Passkeys、Sessions）布局与深色模式 | 6 |
| [18-roles-abac-pages.md](./18-roles-abac-pages.md) | 角色 Tab/层级树/权限复选框、ABAC 策略编辑器与模拟引擎 | 6 |
| [19-analytics-audit-pages.md](./19-analytics-audit-pages.md) | Analytics 趋势图表、Audit Logs 表格、Security Alerts 严重度过滤 | 6 |
| [20-onboarding-auth-flow.md](./20-onboarding-auth-flow.md) | Onboarding 引导、Login 多方式切换、Register、Invite Accept、Tenant Select | 6 |
| [21-tenant-detail-pages.md](./21-tenant-detail-pages.md) | 租户详情概览、Webhooks 配置、SSO 连接器、Invitations 邀请管理 | 5 |
| [23-public-pages-layout.md](./23-public-pages-layout.md) | 公共页面布局（Privacy/Terms/Docs）、prose-glass 排版、卡片网格 | 5 |

---

## 统计概览

| 类别 | 文档数 | 场景数 |
|------|--------|--------|
| 视觉设计 | 3 | 13 |
| 交互体验 | 3 | 14 |
| 可访问性 | 1 | 5 |
| 页面专项 | 16 | 85 |
| **总计** | **23** | **117** |

---

## 测试目标

### 1. 设计美观性
确保 Liquid Glass 设计系统在所有页面正确应用：
- ✨ 玻璃质感效果（半透明、模糊）
- 🎨 颜色系统一致性
- 🔲 圆角和阴影规范
- 💡 光线折射和高光效果

### 2. 交互合理性
验证用户交互流畅、符合直觉：
- 🎬 动画流畅度和时机
- 📱 响应式布局适配
- ⌨️ 键盘导航支持
- 👆 触摸交互优化

### 3. 可访问性
确保符合 WCAG AA 标准：
- 👁️ 颜色对比度
- 🔊 屏幕阅读器兼容
- ⌨️ 完整键盘操作
- 🎯 焦点状态清晰

### 4. 入口发现性（与 QA 联动）
确保关键功能可被用户发现并可达：
- 🧭 页面入口在导航中可见（侧边栏/Tab/按钮）
- 🔗 从入口到目标页面的跳转路径稳定
- 🧪 与 `docs/qa` 的“入口可见性”场景保持一致

---

## 测试环境准备

### 浏览器兼容性测试
| 浏览器 | 最低版本 | 测试重点 |
|--------|---------|---------|
| Chrome | 90+ | backdrop-filter 支持 |
| Safari | 14+ | -webkit-backdrop-filter |
| Firefox | 103+ | backdrop-filter 支持 |
| Edge | 90+ | Chromium 内核 |

### 设备测试
| 设备类型 | 分辨率 | 测试场景 |
|---------|--------|---------|
| 桌面 | 1920×1080 | 完整功能、多列布局 |
| 笔记本 | 1366×768 | 中等屏幕适配 |
| 平板 | 768×1024 | 侧边栏折叠、触摸操作 |
| 手机 | 375×812 | 移动端导航、单列布局 |

### 启动测试环境

```bash
# 启动服务
docker-compose -f docker-compose.yml -f docker-compose.dev.yml up -d
cd auth9-core && cargo run &
cd auth9-portal && npm run dev
```

访问：http://localhost:5173

---

## 测试用例结构

每个测试场景包含：

1. **初始状态** - 测试前置条件
2. **目的** - 验证的 UI/UX 目标
3. **测试操作流程** - 详细步骤（含截图说明）
4. **预期视觉效果** - 界面呈现要求
5. **验证工具** - 浏览器 DevTools、辅助工具

---

## 常用验证工具

### Chrome DevTools
```
1. 检查元素 (F12)
2. 计算样式 (Computed tab)
3. 性能分析 (Performance tab)
4. 响应式模式 (Ctrl+Shift+M)
```

### CSS 属性验证
```javascript
// 检查 backdrop-filter 支持
getComputedStyle(document.querySelector('.liquid-glass'))['backdrop-filter']
// 预期: blur(24px) saturate(180%)

// 检查 CSS 变量
getComputedStyle(document.documentElement).getPropertyValue('--glass-bg')
// Light: rgba(255, 255, 255, 0.72)
// Dark: rgba(44, 44, 46, 0.65)
```

### 辅助功能工具
- **Lighthouse**: 可访问性评分
- **WAVE**: Web Accessibility Evaluation Tool
- **axe DevTools**: 自动化 a11y 检测
- **Color Contrast Checker**: 对比度验证

---

## 问题报告格式

```markdown
## UI/UX Issue: [简短描述]

**测试文档**: [文档路径]
**场景**: #X
**浏览器**: Chrome 120 / Safari 17 / Firefox 120
**设备**: Desktop 1920×1080 / iPhone 14 Pro

**复现步骤**:
1. ...
2. ...

**预期效果**: ...
**实际效果**: ...
**截图**: [附上截图]
**CSS 检查**: [相关样式值]
```

---

## 设计系统速查表

### 玻璃效果参数
```css
background: var(--glass-bg);
backdrop-filter: blur(24px) saturate(180%);
border: 1px solid var(--glass-border);
border-radius: 20px;
box-shadow: 0 8px 32px var(--glass-shadow),
            inset 0 1px 0 var(--glass-highlight);
```

### 颜色 Token
| Token | Light | Dark |
|-------|-------|------|
| `--bg-primary` | `#F2F2F7` | `#000000` |
| `--bg-secondary` | `#FFFFFF` | `#1C1C1E` |
| `--glass-bg` | `rgba(255,255,255,0.72)` | `rgba(44,44,46,0.65)` |
| `--accent-blue` | `#007AFF` | `#007AFF` |
| `--text-primary` | `#1D1D1F` | `#FFFFFF` |
| `--text-secondary` | `#6E6E73` | `#98989D` |
| `--text-tertiary` | `#AEAEB2` | `#636366` |

### 圆角规范
- Cards: `20px`（`liquid-glass`）
- Sidebar: `24px`
- Buttons: `12px`
- Inputs: `12px`
- **Selects / Textareas**: `10px`（与 Input 不同，注意区分）
- Menus / Dropdowns: `14px`
- Tab triggers / Menu items: `8px`
- Badges: `100px` (pill)

### 组件尺寸速查
| 组件 | 高度 | 圆角 | 备注 |
|------|------|------|------|
| Button (default) | 40px (`h-10`) | 12px | `px-5 py-2.5` |
| Button (sm) | 44px (`min-h-[44px]`) | 12px | `px-3` |
| Button (icon) | 44px (`h-11`) | 12px | `min-w-[44px]` |
| Input | 40px (`h-10`) | 12px | `px-3 py-2.5` |
| Select trigger | 40px (`h-10`) | **10px** | `px-3 py-2` |
| Textarea | ≥80px (`min-h-[80px]`) | **10px** | `px-4 py-3` |
| Checkbox | 16px (`h-4 w-4`) | 4px | — |
| Switch | 20×36px (`h-5 w-9`) | full | — |
| Avatar | 40px (`h-10 w-10`) | full | — |
| Badge | auto | full (pill) | `px-2.5 py-0.5`, 11px |
| Tab trigger | ≥44px (`min-h-[44px]`) | 8px | `px-3 py-2.5` |

### 动画时长
- 悬停效果: `0.3s`
- 主题切换: `0.4s`
- 页面进入: `0.5s`
- 背景动画: `20s`

---

## 更新日志

| 日期 | 版本 | 更新内容 |
|------|------|----------|
| 2026-03-14 | 2.3.0 | Action 详情页脚本代码块渲染优化：更新 `10`（Services 页面）场景 5 追加 Action 详情页 CodeBlock 深色代码块验证项（背景色、等宽字体、复制按钮、溢出处理、深色模式兼容）|
| 2026-03-13 | 2.2.0 | Dark Mode 认证页对比度修正同步：更新 `02`（主题切换）中的 Input token 与独立认证页对比度说明，更新 `20`（Onboarding/Auth Flow）中忘记密码页成功态与 Dark Mode 层级预期；与 `docs/qa/auth/15-dark-mode-auth-contrast.md` 对齐 |
| 2026-03-11 | 2.1.0 | 设计系统对齐审计：交叉比对全部组件实现与文档约束值，修正 10 处漂移。`--text-secondary` Light 色值 `#86868B`→`#6E6E73`（`01`/`02`/`06`/`design-system`）；Button padding `px-4 py-2`→`px-5 py-2.5` + 尺寸变体表（`01`）；Select/Textarea 圆角 12px→10px（`03`/`design-system`）；Label 颜色 `--text-primary`→`--text-secondary`（`03`）；表头颜色 `--text-secondary`→`--text-tertiary`（`03`）；Input 背景从硬编码改为 `var(--sidebar-item-hover)`（`02`）；触摸目标尺寸对齐实际值（`05`）；Dialog vs AlertDialog max-width 区分（`03`）；Outline Button 文字/边框从蓝色修正为 `--text-primary`/`--glass-border-subtle`（`01`）；README 新增组件尺寸速查表和圆角细分 |
| 2026-03-11 | 2.0.0 | 覆盖缺口补全：新增 6 个文档 35 个场景。页面专项新增 Account 账户管理（`17`）、Roles/ABAC 角色与策略（`18`）、Analytics/Audit/Alerts 数据页面（`19`）、Onboarding/Auth 引导与认证流程（`20`）、Tenant Detail 租户详情子页面（`21`）；交互体验新增 Dialog/Empty State 跨页面一致性（`22`）。总计 22 个文档 112 个场景 |
| 2026-03-08 | 1.5.0 | 错误消息映射重构：重写 `15`（错误消息 UX）反映 `mapApiError` 两层映射架构、16 种 error code 三语覆盖表、内联错误展示（非 Toast）；Cross-doc 更新 `12` 场景 5 引用新映射架构 |
| 2026-03-07 | 1.4.0 | i18n 三语扩展：更新 `12`（Portal 国际化）和 `16`（Keycloak 主题 i18n）追加日语场景；Cross-doc 同步更新 `14`（全局控件布局）和 `15`（错误消息 UX）的语言描述为三语 |
| 2026-03-07 | 1.3.0 | 新增 4 个缺陷回归专项 UI/UX 测试文档：Landing 页卡片交互（`13`）、全局控件布局无重叠（`14`）、错误消息用户体验（`15`）、Keycloak 主题 i18n（`16`）；共 16 个文档 77 个场景 |
| 2026-03-07 | 1.2.0 | 新增 Portal 国际化与本地化 UI/UX 测试文档（`12-i18n-localization.md`），覆盖语言切换入口、SSR 首屏协商、格式化一致性、表单本地化；并同步修正文档中的语言相关说明；共 12 个文档 59 个场景 |
| 2026-02-21 | 1.1.0 | Services 页面新增 Actions/Branding Tab 布局场景（`10-services-page.md`）；共 11 个文档 54 个场景 |
| 2026-02-06 | 1.0.0 | 初始版本：6 个文档，27 个 UI/UX 测试场景 |
