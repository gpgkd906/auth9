# 基础设施安全 - HTTP 安全头测试

**模块**: 基础设施安全
**测试范围**: HTTP 响应安全头配置
**场景数**: 5
**风险等级**: 🟡 中

---

## 背景知识

关键安全头：
| Header | 作用 |
|--------|------|
| Content-Security-Policy | 防止 XSS、注入 |
| X-Content-Type-Options | 防止 MIME 嗅探 |
| X-Frame-Options | 防止点击劫持 |
| X-XSS-Protection | XSS 过滤 (已废弃) |
| Referrer-Policy | 控制 Referer 信息 |
| Permissions-Policy | 限制浏览器功能 |

---

## 场景 1：必需安全头检查

### 前置条件
- HTTP 端点可访问

### 攻击目标
验证必需的安全头是否配置

### 攻击步骤
1. 获取响应头
2. 检查每个安全头是否存在
3. 验证头的值是否正确

### 预期安全行为
- 所有必需安全头存在
- 值符合安全要求
- API 和 Portal 都有配置

### 验证方法
```bash
# 检查 Portal 响应头
curl -I https://localhost:3000/ | grep -iE "content-security|x-frame|x-content-type|strict-transport|referrer-policy|permissions-policy"

# 检查 API 响应头
curl -I https://localhost:8080/api/v1/health | grep -iE "content-security|x-frame|x-content-type"

# 使用在线工具
# https://securityheaders.com/?q=auth9.example.com

# 期望的头:
# X-Content-Type-Options: nosniff
# X-Frame-Options: DENY
# Strict-Transport-Security: max-age=31536000; includeSubDomains
# Referrer-Policy: strict-origin-when-cross-origin
# Permissions-Policy: geolocation=(), camera=(), microphone=()
```

### 修复建议
- 在反向代理或应用层添加
- 确保所有端点都有配置
- 定期审计配置

---

## 场景 2：Content-Security-Policy 测试

### 前置条件
- Portal 可访问

### 攻击目标
验证 CSP 是否有效防护

### 攻击步骤
1. 分析 CSP 指令
2. 测试各种绕过：
   - 内联脚本
   - eval()
   - 外部脚本加载
3. 检查 CSP 报告

### 预期安全行为
- 禁止危险操作
- 报告违规尝试
- 不影响正常功能

### 验证方法
```bash
# 获取 CSP
curl -I https://localhost:3000 | grep -i content-security-policy

# 分析 CSP 指令
# 使用 CSP Evaluator: https://csp-evaluator.withgoogle.com/

# 浏览器测试 - Console 注入
# 1. 打开开发者工具
# 2. 执行: eval("alert('test')")
# 3. 观察是否被阻止

# 检查 CSP 报告 (如果配置了 report-uri)
# 查看服务器日志或报告端点
```

### 修复建议
```
# 推荐的 CSP
Content-Security-Policy:
  default-src 'self';
  script-src 'self';
  style-src 'self' 'unsafe-inline';
  img-src 'self' data: https:;
  font-src 'self';
  connect-src 'self' https://api.auth9.example.com;
  frame-ancestors 'none';
  form-action 'self';
  base-uri 'self';
  upgrade-insecure-requests;
  report-uri /csp-report
```

---

## 场景 3：X-Frame-Options 测试

### 前置条件
- Portal 可访问

### 攻击目标
验证点击劫持防护

### 攻击步骤
1. 检查 X-Frame-Options 值
2. 尝试在 iframe 中加载
3. 测试不同配置 (DENY vs SAMEORIGIN)

### 预期安全行为
- 敏感页面不可嵌入
- iframe 加载被阻止

### 验证方法
```bash
# 检查响应头
curl -I https://localhost:3000/dashboard | grep -i x-frame-options
# 预期: X-Frame-Options: DENY

# 创建测试页面
cat > clickjack.html << 'EOF'
<!DOCTYPE html>
<html>
<body>
<h1>Clickjacking Test</h1>
<iframe src="https://localhost:3000/dashboard" width="800" height="600"></iframe>
</body>
</html>
EOF

# 在浏览器中打开，iframe 应该不加载
```

### 修复建议
- 使用 `X-Frame-Options: DENY`
- 同时使用 CSP `frame-ancestors 'none'`
- 敏感页面双重保护

---

## 场景 4：缓存控制头测试

### 前置条件
- API 端点可访问
- 认证 Token

