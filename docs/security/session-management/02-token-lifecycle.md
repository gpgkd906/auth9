# 会话管理 - Token 生命周期测试

**模块**: 会话管理
**测试范围**: JWT Token 生命周期管理
**场景数**: 5
**风险等级**: 🟠 高
**ASVS 5.0 矩阵ID**: M-SESS-02
**OWASP ASVS 5.0**: V7.2,V7.4,V9.1
**回归任务映射**: Backlog #1, #4, #11, #20


---

## 背景知识

Auth9 Token 类型：
- **Identity Token**: 用户身份，较长有效期 (1-8 小时)
- **Tenant Access Token**: 租户访问，较短有效期 (15-60 分钟)
- **Refresh Token**: 刷新令牌，长期有效 (7-30 天)

Token 流程：
1. 用户登录 → 获得 Identity Token + Refresh Token
2. Token Exchange → 获得 Tenant Access Token
3. Token 过期 → 使用 Refresh Token 刷新

---

## 场景 1：Token 过期验证

### 前置条件
- 有效的 Token

### 攻击目标
验证 Token 过期机制是否正确实现

### 攻击步骤
1. 获取 Token 并记录过期时间
2. 在过期前后测试 Token
3. 检查服务器时间同步
4. 测试时间偏移容忍度

### 预期安全行为
- 过期 Token 立即失效
- 不接受过期较久的 Token
- 合理的时钟偏移容忍 (< 5秒)

### 验证方法
```bash
# 获取 Token 并解析过期时间
TOKEN=$(get_access_token)
EXP=$(echo $TOKEN | cut -d'.' -f2 | base64 -d 2>/dev/null | jq .exp)
echo "Token expires at: $(date -d @$EXP)"

# 计算剩余时间
NOW=$(date +%s)
REMAINING=$((EXP - NOW))
echo "Remaining: $REMAINING seconds"

# 等待过期
sleep $((REMAINING + 1))

# 使用过期 Token
curl -H "Authorization: Bearer $TOKEN" \
  http://localhost:8080/api/v1/users
# 预期: 401 {"error": "token_expired"}

# 测试刚过期 (1秒)
curl -H "Authorization: Bearer $TOKEN" \
  http://localhost:8080/api/v1/users
# 预期: 401

# 测试服务器时钟
curl http://localhost:8080/api/v1/time
# 比较服务器时间与本地时间
```

### 修复建议
- 精确到秒的过期验证
- 时钟偏移容忍 <= 30 秒
- 使用 NTP 同步时间
- 在错误中说明过期

---

## 场景 2：Refresh Token 安全

### 前置条件
- 有效的 Refresh Token

### 攻击目标
验证 Refresh Token 机制的安全性

### 攻击步骤
1. 测试 Refresh Token 重用：
   - 使用后是否仍有效
   - Token Rotation 是否实现
2. 测试 Refresh Token 泄露检测
3. 测试 Refresh Token 撤销

### 预期安全行为
- Refresh Token 一次性使用 (Rotation)
- 检测异常刷新模式
- 支持撤销 Refresh Token

### 验证方法
```bash
# 获取 Refresh Token
REFRESH_TOKEN=$(get_refresh_token)

# 第一次刷新
curl -X POST http://localhost:8080/api/v1/auth/refresh \
  -d "refresh_token=$REFRESH_TOKEN" | jq .
# 记录新的 access_token 和 refresh_token

# 第二次使用相同 Refresh Token
curl -X POST http://localhost:8080/api/v1/auth/refresh \
  -d "refresh_token=$REFRESH_TOKEN"
# 预期: 400 {"error": "invalid_grant"} (Token Rotation)
# 或: 成功但返回相同 token (无 Rotation)

# 检测泄露
# 如果旧 Refresh Token 被使用，应该:
# 1. 失败
# 2. 吊销所有相关 Token
# 3. 通知用户

# 撤销 Refresh Token
curl -X POST http://localhost:8080/api/v1/auth/revoke \
  -d "token=$REFRESH_TOKEN"
```

### 修复建议
- 实现 Refresh Token Rotation
- 检测重用攻击
- 重用时吊销整个 Token 家族
- Refresh Token 绑定设备

---

## 场景 3：Token 黑名单

### 前置条件
- 有效的 Identity Token（通过 Keycloak 登录获取）
- 已通过 Token Exchange 获取 Tenant Access Token

