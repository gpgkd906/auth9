# 用户管理 - 密码策略测试

**模块**: 用户管理
**测试范围**: 密码策略配置、强制执行、账户锁定
**场景数**: 5
**优先级**: 高

---

## 背景

Auth9 支持租户级别的密码策略配置（`tenants.password_policy` JSON 字段），包括：
- 最小长度要求
- 字符类型要求（大写、小写、数字、特殊字符）
- 密码年龄限制
- 密码历史检查（防止重用）
- 账户锁定策略

---

## 场景 1：最小长度和字符类型要求

### 初始状态
- 租户已配置密码策略：
  ```json
  {
    "min_length": 12,
    "require_uppercase": true,
    "require_lowercase": true,
    "require_number": true,
    "require_symbol": true
  }
  ```

### 目的
验证密码创建时强制执行策略

### 测试操作流程
1. 尝试创建用户，使用弱密码 `password`
2. 尝试使用 `Password1`（缺少特殊字符）
3. 尝试使用 `Password!`（缺少数字）
4. 尝试使用 `Pass1!`（长度不足 12）
5. 使用符合要求的密码 `MySecurePass123!`

### 预期结果
- 步骤 1-4 都失败，返回 400 错误，错误信息具体说明缺少什么
- 步骤 5 成功创建用户

### 预期数据状态
```sql
SELECT COUNT(*) FROM users WHERE email = 'policy-test@example.com';
-- 预期: 1（仅步骤 5 成功）

-- 检查 Keycloak 用户密码是否同步
```

---

## 场景 2：密码历史检查（防止重用）

### 初始状态
- 租户密码策略配置：
  ```json
  {
    "history_count": 5
  }
  ```
- 用户已存在，历史密码为：`OldPass1!`, `OldPass2!`, `OldPass3!`, `OldPass4!`, `OldPass5!`

### 目的
验证用户不能重用最近 5 个密码

### 测试操作流程
1. 用户尝试修改密码为 `OldPass3!`（历史密码）
2. 尝试修改密码为 `OldPass5!`（历史密码）
3. 尝试修改密码为 `NewSecurePass1!`（新密码）

### 预期结果
- 步骤 1-2 失败，错误信息：「Password has been used recently」
- 步骤 3 成功修改密码

### 预期数据状态
```sql
-- 假设有 password_history 表（如果实现）
SELECT COUNT(*) FROM password_history 
WHERE user_id = '{user_id}' 
ORDER BY created_at DESC 
LIMIT 5;
-- 预期: 5 条记录（保留最近 5 个）

-- 或者检查 Keycloak 密码策略配置
```

---

## 场景 3：密码年龄限制（强制定期修改）

### 初始状态
- 租户密码策略：
  ```json
  {
    "max_age_days": 90
  }
  ```
- 用户密码设置于 100 天前

### 目的
验证系统强制用户定期修改密码

### 测试操作流程
1. 用户尝试登录（密码已过期）
2. 系统应提示「密码已过期，请修改密码」
3. 用户修改密码为 `NewPassword123!`
4. 用户使用新密码成功登录

### 预期结果
- 步骤 1 登录失败或跳转到密码修改页面
- 步骤 2 显示明确的过期提示
- 步骤 3 成功修改密码
- 步骤 4 成功登录

### 预期数据状态
```sql
SELECT password_changed_at FROM users WHERE id = '{user_id}';
-- 预期: 当前时间（最新修改时间）

-- 检查审计日志
SELECT action, details FROM audit_logs 
WHERE resource_type = 'user' 
  AND resource_id = '{user_id}' 
  AND action = 'password_change'
ORDER BY created_at DESC LIMIT 1;
-- 预期: 最新一条密码修改记录
```

---

## 场景 4：账户锁定策略（暴力破解防护）

### 初始状态
- 租户密码策略：
  ```json
  {
    "lockout_threshold": 5,
    "lockout_duration_mins": 30
  }
  ```
- 用户账户正常，无锁定

### 目的
验证连续失败登录后账户自动锁定

### 测试操作流程
1. 使用错误密码尝试登录 5 次
2. 第 6 次使用正确密码尝试登录
3. 等待 30 分钟后，使用正确密码登录
4. 检查 `security_alerts` 表是否记录异常

