# 认证安全 - OIDC 流程安全测试

**模块**: 认证安全
**测试范围**: OIDC 协议实现安全性
**场景数**: 5
**风险等级**: 🟠 高
**ASVS 5.0 矩阵ID**: M-AUTH-01
**OWASP ASVS 5.0**: V10.1,V10.2,V10.3,V10.4
**回归任务映射**: Backlog #4, #20


---

## 背景知识

Auth9 使用 Keycloak 作为 OIDC Provider，关键端点：
- `/.well-known/openid-configuration` - OIDC 发现端点
- `/api/v1/auth/authorize` - 授权入口
- `/api/v1/auth/callback` - 回调处理
- `/api/v1/auth/token` - Token 端点

---

## 场景 1：Authorization Code 截获攻击

### 前置条件
- 正常用户账户
- 能够拦截 HTTP 流量

### 攻击目标
验证 Authorization Code 是否可被重放或截获利用

### 攻击步骤
1. 启动 Burp Suite 代理
2. 正常进行 OIDC 登录流程
3. 截获回调 URL 中的 `code` 参数
4. 尝试使用同一 `code` 多次请求 `/api/v1/auth/token`
5. 尝试在不同会话中使用截获的 `code`

### 预期安全行为
- Authorization Code 应仅能使用一次
- Code 应绑定到特定 session/client
- 重放攻击应返回 400 错误

### 验证方法
```bash
# 第一次请求应成功
curl -X POST http://localhost:8080/api/v1/auth/token \
  -d "grant_type=authorization_code" \
  -d "code={captured_code}" \
  -d "client_id=auth9-portal" \
  -d "redirect_uri=http://localhost:3000/callback"

# 第二次请求应失败
# 预期响应: {"error": "invalid_grant", "error_description": "Code has expired or already been used"}
```

### 修复建议
- 确保 Code 一次性使用 (RFC 6749 Section 4.1.2)
- 实现 PKCE (RFC 7636)
- Code 有效期不超过 10 分钟

---

## 场景 2：Redirect URI 验证绕过

### 前置条件
- 已注册的客户端 (如 auth9-portal)

### 攻击目标
验证 redirect_uri 白名单是否可被绕过

### 攻击步骤
1. 获取合法的 redirect_uri (如 `http://localhost:3000/callback`)
2. 尝试以下变体：
   - `http://localhost:3000/callback/../evil`
   - `http://localhost:3000/callback?evil=param`
   - `http://localhost:3000/callback#evil`
   - `http://localhost:3000.attacker.com/callback`
   - `http://localhost:3000@attacker.com/callback`
   - `http://localhost:3000%00.attacker.com/callback`
3. 构造恶意授权请求

### 预期安全行为
- 仅精确匹配白名单中的 URI
- 拒绝任何变体或编码绕过
- 返回 `invalid_redirect_uri` 错误

### 验证方法
```bash
# 正常请求
curl -v "http://localhost:8080/api/v1/auth/authorize?\
client_id=auth9-portal&\
redirect_uri=http://localhost:3000/callback&\
response_type=code&\
scope=openid"
# 预期: 302 重定向到 Keycloak

# 恶意请求
curl -v "http://localhost:8080/api/v1/auth/authorize?\
client_id=auth9-portal&\
redirect_uri=http://attacker.com/callback&\
response_type=code&\
scope=openid"
# 预期: 400 invalid_redirect_uri
```

### 修复建议
- 严格精确匹配 redirect_uri
- 禁止通配符匹配
- URL 规范化后再比较

---

## 场景 3：State 参数 CSRF 防护

### 前置条件
- 正常用户会话

### 攻击目标
验证 state 参数是否有效防护 CSRF 攻击

### 攻击步骤
1. 记录正常登录流程中的 state 值
2. 尝试以下攻击：
   - 不带 state 参数发起授权请求
   - 使用固定/可预测的 state 值
   - 使用他人的 state 值
   - 修改回调中的 state 值
3. 检查系统响应

### 预期安全行为
- 缺少 state 参数应返回错误
- **服务端生成的 OIDC state nonce（UUID v4）应为随机不可预测值**
- state 应绑定用户会话（通过 Redis 存储与 GETDEL 消费）
- 回调时验证 state 一致性

> **实现说明**: Auth9 采用服务端 state 管理模式。客户端传入的 `state` 参数仅作为
> `original_state` 存入 Redis payload 并在回调后原样回传。实际发送给 Keycloak 的
> `state` 是服务端生成的 UUID v4 nonce，具备足够的随机性和不可预测性。
> Redis GETDEL 确保每个 state nonce 只能使用一次（防重放），TTL 为 300 秒。
> 因此，即使客户端传入可预测的 state 值，也不影响 CSRF 防护的安全性。

