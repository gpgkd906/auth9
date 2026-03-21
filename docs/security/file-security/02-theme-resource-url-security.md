# 文件与资源安全 - Theme 外链资源 URL 安全测试

**模块**: 文件与资源安全
**测试范围**: logo/favicon 等 URL 字段的协议、域名与可达性约束
**场景数**: 3
**风险等级**: 🟠 高
**ASVS 5.0 矩阵ID**: M-FILE-02
**OWASP ASVS 5.0**: V5.2,V3.4,V14.2
**回归任务映射**: Backlog #18, #20

---

## 前置条件

- Docker 服务运行中
- 已获取 Platform Admin JWT Token（`$TOKEN`）
- API 端点: `PUT http://localhost:8080/api/v1/system/branding`

### 域名白名单配置（重要）

域名白名单通过环境变量 `BRANDING_ALLOWED_DOMAINS` 控制：

- **未配置（默认）**: 允许任意外部 HTTPS 域名（SSRF 保护仍生效）
- **已配置**: 仅允许白名单内的域名及其子域名

**场景 2 测试前，必须在 auth9-core 容器中设置此环境变量**：
```bash
# docker-compose.yml 中设置
BRANDING_ALLOWED_DOMAINS=cdn.example.com,assets.example.com
```

---

## 场景 1：危险协议注入

### 攻击目标
验证 `javascript:`、`data:`、`file:` 等危险协议被拒绝。

### 测试步骤

```bash
# 1. javascript: 协议
curl -s -o /dev/null -w "%{http_code}" -X PUT "http://localhost:8080/api/v1/system/branding" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"config":{"logo_url":"javascript:alert(1)","primary_color":"#000000","secondary_color":"#111111","background_color":"#222222","text_color":"#333333"}}'
# 预期: 422

# 2. data: 协议
curl -s -o /dev/null -w "%{http_code}" -X PUT "http://localhost:8080/api/v1/system/branding" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"config":{"logo_url":"data:image/png;base64,iVBOR...","primary_color":"#000000","secondary_color":"#111111","background_color":"#222222","text_color":"#333333"}}'
# 预期: 422

# 3. file: 协议
curl -s -o /dev/null -w "%{http_code}" -X PUT "http://localhost:8080/api/v1/system/branding" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"config":{"logo_url":"file:///etc/passwd","primary_color":"#000000","secondary_color":"#111111","background_color":"#222222","text_color":"#333333"}}'
# 预期: 422
```

### 预期结果
所有危险协议请求返回 **422 Unprocessable Entity**。

---

## 场景 2：外链域名控制

### 攻击目标
验证配置域名白名单后，只有受信任域名的资源 URL 被接受。

### 前置条件（必须）
**必须先配置 `BRANDING_ALLOWED_DOMAINS` 环境变量**，否则白名单不生效（设计如此）。

```bash
# 在 docker-compose.yml 或 .env 中设置后重启 auth9-core
BRANDING_ALLOWED_DOMAINS=cdn.example.com,assets.example.com
```

### 测试步骤

```bash
# 1. 白名单内域名 - 应成功
curl -s -o /dev/null -w "%{http_code}" -X PUT "http://localhost:8080/api/v1/system/branding" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"config":{"logo_url":"https://cdn.example.com/logo.png","primary_color":"#000000","secondary_color":"#111111","background_color":"#222222","text_color":"#333333"}}'
# 预期: 200

# 2. 白名单子域名 - 应成功
curl -s -o /dev/null -w "%{http_code}" -X PUT "http://localhost:8080/api/v1/system/branding" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"config":{"logo_url":"https://img.cdn.example.com/logo.png","primary_color":"#000000","secondary_color":"#111111","background_color":"#222222","text_color":"#333333"}}'
# 预期: 200

# 3. 白名单外域名 - 应拒绝
curl -s -o /dev/null -w "%{http_code}" -X PUT "http://localhost:8080/api/v1/system/branding" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"config":{"logo_url":"https://evil-attacker.com/logo.png","primary_color":"#000000","secondary_color":"#111111","background_color":"#222222","text_color":"#333333"}}'
# 预期: 422

# 4. favicon 同样受白名单限制
curl -s -o /dev/null -w "%{http_code}" -X PUT "http://localhost:8080/api/v1/system/branding" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"config":{"favicon_url":"https://random-domain.com/favicon.ico","primary_color":"#000000","secondary_color":"#111111","background_color":"#222222","text_color":"#333333"}}'
# 预期: 422
```

