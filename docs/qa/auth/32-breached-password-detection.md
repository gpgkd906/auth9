# 泄露密码检测（HIBP）

**模块**: auth (Password Security)
**测试范围**: 注册、修改密码、重置密码时检测泄露密码（HIBP k-Anonymity API）、Fail-open 降级策略、环境变量开关
**场景数**: 5
**优先级**: 高

---

## 背景说明

Auth9 现在在用户注册、修改密码、重置密码时，通过 Have I Been Pwned (HIBP) 数据库检测密码是否曾出现在已知的数据泄露事件中。

**安全机制**：
- 使用 k-Anonymity API：仅发送密码 SHA-1 哈希的前 5 个字符到 HIBP，不暴露完整密码
- 阻止模式：检测到泄露密码时返回 HTTP 422 Validation Error
- Fail-open 策略：当 HIBP API 不可达时，密码被正常接受（不阻止用户操作）

**环境变量**：
- `HIBP_ENABLED`：启用/禁用泄露密码检测（默认 `true`）

**受影响端点**：
- `POST /api/v1/users` — 注册（含密码）
- `POST /api/v1/users/me/password` — 修改密码（需认证）
- `POST /api/v1/auth/reset-password` — 重置密码（使用 token）

**错误响应格式**（HTTP 422）：
```json
{
  "error": "validation_error",
  "message": "This password has been found in a data breach. Please choose a different password."
}
```

**测试用密码**：
- 已知泄露：`Password123!`（HIBP 中出现频次高，且满足密码策略：12+ 字符、大小写、数字、特殊字符）
- 已知泄露：`Welcome2024!`（HIBP 中出现频次高，满足密码策略）
- 安全密码：`Auth9-TestSafe-Xk9mR2pQ7vL4nW8j!`（极长随机字符串，不太可能出现在 HIBP 中）

> **注意**: 旧版文档使用 `password` 和 `123456` 作为泄露密码测试用例，但这些密码不满足密码策略要求（最少 12 字符、包含大小写字母、数字、特殊字符），会在密码策略校验阶段被拒绝（HTTP 422 validation_error），而非在 HIBP 泄露检测阶段。测试泄露检测功能时必须使用满足密码策略的泄露密码。

---

## 场景 1：注册时使用泄露密码被拒绝

### 初始状态
- Auth9 Core 运行中，`HIBP_ENABLED=true`（默认）
- HIBP API 可达（需要外网访问）

### 目的
验证通过 `POST /api/v1/users` 注册用户时，使用已知泄露密码会被拒绝（HTTP 422）

### 测试操作流程

#### 步骤 0: Gate Check
```bash
# 确认 auth9-core 运行中
curl -sf http://localhost:8080/health | jq .
# 预期: {"status":"ok",...}

# 获取管理员 Token
TOKEN=$(.claude/skills/tools/gen-admin-token.sh)
echo $TOKEN | head -c 20
# 预期: 输出 JWT 前缀
```

#### API 测试
1. 使用已知泄露密码 `Password123!` 创建用户：
```bash
curl -s -w "\n%{http_code}" -X POST http://localhost:8080/api/v1/users \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "email": "breached-test-1@example.com",
    "password": "Password123!"  # pragma: allowlist secret,
    "name": "Breached Test User"
  }'
```

2. 使用另一个已知泄露密码 `Welcome2024!` 创建用户：
```bash
curl -s -w "\n%{http_code}" -X POST http://localhost:8080/api/v1/users \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "email": "breached-test-2@example.com",
    "password": "Welcome2024!"  # pragma: allowlist secret,
    "name": "Breached Test User 2"
  }'
```

### 预期结果
- 两个请求均返回 HTTP **422**
- 响应体包含 `"message": "This password has been found in a data breach. Please choose a different password."`
- 用户**未**被创建

### 预期数据状态
```sql
SELECT COUNT(*) FROM users
WHERE email IN ('breached-test-1@example.com', 'breached-test-2@example.com');
-- 预期: 0（用户未创建）
```

---

## 场景 2：注册时使用安全密码成功

### 初始状态
- Auth9 Core 运行中，`HIBP_ENABLED=true`（默认）
- HIBP API 可达

### 目的
验证使用未泄露的安全密码可以正常注册

### 测试操作流程

#### 步骤 0: Gate Check
```bash
TOKEN=$(.claude/skills/tools/gen-admin-token.sh)
```

#### API 测试
1. 使用安全密码创建用户：
```bash
curl -s -w "\n%{http_code}" -X POST http://localhost:8080/api/v1/users \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "email": "safe-pwd-test@example.com",
    "password": "Auth9-TestSafe-Xk9mR2pQ7vL4nW8j!"  # pragma: allowlist secret,
    "name": "Safe Password User"
  }'
```

### 预期结果
- 返回 HTTP **201**（或 200）
- 响应体包含创建的用户信息（`id`、`email` 等）

### 预期数据状态
```sql
SELECT id, email FROM users
WHERE email = 'safe-pwd-test@example.com';
-- 预期: 存在 1 条记录
```

---

## 场景 3：修改密码时使用泄露密码被拒绝

### 初始状态
- 用户已注册并登录（持有有效 Identity Token）
- `HIBP_ENABLED=true`

### 目的
验证已登录用户通过 `POST /api/v1/users/me/password` 修改密码时，使用泄露密码会被拒绝

### 测试操作流程

#### 步骤 0: Gate Check
```bash
# 获取用户 Identity Token（使用已有用户或场景 2 创建的用户）
TOKEN=$(.claude/skills/tools/gen-admin-token.sh)
echo $TOKEN | head -c 20
# 预期: 输出 JWT 前缀
```

