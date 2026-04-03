# Portal 新用户 Onboarding 流程

**类型**: 新功能
**严重程度**: Medium
**影响范围**: auth9-portal (onboarding route, route guards, organization creation)
**前置依赖**: 无

---

## 背景

没有组织的新用户应在 `/onboard` 页面看到引导卡片和组织创建表单。当前 `/onboard` 路由直接重定向到 `/login`，未提供新用户引导流程，导致新用户无法完成初始设置。

---

## 期望行为

### R1: 路由守卫检测无组织用户

登录成功后，路由守卫检查用户是否关联了组织。若无组织，重定向到 `/onboard` 而非 `/dashboard`。

**涉及文件**:
- `auth9-portal/app/routes/` — 路由守卫 / loader 中的组织检查逻辑

### R2: Onboarding 页面展示组织创建表单

`/onboard` 页面展示欢迎引导卡片和组织创建表单，包含组织名称、slug 等必填字段，遵循 Liquid Glass 设计风格。

**涉及文件**:
- `auth9-portal/app/routes/` — Onboarding 页面组件
- `auth9-portal/app/components/` — 组织创建表单

### R3: 创建组织后跳转到 Dashboard

用户提交组织创建表单后，后端创建组织并关联用户，前端自动跳转到 `/dashboard`，完成 onboarding 流程。

**涉及文件**:
- `auth9-portal/app/routes/` — Onboarding action 处理和重定向

---

## 验证方法

### 手动验证

1. 创建一个不关联任何组织的测试用户
2. 使用该用户登录
3. 确认被重定向到 `/onboard` 页面（而非 `/dashboard`）
4. 确认页面显示组织创建表单
5. 填写并提交表单，确认组织创建成功
6. 确认自动跳转到 `/dashboard`

### 代码验证

```bash
grep -r "onboard\|Onboarding\|organization.*create" auth9-portal/app/
cd auth9-portal && npm run test
```
