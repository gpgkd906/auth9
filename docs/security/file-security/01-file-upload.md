# 文件安全 - 文件上传安全测试

**模块**: 文件与资源安全
**测试范围**: 文件上传验证、存储安全、下载授权
**场景数**: 4
**风险等级**: 🟠 高
**OWASP ASVS**: V12.1, V12.2, V12.4

---

## 背景知识

Auth9 中涉及文件处理的功能：
- **头像上传**: 用户/租户 Logo 图片上传
- **Branding 资源**: 自定义品牌素材
- **Email 模板**: 可能包含嵌入资源
- **导出功能**: 数据导出生成文件

文件上传攻击可导致：远程代码执行、XSS（恶意 SVG/HTML）、拒绝服务、目录遍历。

---

## 场景 1：文件类型验证绕过

### 前置条件
- 具有文件上传权限的 Token
- 准备各种格式的恶意文件

### 攻击目标
验证文件上传是否仅通过 Content-Type 头验证，可被绕过

### 攻击步骤
1. 上传正常图片确认功能工作
2. 修改 Content-Type 为 `image/png`，但文件内容为 PHP/HTML/JS
3. 使用双扩展名：`avatar.php.png`, `avatar.html.jpg`
4. 使用空字节截断：`avatar.png%00.php`
5. 上传 SVG 文件（含 `<script>` 标签）
6. 上传 polyglot 文件（同时是有效图片和有效 HTML）

### 预期安全行为
- 基于文件内容（magic bytes）验证，不仅依赖 Content-Type
- 拒绝非图片格式文件
- 双扩展名和空字节截断被检测
- SVG 文件被拒绝或清理危险标签
- 返回文件时设置安全的 Content-Type

### 验证方法
```bash
# 正常图片上传
curl -X POST -H "Authorization: Bearer $TOKEN" \
  -F "file=@test.png" \
  http://localhost:8080/api/v1/users/me/avatar
# 预期: 200

# 伪装 Content-Type
echo '<?php phpinfo(); ?>' > /tmp/evil.php
curl -X POST -H "Authorization: Bearer $TOKEN" \
  -F "file=@/tmp/evil.php;type=image/png" \
  http://localhost:8080/api/v1/users/me/avatar
# 预期: 400 - Invalid file type

# 双扩展名
cp test.png /tmp/test.php.png
curl -X POST -H "Authorization: Bearer $TOKEN" \
  -F "file=@/tmp/test.php.png" \
  http://localhost:8080/api/v1/users/me/avatar
# 预期: 400 或仅保存为 .png

# SVG with script
cat > /tmp/evil.svg << 'EOF'
<svg xmlns="http://www.w3.org/2000/svg">
  <script>alert('XSS')</script>
  <rect width="100" height="100" fill="red"/>
</svg>
EOF
curl -X POST -H "Authorization: Bearer $TOKEN" \
  -F "file=@/tmp/evil.svg" \
  http://localhost:8080/api/v1/users/me/avatar
# 预期: 400 - SVG not allowed，或 script 标签被清理

# HTML 伪装
echo '<html><body><script>alert(1)</script></body></html>' > /tmp/evil.html
curl -X POST -H "Authorization: Bearer $TOKEN" \
  -F "file=@/tmp/evil.html;type=image/jpeg;filename=avatar.jpg" \
  http://localhost:8080/api/v1/users/me/avatar
# 预期: 400 - 通过 magic bytes 检测非图片
```

### 修复建议
- 使用 magic bytes 验证文件实际类型（如 Rust `infer` crate）
- 白名单允许的文件类型（如仅 PNG/JPEG/WebP）
- 拒绝 SVG 或使用 SVG sanitizer 清理
- 重命名文件为随机 UUID，丢弃原始扩展名
- 返回文件时设置 `Content-Type: image/png` 和 `Content-Disposition: inline`

---

## 场景 2：文件大小与资源耗尽

### 前置条件
- 文件上传端点
- 能够生成大文件

### 攻击目标
验证文件上传是否有大小限制，防止磁盘或内存耗尽

### 攻击步骤
1. 上传 1MB 图片（正常大小）
2. 上传 100MB 图片（超大）
3. 上传 1GB 图片（极端情况）
4. 发送 `Content-Length: 999999999` 但缓慢传输数据（Slow POST）
5. 上传 zip bomb（小文件解压后极大）
6. 并发上传大量小文件消耗文件描述符

### 预期安全行为
- 文件大小限制（如 ≤ 5MB）
- 请求体大小限制在 Web 框架层
- 超大 Content-Length 在读取完整数据前被拒绝
- 并发上传有频率限制
- 返回 413 Payload Too Large

### 验证方法
```bash
# 生成测试文件
dd if=/dev/urandom of=/tmp/large.bin bs=1M count=100

# 上传超大文件
curl -X POST -H "Authorization: Bearer $TOKEN" \
  -F "file=@/tmp/large.bin;type=image/png" \
  http://localhost:8080/api/v1/users/me/avatar
# 预期: 413 Payload Too Large

# 测试请求体限制
curl -X POST -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/octet-stream" \
  -H "Content-Length: 999999999" \
  --data-binary @/dev/zero \
  --max-time 10 \
  http://localhost:8080/api/v1/users/me/avatar
# 预期: 连接在读取限制大小后被断开

# 并发上传
seq 1 100 | parallel -j50 \
  "curl -s -o /dev/null -w '%{http_code}\n' \
    -X POST -H 'Authorization: Bearer $TOKEN' \
    -F 'file=@test.png' \
    http://localhost:8080/api/v1/users/me/avatar"
# 预期: 前几个成功，后续被限流 (429)
```

