# API 安全 - CORS 与安全头测试

**模块**: API 安全
**测试范围**: CORS 配置、HTTP 安全头
**场景数**: 4
**风险等级**: 🟡 中
**ASVS 5.0 矩阵ID**: M-API-04
**OWASP ASVS 5.0**: V3.4,V12.1,V13.1
**回归任务映射**: Backlog #13, #20


---

## 背景知识

关键安全头：
- **CORS**: 控制跨域资源访问
- **CSP**: 内容安全策略
- **HSTS**: 强制 HTTPS
- **X-Frame-Options**: 防止点击劫持
- **X-Content-Type-Options**: 防止 MIME 嗅探

Auth9 跨域场景：
- Portal (localhost:3000) → Core API (localhost:8080)
- 第三方应用 → OIDC 端点

---

## 场景 1：CORS 配置安全

### 前置条件
- API 服务运行中
- 浏览器开发者工具

### 攻击目标
验证 CORS 是否正确配置

### 攻击步骤
1. 检查 CORS 响应头
2. 测试不同 Origin：
   - 合法 Origin
   - 恶意 Origin
   - null Origin
3. 检查 Credentials 处理

### 预期安全行为
- 仅允许白名单 Origin
- 不返回 `Access-Control-Allow-Origin: *` (带凭证时)
- 不接受 null Origin

### 验证方法
```bash
# 预检请求
curl -i -X OPTIONS http://localhost:8080/api/v1/users \
  -H "Origin: http://localhost:3000" \
  -H "Access-Control-Request-Method: GET"
# 预期: Access-Control-Allow-Origin: http://localhost:3000

# 恶意 Origin
curl -i -X OPTIONS http://localhost:8080/api/v1/users \
  -H "Origin: http://evil.com" \
  -H "Access-Control-Request-Method: GET"
# 预期: 不返回 Access-Control-Allow-Origin 或返回错误

# null Origin
curl -i -X OPTIONS http://localhost:8080/api/v1/users \
  -H "Origin: null" \
  -H "Access-Control-Request-Method: GET"
# 预期: 拒绝

# 通配符 + 凭证
curl -i http://localhost:8080/api/v1/users \
  -H "Origin: http://any.com"
# 不应同时:
# Access-Control-Allow-Origin: *
# Access-Control-Allow-Credentials: true
```

### 修复建议
- 明确列出允许的 Origin
- 禁止通配符 + Credentials
- 拒绝 null Origin
- 验证 Origin 格式

---

## 场景 2：安全响应头检查

### 前置条件
- API 和 Portal 运行中

### 攻击目标
验证安全响应头是否正确配置

### 攻击步骤
1. 检查各端点的响应头
2. 验证每个安全头的值
3. 测试缺失的头是否可被利用

### 预期安全行为
- 所有安全头正确配置
- HTTPS 端点有 HSTS
- 防止 XSS、点击劫持等

### 验证方法
```bash
# 检查 API 响应头
curl -I http://localhost:8080/api/v1/users \
  -H "Authorization: Bearer $TOKEN"

# 检查 Portal 响应头
curl -I http://localhost:3000/dashboard

# 使用 securityheaders.com 扫描
# 或使用本地工具检查

# 期望的响应头:
# X-Content-Type-Options: nosniff
# X-Frame-Options: DENY
# X-XSS-Protection: 1; mode=block
# Strict-Transport-Security: max-age=31536000; includeSubDomains
# Content-Security-Policy: default-src 'self'; ...
# Referrer-Policy: strict-origin-when-cross-origin
# Permissions-Policy: geolocation=(), camera=(), microphone=()
```

### 修复建议
- 添加所有推荐的安全头
- HSTS 有效期至少 1 年
- CSP 尽可能严格
- 定期审计头配置

---

## 场景 3：Content-Security-Policy 测试

### 前置条件
- Portal 运行中
- 浏览器

### 攻击目标
验证 CSP 是否有效防护 XSS

### 攻击步骤
1. 检查 CSP 头内容
2. 尝试违反 CSP 的操作：
   - 内联脚本
   - 外部脚本加载
   - eval() 执行
3. 检查 CSP 报告

### 预期安全行为
- 禁止危险的内联脚本
- 限制脚本来源
- 报告违规尝试

### 验证方法
```bash
# 获取 CSP 头
curl -I http://localhost:3000 | grep -i content-security-policy

# 预期 CSP 指令:
# default-src 'self';
# script-src 'self' 'nonce-...';  # nonce-based CSP for React hydration
# style-src 'self' 'unsafe-inline';  # React 可能需要
# img-src 'self' data: https:;
# connect-src 'self' http://localhost:* https://localhost:* ws://localhost:*;
# frame-ancestors 'none';
# form-action 'self';
# base-uri 'self';
```

**CSP 头验证（推荐方法）**:
```bash
# 验证 script-src 不包含 unsafe-eval
CSP=$(curl -sI http://localhost:3000 | grep -i content-security-policy)
echo "$CSP" | grep -q "unsafe-eval" && echo "FAIL: unsafe-eval found" || echo "PASS: no unsafe-eval"

# 验证 script-src 使用 nonce（不允许任意内联脚本）
echo "$CSP" | grep -q "nonce-" && echo "PASS: nonce-based CSP" || echo "WARN: no nonce found"
```