### 攻击目标
验证敏感数据缓存控制

### 攻击步骤
1. 检查敏感 API 的缓存头
2. 验证浏览器不缓存敏感数据
3. 检查代理缓存行为

### 预期安全行为
- 敏感数据: `no-store`
- 静态资源: 适当缓存
- 私有数据: `private`

### 验证方法
```bash
# 检查敏感 API
curl -I -H "Authorization: Bearer $TOKEN" \
  https://localhost:8080/api/v1/users/me | grep -i cache

# 预期:
# Cache-Control: no-store, no-cache, must-revalidate, private
# Pragma: no-cache
# Expires: 0

# 检查静态资源
curl -I https://localhost:3000/assets/logo.png | grep -i cache
# 可以有缓存: Cache-Control: public, max-age=31536000

# 检查登出后
curl -I https://localhost:3000/dashboard | grep -i cache
# 应该: no-store
```

### 修复建议
```
# 敏感 API
Cache-Control: no-store, no-cache, must-revalidate, private
Pragma: no-cache
Expires: 0

# 静态资源
Cache-Control: public, max-age=31536000, immutable

# HTML 页面
Cache-Control: no-cache, private
```

---

## 场景 5：信息泄露头检查

### 前置条件
- 服务端点可访问

### 攻击目标
验证是否泄露服务器信息

### 攻击步骤
1. 检查敏感头：
   - Server
   - X-Powered-By
   - X-AspNet-Version
2. 检查错误页面
3. 检查 API 错误响应

### 预期安全行为
- 隐藏服务器版本
- 移除技术栈信息
- 错误不泄露详情

### 验证方法
```bash
# 检查响应头
curl -I https://localhost:8080/api/v1/health

# 不应包含:
# Server: nginx/1.19.0
# X-Powered-By: Express
# X-AspNet-Version: ...

# 应该:
# Server: (空或通用名称)

# 检查错误响应
curl https://localhost:8080/nonexistent
# 不应暴露框架信息

# 检查 OPTIONS 响应
curl -X OPTIONS https://localhost:8080/api/v1/users
# 不应暴露过多信息
```

### 修复建议
```nginx
# Nginx 隐藏版本
server_tokens off;
more_clear_headers Server;
proxy_hide_header X-Powered-By;
```

```rust
// Rust/Axum 移除 Server 头
// 使用自定义中间件
```

---

## 检查清单

| # | 场景 | 状态 | 测试日期 | 测试人员 | 发现问题 |
|---|------|------|----------|----------|----------|
| 1 | 必需安全头检查 | ☐ | | | |
| 2 | Content-Security-Policy | ☐ | | | |
| 3 | X-Frame-Options | ☐ | | | |
| 4 | 缓存控制头 | ☐ | | | |
| 5 | 信息泄露头 | ☐ | | | |

---

## 完整安全头配置

### Portal (React/Nginx)

```nginx
# 安全头
add_header X-Content-Type-Options "nosniff" always;
add_header X-Frame-Options "DENY" always;
add_header X-XSS-Protection "1; mode=block" always;
add_header Referrer-Policy "strict-origin-when-cross-origin" always;
add_header Permissions-Policy "geolocation=(), camera=(), microphone=()" always;
add_header Content-Security-Policy "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data: https:; connect-src 'self' https://api.auth9.example.com; frame-ancestors 'none'; form-action 'self'; base-uri 'self'" always;

# HSTS (仅 HTTPS)
add_header Strict-Transport-Security "max-age=31536000; includeSubDomains; preload" always;

# 隐藏版本
server_tokens off;
```

### API (Rust/Axum)

```rust
use tower_http::set_header::SetResponseHeaderLayer;
use http::header;

let security_headers = ServiceBuilder::new()
    .layer(SetResponseHeaderLayer::overriding(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    ))
    .layer(SetResponseHeaderLayer::overriding(
        header::X_FRAME_OPTIONS,
        HeaderValue::from_static("DENY"),
    ))
    .layer(SetResponseHeaderLayer::overriding(
        header::CACHE_CONTROL,
        HeaderValue::from_static("no-store"),
    ));
```

---

## 参考资料

- [OWASP Secure Headers](https://owasp.org/www-project-secure-headers/)
- [Mozilla Observatory](https://observatory.mozilla.org/)
- [SecurityHeaders.com](https://securityheaders.com/)
- [Content Security Policy Reference](https://content-security-policy.com/)
