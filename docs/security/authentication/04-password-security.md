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

Auth9 密码管理由 Keycloak 处理：
- 密码哈希: Argon2 或 PBKDF2
- 密码重置: 通过邮件链接
- 密码策略: 可配置强度要求

### 架构对齐说明（Headless Keycloak）

- Auth9 采用 Headless Keycloak 架构，Keycloak 作为 OIDC/认证引擎使用
- 本文档不要求必须通过 Keycloak 托管登录页进行测试
- 密码安全测试以接口、事件、数据库和管理 API 验证为主
- 如需做页面回归，仅作为补充验证（例如主题/交互），不作为安全结论前置条件

---

## 场景 1：密码暴力破解防护

### 前置条件
- 已知用户名
- **Docker 环境已完全启动且 auth9-core 完成初始化（seeder 已执行）**
- **验证方法**: 检查 auth9-core 日志包含 `"Configured realm 'auth9' security: bruteForceProtected=true"` 或通过 Keycloak Admin API 确认: `curl -s http://localhost:8081/admin/realms/auth9 -H "Authorization: Bearer $TOKEN" | jq '.bruteForceProtected'` 返回 `true`

### 攻击目标
验证登录是否存在暴力破解风险

### 攻击步骤
1. 使用有效用户名
2. 自动化提交错误密码 (如 1000 次)
3. 测试不同 IP 地址的行为
4. 检查账户锁定和解锁机制

### 预期安全行为
- 连续 5 次失败后账户临时锁定（Keycloak `failureFactor=5`）
- 锁定等待时间渐进增长（`waitIncrementSeconds=60`，最大 `maxFailureWaitSeconds=900`）
- 不泄露用户是否存在

### 验证方法
```bash
# 步骤 0（必需）：确认 brute force 已启用
# auth9-core 的 seeder 通过 Keycloak Admin API 配置 bruteForceProtected=true。
# 如果 Keycloak 刚启动但 auth9-core 尚未运行 seeder，配置为默认值 (null)。
# 必须先启动 auth9-core 并等待 seeder 完成。
#
# ⚠️ 重要：nginx gateway 阻止从宿主机访问 Keycloak /admin 端点。
# 必须从 Docker 网络内部验证：
KC_TOKEN=$(docker exec auth9-core curl -s -X POST \
  "http://keycloak:8080/realms/master/protocol/openid-connect/token" \
  -d "client_id=admin-cli" -d "username=admin" -d "password=admin" \
  -d "grant_type=password" | jq -r '.access_token')
docker exec auth9-core curl -s "http://keycloak:8080/admin/realms/auth9" \
  -H "Authorization: Bearer $KC_TOKEN" | jq '{bruteForceProtected, failureFactor}'
# 预期: {"bruteForceProtected": true, "failureFactor": 5}

# 方法 A（推荐）：直接对 OIDC token endpoint 发起错误密码请求
# 说明：仅当测试客户端开启 Direct Access Grants 时可用
# 默认开发环境可使用已 seed 的管理员邮箱 `admin@auth9.local` 作为“已知存在用户”。
for i in {1..50}; do
  curl -X POST http://localhost:8081/realms/auth9/protocol/openid-connect/token \
    -d "grant_type=password" \
    -d "client_id=auth9-portal" \
    -d "username=admin@auth9.local" \
    -d "password=wrong_$i"
  echo "Attempt: $i"
done

# 预期: 第 6 次后返回 user_disabled / account locked 或出现显著延迟

# 方法 B（无 Direct Access Grants 场景）：
# 通过自动化脚本驱动标准 OIDC 授权流程提交错误口令，
# 或通过 Keycloak 事件链路验证 LOGIN_ERROR 累积与锁定状态。
```

### 常见失败排查

