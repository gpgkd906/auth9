# 会话管理 - 会话安全测试

**模块**: 会话管理
**测试范围**: Session 生成、存储和保护
**场景数**: 4
**风险等级**: 🟠 高
**ASVS 5.0 矩阵ID**: M-SESS-01
**OWASP ASVS 5.0**: V7.1,V7.2,V7.3
**回归任务映射**: Backlog #8, #20


---

## 背景知识

Auth9 会话机制：
- **Auth9 OIDC Session**: OIDC 登录会话
- **Portal Session**: React Router 应用会话
- **API Session**: JWT Token (无状态)
- **Redis 存储**: Session 数据缓存

---

## 场景 1：Session ID 安全性

### 前置条件
- 能够获取 Session Cookie

### 攻击目标
验证 Session ID 是否安全生成

### 攻击步骤
1. 获取多个 Session ID
2. 分析随机性和熵
3. 检查是否可预测
4. 检查 Cookie 属性

### 预期安全行为
- Session ID >= 128 位熵
- 不可预测
- 安全的 Cookie 属性

### 验证方法
```bash
# 获取多个 Session
for i in {1..10}; do
  curl -c - -X POST http://localhost:3000/login \
    -d '{"username":"test","password":"test123"}' 2>/dev/null | \
    grep -i set-cookie
done

# 分析 Session ID
# 1. 长度检查 (>= 32 字符)
# 2. 字符集 (应包含大小写字母+数字)
# 3. 模式分析 (不应有规律)

# 检查 Cookie 属性
curl -I -c - http://localhost:3000/login
# 检查:
# Set-Cookie: session=xxx; HttpOnly; Secure; SameSite=Strict; Path=/
```

### 常见误报排查

| 现象 | 原因 | 结论 |
|------|------|------|
| Docker dev 环境中 `Secure` 标志为 false | `SECURE_COOKIES=false` 是 Docker dev 环境的设计行为，因为本地使用 HTTP 而非 HTTPS。代码逻辑：当 `SECURE_COOKIES` 未设置时，默认跟随 `NODE_ENV=production`。Docker dev 显式设置为 false 以允许 HTTP 开发 | **非安全缺陷** — 生产部署必须设置 `SECURE_COOKIES=true` 或使用 HTTPS |
| 无法获取有效登录凭证完成测试 | Docker dev 环境需要先执行 `./scripts/reset-docker.sh` 初始化种子数据（用户、租户等） | 测试前必须执行环境重建脚本，登录凭证见种子数据 |

> **Auth9 Cookie 安全要求**
>
> 浏览器中的 Auth9 Cookie：
>
> | Cookie | 来源 | 检查重点 |
> |--------|------|----------|
> | `auth9_session` | Auth9 Portal | **必须** HttpOnly=true, SameSite=Lax, Secure=true(生产) |
> | `oauth_state` | Auth9 Portal | **必须** HttpOnly=true, SameSite=Lax, TTL=5min |
>
> **本场景验证 Auth9 控制的 Cookie 安全属性**（`auth9_session`、`oauth_state`）。

### 修复建议
- 使用 CSPRNG 生成
- 至少 128 位熵
- 设置 HttpOnly, Secure, SameSite
- 定期更换 Session ID

---

## 场景 2：Session 固定攻击

### 前置条件
- 能够设置 Cookie
- **Docker 环境已完全启动**（auth9-core、auth9-portal 均健康运行）
- **必须能够完成 OIDC 登录流程**（如登录失败，请先执行 `./scripts/reset-docker.sh` 重建环境）

### 攻击目标
验证是否存在 Session 固定漏洞

### 攻击步骤
1. 获取未登录的 Session ID
2. 设置该 Session ID 到受害者浏览器
3. 受害者登录
4. 检查攻击者是否获得访问权限

### 预期安全行为
- 登录后生成新 Session ID
- 旧 Session ID 失效
- 不接受客户端设置的 Session ID

**重要说明 - Auth9 会话架构**:

Auth9 使用内置 OIDC Engine + JWT 的会话保护架构。测试时需要监控**正确的 Cookie**：

| Cookie | 用途 | 登录后是否更新 |
|--------|------|---------------|
| `auth9_session` | Auth9 Portal 会话 Cookie（含 JWT） | **是** - 登录后新生成 |

