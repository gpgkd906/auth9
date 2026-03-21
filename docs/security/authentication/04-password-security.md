# 认证安全 - 密码安全测试

**模块**: 认证安全
**测试范围**: 密码策略、存储和重置安全
**场景数**: 4
**风险等级**: 🟠 高
**ASVS 5.0 矩阵ID**: M-AUTH-04
**OWASP ASVS 5.0**: V6.1,V6.2,V6.3,V6.6
**回归任务映射**: Backlog #11, #20


---

## 背景知识

Auth9 自行管理密码全生命周期：
- 密码哈希: Argon2id（auth9-oidc 本地存储）
- 密码重置: Auth9 自签发 reset token，通过邮件链接消费
- 密码策略: Auth9 自管（`GET/PUT /api/v1/tenants/{id}/password-policy`），支持 min_length、大小写/数字/符号要求、max_age_days、history_count、lockout 阈值

### 架构对齐说明

- 密码验证、重置、策略全部由 Auth9 OIDC Engine 本地处理
- 密码安全测试以 Auth9 API 端点验证为主（`/api/v1/hosted-login/password`、`/api/v1/auth/forgot-password`、`/api/v1/auth/reset-password`、`/api/v1/users/me/password`）

---

## 场景 1：密码暴力破解防护

### 前置条件
- 已知用户名
- **Docker 环境已完全启动且 auth9-core 完成初始化**
- **验证方法**: 通过 Auth9 密码策略 API 确认暴力破解保护已启用: `curl -s http://localhost:8080/api/v1/tenants/{id}/password-policy -H "Authorization: Bearer $TOKEN" | jq '.lockout_threshold'` 返回非零值（如 5）

### 攻击目标
验证登录是否存在暴力破解风险

### 攻击步骤
1. 使用有效用户名
2. 自动化提交错误密码 (如 1000 次)
3. 测试不同 IP 地址的行为
4. 检查账户锁定和解锁机制

### 预期安全行为
- 连续 5 次失败后账户临时锁定
- 锁定等待时间渐进增长
- 不泄露用户是否存在

### 验证方法
```bash
# 步骤 0（必需）：确认暴力破解保护已启用
# 通过 Auth9 密码策略 API 验证：
curl -s http://localhost:8080/api/v1/tenants/{tenant_id}/password-policy \
  -H “Authorization: Bearer $TOKEN” | jq '{lockout_threshold, max_age_days}'
# 预期: lockout_threshold 为 5（5 次失败后锁定）

# 方法 A（推荐）：通过 Auth9 托管认证页提交错误密码
# 默认开发环境可使用已 seed 的管理员邮箱 `admin@auth9.local` 作为”已知存在用户”。
for i in {1..50}; do
  curl -X POST http://localhost:8080/api/v1/hosted-login/password \
    -H “Content-Type: application/json” \
    -d “{\”email\”:\”admin@auth9.local\”,\”password\”:\”wrong_$i\”}”
  echo “Attempt: $i”
done

# 预期: 第 6 次后返回账户锁定错误或出现显著延迟

# 方法 B：通过自动化脚本驱动标准 OIDC 授权流程提交错误口令。
```

### 常见失败排查

| 症状 | 原因 | 修复方法 |
|------|------|---------|
| `lockout_threshold` 为 0 或 null | 密码策略未配置 | 通过 `PUT /api/v1/tenants/{id}/password-policy` 配置锁定阈值 |
| 50 次错误后仍无锁定 | 环境未初始化 | 执行 `./scripts/reset-docker.sh` 重建环境 |
| 只有 0-1 条 `LOGIN_ERROR`，且始终不锁定 | 测试使用了不存在的用户名 | 改用默认存在用户 `admin@auth9.local` |
| 锁定后无法恢复 | 永久锁定被启用 | 检查密码策略配置 |

### 修复建议
- 5 次失败后锁定 15 分钟
- 渐进式延迟 (指数退避)
- IP 级别限制: 100 次/分钟
- CAPTCHA 在多次失败后启用
- 账户锁定通知邮件

