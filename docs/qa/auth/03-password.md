# 认证流程 - 密码管理测试

**模块**: 认证流程
**测试范围**: 密码重置、修改、强度验证
**场景数**: 5

---

## 架构说明

Auth9 密码管理完全由 Auth9 自身控制，不依赖 Keycloak credentials API：

1. **忘记密码页面** → Auth9 Portal `/forgot-password`
2. **重置密码页面** → Auth9 Portal `/reset-password?token=...`
3. **密码强度验证** → 由 Auth9 后端本地执行（tenant 级别 PasswordPolicy）
4. **修改密码（已登录用户）** → Auth9 Portal `Account -> Security` 页面
5. **密码哈希与验证** → Auth9 使用 argon2id 本地哈希，存储在 `credentials` 表
6. **重置令牌** → HMAC-SHA256 哈希存储在 `password_reset_tokens` 表，支持 replay 防护与过期校验

**推荐测试入口**：
- 未登录用户：从 `/login` 点击「Forgot Password」，并验证跳转到 `/forgot-password`
- 收到邮件后：直接访问 `/reset-password?token={token}`
- 已登录用户：进入 `Account -> Security` 修改密码

**测试原则**：
- 所有密码流程从 Auth9 Portal 触发，不涉及 Keycloak UI
- 密码验证由 Auth9 本地 argon2id 执行，不调用 Keycloak credentials API
- Dark Mode 视觉层级与对比度回归由 [15-dark-mode-auth-contrast.md](./15-dark-mode-auth-contrast.md) 单独覆盖

---

## 场景 1：忘记密码 - 发送重置邮件

### 初始状态
- 用户已注册但忘记密码
- 邮件服务已配置

### 目的
验证忘记密码功能

### 测试操作流程
1. 访问 Auth9 Portal `/login`
2. 点击 `Forgot password?`
3. 确认跳转到 `/forgot-password`
4. 输入注册邮箱：`user@example.com`
5. 点击「Send reset link」

### 预期结果
- 显示「如果该邮箱存在，我们已发送重置链接」
- 用户收到重置邮件

### 预期数据状态
```sql
-- 验证重置令牌已创建（如果有 password_reset_tokens 表）
SELECT id, user_id, expires_at FROM password_reset_tokens
WHERE user_id = (SELECT id FROM users WHERE email = 'user@example.com')
ORDER BY created_at DESC LIMIT 1;
-- 预期: 存在记录，expires_at 为未来时间
```

---

## 场景 2：重置密码

### 初始状态
- 用户有有效的密码重置令牌
- Mailpit 已配置为底层认证邮件服务（开发环境自动配置）

### 目的
验证密码重置流程

### 测试操作流程
1. 从 Mailpit 获取重置链接：
   - **方法 A（Web UI）**：打开 `http://localhost:8025`，找到最新的重置邮件，点击邮件中的链接
   - **方法 B（API）**：
     ```bash
     # 获取最新邮件中的重置链接
     curl -s http://localhost:8025/api/v1/messages | \
       python3 -c "import sys,json; msgs=json.load(sys.stdin)['messages']; print(msgs[0]['ID'])" | \
       xargs -I{} curl -s http://localhost:8025/api/v1/message/{} | \
       python3 -c "import sys,json,re; msg=json.load(sys.stdin); links=re.findall(r'http[s]?://[^\s\"<>]+action-token[^\s\"<>]+', msg.get('HTML','')); print(links[0] if links else 'No reset link found')"
     ```
2. 在 Auth9 `/reset-password` 页面输入新密码：`NewSecurePass123!`
3. 确认新密码
4. 提交

### 预期结果
- 显示密码重置成功
- 可以使用新密码登录
- 重置令牌失效

### 预期数据状态
```sql
-- 重置令牌应被标记为已使用
SELECT used_at FROM password_reset_tokens WHERE id = '{token_id}';
-- 预期: used_at 有值

-- 密码凭据已更新（argon2id 哈希）
SELECT credential_type, JSON_EXTRACT(credential_data, '$.algorithm') AS algorithm,
       JSON_EXTRACT(credential_data, '$.temporary') AS temporary
FROM credentials
WHERE user_id = (SELECT identity_subject FROM users WHERE email = 'user@example.com')
  AND credential_type = 'password' AND is_active = 1;
-- 预期: algorithm = "argon2id", temporary = false
```

---

## 场景 3：使用过期重置令牌

### 初始状态
- 重置令牌已过期（`expires_at` 早于 `NOW()`）

### 目的
验证过期令牌处理

### 步骤 0：验证令牌已真正过期

**必须在测试前确认令牌已过期，否则测试无效：**