### 预期结果
- 步骤 1: 每次失败返回「用户名或密码错误」
- 步骤 2: 返回「账户已锁定，请 30 分钟后重试」
- 步骤 3: 成功登录，锁定自动解除
- `security_alerts` 记录暴力破解告警

### 预期数据状态
```sql
-- 检查锁定状态（如果有 locked_until 字段）
SELECT locked_until FROM users WHERE id = '{user_id}';
-- 步骤 2 时预期: 当前时间 + 30 分钟
-- 步骤 3 后预期: NULL

-- 检查安全告警
SELECT alert_type, severity FROM security_alerts 
WHERE user_id = '{user_id}' 
  AND alert_type = 'brute_force_attempt'
ORDER BY created_at DESC LIMIT 1;
-- 预期: 1 条记录，severity = 'high'

-- 检查登录事件
SELECT event_type, COUNT(*) FROM login_events 
WHERE user_id = '{user_id}' 
  AND created_at >= DATE_SUB(NOW(), INTERVAL 1 HOUR)
GROUP BY event_type;
-- 预期: LOGIN_ERROR: 5, LOGIN: 1
```

---

## 场景 5：管理员绕过密码策略（特殊场景）

### 初始状态
- 租户密码策略要求 12 位，包含大小写数字特殊字符
- 平台管理员需要为用户设置临时密码

### 目的
验证管理员可以设置临时弱密码，但用户首次登录必须修改

### 测试操作流程
1. 管理员为新用户设置临时密码 `Temp123!`（较弱）
2. 系统标记该密码为「临时密码」（`temporary: true`）
3. 用户使用临时密码登录
4. 系统强制跳转到密码修改页面
5. 用户修改为符合策略的密码 `MyNewPassword456!`

### 预期结果
- 步骤 1 成功，管理员可以绕过策略（仅限临时密码）
- 步骤 3 登录成功但立即触发密码修改流程
- 步骤 5 修改成功，取消「临时」标记

### 预期数据状态
```sql
-- Keycloak 中检查用户凭证
-- 临时密码标记应在首次登录后清除

-- 检查审计日志
SELECT action, actor_id FROM audit_logs 
WHERE resource_type = 'user' 
  AND resource_id = '{user_id}' 
  AND action = 'password_set'
ORDER BY created_at DESC LIMIT 1;
-- 预期: actor_id 为管理员 ID

SELECT action FROM audit_logs 
WHERE resource_type = 'user' 
  AND resource_id = '{user_id}' 
  AND action = 'password_change'
ORDER BY created_at DESC LIMIT 1;
-- 预期: 用户自己修改密码的记录
```

---

## 测试数据准备

### 设置租户密码策略
```sql
UPDATE tenants SET password_policy = JSON_OBJECT(
  'min_length', 12,
  'require_uppercase', true,
  'require_lowercase', true,
  'require_number', true,
  'require_symbol', true,
  'history_count', 5,
  'max_age_days', 90,
  'lockout_threshold', 5,
  'lockout_duration_mins', 30
)
WHERE slug = 'test-tenant';
```

### 模拟历史密码（如果有 password_history 表）
```sql
-- 插入 5 个历史密码哈希
INSERT INTO password_history (user_id, password_hash, created_at)
VALUES 
  ('{user_id}', '$2a$...', DATE_SUB(NOW(), INTERVAL 120 DAY)),
  ('{user_id}', '$2a$...', DATE_SUB(NOW(), INTERVAL 90 DAY)),
  ('{user_id}', '$2a$...', DATE_SUB(NOW(), INTERVAL 60 DAY)),
  ('{user_id}', '$2a$...', DATE_SUB(NOW(), INTERVAL 30 DAY)),
  ('{user_id}', '$2a$...', NOW());
```

---

## 注意事项

1. **密码策略同步**：Auth9 配置的策略需要同步到 Keycloak
2. **向后兼容**：现有用户的密码可能不符合新策略，但不强制立即修改
3. **多因素认证**：密码策略与 MFA 配合使用，提供更强安全性
4. **审计合规**：所有密码修改操作必须记录到 `audit_logs`
