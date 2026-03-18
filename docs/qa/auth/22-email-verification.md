# Auth - 邮箱验证流程

**模块**: Auth
**测试范围**: Email Verification API 端点（发送验证邮件、验证 token、防枚举）
**场景数**: 5
**优先级**: 高

---

## 背景说明

Auth9 自行签发和消费 email verification token，不依赖 Keycloak required actions。验证流程：

1. 调用 `POST /api/v1/hosted-login/send-verification` 发送验证邮件
2. 用户点击邮件中的链接，Portal 加载 `/verify-email?token=<raw_token>`
3. Portal loader 调用 `POST /api/v1/hosted-login/verify-email` 消费 token
4. 验证成功后 `user_verification_status.email_verified` 标记为 `true`

端点：
- `POST /api/v1/hosted-login/send-verification` — 发送验证邮件（公开，无需认证）
- `POST /api/v1/hosted-login/verify-email` — 消费 token 并标记已验证（公开，无需认证）

Token 特性：
- 32 字节随机值，URL-safe base64 编码
- 存储时使用 SHA-256 hash
- 24 小时有效期
- `used_at` 字段提供 replay 防护

---

## 场景 1：发送验证邮件成功

### 步骤 0（Gate Check）
- Auth9 Core 服务运行中：`curl -sf http://localhost:8080/health`
- **`IDENTITY_BACKEND=auth9_oidc`**：邮箱验证功能仅在 auth9-oidc 后端下可用。在 docker-compose 中设置 `IDENTITY_BACKEND=auth9_oidc` 并重启 auth9-core
- 系统中存在已注册用户

### 初始状态
- 系统中存在用户 `qa-user@example.com`

### 目的
验证发送验证邮件端点返回统一成功消息（防止邮箱枚举）

### 测试操作流程
1. 发送验证邮件请求：
```bash
curl -s -X POST http://localhost:8080/api/v1/hosted-login/send-verification \
  -H "Content-Type: application/json" \
  -d '{"email": "qa-user@example.com"}' | jq .
```

### 预期结果
- HTTP 状态码：200
- 响应体：
```json
{
  "message": "If an account exists with this email, a verification link has been sent."
}
```

### 预期数据状态
```sql
-- 验证 token 已创建
SELECT id, user_id, LEFT(token_hash, 16) AS hash_prefix,
       expires_at, used_at, created_at
FROM auth9_oidc.email_verification_tokens
WHERE user_id = (SELECT identity_subject FROM users WHERE email = 'qa-user@example.com')
ORDER BY created_at DESC LIMIT 1;
-- 预期: 1 行, used_at IS NULL, expires_at 约为 24 小时后

-- 验证审计日志
SELECT action, resource_type, created_at
FROM audit_logs
WHERE action = 'email_verification.sent'
ORDER BY created_at DESC LIMIT 1;
-- 预期: 1 行, resource_type = 'user'
```

---

## 场景 2：防邮箱枚举 — 不存在的邮箱

### 初始状态
- 系统中不存在邮箱 `nonexistent@example.com`

### 目的
验证对不存在的邮箱发送验证邮件时，返回与存在邮箱相同的成功消息（防枚举）

### 测试操作流程
1. 发送验证邮件请求（不存在的邮箱）：
```bash
curl -s -X POST http://localhost:8080/api/v1/hosted-login/send-verification \
  -H "Content-Type: application/json" \
  -d '{"email": "nonexistent@example.com"}' | jq .
```

### 预期结果
- HTTP 状态码：200
- 响应体与场景 1 **完全相同**：
```json
{
  "message": "If an account exists with this email, a verification link has been sent."
}
```

### 预期数据状态
```sql
-- 确认没有生成 token（无对应用户）
SELECT COUNT(*) FROM auth9_oidc.email_verification_tokens
WHERE user_id = 'nonexistent';
-- 预期: 0
```

---

