# 认证安全 - 多因素认证安全测试

**模块**: 认证安全
**测试范围**: MFA 实现安全性 (TOTP, WebAuthn)
**场景数**: 5
**风险等级**: 🟠 高
**ASVS 5.0 矩阵ID**: M-AUTH-03
**OWASP ASVS 5.0**: V6.7,V6.8,V7.3
**回归任务映射**: Backlog #20


---

## 背景知识

Auth9 通过 Keycloak 支持多种 MFA 方式：
- **TOTP**: 基于时间的一次性密码 (Google Authenticator)
- **WebAuthn/Passkeys**: 硬件安全密钥或平台认证
- **Email OTP**: 邮件验证码

---

## 场景 1：TOTP 暴力破解攻击

### 前置条件
- 启用了 TOTP 的用户账户
- 已知用户名和密码

### 攻击目标
验证 TOTP 验证是否存在暴力破解风险

### 攻击步骤
1. 正常输入用户名密码
2. 在 MFA 页面自动化提交 TOTP 代码：
   - 遍历 000000 - 999999
   - 测试速率限制
3. 检查是否有账户锁定机制

### 预期安全行为
- 连续错误后临时锁定 (如 5 次错误锁定 5 分钟)
- 请求速率限制
- 记录失败尝试

### 验证方法
```bash
# 使用脚本快速提交错误 TOTP
for i in {1..20}; do
  curl -X POST "http://localhost:8081/realms/auth9/login-actions/authenticate" \
    -d "session_code=xxx&otp=$i"
  sleep 0.1
done

# 检查是否被锁定或限速
# 预期: 第 6 次后返回 "Account locked temporarily"
```

### 修复建议
- 实现指数退避锁定策略
- 添加 CAPTCHA 防护
- 记录并告警异常登录尝试
- 考虑使用 8 位 TOTP

---

## 场景 2：TOTP 时间窗口攻击

### 前置条件
- 启用 TOTP 的用户账户
- 能够获取用户的 TOTP 密钥 (模拟泄露场景)

### 攻击目标
验证 TOTP 时间窗口容忍度是否过大

### 攻击步骤
1. 生成当前 TOTP 代码
2. 测试以下时间偏移的代码：
   - 当前时间段 (应成功)
   - 前 1 个时间段 (30秒前)
   - 前 2 个时间段 (60秒前)
   - 后 1 个时间段 (30秒后)
3. 检查可接受的时间窗口范围

### 预期安全行为
- 仅接受当前时间段 ± 1 个时间段
- 过早或过晚的代码应被拒绝

### 验证方法
```python
import pyotp
import time

totp = pyotp.TOTP("BASE32_SECRET")

# 测试不同时间偏移
for offset in [-120, -60, -30, 0, 30, 60, 120]:
    code = totp.at(time.time() + offset)
    # 提交 code 测试是否被接受
```

### 修复建议
- 时间窗口不超过 ± 30 秒
- 实现代码使用记录防止重放
- 服务器时间同步 (NTP)

---

## 场景 3：MFA 绕过测试

### 前置条件
- 启用 MFA 的用户账户

### 攻击目标
验证是否可以绕过 MFA 验证

### 攻击步骤
1. 正常登录触发 MFA
2. 尝试以下绕过方法：
   - 直接访问登录后页面
   - 修改 session 状态
   - 使用旧的 (MFA 前) session cookie
   - 并发请求绕过
   - 使用 API 端点绕过 (如直接调用 /api/v1/auth/callback)

### 预期安全行为
- 所有受保护资源都要求完整认证
- MFA 状态在服务端验证
- 不依赖客户端状态

### 验证方法
```bash
# 获取 MFA 前的 session cookie
# 完成密码验证后，立即访问受保护页面

curl -b "session=pre_mfa_session" \
  http://localhost:3000/dashboard
# 预期: 重定向回 MFA 页面

# 直接调用 API
curl -X GET http://localhost:8080/api/v1/users \
  -b "session=pre_mfa_session"
# 预期: 401 MFA required
```

### 修复建议
- 服务端维护 MFA 完成状态
- 所有 API 检查完整认证状态
- 敏感操作需要 step-up 认证

---

## 场景 4：MFA 注册流程安全

### 前置条件
- 未启用 MFA 的用户账户

### 攻击目标
验证 MFA 注册流程是否安全

### 攻击步骤
1. 开始 MFA 注册流程
2. 尝试以下攻击：
   - 在不验证当前密码的情况下启用 MFA
   - 注册 MFA 到其他用户账户
   - TOTP 密钥是否可预测
   - 备份码是否安全生成