### 攻击目标
验证 Token 吊销/黑名单机制 - 登出后所有 Token（Identity 和 Tenant Access）立即失效

### 实现说明

Auth9 使用 `sid`（Session ID）作为黑名单键：
- Identity Token 包含 `sid` 字段（来自 Keycloak 会话）
- Tenant Access Token 继承 Identity Token 的 `sid`（通过 Token Exchange 传播）
- 登出时，`sid` 被加入 Redis 黑名单，所有同一会话的 Token 立即失效
- 黑名单 TTL = Token 剩余有效期（自动清理）

**关键**: `POST /api/v1/auth/logout` 需要 **Bearer Token**（Identity Token），GET 版本仅做重定向不执行撤销。

### 攻击步骤
1. 通过 Keycloak 登录获取 Identity Token
2. 通过 Token Exchange 获取 Tenant Access Token
3. 验证两种 Token 都有效（200 OK）
4. 调用 `POST /api/v1/auth/logout`（带 Bearer Token）
5. 使用已登出的 Identity Token 和 Tenant Access Token 再次访问 API

### 预期安全行为
- `POST /api/v1/auth/logout` 返回 302 重定向到 Keycloak（同时完成 session 撤销）
- 登出后 Identity Token 访问返回 401 `"Token has been revoked"`
- 登出后 Tenant Access Token 访问返回 401 `"Token has been revoked"`
- Redis 黑名单高效检查（fail-closed: Redis 不可用时返回 503）
- TTL 与 Token 剩余过期时间一致

### 验证方法
```bash
# 1. 获取 Identity Token（通过 Keycloak 登录流程）
IDENTITY_TOKEN=$(... # 通过 OIDC 登录获取)

# 2. Token Exchange 获取 Tenant Access Token
TENANT_TOKEN=$(curl -s -X POST http://localhost:8080/api/v1/auth/tenant-token \
  -H "Authorization: Bearer $IDENTITY_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"tenant_id":"<tenant-id>","service_id":"auth9-portal"}' | jq -r .access_token)

# 3. 验证 Tenant Access Token 有效
curl -H "Authorization: Bearer $TENANT_TOKEN" \
  http://localhost:8080/api/v1/users/me
# 预期: 200

# 4. 登出（必须用 POST + Bearer Token）
curl -X POST -H "Authorization: Bearer $IDENTITY_TOKEN" \
  http://localhost:8080/api/v1/auth/logout
# 预期: 302 重定向到 Keycloak

# 5. 使用已登出的 Tenant Access Token（应失败）
curl -H "Authorization: Bearer $TENANT_TOKEN" \
  http://localhost:8080/api/v1/users/me
# 预期: 401 {"error": "Token has been revoked"}

# 6. 检查 Redis 黑名单
docker exec auth9-redis redis-cli KEYS "auth9:token_blacklist:*"
```

### 故障排除

| 症状 | 原因 | 解决方法 |
|------|------|----------|
| 登出后 Token 仍有效 (200) | 使用了 GET /logout（仅重定向） | 改用 POST /api/v1/auth/logout 并携带 Bearer Token |
| Tenant Access Token 登出后仍有效 | Token 缺少 sid 字段（旧版本 Token） | 重新通过 Token Exchange 获取新 Token（新版本包含 sid） |
| 登出返回 401 | Token 已过期 | 使用有效的 Identity Token 登出 |
| Redis 503 错误 | Redis 不可用（fail-closed 设计） | 检查 Redis 容器健康状态 |

---

## 场景 4：Token 范围限制

### 前置条件
- 不同 scope 的 Token

### 攻击目标
验证 Token scope 是否正确限制

### 攻击步骤
1. 获取限制 scope 的 Token
2. 尝试访问超出 scope 的资源
3. 检查 scope 验证实现

### 预期安全行为
- 仅允许 scope 内的操作
- 明确的错误信息
- scope 不可篡改

### 验证方法
```bash
# 获取 read-only scope 的 Token
TOKEN=$(get_token_with_scope "read")

# 尝试写操作
curl -X POST -H "Authorization: Bearer $TOKEN" \
  http://localhost:8080/api/v1/users \
  -d '{"email":"test@example.com"}'
# 预期: 403 {"error": "insufficient_scope", "required": "write"}

# 解析 Token 检查 scope
echo $TOKEN | cut -d'.' -f2 | base64 -d | jq .scope

# 尝试篡改 scope (应失败因为签名验证)
```

