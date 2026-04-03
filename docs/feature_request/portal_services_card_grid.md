# Portal Services 卡片网格布局

**类型**: UI 增强
**严重程度**: Low
**影响范围**: auth9-portal (services list component)
**前置依赖**: 无

---

## 背景

Services 页面当前使用垂直列表布局展示服务。QA 设计规范期望使用响应式卡片网格布局，根据屏幕宽度自适应列数（移动端 1 列、平板 2 列、桌面端 3 列）。

---

## 期望行为

### R1: Services 列表使用 CSS Grid 响应式列布局

将 Services 列表从垂直列表改为 CSS Grid 布局，支持响应式列数调整。

**涉及文件**:
- `auth9-portal/app/routes/` — Services 列表页面组件及样式

### R2: 每个 Service 卡片展示关键信息

每个卡片包含：服务名称、状态标识、Client ID、最后更新时间。卡片样式遵循 Liquid Glass 设计系统。

**涉及文件**:
- `auth9-portal/app/routes/` — Service 卡片组件
- `auth9-portal/app/components/` — 可复用卡片组件（如已有）

### R3: 响应式断点规则

| 屏幕宽度 | 列数 |
|----------|------|
| < 768px | 1 列 |
| 768px - 1023px | 2 列 |
| >= 1024px | 3 列 |

**涉及文件**:
- `auth9-portal/app/routes/` — CSS Grid media query 或 Tailwind 响应式类

---

## 验证方法

### 手动验证

1. 创建 3 个以上 Service
2. 访问 Services 列表页
3. 桌面端（>= 1024px）确认 3 列网格
4. 缩小到平板宽度（768-1023px）确认 2 列
5. 缩小到手机宽度（< 768px）确认 1 列
6. 确认卡片显示名称、状态、Client ID、更新时间

### 代码验证

```bash
grep -r "grid\|grid-cols\|service-card" auth9-portal/app/
cd auth9-portal && npm run test
```
