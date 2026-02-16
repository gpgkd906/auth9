# URL 输入安全 - 路径遍历与注入测试

**模块**: 文件与资源安全
**测试范围**: URL 字段输入验证（路径遍历、Scheme 注入、SSRF）
**场景数**: 3
**风险等级**: 🟠 高
**OWASP ASVS**: V5.1, V12.4, V13.2

---

## 背景知识

Auth9 中**不存在文件上传功能**，所有图片/资源通过 **URL 字符串** 引用。涉及 URL 输入的字段：

| 字段 | 所在模块 | 验证函数 |
|------|---------|---------|
| `avatar_url` | User (CreateUserInput, UpdateUserInput) | `validate_avatar_url` |
| `logo_url` | Tenant (CreateTenantInput, UpdateTenantInput) | `validate_url_no_ssrf_strict` |
| `logo_url` | TenantBranding | `validate_branding_logo_url` |
| `logo_url` | BrandingConfig | `validate_url_no_ssrf_strict_option` |
| `favicon_url` | BrandingConfig | `validate_url_no_ssrf_strict_option` |
| `url` | Webhook (CreateWebhookInput) | `validate_url_no_ssrf_strict` |

前端直接将 URL 字符串通过 `<img src="...">` 渲染，若 URL 未经充分验证，可能导致：
- **路径遍历**：`../../etc/passwd` 等恶意路径注入
- **Scheme 注入**：`javascript:alert(1)` 或 `data:text/html,...` 导致 XSS
- **SSRF**：指向内网 IP 或云元数据端点，导致敏感信息泄露

---

## 场景 1：URL 路径遍历攻击

### 前置条件
- 具有用户/租户管理权限的 Token
- API 端点可接受 URL 字段

### 攻击目标
验证 URL 字段是否拒绝包含 `../`、null 字节等路径遍历字符的恶意输入

### 攻击步骤
1. 提交 `avatar_url` 包含 `../../etc/passwd`（无 scheme）
2. 提交 `avatar_url` 包含 `https://example.com/../../etc/passwd`（有 scheme + 遍历）
3. 提交 URL 编码遍历：`..%2F..%2Fetc%2Fpasswd`
4. 提交 null 字节注入：`https://example.com/avatar\x00.png`
5. 提交 Tenant `logo_url` 包含路径遍历字符
6. ⚠️ 提交 TenantBranding `logo_url` 包含路径遍历（`validate_branding_logo_url` 仅检查 scheme，未检查 `..`）
7. ⚠️ 提交 TenantBranding `logo_url` 包含 null 字节

### 预期安全行为
- 无 scheme 的路径遍历被拒绝（`validate_avatar_url` 要求 http/https）
- 包含 `..` 的 URL 被拒绝（`validate_avatar_url` 检查 `..`）
- null 字节被拒绝
- `logo_url` 通过 `url::Url::parse` 解析，畸形 URL 被拒绝
- ⚠️ TenantBranding `logo_url` 应拒绝包含 `..` 的 URL，但**当前实现未检查**