### 预期安全行为
- 注册 MFA 需要当前密码验证
- TOTP 密钥使用 CSPRNG 生成
- 备份码足够随机且安全存储

### 验证方法
```bash
# 尝试不提供密码启用 MFA
curl -X POST http://localhost:8080/api/v1/users/me/mfa \
  -H "Authorization: Bearer $TOKEN" \
  -d '{"type": "totp"}'
# 预期: 400 "Current password required"

# 检查 TOTP 密钥熵
# 密钥应至少 128 位
```

### 修复建议
- 注册/修改 MFA 需要密码验证
- 使用 CSPRNG 生成所有密钥
- 备份码单向哈希存储
- 通知用户 MFA 变更

---

## 场景 5：MFA 恢复机制安全

### 前置条件
- 启用 MFA 的用户账户
- 备份码

### 攻击目标
验证 MFA 恢复机制的安全性

### 攻击步骤
1. 使用备份码登录
2. 测试以下场景：
   - 备份码是否一次性
   - 备份码是否可暴力破解
   - 管理员禁用他人 MFA 的流程
   - MFA 重置邮件是否安全

### 预期安全行为
- 备份码一次性使用
- 备份码足够长防止暴力破解
- 管理员禁用 MFA 需要额外验证
- MFA 重置有时间限制

### 验证方法
```bash
# 使用备份码登录
# 第一次应成功
# 第二次使用同一备份码应失败

# 测试管理员禁用他人 MFA
curl -X DELETE http://localhost:8080/api/v1/users/{other_user_id}/mfa \
  -H "Authorization: Bearer $ADMIN_TOKEN"
# 预期: 需要额外验证 (OTP 或密码确认)
```

### 修复建议
- 备份码仅能使用一次
- 备份码至少 12 位随机字符
- 管理员禁用 MFA 需要二次验证
- 记录审计日志
- 通知用户 MFA 被禁用

---

## 检查清单

| # | 场景 | 状态 | 测试日期 | 测试人员 | 发现问题 |
|---|------|------|----------|----------|----------|
| 1 | TOTP 暴力破解攻击 | ⚠️ 部分验证 | 2026-02-22 | QA Test | 需进一步测试速率限制 |
| 2 | TOTP 时间窗口攻击 | ✅ PASS | 2026-02-22 | QA Test | 系统拒绝时间偏移代码 |
| 3 | MFA 绕过测试 | ⚠️ 部分验证 | 2026-02-22 | QA Test | 无效session正确拒绝401 |
| 4 | MFA 注册流程安全 | ✅ PASS | 2026-02-22 | QA Test | 启用MFA需要密码确认 |
| 5 | MFA 恢复机制安全 | ⚠️ 未测试 | 2026-02-22 | QA Test | 需测试备份码功能 |

---

## 参考资料

- [RFC 6238 - TOTP](https://datatracker.ietf.org/doc/html/rfc6238)
- [WebAuthn Specification](https://www.w3.org/TR/webauthn/)
- [OWASP MFA Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Multifactor_Authentication_Cheat_Sheet.html)
- [CWE-308: Use of Single-factor Authentication](https://cwe.mitre.org/data/definitions/308.html)

---


---

## 标准化回归 Checklist（ASVS 5.0）

**矩阵ID**: M-AUTH-03  
**适用控制**: V6.7,V6.8,V7.3  
**关联任务**: Backlog #20  
**建议回归频率**: 每次发布前 + 缺陷修复后必跑  
**场景总数**: 5

### 执行清单
- [ ] M-AUTH-03-C01 | 控制: V6.7 | 任务: #20 | 动作: 执行文档内相关攻击步骤并记录证据
- [ ] M-AUTH-03-C02 | 控制: V6.8 | 任务: #20 | 动作: 执行文档内相关攻击步骤并记录证据
- [ ] M-AUTH-03-C03 | 控制: V7.3 | 任务: #20 | 动作: 执行文档内相关攻击步骤并记录证据

### 回归记录表
| 检查项ID | 执行结果(pass/fail) | 风险等级 | 证据（请求/响应/日志/截图） | 备注 |
|---|---|---|---|---|
|  |  |  |  |  |

### 退出准则
1. 所有检查项执行完成，且高风险项无 `fail`。
2. 如存在 `fail`，必须附带漏洞单号、修复计划和复测结论。
3. 回归报告需同时记录矩阵ID与 Backlog 任务号，便于跨版本追溯。
