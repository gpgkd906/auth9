# 会话管理 - 登出安全测试

**模块**: 会话管理
**测试范围**: 登出流程安全性
**场景数**: 4
**风险等级**: 🟡 中

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
- 管理员权限
- 目标用户活跃 Session

### 攻击目标
验证管理员强制登出功能

### 攻击步骤
1. 用户正常登录
2. 管理员执行强制登出
3. 检查用户 Session 状态
4. 验证用户需要重新登录

### 预期安全行为
- 管理员可踢出任意用户
- 用户 Session 立即失效
- 用户收到通知 (可选)

### 验证方法
```bash
# 用户登录
curl -c user.txt -X POST http://localhost:3000/login \
  -d '{"username":"victim","password":"pass123"}'

# 验证用户 Session 有效
curl -b user.txt http://localhost:3000/dashboard
# 预期: 200

# 管理员强制登出
curl -X POST -H "Authorization: Bearer $ADMIN_TOKEN" \
  http://localhost:8080/api/v1/users/{victim_id}/sessions/revoke-all

# 用户尝试访问
curl -b user.txt http://localhost:3000/dashboard
# 预期: 重定向到登录

# 检查数据库
SELECT * FROM sessions WHERE user_id = 'victim_id' AND revoked_at IS NULL;
# 预期: 无记录
```

### 修复建议
- 仅管理员可强制登出
- 记录审计日志
- 可选: 通知被踢用户
- 立即生效无延迟

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
