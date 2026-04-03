# Portal 统一自定义 Select 组件

**类型**: UI 增强
**严重程度**: Low
**影响范围**: auth9-portal (settings page, shared Select component)
**前置依赖**: 无

---

## 背景

Settings 页面的 Tenant 选择器使用原生 `<select>` 元素，与 Auth9 Portal 的 Liquid Glass 设计系统风格不一致。需要替换为统一的自定义 Select 组件，以保持 UI 一致性。

---

## 期望行为

### R1: 替换原生 `<select>` 为自定义 Select 组件

创建或使用已有的自定义 Select 组件替换 Settings 页面中的原生 `<select>` 元素，保留完整的选项数据和交互功能。

**涉及文件**:
- `auth9-portal/app/routes/` — Settings 页面中 Tenant 选择器
- `auth9-portal/app/components/` — 自定义 Select 组件

### R2: 遵循 Liquid Glass 设计系统样式

自定义 Select 组件的视觉样式匹配 Liquid Glass 设计系统：玻璃拟态效果、一致的边框圆角、下拉菜单动画、选中态高亮等。

**涉及文件**:
- `auth9-portal/app/components/` — Select 组件样式
- `docs/design-system.md` — 设计系统参考

### R3: 键盘导航与无障碍访问

自定义 Select 支持完整的键盘操作（Enter/Space 展开、Arrow Up/Down 导航、Escape 关闭）和 ARIA 角色属性（`role="listbox"`、`aria-expanded`、`aria-selected` 等）。

**涉及文件**:
- `auth9-portal/app/components/` — Select 组件的键盘事件和 ARIA 属性

---

## 验证方法

### 手动验证

1. 打开 Settings 页面
2. 确认 Tenant 选择器为自定义 Select（非原生 `<select>`）
3. 确认视觉样式与 Liquid Glass 设计系统一致
4. 使用键盘操作验证：Tab 聚焦、Enter 展开、Arrow 导航、Escape 关闭
5. 使用屏幕阅读器验证 ARIA 属性正确

### 代码验证

```bash
grep -r "CustomSelect\|SelectComponent\|role=\"listbox\"" auth9-portal/app/
cd auth9-portal && npm run test
```