> **注意**: `auth9_session` 是实际认证凭证（含签名 JWT）。登录后必须生成新的 session Cookie。

### 验证方法
```bash
# 1. 获取未认证 Session（监控 auth9_session Cookie）
PRE_LOGIN=$(curl -c - http://localhost:3000/ 2>/dev/null | grep auth9_session | awk '{print $7}')
echo "Pre-login auth9_session: $PRE_LOGIN"

# 2. 完成 OIDC 登录流程后获取新 Cookie
# 登录后检查 auth9_session 是否为新值
POST_LOGIN=$(curl -c - -b "auth9_session=$PRE_LOGIN" \
  http://localhost:3000/dashboard 2>/dev/null | grep auth9_session | awk '{print $7}')
echo "Post-login auth9_session: $POST_LOGIN"

# 3. 验证旧 Session 是否失效
curl -b "auth9_session=$PRE_LOGIN" \
  http://localhost:3000/dashboard
# 预期: 重定向到登录页 (旧 Session 无效)

# 4. 验证 Session Token 更新（可选）
# 使用浏览器 DevTools 比较登录前后:
# - auth9_session: 应该登录后为新值
```

### 常见误报排查

| 现象 | 原因 | 结论 |
|------|------|------|
| auth9_session 登录后变化 | 正常行为，JWT session 重新生成 | 安全 |

### 修复建议
- 登录成功后重新生成 Session ID
- 销毁旧 Session
- 不信任客户端 Session
- 绑定 Session 到 IP/User-Agent (可选)

---

## 场景 3：Session 劫持防护

### 前置条件
- 有效的 Session
- 网络监控能力

### 攻击目标
验证 Session 劫持防护机制

### 攻击步骤
1. 获取有效 Session ID
2. 从不同 IP/设备使用该 Session
3. 检查是否被检测或阻止
4. 检查 HTTPS 强制

### 预期安全行为
- 检测异常使用
- 可选的设备绑定
- 强制 HTTPS

### 验证方法
```bash
# 从不同 IP 使用 Session
curl -b "session=$STOLEN_SESSION" \
  -H "X-Forwarded-For: 1.2.3.4" \
  http://localhost:3000/dashboard
# 检查是否允许或触发安全检查

# 检查 HTTPS 强制
curl -k http://localhost:3000/dashboard
# 应重定向到 HTTPS

# 检查 Cookie Secure 属性
# Secure 属性确保仅 HTTPS 传输

# 检查安全告警
SELECT * FROM security_alerts
WHERE alert_type = 'session_anomaly'
ORDER BY created_at DESC;
```

### 修复建议
- Secure Cookie 强制 HTTPS
- 可选 IP/设备绑定
- 异常检测和告警
- Session 活动日志

---

## 场景 4：并发 Session 控制

> **环境排查注意（管理员强制登出）**: The endpoint `POST /api/v1/admin/users/{id}/logout` exists and is implemented. If you get 404, verify the admin token has correct permissions (PolicyAction::SessionForceLogout requires platform admin role).

### 前置条件
- 单个用户账户

### 攻击目标
验证并发 Session 是否有控制

### 攻击步骤
1. 从多个设备/浏览器同时登录
2. 检查 Session 数量限制
3. 检查用户是否可以查看/管理 Session
4. 测试踢出其他 Session

### 预期安全行为

> **重要说明 — 并发会话设计**:
> 系统**有意允许**每个用户最多 10 个并发会话（`MAX_SESSIONS_PER_USER = 10`，定义于 `session.rs`）。
> 这是为了支持用户在多个设备（手机、平板、桌面浏览器等）上同时登录的正常使用场景。
>
> - 创建第 1~10 个会话时，所有会话均保持有效。
> - 创建第 11 个会话时，系统**自动撤销最旧的会话**（FIFO 淘汰策略），确保活跃会话数不超过 10。
> - 测试中如果预期"只有 1 个活跃会话"，这是**不正确的** — 系统设计为允许多设备并发。

- 每用户最多 10 个并发 Session（超出时自动淘汰最旧 Session）
- 用户可查看活跃 Session
- 可撤销其他 Session