## 场景 3：验证邮件 Token 消费成功

### 步骤 0（Gate Check）
- 场景 1 已执行，`email_verification_tokens` 中存在有效 token
- 获取 raw token（需从邮件内容或数据库反查；测试环境可直接从发送时记录获取）

### 初始状态
- 存在一条 `used_at IS NULL` 且 `expires_at > NOW()` 的 token 记录

### 目的
验证使用有效 token 可成功标记邮箱为已验证

### 测试操作流程
1. 使用有效 token 验证邮箱：
```bash
# 替换 <RAW_TOKEN> 为实际 token 值
curl -s -X POST http://localhost:8080/api/v1/hosted-login/verify-email \
  -H "Content-Type: application/json" \
  -d '{"token": "<RAW_TOKEN>"}' | jq .
```

### 预期结果
- HTTP 状态码：200
- 响应体：
```json
{
  "message": "Email verified successfully."
}
```

### 预期数据状态
```sql
-- 验证状态已更新
SELECT user_id, email_verified, email_verified_at
FROM auth9_oidc.user_verification_status
WHERE user_id = (SELECT identity_subject FROM users WHERE email = 'qa-user@example.com');
-- 预期: email_verified = 1, email_verified_at IS NOT NULL

-- Token 已标记为已使用（replay 防护）
SELECT id, used_at
FROM auth9_oidc.email_verification_tokens
WHERE user_id = (SELECT identity_subject FROM users WHERE email = 'qa-user@example.com')
ORDER BY created_at DESC LIMIT 1;
-- 预期: used_at IS NOT NULL

-- 验证审计日志
SELECT action, resource_type, created_at
FROM audit_logs
WHERE action = 'email_verification.completed'
ORDER BY created_at DESC LIMIT 1;
-- 预期: 1 行
```

---

## 场景 4：Replay 防护 — 已使用的 Token 拒绝

### 步骤 0（Gate Check）
- 场景 3 已执行，token 的 `used_at` 已设置

### 初始状态
- 同一 token 已被消费

### 目的
验证已使用的 token 不能被重复消费

### 测试操作流程
1. 再次使用相同 token：
```bash
curl -s -w "\nHTTP_STATUS: %{http_code}\n" \
  -X POST http://localhost:8080/api/v1/hosted-login/verify-email \
  -H "Content-Type: application/json" \
  -d '{"token": "<SAME_RAW_TOKEN>"}' | jq .
```

### 预期结果
- HTTP 状态码：400
- 响应体包含错误信息：
```json
{
  "error": "bad_request",
  "message": "Invalid or expired verification token."
}
```

---

## 场景 5：无效 / 空 Token 校验

### 初始状态
- 无特殊前置条件

### 目的
验证空 token 和伪造 token 返回明确错误

### 测试操作流程
1. 空 token 请求：
```bash
curl -s -w "\nHTTP_STATUS: %{http_code}\n" \
  -X POST http://localhost:8080/api/v1/hosted-login/verify-email \
  -H "Content-Type: application/json" \
  -d '{"token": ""}' | jq .
```

2. 伪造 token 请求：
```bash
curl -s -w "\nHTTP_STATUS: %{http_code}\n" \
  -X POST http://localhost:8080/api/v1/hosted-login/verify-email \
  -H "Content-Type: application/json" \
  -d '{"token": "fake-token-that-does-not-exist"}' | jq .
```

3. 无效邮箱格式发送验证：
```bash
curl -s -w "\nHTTP_STATUS: %{http_code}\n" \
  -X POST http://localhost:8080/api/v1/hosted-login/send-verification \
  -H "Content-Type: application/json" \
  -d '{"email": "not-an-email"}' | jq .
```

### 预期结果
1. 空 token → HTTP 400，`"Verification token is required."`
2. 伪造 token → HTTP 400，`"Invalid or expired verification token."`
3. 无效邮箱 → HTTP 400，`"Invalid email address."`