### 修复建议
- 每个端点定义所需 scope
- 验证时检查 scope claim
- 明确返回所需 scope
- 最小权限原则

---

## 场景 5：Token 绑定（路线图 — 当前未实现）

> **状态**: 此场景描述的是**未来规划功能**，当前版本未实现 Token 绑定或 DPoP。测试结果为"未绑定"属于预期行为，不应作为 bug 提交。

### 前置条件
- 有效的 Token
- 多个客户端环境

### 攻击目标
验证 Token 是否绑定到特定上下文（当前未实现，仅做基线记录）

### 攻击步骤
1. 获取 Token (记录 IP/设备)
2. 从不同 IP/设备使用 Token
3. 检查绑定验证
4. 测试 DPoP (如果实现)

### 预期安全行为
- 可选的 Token 绑定
- 检测异常使用
- DPoP 防止 Token 盗用

### 验证方法
```bash
# 从原始 IP 获取 Token
TOKEN=$(curl -X POST http://localhost:8080/api/v1/auth/token ...)

# 从不同 IP 使用
curl -H "Authorization: Bearer $TOKEN" \
  -H "X-Forwarded-For: 1.2.3.4" \
  http://localhost:8080/api/v1/users/me
# 检查是否有告警或拒绝

# 检查 Token 中的绑定信息
echo $TOKEN | cut -d'.' -f2 | base64 -d | jq .
# 可能包含: client_ip, user_agent, fingerprint

# DPoP 测试 (如果支持)
# 需要生成 DPoP Proof
```

### 修复建议
- 高安全场景启用 Token 绑定
- 实现 DPoP (RFC 9449)
- 检测并告警异常使用
- 绑定松紧度可配置

---

## 检查清单

| # | 场景 | 状态 | 测试日期 | 测试人员 | 发现问题 |
|---|------|------|----------|----------|----------|
| 1 | Token 过期验证 | ☐ | | | |
| 2 | Refresh Token 安全 | ☐ | | | |
| 3 | Token 黑名单 | ☐ | | | |
| 4 | Token 范围限制 | ☐ | | | |
| 5 | Token 绑定 | ☐ | | | |

---

## 推荐 Token 配置

| Token 类型 | 有效期 | 存储 | 刷新策略 |
|-----------|-------|------|---------|
| Identity Token | 1-8 小时 | 内存/Cookie | Refresh Token |
| Access Token | 15-60 分钟 | 内存 | Refresh Token |
| Refresh Token | 7-30 天 | 安全存储 | Rotation |

---

## 参考资料

- [RFC 6749 - OAuth 2.0](https://datatracker.ietf.org/doc/html/rfc6749)
- [RFC 7009 - Token Revocation](https://datatracker.ietf.org/doc/html/rfc7009)
- [RFC 9449 - DPoP](https://datatracker.ietf.org/doc/html/rfc9449)
- [OWASP JWT Security](https://cheatsheetseries.owasp.org/cheatsheets/JSON_Web_Token_for_Java_Cheat_Sheet.html)

---


---

## 标准化回归 Checklist（ASVS 5.0）

**矩阵ID**: M-SESS-02  
**适用控制**: V7.2,V7.4,V9.1  
**关联任务**: Backlog #1, #4, #11, #20  
**建议回归频率**: 每次发布前 + 缺陷修复后必跑  
**场景总数**: 5

### 执行清单
- [ ] M-SESS-02-C01 | 控制: V7.2 | 任务: #1, #4, #11, #20 | 动作: 执行文档内相关攻击步骤并记录证据
- [ ] M-SESS-02-C02 | 控制: V7.4 | 任务: #1, #4, #11, #20 | 动作: 执行文档内相关攻击步骤并记录证据
- [ ] M-SESS-02-C03 | 控制: V9.1 | 任务: #1, #4, #11, #20 | 动作: 执行文档内相关攻击步骤并记录证据

### 回归记录表
| 检查项ID | 执行结果(pass/fail) | 风险等级 | 证据（请求/响应/日志/截图） | 备注 |
|---|---|---|---|---|
|  |  |  |  |  |

### 退出准则
1. 所有检查项执行完成，且高风险项无 `fail`。
2. 如存在 `fail`，必须附带漏洞单号、修复计划和复测结论。
3. 回归报告需同时记录矩阵ID与 Backlog 任务号，便于跨版本追溯。