> **注意**: 不要在浏览器 DevTools Console 中测试 `eval()`。大多数浏览器（Chrome, Firefox）的 DevTools Console 运行在特殊执行上下文中，**不受 CSP 限制**。在 Console 中执行 `eval()` 成功并不代表 CSP 配置有误。正确的测试方法是直接检查 CSP 头中 `script-src` 是否包含 `'unsafe-eval'`。

```bash
# CSP 报告
# 检查 report-uri 或 report-to 配置
```

### 修复建议
- 避免 'unsafe-inline' 和 'unsafe-eval'
- 使用 nonce 或 hash
- 配置 report-uri 收集违规
- 从 Report-Only 开始测试

---

## 场景 4：点击劫持防护

### 前置条件
- Portal 运行中
- 能创建测试 HTML

### 攻击目标
验证是否可将应用嵌入 iframe 进行点击劫持

### 攻击步骤
1. 创建恶意页面嵌入目标
2. 尝试在 iframe 中加载敏感页面：
   - 登录页
   - 设置页
   - 操作确认页
3. 检查是否被阻止

### 预期安全行为
- iframe 加载被阻止
- 返回空白或错误
- X-Frame-Options 或 CSP frame-ancestors 生效

### 验证方法
```html
<!-- clickjack-test.html -->
<!DOCTYPE html>
<html>
<head><title>Clickjacking Test</title></head>
<body>
  <h1>Click the button below!</h1>
  <iframe src="http://localhost:3000/dashboard"
          style="opacity: 0.3; position: absolute; top: 100px; left: 100px;
                 width: 800px; height: 600px; z-index: 2;">
  </iframe>
  <button style="position: absolute; top: 200px; left: 300px; z-index: 1;">
    Win a Prize!
  </button>
</body>
</html>
```

```bash
# 检查响应头
curl -I http://localhost:3000/dashboard
# 预期:
# X-Frame-Options: DENY
# 或 Content-Security-Policy: frame-ancestors 'none'

# 打开测试页面
# iframe 应该不加载或显示错误
```

### 修复建议
- X-Frame-Options: DENY (或 SAMEORIGIN)
- CSP frame-ancestors: 'none'
- 敏感操作需要确认
- JavaScript frame-busting (作为备用)

---

## 检查清单

| # | 场景 | 状态 | 测试日期 | 测试人员 | 发现问题 |
|---|------|------|----------|----------|----------|
| 1 | CORS 配置安全 | ☐ | | | |
| 2 | 安全响应头检查 | ☐ | | | |
| 3 | Content-Security-Policy | ☐ | | | |
| 4 | 点击劫持防护 | ☐ | | | |

---

## 推荐的安全头配置

### API (auth9-core)

```
X-Content-Type-Options: nosniff
X-Frame-Options: DENY
Cache-Control: no-store, no-cache, must-revalidate
Pragma: no-cache
```

### Portal (auth9-portal)

```
Strict-Transport-Security: max-age=31536000; includeSubDomains; preload
X-Content-Type-Options: nosniff
X-Frame-Options: DENY
X-XSS-Protection: 1; mode=block
Content-Security-Policy: default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data: https:; connect-src 'self' http://localhost:8080; frame-ancestors 'none'; form-action 'self'; base-uri 'self'
Referrer-Policy: strict-origin-when-cross-origin
Permissions-Policy: geolocation=(), camera=(), microphone=()
```

### CORS 配置示例 (Rust axum)

```rust
use tower_http::cors::{CorsLayer, AllowOrigin};
use http::{Method, header};

let cors = CorsLayer::new()
    .allow_origin(AllowOrigin::list([
        "http://localhost:3000".parse().unwrap(),
        "https://portal.auth9.example.com".parse().unwrap(),
    ]))
    .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
    .allow_headers([header::AUTHORIZATION, header::CONTENT_TYPE])
    .allow_credentials(true)
    .max_age(Duration::from_secs(3600));
```

---

## 参考资料

- [MDN CORS](https://developer.mozilla.org/en-US/docs/Web/HTTP/CORS)
- [MDN CSP](https://developer.mozilla.org/en-US/docs/Web/HTTP/CSP)
- [OWASP Secure Headers](https://owasp.org/www-project-secure-headers/)
- [securityheaders.com](https://securityheaders.com/)
- [CWE-942: Permissive CORS Policy](https://cwe.mitre.org/data/definitions/942.html)

---


---

## 标准化回归 Checklist（ASVS 5.0）

**矩阵ID**: M-API-04  
**适用控制**: V3.4,V12.1,V13.1  
**关联任务**: Backlog #13, #20  
**建议回归频率**: 每次发布前 + 缺陷修复后必跑  
**场景总数**: 4

### 执行清单
- [ ] M-API-04-C01 | 控制: V3.4 | 任务: #13, #20 | 动作: 执行文档内相关攻击步骤并记录证据
- [ ] M-API-04-C02 | 控制: V12.1 | 任务: #13, #20 | 动作: 执行文档内相关攻击步骤并记录证据
- [ ] M-API-04-C03 | 控制: V13.1 | 任务: #13, #20 | 动作: 执行文档内相关攻击步骤并记录证据

### 回归记录表
| 检查项ID | 执行结果(pass/fail) | 风险等级 | 证据（请求/响应/日志/截图） | 备注 |
|---|---|---|---|---|
|  |  |  |  |  |

### 退出准则
1. 所有检查项执行完成，且高风险项无 `fail`。
2. 如存在 `fail`，必须附带漏洞单号、修复计划和复测结论。
3. 回归报告需同时记录矩阵ID与 Backlog 任务号，便于跨版本追溯。
