# 输入验证 - CSRF 攻击测试

**模块**: 输入验证
**测试范围**: 跨站请求伪造防护
**场景数**: 5
**风险等级**: 🟠 高
**ASVS 5.0 矩阵ID**: M-INPUT-03
**OWASP ASVS 5.0**: V3.3,V7.1,V10.2
**回归任务映射**: Backlog #8, #20


---

## 背景知识

Auth9 CSRF 防护机制：
- OIDC state 参数
- SameSite Cookie 属性
- CSRF Token (表单)
- JWT Bearer Token (API)

高风险操作：
- 账户设置修改
- 角色/权限变更
- 密码修改
- Token 管理

---

## 场景 1：OIDC 登录 CSRF

### 前置条件
- 目标用户已登录
- 攻击者控制恶意网站

### 攻击目标
验证 OIDC 登录流程是否防护 CSRF

### 攻击步骤
1. 在恶意网站构造登录请求：
   ```html
   <a href="http://localhost:8080/api/v1/auth/authorize?
     client_id=auth9-portal&
     redirect_uri=http://attacker.com/callback&
     response_type=code">Login</a>
   ```
2. 诱导用户点击
3. 检查 state 参数验证

### 预期安全行为
- 验证 redirect_uri 白名单
- 验证 state 参数
- 拒绝未授权的回调地址

### 验证方法
```bash
# 不带 state 的授权请求
curl -v "http://localhost:8080/api/v1/auth/authorize?\
client_id=auth9-portal&\
redirect_uri=http://localhost:3000/callback&\
response_type=code"
# 检查是否强制要求 state

# 恶意 redirect_uri
curl -v "http://localhost:8080/api/v1/auth/authorize?\
client_id=auth9-portal&\
redirect_uri=http://attacker.com/callback&\
response_type=code&\
state=random"
# 预期: 400 Invalid redirect_uri
```

### 修复建议
- 强制 state 参数
- 严格 redirect_uri 白名单
- state 绑定会话
- 使用 PKCE (code_verifier)

---

## 场景 2：敏感操作 CSRF

### 前置条件
- 用户已登录
- 攻击者了解 API 结构

### 攻击目标
验证敏感操作是否防护 CSRF

### 攻击步骤
1. 构造恶意页面：
   ```html
   <form action="http://localhost:8080/api/v1/users/me/password" method="POST">
     <input name="new_password" value="hacked123">
     <input type="submit">
   </form>
   <script>document.forms[0].submit();</script>
   ```
2. 诱导已登录用户访问
3. 检查操作是否执行

### 预期安全行为
- 需要 CSRF Token 或
- 需要 Bearer Token (不自动携带)
- 操作被拒绝

### 验证方法
```bash
# 模拟 CSRF 攻击 (不带 Authorization header)
curl -X PUT http://localhost:8080/api/v1/users/me \
  -H "Content-Type: application/json" \
  -H "Cookie: session=valid_session_cookie" \
  -d '{"display_name": "hacked"}'
# 预期: 401 (如果使用 Bearer Token)
# 或 403 (如果使用 CSRF Token 且缺失)

# 检查实际保护机制
# 1. API 是否接受 Cookie 认证
# 2. 是否有 CSRF Token 要求
```

### 修复建议
- API 仅接受 Bearer Token
- 禁用 Cookie 认证的状态修改
- 敏感操作要求 CSRF Token
- 设置 SameSite=Strict

---

## 场景 3：Cookie SameSite 配置

### 前置条件
- 系统使用 Cookie

### 攻击目标
验证 Cookie 的 SameSite 属性配置

### 攻击步骤
1. 检查所有设置的 Cookie
2. 验证 SameSite 属性
3. 测试跨站请求是否携带 Cookie

### 预期安全行为
- Session Cookie: SameSite=Strict 或 Lax
- 认证 Cookie 设置 Secure 和 HttpOnly

### 验证方法
```bash
# 登录获取 Cookie
curl -c cookies.txt -X POST http://localhost:8080/api/v1/auth/login \
  -d '{"username":"test","password":"test123"}'

# 检查 Cookie 属性
cat cookies.txt
# 查看 SameSite 设置

# 通过浏览器开发者工具
# Application -> Cookies -> 检查每个 Cookie 的属性
```

### 修复建议
- Session: `SameSite=Strict; Secure; HttpOnly`
- 必要的跨站 Cookie: `SameSite=Lax`
- 所有认证 Cookie: `Secure; HttpOnly`
- 避免使用 `SameSite=None`

---

## 场景 4：JSON API CSRF

### 前置条件
- API 使用 JSON 格式

### 攻击目标
验证 JSON API 是否可被 CSRF 攻击

### 攻击步骤
1. 尝试通过 HTML form 发送 JSON：
   ```html
   <form action="http://localhost:8080/api/v1/users" method="POST"
         enctype="text/plain">
     <input name='{"email":"attacker@evil.com","password":"test123"}'
            value=''>
   </form>
   ```
2. 检查服务器是否接受

### 预期安全行为
- 严格验证 Content-Type
- 拒绝非 application/json 请求
- 要求 Bearer Token