### 验证方法
```bash
# 从多个客户端登录（5 个会话均应有效）
for i in {1..5}; do
  curl -c "session_$i.txt" -X POST http://localhost:3000/login \
    -d '{"username":"test","password":"test123"}'
done

# 检查 Session 列表（预期: 5 个活跃会话）
curl -b "session_1.txt" \
  http://localhost:8080/api/v1/users/me/sessions

# 验证所有 5 个 Session 都有效（均不应被淘汰）
for i in {1..5}; do
  curl -b "session_$i.txt" http://localhost:3000/dashboard
done

# 撤销特定 Session
curl -X DELETE -b "session_1.txt" \
  http://localhost:8080/api/v1/sessions/{session_id}

# 验证被撤销的 Session 失效
curl -b "session_2.txt" http://localhost:3000/dashboard
# 预期: 如果 session_2 是被撤销的那个，重定向到登录页

# [可选] 测试超出上限的淘汰行为
# 创建 11 个会话，验证最旧的会话被自动撤销
for i in {1..11}; do
  curl -c "session_overflow_$i.txt" -X POST http://localhost:3000/login \
    -d '{"username":"test","password":"test123"}'
done
# 验证 session_overflow_1 已失效（被第 11 个会话淘汰）
curl -b "session_overflow_1.txt" http://localhost:3000/dashboard
# 预期: 重定向到登录页（最旧会话已被自动撤销）
```

### 修复建议
- 默认限制 10 个并发 Session（`MAX_SESSIONS_PER_USER = 10`）
- 提供 Session 管理界面
- 新登录时通知其他 Session
- 敏感操作可强制单 Session

---

## 检查清单

| # | 场景 | 状态 | 测试日期 | 测试人员 | 发现问题 |
|---|------|------|----------|----------|----------|
| 1 | Session ID 安全性 | ☐ | | | |
| 2 | Session 固定攻击 | ☐ | | | |
| 3 | Session 劫持防护 | ☐ | | | |
| 4 | 并发 Session 控制 | ☐ | | | |

---

## 推荐 Session 配置

| 配置项 | 推荐值 | 说明 |
|-------|-------|------|
| Session ID 长度 | >= 128 bits | CSPRNG 生成 |
| HttpOnly | true | 防止 XSS 窃取 |
| Secure | true | 仅 HTTPS |
| SameSite | Strict/Lax | 防止 CSRF |
| 空闲超时 | 8 小时 (28800s) | Portal session 超时（`session.server.ts`）|
| 绝对超时 | 8-24 小时 | 最大生存期 |
| 并发限制 | 10 | 每用户最大 Session（`session.rs:MAX_SESSIONS_PER_USER`）|

---

## 参考资料

- [OWASP Session Management](https://cheatsheetseries.owasp.org/cheatsheets/Session_Management_Cheat_Sheet.html)
- [CWE-384: Session Fixation](https://cwe.mitre.org/data/definitions/384.html)
- [CWE-613: Insufficient Session Expiration](https://cwe.mitre.org/data/definitions/613.html)

---


---

## 标准化回归 Checklist（ASVS 5.0）

**矩阵ID**: M-SESS-01  
**适用控制**: V7.1,V7.2,V7.3  
**关联任务**: Backlog #8, #20  
**建议回归频率**: 每次发布前 + 缺陷修复后必跑  
**场景总数**: 4

### 执行清单
- [ ] M-SESS-01-C01 | 控制: V7.1 | 任务: #8, #20 | 动作: 执行文档内相关攻击步骤并记录证据
- [ ] M-SESS-01-C02 | 控制: V7.2 | 任务: #8, #20 | 动作: 执行文档内相关攻击步骤并记录证据
- [ ] M-SESS-01-C03 | 控制: V7.3 | 任务: #8, #20 | 动作: 执行文档内相关攻击步骤并记录证据

### 回归记录表
| 检查项ID | 执行结果(pass/fail) | 风险等级 | 证据（请求/响应/日志/截图） | 备注 |
|---|---|---|---|---|
|  |  |  |  |  |

### 退出准则
1. 所有检查项执行完成，且高风险项无 `fail`。
2. 如存在 `fail`，必须附带漏洞单号、修复计划和复测结论。
3. 回归报告需同时记录矩阵ID与 Backlog 任务号，便于跨版本追溯。