| 症状 | 原因 | 修复方法 |
|------|------|---------|
| `bruteForceProtected` 为 null | auth9-core seeder 未执行 | 启动 auth9-core 并等待 seeder 完成 |
| 从宿主机查询 Admin API 返回 401/403 | nginx gateway 阻止宿主机访问 `/admin` | 使用 `docker exec auth9-core curl ...` 从 Docker 内部查询 |
| 50 次错误后仍无锁定 | 环境未初始化或使用了错误的 realm | 执行 `./scripts/reset-docker.sh` 重建环境 |
| 只有 0-1 条 `LOGIN_ERROR`，且始终不锁定 | 测试使用了不存在的用户名，或客户端未开启 Direct Access Grants | 改用默认存在用户 `admin@auth9.local`；若返回 `unauthorized_client`，切换到方法 B，通过标准登录流程或事件链路验证 |
| 锁定后无法恢复 | `permanentLockout` 意外设为 true | 检查 seeder 配置，默认 `permanentLockout=false` |

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

# 如果系统未暴露上述 API，而是完全委托 Keycloak 托管流程：
# 1) 触发 Keycloak reset credentials 流程
# 2) 验证不存在邮箱时返回语义一致
# 3) 验证重置链接一次性与过期策略
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

> **注意**: Keycloak 默认使用嵌入式 H2 数据库，无法直接查询。推荐使用以下方法验证。

```bash
# 方法 A（推荐）：通过 Keycloak Admin API 查询密码策略配置
# ⚠️ 重要：必须从 Docker 网络内部访问 Keycloak Admin API（nginx gateway 阻止宿主机访问 /admin）
KC_TOKEN=$(docker exec auth9-core curl -s -X POST \
  "http://keycloak:8080/realms/master/protocol/openid-connect/token" \
  -d "client_id=admin-cli" -d "username=admin" -d "password=admin" \
  -d "grant_type=password" | jq -r '.access_token')

# 查询 realm 密码策略
docker exec auth9-core curl -s "http://keycloak:8080/admin/realms/auth9" \
  -H "Authorization: Bearer $KC_TOKEN" | jq '{passwordPolicy, bruteForceProtected}'
# 预期: passwordPolicy 包含 "hashAlgorithm(pbkdf2-sha512)" 和 "hashIterations(210000)"

# 方法 B：直接查询 credential 表（仅当 Keycloak 使用外部数据库时可用）
# SELECT credential_data FROM credential WHERE user_id = 'xxx';
# 应返回: {"hashIterations":210000,"algorithm":"pbkdf2-sha512",...}
```

### 常见失败排查

| 症状 | 原因 | 修复方法 |
|------|------|---------|
| H2 数据库无法查询 | Keycloak 使用嵌入式 H2 | 改用 Keycloak Admin API 方法 A |
| passwordPolicy 为空 | auth9-core seeder 未执行 | 启动 auth9-core 并等待 seeder 完成 |
| 从宿主机查询返回 401/"HTTPS required" | nginx gateway 阻止宿主机访问 `/admin` | 使用 `docker exec auth9-core curl ...` 从 Docker 内部查询 |

### 修复建议
- 使用 Argon2id (推荐) 或 bcrypt
- PBKDF2 至少 100,000 次迭代
- 定期审计哈希参数
- 考虑密码迁移策略

---

## 场景 4：密码更改安全

### 前置条件
- 已登录用户
- **Docker 环境已完全启动且 auth9-core 完成初始化（seeder 已执行）**
- **验证方法**: 检查 auth9-core 日志包含密码策略同步信息，或通过 Keycloak Admin API 确认 `passwordPolicy` 不为空（参见场景 1 验证方法）

### 攻击目标
验证密码更改流程安全性

### 攻击步骤
1. 尝试更改密码：
   - 不提供当前密码
   - 新密码与旧密码相同
   - 通过 CSRF 攻击更改
2. 检查是否强制注销其他会话
3. 检查密码历史检查

> **Note**: Revoking other sessions after password change depends on Keycloak realm configuration (`revokeRefreshToken`), not auth9-core code. This behavior is configured in Keycloak, not managed by Auth9.

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

# 如果密码修改完全在 Keycloak 侧执行：
# 通过 Keycloak Admin API 或用户动作策略验证
# - requiredActions / re-authentication 约束
# - session invalidation 是否生效
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
