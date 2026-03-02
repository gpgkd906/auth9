# 会话管理 - 登出安全测试

**模块**: 会话管理
**测试范围**: 登出流程安全性
**场景数**: 4
**风险等级**: 🟡 中
**ASVS 5.0 矩阵ID**: M-SESS-03
**OWASP ASVS 5.0**: V7.5,V7.2,V16.2
**回归任务映射**: Backlog #12, #20


---

## 背景知识

Auth9 登出场景：
- **Portal 登出**: 前端应用登出
- **SSO 登出**: 单点登出 (OIDC)
- **强制登出**: 管理员踢出用户
- **全局登出**: 撤销所有 Session

涉及的清理：
- Browser Cookie
- Keycloak Session
- Redis 缓存
- Token 黑名单

---

## 场景 1：完整登出验证

### 前置条件
- 有效的登录 Session

### 攻击目标
验证登出是否完整清理所有状态

### 攻击步骤
1. 登录并记录所有凭证
2. 执行登出
3. 尝试使用各种凭证：
   - Session Cookie
   - Access Token
   - Refresh Token
4. 检查服务端状态

### 预期安全行为
- 所有凭证失效
- Cookie 被清除
- Token 进入黑名单

### 验证方法
```bash
# 登录获取凭证
curl -c cookies.txt -X POST http://localhost:3000/login \
  -d '{"username":"test","password":"test123"}'

TOKEN=$(cat cookies.txt | grep access_token | awk '{print $7}')
REFRESH=$(cat cookies.txt | grep refresh_token | awk '{print $7}')
SESSION=$(cat cookies.txt | grep session | awk '{print $7}')

# 登出
curl -b cookies.txt -X POST http://localhost:8080/api/v1/auth/logout

# 尝试使用旧 Session
curl -b "session=$SESSION" http://localhost:3000/dashboard
# 预期: 重定向到登录

# 尝试使用旧 Access Token
curl -H "Authorization: Bearer $TOKEN" \
  http://localhost:8080/api/v1/users/me
# 预期: 401

# 尝试使用旧 Refresh Token
curl -X POST http://localhost:8080/api/v1/auth/refresh \
  -d "refresh_token=$REFRESH"
# 预期: 400 invalid_grant

# 检查服务端
redis-cli KEYS "*session*$SESSION*"
# 预期: 无匹配
```

### 修复建议
- 清除所有相关 Cookie
- Token 加入黑名单
- 删除 Redis Session
- 通知 Keycloak 登出

---

## 场景 2：OIDC 单点登出 (SLO)

### 前置条件
- 多个 OIDC 客户端登录

### 攻击目标
验证单点登出是否影响所有客户端

### 攻击步骤
1. 同一用户登录多个应用
2. 在一个应用登出
3. 检查其他应用的会话状态
4. 测试 front-channel 和 back-channel SLO

### 预期安全行为
- 单点登出影响所有应用
- back-channel 通知其他应用
- front-channel 重定向清理

### 验证方法
```bash
# 用户在 App A 登录
curl -c appA.txt -L http://localhost:3000/login

# 同用户在 App B 登录 (如果有)
curl -c appB.txt -L http://localhost:4000/login

# 在 App A 登出
curl -b appA.txt -X POST http://localhost:3000/logout

# 检查 App B 的 Session
curl -b appB.txt http://localhost:4000/dashboard
# 如果启用 SLO，应该要求重新登录

# 检查 Keycloak Session
# Admin API 查询用户 Session
curl -H "Authorization: Bearer $ADMIN_TOKEN" \
  http://localhost:8081/admin/realms/auth9/users/{user_id}/sessions
# 预期: 无活跃 Session
```

### 修复建议
- 实现 back-channel logout
- 配置 logout_uri
- 监听 Keycloak 登出事件
- 清理所有关联 Session

---

## 场景 3：强制登出机制

### 前置条件
- Platform Admin 权限（`config.platform_admin_emails` 中的邮箱）
- 目标用户活跃 Session
- **Token 类型**: Identity Token 或 Tenant Access Token 均可（`/api/v1/admin/` 路径已加入 identity token 白名单）

### 攻击目标
验证管理员强制登出功能

### 攻击步骤
1. 用户正常登录
2. 管理员使用 Platform Admin token 执行强制登出
3. 检查用户 Session 状态
4. 验证用户需要重新登录

### 预期安全行为
- 管理员可踢出任意用户
- 用户 Session 立即失效
- 用户收到通知 (可选)

### 验证方法

**方法 A: 通过 Portal UI 测试**
1. 以管理员 (admin@auth9.local) 登录 Portal
2. 导航到 Users 页面
3. 点击目标用户行的 "Open menu" 按钮
4. 点击 "Force Logout" 菜单项
5. 在确认对话框中点击 "Force Logout"
6. 预期: UI 操作成功完成，无错误提示