```sql
-- 确认令牌已过期（expires_at 在过去）
SELECT id, expires_at, used_at,
       CASE WHEN expires_at < NOW() THEN 'EXPIRED' ELSE 'VALID' END AS status
FROM password_reset_tokens
WHERE user_id = (SELECT id FROM users WHERE email = 'admin@auth9.local')
ORDER BY created_at DESC LIMIT 1;
-- 必须: status = 'EXPIRED'
-- 若 used_at IS NOT NULL，令牌已使用（测试会因无法找到未使用令牌而失败）
```

> **系统默认令牌有效期：1 小时**。建议通过直接修改数据库使令牌过期，而非等待：
> ```sql
> UPDATE password_reset_tokens
> SET expires_at = '2020-01-01 00:00:00'
> WHERE user_id = (SELECT id FROM users WHERE email = 'admin@auth9.local')
>   AND used_at IS NULL;
> ```

### 测试操作流程
1. 首先通过场景 2 步骤获取重置链接（`/reset-password?token=xxx`）
2. **执行步骤 0 验证令牌已过期**（通过等待或直接修改 DB）
3. 访问已过期的重置链接并提交新密码

### 预期结果
- 显示错误：「链接已过期，请重新申请」

> **故障排除**
>
> | 症状 | 原因 | 解决方案 |
> |------|------|---------|
> | 密码重置成功（而非报错） | 令牌实际未过期 | 执行步骤 0 确认 status=EXPIRED |
> | 页面直接报"无效令牌" | 令牌已被使用（used_at 非空） | 申请新令牌并确保未使用前手动使其过期 |

---

## 场景 4：修改密码（已登录用户）

### 初始状态
- 用户已登录
- 用户知道当前密码

### 目的
验证修改密码功能

### 测试操作流程
1. 进入「Account」→「Security」
2. 在「Change Password」表单中操作
3. 输入当前密码
4. 输入新密码
5. 确认新密码
6. 提交

### 预期结果
- 显示密码修改成功
- 可以使用新密码登录

### 预期数据状态
```sql
-- 密码凭据已更新为新哈希
SELECT credential_type, JSON_EXTRACT(credential_data, '$.algorithm') AS algorithm,
       is_active, updated_at
FROM credentials
WHERE user_id = (SELECT identity_subject FROM users WHERE email = '{user_email}')
  AND credential_type = 'password' AND is_active = 1;
-- 预期: algorithm = "argon2id", is_active = 1, updated_at 为最近时间

-- password_changed_at 已更新
SELECT password_changed_at FROM users WHERE email = '{user_email}';
-- 预期: password_changed_at 为最近时间
```

---

## 场景 5：密码强度验证

### 初始状态
- **密码策略已由 Seeder 配置**（auth9-core 启动时自动执行）
- 策略要求：最少 12 字符、至少 1 个大写字母、1 个小写字母、1 个数字、1 个特殊字符
- 密码策略存储在 Auth9 `tenants` 表的 `password_policy` JSON 字段，由 Auth9 后端本地执行校验
- 验证当前策略：
  ```bash
  TOKEN=$(.claude/skills/tools/gen-admin-token.sh)
  curl -s http://localhost:8080/api/v1/tenants/{tenant_id}/password-policy \
    -H "Authorization: Bearer $TOKEN" | python3 -m json.tool
  # 预期: min_length=12, require_uppercase=true, require_lowercase=true, require_numbers=true, require_symbols=true
  ```

### 目的
验证密码强度验证

### 测试操作流程
通过 Auth9 `/reset-password` 页面或 `Account -> Security` 页面测试以下弱密码：
1. 太短：`abc123`
2. 无大写：`password123!`
3. 无数字：`Password!`
4. 无特殊字符：`Password123`

### 预期结果
- 每种情况显示相应的密码强度错误
- 密码不被接受

### 常见误报

| 症状 | 原因 | 解决方法 |
|------|------|----------|
| 密码策略 API 返回默认值 | Seeder 未完成或 tenant 无自定义策略 | 检查 `docker logs auth9-init` 确认 seeder 完成 |
| `/login` 页面未显示找回密码入口 | Portal 登录页渲染异常或入口回归缺失 | 转测 [15-dark-mode-auth-contrast.md](./15-dark-mode-auth-contrast.md) 场景 1，并检查 `/login` 底部链接区 |
| 弱密码被接受 | tenant 的 `password_policy` JSON 为空或宽松 | 通过 API 更新密码策略或重启 auth9-core 让 seeder 重新初始化 |

---

## 检查清单

| # | 场景 | 状态 | 测试日期 | 测试人员 | 备注 |
|---|------|------|----------|----------|------|
| 1 | 忘记密码 | ☐ | | | |
| 2 | 重置密码 | ☐ | | | |
| 3 | 过期重置令牌 | ☐ | | | |
| 4 | 修改密码 | ☐ | | | |
| 5 | 密码强度验证 | ☐ | | | |