#### API 测试
1. 使用泄露密码修改密码：
```bash
curl -s -w "\n%{http_code}" -X POST http://localhost:8080/api/v1/users/me/password \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "current_password": "Auth9-TestSafe-Xk9mR2pQ7vL4nW8j!",
    "new_password": "password"
  }'
# pragma: allowlist secret
```

2. 使用安全密码修改密码（对照组）：
```bash
curl -s -w "\n%{http_code}" -X POST http://localhost:8080/api/v1/users/me/password \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "current_password": "Auth9-TestSafe-Xk9mR2pQ7vL4nW8j!",
    "new_password": "Auth9-NewSafe-Yp3nT8wK5mQ2xR6j!"
  }'
# pragma: allowlist secret
```

### 预期结果
- 第一个请求（泄露密码）：返回 HTTP **422**，消息为 `"This password has been found in a data breach. Please choose a different password."`
- 第二个请求（安全密码）：返回 HTTP **200**，密码修改成功

---

## 场景 4：重置密码时使用泄露密码被拒绝

### 初始状态
- 用户已通过忘记密码流程获取重置 token
- `HIBP_ENABLED=true`

### 目的
验证通过 `POST /api/v1/auth/reset-password` 重置密码时，使用泄露密码会被拒绝

### 测试操作流程

#### 步骤 0: Gate Check
```bash
# 确认 auth9-core 运行中
curl -sf http://localhost:8080/health | jq .

# 先触发密码重置邮件获取 token（或从数据库获取）
# 从数据库获取最新的 reset token:
```
```sql
SELECT token, user_id, expires_at FROM password_reset_tokens
ORDER BY created_at DESC LIMIT 1;
-- 记录 {reset_token} 值
```

#### API 测试
1. 使用泄露密码重置：
```bash
curl -s -w "\n%{http_code}" -X POST http://localhost:8080/api/v1/auth/reset-password \
  -H "Content-Type: application/json" \
  -d '{
    "token": "{reset_token}",
    "new_password": "password"
  }'
```

2. 使用安全密码重置（需要新的 reset token）：
```bash
curl -s -w "\n%{http_code}" -X POST http://localhost:8080/api/v1/auth/reset-password \
  -H "Content-Type: application/json" \
  -d '{
    "token": "{new_reset_token}",
    "new_password": "Auth9-ResetSafe-Zm7bN4cF9qH1wP5k!"
  }'
# pragma: allowlist secret
```

### 预期结果
- 第一个请求（泄露密码）：返回 HTTP **422**，消息为 `"This password has been found in a data breach. Please choose a different password."`
- 第二个请求（安全密码）：返回 HTTP **200**，密码重置成功

---

## 场景 5：HIBP_ENABLED=false 时跳过检测

### 初始状态
- Auth9 Core 以 `HIBP_ENABLED=false` 启动
- HIBP API 是否可达不影响此场景

### 目的
验证当 `HIBP_ENABLED=false` 时，泄露密码不会被拒绝（功能完全关闭）

### 测试操作流程

#### 步骤 0: Gate Check
```bash
# 以 HIBP_ENABLED=false 重启 auth9-core
# （在 .env 或启动命令中设置 HIBP_ENABLED=false）

curl -sf http://localhost:8080/health | jq .
# 预期: {"status":"ok",...}

TOKEN=$(.claude/skills/tools/gen-admin-token.sh)
```

#### API 测试
1. 使用已知泄露密码创建用户：
```bash
curl -s -w "\n%{http_code}" -X POST http://localhost:8080/api/v1/users \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "email": "hibp-disabled-test@example.com",
    "password": "password"  # pragma: allowlist secret,
    "name": "HIBP Disabled Test"
  }'
```

### 预期结果
- 返回 HTTP **201**（或 200）
- 用户创建成功，泄露密码**不**被拦截

### 预期数据状态
```sql
SELECT id, email FROM users
WHERE email = 'hibp-disabled-test@example.com';
-- 预期: 存在 1 条记录

-- 测试完成后清理
DELETE FROM users WHERE email = 'hibp-disabled-test@example.com';
```

---

## 通用场景：Fail-open 降级策略

### 测试操作流程
1. 模拟 HIBP API 不可达（如通过防火墙规则阻断对 `api.pwnedpasswords.com` 的出站连接，或断开外网）
2. 使用已知泄露密码 `password` 创建用户：
```bash
TOKEN=$(.claude/skills/tools/gen-admin-token.sh)
curl -s -w "\n%{http_code}" -X POST http://localhost:8080/api/v1/users \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "email": "failopen-test@example.com",
    "password": "password"  # pragma: allowlist secret,
    "name": "Fail Open Test"
  }'
```
3. 恢复网络连接

### 预期结果
- 即使密码 `password` 是已知泄露密码，因 HIBP API 不可达，请求返回 HTTP **201**（或 200）
- 用户创建成功（fail-open 策略：可用性优先于安全检测）
- 建议检查日志中是否有 HIBP 连接超时/失败的 warning 级别日志

---

## 检查清单

| # | 场景 | 状态 | 测试日期 | 测试人员 | 备注 |
|---|------|------|----------|----------|------|
| 1 | 注册时使用泄露密码被拒绝 | ☐ | | | |
| 2 | 注册时使用安全密码成功 | ☐ | | | |
| 3 | 修改密码时使用泄露密码被拒绝 | ☐ | | | |
| 4 | 重置密码时使用泄露密码被拒绝 | ☐ | | | |
| 5 | HIBP_ENABLED=false 时跳过检测 | ☐ | | | |
| - | 通用：Fail-open 降级策略 | ☐ | | | 需模拟网络不可达 |