**方法 B: 通过 API 测试**
```bash
# 生成管理员 Token
ADMIN_TOKEN=$(node .claude/skills/tools/gen-test-tokens.js platform-admin)

# 查找目标用户 ID
USER_ID=$(mysql -h 127.0.0.1 -P 4000 -u root auth9 -N -e "SELECT id FROM users WHERE email != 'admin@auth9.local' LIMIT 1;")

# 管理员强制登出 (正确的 API 端点)
curl -s -w "\nHTTP: %{http_code}" -X POST \
  -H "Authorization: Bearer $ADMIN_TOKEN" \
  http://localhost:8080/api/v1/admin/users/${USER_ID}/logout
# 预期: 200 {"data":{"revoked_count": N}}

# 检查数据库 - 所有 Session 应已撤销
mysql -h 127.0.0.1 -P 4000 -u root auth9 -e \
  "SELECT id, revoked_at FROM sessions WHERE user_id = '${USER_ID}' AND revoked_at IS NULL;"
# 预期: 空结果 (无活跃 Session)
```

### 修复建议
- ✅ 仅管理员可强制登出 (已实现，`SessionForceLogout` 策略)
- ✅ 记录审计日志 (已实现)
- 可选: 通知被踢用户
- ✅ 立即生效无延迟 (已实现，Token 黑名单)

---

## 场景 4：登出后的浏览器缓存

### 前置条件
- 浏览器访问

### 攻击目标
验证登出后浏览器缓存是否安全

### 攻击步骤
1. 登录并访问敏感页面
2. 登出
3. 使用浏览器后退按钮
4. 检查缓存的页面内容

### 预期安全行为
- 敏感页面不缓存
- 后退时要求重新认证
- 显示已登出状态

### 验证方法
```bash
# 检查响应头
curl -I -H "Authorization: Bearer $TOKEN" \
  http://localhost:3000/dashboard

# 期望的头:
# Cache-Control: no-store, no-cache, must-revalidate, private
# Pragma: no-cache
# Expires: 0

# 浏览器测试
# 1. 登录
# 2. 访问 /dashboard
# 3. 登出
# 4. 点击后退按钮
# 5. 观察是否显示缓存内容
```

### 修复建议
- 敏感页面: `Cache-Control: no-store`
- 设置 `Pragma: no-cache`
- 前端检测登出状态
- 强制刷新敏感数据

---

## 检查清单

| # | 场景 | 状态 | 测试日期 | 测试人员 | 发现问题 |
|---|------|------|----------|----------|----------|
| 1 | 完整登出验证 | ☐ | | | |
| 2 | OIDC 单点登出 | ☐ | | | |
| 3 | 强制登出机制 | ☐ | | | |
| 4 | 登出后浏览器缓存 | ☐ | | | |

---

## 登出流程清单

登出时需要清理:

| 项目 | 位置 | 清理方式 |
|-----|------|---------|
| Session Cookie | 浏览器 | Set-Cookie: session=; Max-Age=0 |
| Access Token | 浏览器/内存 | Token 黑名单 + 清除 |
| Refresh Token | 安全存储 | 数据库标记撤销 |
| Keycloak Session | Keycloak | Admin API 撤销 |
| Redis 缓存 | Redis | DEL session:xxx |
| 本地存储 | localStorage | 前端 clear() |

---

## 参考资料

- [OIDC RP-Initiated Logout](https://openid.net/specs/openid-connect-rpinitiated-1_0.html)
- [OIDC Back-Channel Logout](https://openid.net/specs/openid-connect-backchannel-1_0.html)
- [OWASP Logout Best Practices](https://cheatsheetseries.owasp.org/cheatsheets/Session_Management_Cheat_Sheet.html#logout)

---


---

## 标准化回归 Checklist（ASVS 5.0）

**矩阵ID**: M-SESS-03  
**适用控制**: V7.5,V7.2,V16.2  
**关联任务**: Backlog #12, #20  
**建议回归频率**: 每次发布前 + 缺陷修复后必跑  
**场景总数**: 4

### 执行清单
- [ ] M-SESS-03-C01 | 控制: V7.5 | 任务: #12, #20 | 动作: 执行文档内相关攻击步骤并记录证据
- [ ] M-SESS-03-C02 | 控制: V7.2 | 任务: #12, #20 | 动作: 执行文档内相关攻击步骤并记录证据
- [ ] M-SESS-03-C03 | 控制: V16.2 | 任务: #12, #20 | 动作: 执行文档内相关攻击步骤并记录证据

### 回归记录表
| 检查项ID | 执行结果(pass/fail) | 风险等级 | 证据（请求/响应/日志/截图） | 备注 |
|---|---|---|---|---|
|  |  |  |  |  |

### 退出准则
1. 所有检查项执行完成，且高风险项无 `fail`。
2. 如存在 `fail`，必须附带漏洞单号、修复计划和复测结论。
3. 回归报告需同时记录矩阵ID与 Backlog 任务号，便于跨版本追溯。