### 验证方法
```bash
# 1. avatar_url - 纯路径遍历（无 scheme）
curl -s -X PUT -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"avatar_url": "../../etc/passwd"}' \
  http://localhost:8080/api/v1/users/me
# 预期: 400 - Avatar URL must use http:// or https:// scheme

# 2. avatar_url - https + 路径遍历
curl -s -X PUT -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"avatar_url": "https://example.com/../../etc/passwd"}' \
  http://localhost:8080/api/v1/users/me
# 预期: 400 - Avatar URL contains invalid characters

# 3. avatar_url - URL 编码遍历（无 scheme）
curl -s -X PUT -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"avatar_url": "..%2F..%2Fetc%2Fpasswd"}' \
  http://localhost:8080/api/v1/users/me
# 预期: 400 - 无 http(s):// scheme

# 4. avatar_url - null 字节注入
curl -s -X PUT -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"avatar_url": "https://example.com/avatar\u0000.png"}' \
  http://localhost:8080/api/v1/users/me
# 预期: 400 - Avatar URL contains invalid characters

# 5. tenant logo_url - 路径遍历（validate_url_no_ssrf_strict）
curl -s -X PUT -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"logo_url": "https://example.com/../../etc/passwd"}' \
  http://localhost:8080/api/v1/tenants/$TENANT_ID
# 预期: 400
# 注意: url::Url::parse 会将 /../ 规范化为 /，可能不会报错（需验证）

# 6. ⚠️ [漏洞] TenantBranding logo_url - 路径遍历
#    validate_branding_logo_url 仅检查 scheme，不检查 .. 字符
curl -s -X PUT -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"settings": {"branding": {"logo_url": "https://example.com/../../etc/passwd"}}}' \
  http://localhost:8080/api/v1/tenants/$TENANT_ID
# 预期应为: 400
# 当前实际: 200 - url::Url::parse 规范化路径后未拒绝

# 7. ⚠️ [漏洞] TenantBranding logo_url - null 字节
#    validate_branding_logo_url 未检查 null 字节
curl -s -X PUT -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"settings": {"branding": {"logo_url": "https://example.com/logo\u0000.png"}}}' \
  http://localhost:8080/api/v1/tenants/$TENANT_ID
# 预期应为: 400
# 当前实际: 取决于 url::Url::parse 对 null 字节的处理

# 8. 正常 URL 应该通过
curl -s -X PUT -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"avatar_url": "https://cdn.example.com/avatars/user123.png"}' \
  http://localhost:8080/api/v1/users/me
# 预期: 200
```

---

## 场景 2：URL Scheme 注入

### 前置条件
- 具有用户/租户管理权限的 Token
- 前端通过 `<img src="...">` 渲染 URL

### 攻击目标
验证 URL 字段是否拒绝 `javascript:`、`data:`、`ftp:` 等危险 scheme，防止 XSS

### 攻击步骤
1. 提交 `avatar_url = "javascript:alert(document.cookie)"`
2. 提交 `logo_url = "data:text/html,<script>alert(1)</script>"`
3. 提交 `favicon_url = "ftp://evil.com/malware.exe"`
4. 提交大小写绕过 `Java\x00Script:alert(1)`
5. 提交 `logo_url = "data:image/svg+xml;base64,PHN2Zy..."`（Base64 编码的恶意 SVG）

### 预期安全行为
- 所有字段仅允许 `http://` 和 `https://` scheme
- `javascript:`、`data:`、`ftp:` 等被拒绝
- 大小写变体和编码绕过被拒绝

### 验证方法
```bash
# 1. avatar_url - javascript scheme
curl -s -X PUT -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"avatar_url": "javascript:alert(document.cookie)"}' \
  http://localhost:8080/api/v1/users/me
# 预期: 400 - Avatar URL must use http:// or https:// scheme

# 2. tenant logo_url - data scheme
curl -s -X PUT -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"logo_url": "data:text/html,<script>alert(1)</script>"}' \
  http://localhost:8080/api/v1/tenants/$TENANT_ID
# 预期: 400 - invalid_scheme

# 3. branding favicon_url - ftp scheme
curl -s -X PUT -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"config": {"favicon_url": "ftp://evil.com/malware.exe", "primary_color": "#007AFF", "secondary_color": "#5856D6", "background_color": "#F5F5F7", "text_color": "#1D1D1F"}}' \
  http://localhost:8080/api/v1/tenants/$TENANT_ID/branding
# 预期: 400 - invalid_scheme

# 4. data URI with base64 SVG
curl -s -X PUT -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"logo_url": "data:image/svg+xml;base64,PHN2ZyB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciPjxzY3JpcHQ+YWxlcnQoMSk8L3NjcmlwdD48L3N2Zz4="}' \
  http://localhost:8080/api/v1/tenants/$TENANT_ID
# 预期: 400

# 5. 正常 HTTPS URL 应通过
curl -s -X PUT -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"logo_url": "https://cdn.example.com/logo.png"}' \
  http://localhost:8080/api/v1/tenants/$TENANT_ID
# 预期: 200
```

