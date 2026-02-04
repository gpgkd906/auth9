# 输入验证 - 跨站脚本攻击测试

**模块**: 输入验证
**测试范围**: XSS (Stored, Reflected, DOM)
**场景数**: 5
**风险等级**: 🟠 高

---

## 背景知识

Auth9 XSS 风险点：
- **Stored XSS**: 用户资料、租户名称、服务描述等
- **Reflected XSS**: 搜索结果、错误消息
- **DOM XSS**: React 前端 (通常较安全)

技术栈防护：
- React 默认转义
- 后端 JSON API 响应

---

## 场景 1：存储型 XSS - 用户资料

### 前置条件
- 可编辑用户资料的账户

### 攻击目标
验证用户资料字段是否可注入 XSS

### 攻击步骤
1. 编辑用户资料，在以下字段注入：
   - 显示名称: `<script>alert('XSS')</script>`
   - 头像 URL: `javascript:alert('XSS')`
   - 个人简介: `<img src=x onerror=alert('XSS')>`
2. 保存后访问个人页面
3. 让其他用户查看该资料

### 预期安全行为
- 脚本标签被转义或过滤
- 不执行任何 JavaScript
- 安全显示原始文本

### 验证方法
```bash
# 更新用户名
curl -X PUT -H "Authorization: Bearer $TOKEN" \
  http://localhost:8080/api/v1/users/me \
  -H "Content-Type: application/json" \
  -d '{"display_name": "<script>alert(document.cookie)</script>"}'

# 获取用户资料
curl -H "Authorization: Bearer $TOKEN" \
  http://localhost:8080/api/v1/users/me
# 检查响应中的转义: &lt;script&gt; 或 被过滤

# 在浏览器中访问用户页面
# 打开开发者工具 Console，确认无脚本执行
```

### 修复建议
- 输入时过滤危险标签
- 输出时 HTML 实体编码
- Content-Type: application/json
- React 使用 dangerouslySetInnerHTML 需审查

---

## 场景 2：存储型 XSS - 租户/服务配置

### 前置条件
- 租户管理员权限

### 攻击目标
验证租户和服务配置是否可注入 XSS

### 攻击步骤
1. 在租户配置中注入：
   - 租户名称: `<img src=x onerror=alert('XSS')>`
   - Logo URL: `javascript:alert('XSS')`
   - 自定义设置 JSON
2. 在服务配置中注入：
   - 服务名称
   - 回调 URL
   - 描述字段
3. 检查管理界面显示

### 预期安全行为
- 所有字段安全显示
- URL 验证格式
- JSON 正确解析

### 验证方法
```bash
# 创建恶意租户
curl -X POST -H "Authorization: Bearer $ADMIN_TOKEN" \
  http://localhost:8080/api/v1/tenants \
  -H "Content-Type: application/json" \
  -d '{
    "name": "<svg onload=alert(1)>",
    "slug": "test-xss",
    "logo_url": "javascript:alert(1)"
  }'
# 预期: 400 Invalid name 或 logo_url

# 如果创建成功，访问租户列表
# 在浏览器中检查是否触发 XSS
```

### 修复建议
- 名称字段白名单字符
- URL 字段验证协议 (http/https)
- 使用 CSP 头防护
- 审计所有用户可控字段

---

## 场景 3：反射型 XSS - 搜索/错误

### 前置条件
- 搜索功能或错误页面

### 攻击目标
验证搜索参数或错误消息是否可触发 XSS

### 攻击步骤
1. 在搜索 URL 中注入：
   - `?search=<script>alert('XSS')</script>`
   - `?q="><img src=x onerror=alert(1)>`
2. 检查错误页面：
   - `?error=<script>alert('XSS')</script>`
   - 404 页面的路径反射
3. 分析响应 HTML

### 预期安全行为
- URL 参数被转义
- 错误消息不反射原始输入
- SPA 路由不执行脚本

### 验证方法
```bash
# 搜索参数反射
curl "http://localhost:3000/dashboard/users?search=<script>alert(1)</script>"
# 检查响应 HTML

# 错误页面
curl "http://localhost:3000/<script>alert(1)</script>"
# 检查 404 页面

# 在浏览器中访问
# 打开开发者工具观察
```

### 修复建议
- URL 参数不直接渲染到 HTML
- 使用 React 安全渲染
- 错误消息固定模板
- 设置 X-XSS-Protection 头

---