### 预期结果
| 测试 | URL 域名 | 预期状态码 |
|------|----------|-----------|
| 白名单内 | cdn.example.com | 200 |
| 子域名 | img.cdn.example.com | 200 |
| 白名单外 | evil-attacker.com | 422 |
| favicon 白名单外 | random-domain.com | 422 |

### 故障排除

| 症状 | 原因 | 解决方法 |
|------|------|----------|
| 任意域名都返回 200 | 未配置 `BRANDING_ALLOWED_DOMAINS` | 设置环境变量并重启 auth9-core |
| 所有域名都返回 422 | 白名单配置错误 | 检查域名拼写，多个域名用逗号分隔 |
| 子域名被拒绝 | 白名单只填了子域名 | 白名单中填写基础域名（如 `example.com`），子域名自动允许 |

---

## 场景 3：资源 URL 导致隐私泄露

### 攻击目标
验证登录页加载外链资源是否泄露访问元数据与 referrer。

### 重要说明

**登录页是 Auth9 Portal 的 `/login` 路由**（Auth9 托管认证页）。

- 默认入口页是 **Auth9 Portal `/login`**
- Auth9 品牌认证页通过 `useBranding()` hook 从 `/api/v1/public/branding` 获取 logo
- Logo 组件位于 Auth9 Portal 登录页组件中

### 测试步骤

1. 设置 logo URL 为可监控的外部地址：
   ```bash
   curl -X PUT http://localhost:8080/api/v1/system/branding \
     -H "Authorization: Bearer $TOKEN" \
     -H "Content-Type: application/json" \
     -d '{"config":{"logo_url":"https://your-monitoring-server.com/logo.png"}}'
   ```
2. 访问 Auth9 Portal `/login`，确认主入口页直接显示品牌 logo 或默认品牌缩写
3. 检查浏览器开发者工具中外部请求的 `Referer` 头
5. 检查 `<img>` 标签的 `referrerPolicy` 属性

### 预期结果
- Portal `/login` 页中的 `<img>` 标签应包含 `referrerPolicy="no-referrer"` 属性
- `<img>` 标签还应包含 `crossOrigin="anonymous"` 属性
- 外部请求不应携带来源页面的 URL 信息

### 故障排除

| 症状 | 原因 | 解决方法 |
|------|------|----------|
| auth9-portal `/login` 没有 `<img>` 标签 | 当前品牌未配置 `logo_url`，页面可能显示默认品牌缩写 | 配置 `logo_url` 后重试，或确认默认品牌缩写是否显示 |
| 登录页没有显示 logo | branding API 未配置 logo_url | 通过 `PUT /api/v1/system/branding` 设置 `logo_url` |
| logo 显示但缺少 referrerPolicy | Logo 组件未正确实现 | 检查 Auth9 Portal 登录页 Logo 组件 |

---

## SSRF 保护（始终生效，无需配置）

无论是否配置域名白名单，以下 SSRF 保护始终生效：

| 保护项 | 被拦截的 URL 示例 |
|--------|------------------|
| 内网 IP | `http://192.168.1.1/logo.png`, `http://10.0.0.1/img.png` |
| 回环地址 | `http://127.0.0.1/logo.png`, `http://localhost/logo.png` |
| 云元数据 | `http://169.254.169.254/latest/meta-data/` |
| HTTP 外链 | `http://example.com/logo.png`（强制要求 HTTPS） |

---

## 标准化回归 Checklist（ASVS 5.0）

**矩阵ID**: M-FILE-02
**适用控制**: V5.2,V3.4,V14.2
**关联任务**: Backlog #18, #20
**建议回归频率**: 每次发布前 + 缺陷修复后必跑
**场景总数**: 3

### 执行清单
- [ ] M-FILE-02-C01 | 控制: V5.2 | 任务: #18, #20 | 动作: 执行场景 1 攻击步骤并记录证据
- [ ] M-FILE-02-C02 | 控制: V3.4 | 任务: #18, #20 | 动作: 配置白名单后执行场景 2 攻击步骤并记录证据
- [ ] M-FILE-02-C03 | 控制: V14.2 | 任务: #18, #20 | 动作: 执行场景 3 referrer 检查并记录证据

### 回归记录表
| 检查项ID | 执行结果(pass/fail) | 风险等级 | 证据（请求/响应/日志/截图） | 备注 |
|---|---|---|---|---|
|  |  |  |  |  |