### 验证方法
```bash
# 尝试 text/plain Content-Type
curl -X POST http://localhost:8080/api/v1/users \
  -H "Content-Type: text/plain" \
  -H "Cookie: session=valid" \
  -d '{"email":"test@example.com"}'
# 预期: 400 或 415 Unsupported Media Type

# 尝试 application/x-www-form-urlencoded
curl -X POST http://localhost:8080/api/v1/users \
  -H "Content-Type: application/x-www-form-urlencoded" \
  -H "Cookie: session=valid" \
  -d 'email=test@example.com'
# 预期: 400 或 415
```

### 修复建议
- 严格验证 Content-Type
- 仅接受 application/json
- 使用 Bearer Token 认证
- 添加自定义请求头验证

---

## 场景 5：登出 CSRF

### 前置条件
- 用户已登录

### 攻击目标
验证登出是否可被 CSRF 触发

### 攻击步骤
1. 构造恶意页面：
   ```html
   <img src="http://localhost:8080/api/v1/auth/logout">
   <!-- 或 -->
   <iframe src="http://localhost:8080/api/v1/auth/logout"></iframe>
   ```
2. 诱导用户访问
3. 检查是否被登出

### 预期安全行为
- 登出需要 POST 请求
- 或需要确认
- GET 请求不执行登出

### 验证方法
```bash
# GET 请求登出
curl -X GET http://localhost:8080/api/v1/auth/logout \
  -H "Cookie: session=valid"
# 预期: 405 Method Not Allowed 或 不执行登出

# POST 请求登出
curl -X POST http://localhost:8080/api/v1/auth/logout \
  -H "Cookie: session=valid"
# 检查是否需要 CSRF Token
```

### 修复建议
- 登出仅接受 POST
- 考虑 CSRF Token 保护
- 或使用 Bearer Token
- 登出后清除所有会话

---

## 检查清单

| # | 场景 | 状态 | 测试日期 | 测试人员 | 发现问题 |
|---|------|------|----------|----------|----------|
| 1 | OIDC 登录 CSRF | ☐ | | | |
| 2 | 敏感操作 CSRF | ☐ | | | |
| 3 | Cookie SameSite 配置 | ☐ | | | |
| 4 | JSON API CSRF | ☐ | | | |
| 5 | 登出 CSRF | ☐ | | | |

---

## CSRF 测试 HTML 模板

```html
<!DOCTYPE html>
<html>
<head><title>CSRF PoC</title></head>
<body>
  <h1>CSRF Test Page</h1>

  <!-- Form-based CSRF -->
  <form id="csrf-form" action="http://target/api/endpoint" method="POST">
    <input type="hidden" name="param1" value="value1">
    <input type="hidden" name="param2" value="value2">
  </form>

  <!-- Auto-submit -->
  <script>
    // document.getElementById('csrf-form').submit();
  </script>

  <!-- Image-based (GET only) -->
  <img src="http://target/api/logout" style="display:none">

  <!-- XHR-based (blocked by CORS) -->
  <script>
    var xhr = new XMLHttpRequest();
    xhr.open('POST', 'http://target/api/endpoint', true);
    xhr.withCredentials = true;
    xhr.setRequestHeader('Content-Type', 'application/json');
    // xhr.send(JSON.stringify({key: 'value'}));
  </script>
</body>
</html>
```

---

## 参考资料

- [OWASP CSRF Prevention](https://cheatsheetseries.owasp.org/cheatsheets/Cross-Site_Request_Forgery_Prevention_Cheat_Sheet.html)
- [SameSite Cookies Explained](https://web.dev/samesite-cookies-explained/)
- [CWE-352: Cross-Site Request Forgery](https://cwe.mitre.org/data/definitions/352.html)
- [PortSwigger CSRF](https://portswigger.net/web-security/csrf)

---


---

## 标准化回归 Checklist（ASVS 5.0）

**矩阵ID**: M-INPUT-03  
**适用控制**: V3.3,V7.1,V10.2  
**关联任务**: Backlog #8, #20  
**建议回归频率**: 每次发布前 + 缺陷修复后必跑  
**场景总数**: 5

### 执行清单
- [ ] M-INPUT-03-C01 | 控制: V3.3 | 任务: #8, #20 | 动作: 执行文档内相关攻击步骤并记录证据
- [ ] M-INPUT-03-C02 | 控制: V7.1 | 任务: #8, #20 | 动作: 执行文档内相关攻击步骤并记录证据
- [ ] M-INPUT-03-C03 | 控制: V10.2 | 任务: #8, #20 | 动作: 执行文档内相关攻击步骤并记录证据

### 回归记录表
| 检查项ID | 执行结果(pass/fail) | 风险等级 | 证据（请求/响应/日志/截图） | 备注 |
|---|---|---|---|---|
|  |  |  |  |  |

### 退出准则
1. 所有检查项执行完成，且高风险项无 `fail`。
2. 如存在 `fail`，必须附带漏洞单号、修复计划和复测结论。
3. 回归报告需同时记录矩阵ID与 Backlog 任务号，便于跨版本追溯。