### 验证方法
```bash
# 不带 state 的请求 - 应返回 400
curl -v "http://localhost:8080/api/v1/auth/authorize?\
client_id=auth9-portal&\
redirect_uri=http://localhost:3000/callback&\
response_type=code&\
scope=openid"

# 验证重定向 URL 中的 state 参数是 UUID v4 格式（服务端生成）
curl -v "http://localhost:8080/api/v1/auth/authorize?\
client_id=auth9-portal&\
redirect_uri=http://localhost:3000/auth/callback&\
response_type=code&\
scope=openid&\
state=test-csrf"
# 检查 Location header 中 state= 的值为 UUID v4（非客户端传入的值）

# 验证 state 不可重放 - 同一 state 值回调两次应失败
```

---

## 场景 4：Scope 权限扩大攻击

### 前置条件
- 客户端配置了有限的 scope 权限

### 攻击目标
验证是否可以请求超出授权范围的 scope

### 攻击步骤
1. 查看客户端允许的 scope 列表
2. 尝试请求额外的 scope：
   - `openid profile email admin`
   - `openid offline_access`
   - 自定义高权限 scope
3. 检查返回的 access_token 中的权限

### 预期安全行为
- 仅授予客户端预配置的 scope
- 忽略或拒绝未授权的 scope 请求
- Token 中不包含未授权的 scope

### 验证方法
```bash
# 请求超出范围的 scope
curl -v "http://localhost:8080/api/v1/auth/authorize?\
client_id=auth9-portal&\
redirect_uri=http://localhost:3000/callback&\
response_type=code&\
scope=openid+profile+email+admin+offline_access"

# 解析获得的 access_token
# 检查 scope claim 是否仅包含授权的值
```

### 修复建议
- 在客户端配置中限制允许的 scope
- 请求时过滤非法 scope
- 审计日志记录 scope 请求

---

## 场景 5：OIDC 元数据篡改

### 前置条件
- 能够访问 OIDC 发现端点

### 攻击目标
验证 OIDC 元数据端点的安全性

### 攻击步骤
1. 访问 `/.well-known/openid-configuration`
2. 检查返回的端点配置
3. 验证是否使用 HTTPS
4. 检查是否可以通过缓存投毒攻击

### 预期安全行为
- 生产环境所有端点应为 HTTPS
- 设置适当的缓存控制头
- issuer 与实际域名一致

### 验证方法
```bash
# 获取 OIDC 配置
curl http://localhost:8080/.well-known/openid-configuration | jq .

# 检查响应头
curl -I http://localhost:8080/.well-known/openid-configuration

# 验证 issuer 一致性
# issuer 应与访问域名匹配
```

### 修复建议
- 生产环境强制 HTTPS
- 设置 `Cache-Control: no-store`
- 验证 issuer 一致性
- 使用 HSTS 头

---

## 检查清单

| # | 场景 | 状态 | 测试日期 | 测试人员 | 发现问题 |
|---|------|------|----------|----------|----------|
| 1 | Authorization Code 截获攻击 | ☐ | | | |
| 2 | Redirect URI 验证绕过 | ☐ | | | |
| 3 | State 参数 CSRF 防护 | ☐ | | | |
| 4 | Scope 权限扩大攻击 | ☐ | | | |
| 5 | OIDC 元数据篡改 | ☐ | | | |

---

## 参考资料

- [RFC 6749 - OAuth 2.0](https://datatracker.ietf.org/doc/html/rfc6749)
- [RFC 7636 - PKCE](https://datatracker.ietf.org/doc/html/rfc7636)
- [OWASP OAuth Security](https://cheatsheetseries.owasp.org/cheatsheets/OAuth2_Cheat_Sheet.html)
- [CWE-601: URL Redirection](https://cwe.mitre.org/data/definitions/601.html)

---


---

## 标准化回归 Checklist（ASVS 5.0）

**矩阵ID**: M-AUTH-01  
**适用控制**: V10.1,V10.2,V10.3,V10.4  
**关联任务**: Backlog #4, #20  
**建议回归频率**: 每次发布前 + 缺陷修复后必跑  
**场景总数**: 5

### 执行清单
- [ ] M-AUTH-01-C01 | 控制: V10.1 | 任务: #4, #20 | 动作: 执行文档内相关攻击步骤并记录证据
- [ ] M-AUTH-01-C02 | 控制: V10.2 | 任务: #4, #20 | 动作: 执行文档内相关攻击步骤并记录证据
- [ ] M-AUTH-01-C03 | 控制: V10.3 | 任务: #4, #20 | 动作: 执行文档内相关攻击步骤并记录证据
- [ ] M-AUTH-01-C04 | 控制: V10.4 | 任务: #4, #20 | 动作: 执行文档内相关攻击步骤并记录证据

### 回归记录表
| 检查项ID | 执行结果(pass/fail) | 风险等级 | 证据（请求/响应/日志/截图） | 备注 |
|---|---|---|---|---|
|  |  |  |  |  |

### 退出准则
1. 所有检查项执行完成，且高风险项无 `fail`。
2. 如存在 `fail`，必须附带漏洞单号、修复计划和复测结论。
3. 回归报告需同时记录矩阵ID与 Backlog 任务号，便于跨版本追溯。