---

## 场景 3：SSRF - 通过 URL 字段探测内网

### 前置条件
- 具有租户/品牌管理权限的 Token
- 目标服务运行在内网环境

### 攻击目标
验证 URL 字段是否阻止指向内网 IP、localhost 和云元数据端点的 URL，防止 SSRF

### 攻击步骤
1. 提交 Tenant `logo_url = "http://127.0.0.1:8080/admin"`
2. 提交 Tenant `logo_url = "https://192.168.1.1/internal"`
3. 提交 Tenant `logo_url = "http://10.0.0.1/secret"`
4. 提交 Tenant `logo_url = "http://169.254.169.254/latest/meta-data/"` (AWS 元数据)
5. 提交 Tenant `logo_url = "http://metadata.google.internal/"` (GCP 元数据)
6. 提交 Tenant `logo_url = "http://[::1]/admin"` (IPv6 localhost)
7. 提交 Tenant `logo_url = "http://0.0.0.0/admin"`
8. 提交外部 HTTP（非 HTTPS）：`http://example.com/logo.png`
9. ⚠️ 提交 `avatar_url` 指向 localhost / 私有 IP / 云元数据（`validate_avatar_url` 不检查 SSRF）
10. ⚠️ 提交 `avatar_url` 指向 `http://0.0.0.0`、`http://[::1]` 等变体

### 预期安全行为
- `validate_url_no_ssrf_strict` 阻止所有私有/回环 IP（Tenant logo_url, BrandingConfig, Webhook）
- 云元数据端点被阻止
- 外部 HTTP URL 被拒绝（仅允许 HTTPS）
- ⚠️ `validate_avatar_url` 应阻止私有 IP / 云元数据，但**当前实现未检查 SSRF**

### 验证方法
```bash
# 1. tenant logo_url - localhost
curl -s -X PUT -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"logo_url": "http://127.0.0.1:8080/admin"}' \
  http://localhost:8080/api/v1/tenants/$TENANT_ID
# 预期: 400 - Internal IP addresses are not allowed

# 2. tenant logo_url - 私有网段
curl -s -X PUT -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"logo_url": "https://192.168.1.1/internal"}' \
  http://localhost:8080/api/v1/tenants/$TENANT_ID
# 预期: 400 - Internal IP addresses are not allowed

# 3. branding logo_url - AWS 元数据
curl -s -X PUT -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"config": {"logo_url": "http://169.254.169.254/latest/meta-data/", "primary_color": "#007AFF", "secondary_color": "#5856D6", "background_color": "#F5F5F7", "text_color": "#1D1D1F"}}' \
  http://localhost:8080/api/v1/tenants/$TENANT_ID/branding
# 预期: 400 - ssrf_blocked 或 internal_ip_blocked

# 4. tenant logo_url - 外部 HTTP（非 HTTPS）
curl -s -X PUT -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"logo_url": "http://example.com/logo.png"}' \
  http://localhost:8080/api/v1/tenants/$TENANT_ID
# 预期: 400 - Only HTTPS URLs are allowed

# 5. webhook url - IPv6 localhost
curl -s -X POST -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"name": "test", "url": "http://[::1]/hook", "events": ["user.created"]}' \
  http://localhost:8080/api/v1/tenants/$TENANT_ID/webhooks
# 预期: 400 - Internal IP addresses are not allowed

# 6. ⚠️ [漏洞] avatar_url - AWS 云元数据
#    validate_avatar_url 仅检查 scheme + .. / null，不做 SSRF 防护
curl -s -X PUT -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"avatar_url": "http://169.254.169.254/latest/meta-data/"}' \
  http://localhost:8080/api/v1/users/me
# 预期应为: 400
# 当前实际: 200 - validate_avatar_url 不检查 IP 地址

# 7. ⚠️ [漏洞] avatar_url - localhost
curl -s -X PUT -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"avatar_url": "http://127.0.0.1:8080/admin"}' \
  http://localhost:8080/api/v1/users/me
# 预期应为: 400
# 当前实际: 200 - validate_avatar_url 不检查 IP 地址

# 8. ⚠️ [漏洞] avatar_url - 私有网段
curl -s -X PUT -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"avatar_url": "http://192.168.1.1/internal-dashboard"}' \
  http://localhost:8080/api/v1/users/me
# 预期应为: 400
# 当前实际: 200 - validate_avatar_url 不检查 IP 地址

# 9. ⚠️ [漏洞] avatar_url - GCP 元数据
curl -s -X PUT -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"avatar_url": "http://metadata.google.internal/computeMetadata/v1/"}' \
  http://localhost:8080/api/v1/users/me
# 预期应为: 400
# 当前实际: 200 - validate_avatar_url 不检查主机名

# 10. ⚠️ [漏洞] avatar_url - IPv6 localhost
curl -s -X PUT -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"avatar_url": "http://[::1]/admin"}' \
  http://localhost:8080/api/v1/users/me
# 预期应为: 400
# 当前实际: 200 - validate_avatar_url 不检查 IP 地址

# 11. ⚠️ [漏洞] avatar_url - 0.0.0.0
curl -s -X PUT -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"avatar_url": "http://0.0.0.0/admin"}' \
  http://localhost:8080/api/v1/users/me
# 预期应为: 400
# 当前实际: 200 - validate_avatar_url 不检查 IP 地址
```