---

## 场景 2：密码重置流程安全

### 前置条件
- 有效用户账户和邮箱

### 攻击目标
验证密码重置流程是否安全

### 攻击步骤
1. 请求密码重置
2. 检查重置链接：
   - Token 长度和熵
   - Token 有效期
   - Token 是否一次性
   - 是否可预测
3. 测试：
   - 不存在邮箱的响应
   - 并发重置请求
   - 重置后旧 Token 是否失效

### 预期安全行为
- Token 足够随机 (>= 128 bits)
- Token 短期有效 (< 1 小时)
- Token 一次性使用
- 不泄露邮箱是否存在

### 验证方法
```bash
# 通过 Auth9 对外认证入口请求重置（示例端点，按实际部署调整）
curl -X POST http://localhost:8080/api/v1/auth/forgot-password \
  -H "Content-Type: application/json" \
  -d '{"email":"user@test.com"}'

# 检查返回响应 (应与不存在邮箱相同)
curl -X POST http://localhost:8080/api/v1/auth/forgot-password \
  -H "Content-Type: application/json" \
  -d '{"email":"nonexistent@test.com"}'
# 预期: 相同的成功响应

# 测试 Token 重用（示例端点，按实际部署调整）
curl -X POST http://localhost:8080/api/v1/auth/reset-password \
  -H "Content-Type: application/json" \
  -d '{"token":"used_token","new_password":"NewPass123!"}' # pragma: allowlist secret
# 预期: 400 "Token invalid or expired"

# Auth9 自管密码重置流程，验证:
# 1) 不存在邮箱时返回语义一致
# 2) 重置链接一次性与过期策略
```

### 修复建议
- 使用 CSPRNG 生成至少 256 位 Token
- 有效期不超过 1 小时
- 成功重置后使所有旧 Token 失效
- 限制重置请求频率
- 记录审计日志

---

## 场景 3：密码存储安全

### 前置条件
- 数据库访问权限 (测试环境)

### 攻击目标
验证密码是否安全存储

### 攻击步骤
1. 检查数据库中的密码存储格式
2. 验证哈希算法
3. 检查是否有盐值
4. 尝试彩虹表攻击 (如果可能)

### 预期安全行为
- 使用强哈希算法 (Argon2id, bcrypt, PBKDF2)
- 每个密码有唯一盐值
- 密码不以明文存储

### 验证方法

Auth9 使用 Argon2id 本地哈希存储密码，可通过数据库和 API 验证。

```bash
# 方法 A（推荐）：通过 Auth9 密码策略 API 查询配置
curl -s http://localhost:8080/api/v1/tenants/{tenant_id}/password-policy \
  -H "Authorization: Bearer $TOKEN" | jq '.'
# 预期: 包含 min_length、require_uppercase 等策略字段

# 方法 B：直接查询数据库验证密码哈希格式
# SELECT password_hash FROM users WHERE email = 'xxx';
# 应返回 Argon2id 格式哈希: $argon2id$v=19$m=...
```

### 常见失败排查

| 症状 | 原因 | 修复方法 |
|------|------|---------|
| 密码策略 API 返回空 | 租户未配置密码策略 | 通过 `PUT /api/v1/tenants/{id}/password-policy` 配置 |
| password_hash 不是 Argon2id 格式 | 早期迁移用户可能使用旧格式 | 用户下次登录时自动升级哈希算法 |

### 修复建议
- 使用 Argon2id (推荐) 或 bcrypt
- PBKDF2 至少 100,000 次迭代
- 定期审计哈希参数
- 考虑密码迁移策略

---

## 场景 4：密码更改安全

### 前置条件
- 已登录用户
- **Docker 环境已完全启动且 auth9-core 完成初始化**
- **验证方法**: 通过 Auth9 密码策略 API 确认策略不为空（参见场景 1 验证方法）

