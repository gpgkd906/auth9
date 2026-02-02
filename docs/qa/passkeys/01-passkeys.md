# Passkeys (WebAuthn) 管理测试

**模块**: 安全设置
**测试范围**: Passkey 注册、列表、删除
**场景数**: 5

---

## 功能概述

Passkeys（基于 WebAuthn 标准）提供无密码、抗钓鱼的身份验证方式。用户可以使用设备的生物识别（指纹、面部识别）或屏幕锁定来登录。

---

## 场景 1：查看 Passkeys 列表（无 Passkey）

### 初始状态
- 用户已登录
- 用户没有注册任何 Passkey

### 目的
验证空状态页面正确显示

### 测试操作流程
1. 进入「设置」→「Passkeys」

### 预期结果
- 显示空状态提示「No passkeys yet」
- 显示说明文字
- 显示「Add your first passkey」按钮
- 显示 Passkeys 优势说明卡片

### 预期数据状态
```sql
-- Keycloak Admin API 验证
-- GET /admin/realms/auth9/users/{userId}/credentials
-- 预期: 无 webauthn 类型的凭据
```

---

## 场景 2：注册新 Passkey

### 初始状态
- 用户已登录
- 用户的设备支持 WebAuthn（如 Touch ID、Windows Hello）

### 目的
验证 Passkey 注册流程

### 测试操作流程
1. 进入「设置」→「Passkeys」
2. 点击「Add passkey」按钮
3. 系统跳转到 Keycloak WebAuthn 注册页面
4. 在设备上完成生物识别或 PIN 验证
5. 可选：输入 Passkey 名称
6. 完成注册

### 预期结果
- 成功跳转到 Keycloak 注册流程
- 设备弹出生物识别/PIN 验证请求
- 注册成功后跳回 Passkeys 页面
- 列表中显示新注册的 Passkey
- 显示 Passkey 类型（Passwordless 或 Two-Factor）和创建日期

### 预期数据状态
```sql
-- Keycloak Admin API 验证
-- GET /admin/realms/auth9/users/{userId}/credentials
-- 预期: 存在 type=webauthn 或 type=webauthn-passwordless 的凭据
```

---

## 场景 3：查看已注册的 Passkeys

### 初始状态
- 用户已登录
- 用户已注册 1 个或多个 Passkeys

### 目的
验证 Passkey 列表正确显示

### 测试操作流程
1. 进入「设置」→「Passkeys」

### 预期结果
- 显示所有已注册的 Passkeys
- 每个 Passkey 显示：
  - 名称（用户自定义或默认名称）
  - 类型标签（Passwordless / Two-Factor）
  - 添加日期
  - 「Remove」按钮
- 顶部显示「Add passkey」按钮

### 预期数据状态
```sql
-- Keycloak Admin API 验证
-- GET /admin/realms/auth9/users/{userId}/credentials
-- 预期: 返回凭据列表，包含 userLabel, type, createdDate
```

---

## 场景 4：删除 Passkey

### 初始状态
- 用户已登录
- 用户已注册至少 1 个 Passkey

### 目的
验证 Passkey 删除功能

### 测试操作流程
1. 进入「设置」→「Passkeys」
2. 找到要删除的 Passkey
3. 点击「Remove」按钮
4. 确认删除（如有确认弹窗）

### 预期结果
- 显示「Passkey deleted」提示
- Passkey 从列表中消失
- 该 Passkey 不能再用于登录

### 预期数据状态
```sql
-- Keycloak Admin API 验证
-- GET /admin/realms/auth9/users/{userId}/credentials
-- 预期: 已删除的凭据不再出现
```

---

## 场景 5：使用 Passkey 登录

### 初始状态
- 用户已注册 Passkey
- 用户已登出

### 目的
验证使用 Passkey 进行无密码登录

### 测试操作流程
1. 访问登录页面
2. 输入用户名/邮箱
3. 选择使用 Passkey 登录（如果有选项）
4. 或：点击登录按钮后，系统自动检测到 Passkey
5. 在设备上完成生物识别验证

### 预期结果
- 设备弹出 Passkey 验证请求
- 验证成功后直接登录，无需输入密码
- 跳转到应用主页

### 预期数据状态
```sql
SELECT event_type, auth_method, created_at
FROM login_events
WHERE user_id = '{user_id}'
ORDER BY created_at DESC LIMIT 1;
-- 预期: event_type = 'success', auth_method 包含 webauthn
```

---

## 兼容性说明

### 支持的浏览器/设备
| 平台 | 浏览器 | 支持状态 |
|------|--------|----------|
| macOS | Safari, Chrome | Touch ID |
| Windows | Edge, Chrome | Windows Hello |
| iOS | Safari | Face ID / Touch ID |
| Android | Chrome | 指纹/面部识别 |

### 注意事项
- Passkey 注册需要 HTTPS（localhost 除外）
- 某些浏览器可能需要启用 WebAuthn 功能
- 跨设备同步依赖于 iCloud Keychain / Google Password Manager

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
| 1 | 查看 Passkeys 列表（无 Passkey） | ☐ | | | |
| 2 | 注册新 Passkey | ☐ | | | |
| 3 | 查看已注册的 Passkeys | ☐ | | | |
| 4 | 删除 Passkey | ☐ | | | |
| 5 | 使用 Passkey 登录 | ☐ | | | |
| 6 | 认证状态检查 | ☐ | | | |