## 场景 4：DOM XSS - 前端处理

### 前置条件
- 前端使用 URL hash 或查询参数

### 攻击目标
验证前端 JavaScript 是否安全处理用户输入

### 攻击步骤
1. 检查前端路由：
   - `/#<img src=x onerror=alert(1)>`
   - `/?redirect=javascript:alert(1)`
2. 检查 postMessage 处理
3. 检查 localStorage/sessionStorage 读取
4. 检查 eval() 或 innerHTML 使用

### 预期安全行为
- Hash 不执行脚本
- Redirect 验证目标 URL
- postMessage 验证来源
- 不使用危险 API

### 验证方法
```javascript
// 在浏览器控制台测试
// 1. 检查 URL hash 处理
window.location.hash = '<img src=x onerror=alert(1)>';
// 观察页面行为

// 2. 检查 redirect 参数
// 访问: http://localhost:3000/login?redirect=javascript:alert(1)
// 登录后观察是否执行

// 3. 检查 postMessage
// 从外部页面发送消息
window.postMessage('<script>alert(1)</script>', '*');
```

### 修复建议
- 使用 React Router 安全导航
- 验证 redirect URL 白名单
- postMessage 检查 origin
- 代码审查 innerHTML 使用

---

## 场景 5：XSS 通过文件上传

### 前置条件
- 文件上传功能 (头像、文档等)

### 攻击目标
验证上传文件是否可触发 XSS

### 攻击步骤
1. 上传 SVG 文件包含脚本：
   ```xml
   <svg xmlns="http://www.w3.org/2000/svg">
     <script>alert('XSS')</script>
   </svg>
   ```
2. 上传 HTML 文件
3. 上传带 XSS payload 的图片元数据
4. 直接访问上传的文件

### 预期安全行为
- SVG 脚本被移除或文件被拒绝
- HTML 文件不允许上传
- 文件以安全 Content-Type 提供
- 设置 Content-Disposition: attachment

### 验证方法
```bash
# 创建恶意 SVG
cat > xss.svg << 'EOF'
<svg xmlns="http://www.w3.org/2000/svg">
  <script>alert(document.domain)</script>
</svg>
EOF

# 上传
curl -X POST -H "Authorization: Bearer $TOKEN" \
  -F "file=@xss.svg" \
  http://localhost:8080/api/v1/users/me/avatar
# 预期: 400 Invalid file type 或 sanitized

# 如果上传成功，直接访问文件
curl -I http://localhost:8080/uploads/avatar/xss.svg
# 检查 Content-Type 和 Content-Disposition
```

### 修复建议
- 白名单文件类型
- SVG 文件净化或转换
- 文件服务设置安全头
- 使用独立域名提供文件

---

## 检查清单

| # | 场景 | 状态 | 测试日期 | 测试人员 | 发现问题 |
|---|------|------|----------|----------|----------|
| 1 | 存储型 XSS - 用户资料 | ☐ | | | |
| 2 | 存储型 XSS - 租户/服务配置 | ☐ | | | |
| 3 | 反射型 XSS - 搜索/错误 | ☐ | | | |
| 4 | DOM XSS - 前端处理 | ☐ | | | |
| 5 | XSS 通过文件上传 | ☐ | | | |

---

## XSS Payload 清单

```
// 基础
<script>alert('XSS')</script>
<img src=x onerror=alert('XSS')>
<svg onload=alert('XSS')>

// 绕过过滤
<ScRiPt>alert('XSS')</ScRiPt>
<img src=x onerror="alert('XSS')">
<img src=x onerror='alert(`XSS`)'>
<body onload=alert('XSS')>
<iframe src="javascript:alert('XSS')">

// 编码绕过
<img src=x onerror=&#97;&#108;&#101;&#114;&#116;('XSS')>
<a href="&#106;&#97;&#118;&#97;&#115;&#99;&#114;&#105;&#112;&#116;:alert('XSS')">click</a>
```

---

## 参考资料

- [OWASP XSS Prevention](https://cheatsheetseries.owasp.org/cheatsheets/Cross_Site_Scripting_Prevention_Cheat_Sheet.html)
- [OWASP XSS Filter Evasion](https://owasp.org/www-community/xss-filter-evasion-cheatsheet)
- [CWE-79: Cross-site Scripting](https://cwe.mitre.org/data/definitions/79.html)
- [PortSwigger XSS](https://portswigger.net/web-security/cross-site-scripting)