### 攻击目标
验证密码更改流程安全性

### 攻击步骤
1. 尝试更改密码：
   - 不提供当前密码
   - 新密码与旧密码相同
   - 通过 CSRF 攻击更改
2. 检查是否强制注销其他会话
3. 检查密码历史检查

> **Note**: Auth9 在密码更改后会撤销用户的其他活跃会话，确保安全性。

### 预期安全行为
- 更改密码需强身份校验（当前密码、有效会话或等效再认证机制）
- 禁止使用最近 N 个密码
- 更改后注销其他会话
- 发送通知邮件

### 验证方法
```bash
# 通过 Auth9 用户密码更新入口验证（示例端点，按实际部署调整）
# 不提供当前密码/再认证信息
curl -X PUT http://localhost:8080/api/v1/users/me/password \
  -H "Authorization: Bearer $TOKEN" \
  -d '{"new_password":"NewPass123!"}' # pragma: allowlist secret
# 预期: 400/401（缺少必要校验）

# 使用旧密码
curl -X PUT http://localhost:8080/api/v1/users/me/password \
  -H "Authorization: Bearer $TOKEN" \
  -d '{"current_password":"OldPass123!","new_password":"OldPass123!"}' # pragma: allowlist secret
# 预期: 400 "Cannot reuse recent passwords"

# 检查其他会话是否被注销
# 用旧 session 访问应失败

# 验证密码修改后的会话撤销:
# 用旧 session 访问应失败（Auth9 自动撤销同一用户的其他会话）
```

### 修复建议
- 强制验证当前密码
- 保留最近 5-10 个密码哈希
- 更改后注销所有其他会话
- 发送密码更改通知
- 记录审计日志

---

## 检查清单

| # | 场景 | 状态 | 测试日期 | 测试人员 | 发现问题 |
|---|------|------|----------|----------|----------|
| 1 | 密码暴力破解防护 | ☐ | | | |
| 2 | 密码重置流程安全 | ☐ | | | |
| 3 | 密码存储安全 | ☐ | | | |
| 4 | 密码更改安全 | ☐ | | | |

---

## 参考资料

- [OWASP Password Storage Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Password_Storage_Cheat_Sheet.html)
- [OWASP Authentication Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Authentication_Cheat_Sheet.html)
- [NIST Digital Identity Guidelines](https://pages.nist.gov/800-63-3/sp800-63b.html)
- [CWE-521: Weak Password Requirements](https://cwe.mitre.org/data/definitions/521.html)

---


---

## 标准化回归 Checklist（ASVS 5.0）

**矩阵ID**: M-AUTH-04  
**适用控制**: V6.1,V6.2,V6.3,V6.6  
**关联任务**: Backlog #11, #20  
**建议回归频率**: 每次发布前 + 缺陷修复后必跑  
**场景总数**: 4

### 执行清单
- [ ] M-AUTH-04-C01 | 控制: V6.1 | 任务: #11, #20 | 动作: 执行文档内相关攻击步骤并记录证据
- [ ] M-AUTH-04-C02 | 控制: V6.2 | 任务: #11, #20 | 动作: 执行文档内相关攻击步骤并记录证据
- [ ] M-AUTH-04-C03 | 控制: V6.3 | 任务: #11, #20 | 动作: 执行文档内相关攻击步骤并记录证据
- [ ] M-AUTH-04-C04 | 控制: V6.6 | 任务: #11, #20 | 动作: 执行文档内相关攻击步骤并记录证据

### 回归记录表
| 检查项ID | 执行结果(pass/fail) | 风险等级 | 证据（请求/响应/日志/截图） | 备注 |
|---|---|---|---|---|
|  |  |  |  |  |

### 退出准则
1. 所有检查项执行完成，且高风险项无 `fail`。
2. 如存在 `fail`，必须附带漏洞单号、修复计划和复测结论。
3. 回归报告需同时记录矩阵ID与 Backlog 任务号，便于跨版本追溯。