---

## 已知验证漏洞汇总

| # | 漏洞 | 影响字段 | 验证函数 | 缺失检查 | 建议修复 |
|---|------|---------|---------|---------|---------|
| V1 | avatar_url 缺少 SSRF 防护 | `User.avatar_url` | `validate_avatar_url` | 私有 IP / 回环地址 / 云元数据 | 改用 `validate_url_no_ssrf_strict` 或添加 IP 检查 |
| V2 | TenantBranding logo_url 缺少路径遍历检查 | `TenantBranding.logo_url` | `validate_branding_logo_url` | `..` 和 null 字节 | 添加 `..` / `\0` 检查或改用 `validate_url_no_ssrf_strict` |

---

## 检查清单

| # | 场景 | 状态 | 测试日期 | 测试人员 | 发现问题 |
|---|------|------|----------|----------|----------|
| 1 | URL 路径遍历攻击 | ☐ | | | |
| 1.6 | ⚠️ TenantBranding logo_url 路径遍历（漏洞 V2） | ☐ | | | |
| 1.7 | ⚠️ TenantBranding logo_url null 字节（漏洞 V2） | ☐ | | | |
| 2 | URL Scheme 注入 | ☐ | | | |
| 3 | SSRF - 通过 URL 字段探测内网 | ☐ | | | |
| 3.6 | ⚠️ avatar_url AWS 云元数据 SSRF（漏洞 V1） | ☐ | | | |
| 3.7 | ⚠️ avatar_url localhost SSRF（漏洞 V1） | ☐ | | | |
| 3.8 | ⚠️ avatar_url 私有网段 SSRF（漏洞 V1） | ☐ | | | |
| 3.9 | ⚠️ avatar_url GCP 元数据 SSRF（漏洞 V1） | ☐ | | | |
| 3.10 | ⚠️ avatar_url IPv6 localhost SSRF（漏洞 V1） | ☐ | | | |
| 3.11 | ⚠️ avatar_url 0.0.0.0 SSRF（漏洞 V1） | ☐ | | | |

---

## 参考资料

- [OWASP SSRF Prevention Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Server-Side_Request_Forgery_Prevention_Cheat_Sheet.html)
- [CWE-22: Path Traversal](https://cwe.mitre.org/data/definitions/22.html)
- [CWE-918: Server-Side Request Forgery (SSRF)](https://cwe.mitre.org/data/definitions/918.html)
- [CWE-79: XSS via Scheme Injection](https://cwe.mitre.org/data/definitions/79.html)