### 修复建议
- axum/tower 层设置 `content_length_limit`
- 流式读取文件，不一次性加载到内存
- 文件大小限制：头像 ≤ 2MB，其他 ≤ 10MB
- 每用户上传频率限制
- 磁盘使用监控和告警

---

## 场景 3：文件存储路径遍历

### 前置条件
- 文件上传功能
- 了解文件存储路径结构

### 攻击目标
验证上传文件名是否可被利用进行目录遍历

### 攻击步骤
1. 上传文件名包含路径遍历字符：`../../etc/crontab`
2. 上传文件名包含 URL 编码遍历：`..%2F..%2Fetc%2Fpasswd`
3. 上传文件名包含 null 字节：`avatar.png\x00../../etc/passwd`
4. 上传文件名包含特殊字符：`avatar\n.png`, `avatar;.png`

### 预期安全行为
- 服务端忽略客户端提供的文件名，使用随机生成的文件名
- 路径遍历字符被过滤
- 文件存储在固定目录下，不受用户输入影响
- null 字节被正确处理

### 验证方法
```bash
# 路径遍历文件名
curl -X POST -H "Authorization: Bearer $TOKEN" \
  -F "file=@test.png;filename=../../etc/crontab" \
  http://localhost:8080/api/v1/users/me/avatar
# 预期: 200 但文件名被忽略/重命名

# URL 编码遍历
curl -X POST -H "Authorization: Bearer $TOKEN" \
  -F "file=@test.png;filename=..%2F..%2Fetc%2Fpasswd" \
  http://localhost:8080/api/v1/users/me/avatar
# 预期: 200 但文件安全存储

# 检查实际存储路径
# 如果可以访问存储目录，验证文件名是 UUID 而非用户提供的名称
ls -la /path/to/upload/dir/
# 预期: 文件名为 uuid.png 格式

# Null byte
curl -X POST -H "Authorization: Bearer $TOKEN" \
  -F "file=@test.png;filename=avatar.png%00../../etc/passwd" \
  http://localhost:8080/api/v1/users/me/avatar
# 预期: 正常处理，忽略 null 字节后的内容
```

### 修复建议
- 服务端始终使用随机生成的文件名（UUID）
- 文件存储路径由服务端完全控制，不包含用户输入
- 使用 Rust 的 `Path::file_name()` 提取纯文件名
- 过滤 `..`, `/`, `\`, null 字节等特殊字符

---

## 场景 4：文件下载授权验证

### 前置条件
- 已上传的文件（不同用户/租户）
- 文件访问 URL

### 攻击目标
验证文件下载是否有访问控制，防止越权访问其他用户/租户的文件

### 攻击步骤
1. 用户 A 上传文件，获取文件 URL
2. 用户 B 尝试直接访问用户 A 的文件 URL
3. 尝试枚举文件 URL（如递增 ID 或可预测的文件名）
4. 不带认证 Token 直接访问文件 URL
5. 使用其他租户的 Token 访问文件

### 预期安全行为
- 文件 URL 不可预测（使用 UUID 或签名 URL）
- 文件下载需要认证
- 跨用户/跨租户文件访问被拒绝
- 未认证访问返回 401
- 文件 URL 有时效性（签名 URL 过期机制）

### 验证方法
```bash
# 用户 A 上传文件
UPLOAD=$(curl -s -X POST -H "Authorization: Bearer $TOKEN_A" \
  -F "file=@test.png" \
  http://localhost:8080/api/v1/users/me/avatar)
FILE_URL=$(echo $UPLOAD | jq -r '.url')

# 用户 B 尝试访问
curl -s -o /dev/null -w "%{http_code}" \
  -H "Authorization: Bearer $TOKEN_B" \
  "$FILE_URL"
# 预期: 403 Forbidden

# 无认证访问
curl -s -o /dev/null -w "%{http_code}" "$FILE_URL"
# 预期: 401 Unauthorized

# URL 枚举
# 如果 URL 包含 UUID，尝试修改 UUID
MODIFIED_URL=$(echo $FILE_URL | sed 's/[0-9a-f]\{8\}/00000000/')
curl -s -o /dev/null -w "%{http_code}" \
  -H "Authorization: Bearer $TOKEN_A" \
  "$MODIFIED_URL"
# 预期: 404 Not Found

# 检查响应头
curl -s -I -H "Authorization: Bearer $TOKEN_A" "$FILE_URL"
# 预期包含:
# Content-Type: image/png
# X-Content-Type-Options: nosniff
# Content-Disposition: inline (或 attachment)
# Cache-Control: private
```

### 修复建议
- 文件 URL 使用 UUID，不可枚举
- 文件下载需验证请求者与文件所有者的关系
- 考虑使用签名 URL（预签名 + 过期时间）
- 返回文件时设置 `X-Content-Type-Options: nosniff`
- 非图片文件使用 `Content-Disposition: attachment`

---

## 检查清单

| # | 场景 | 状态 | 测试日期 | 测试人员 | 发现问题 |
|---|------|------|----------|----------|----------|
| 1 | 文件类型验证绕过 | ☐ | | | |
| 2 | 文件大小与资源耗尽 | ☐ | | | |
| 3 | 文件存储路径遍历 | ☐ | | | |
| 4 | 文件下载授权验证 | ☐ | | | |

---

## 参考资料

- [OWASP File Upload Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/File_Upload_Cheat_Sheet.html)
- [CWE-434: Unrestricted Upload of File with Dangerous Type](https://cwe.mitre.org/data/definitions/434.html)
- [CWE-22: Path Traversal](https://cwe.mitre.org/data/definitions/22.html)
- [CWE-400: Uncontrolled Resource Consumption](https://cwe.mitre.org/data/definitions/400.html)
