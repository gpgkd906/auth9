# 输入验证 - 参数篡改测试

**模块**: 输入验证
**测试范围**: 请求参数操纵
**场景数**: 4
**风险等级**: 🟡 中

---

## 背景知识

参数篡改攻击类型：
- **隐藏字段篡改**: 修改表单中的隐藏值
- **URL 参数篡改**: 修改查询字符串
- **请求体篡改**: 修改 POST/PUT 数据
- **Header 篡改**: 修改 HTTP 头

---

## 场景 1：隐藏/只读字段篡改

### 前置条件
- 具有资源编辑权限的用户

### 攻击目标
验证是否可以修改应该只读的字段

### 攻击步骤
1. 分析 API 请求和响应
2. 在更新请求中添加只读字段：
   - `id` - 资源 ID
   - `created_at` - 创建时间
   - `created_by` - 创建者
   - `tenant_id` - 租户 ID
   - `keycloak_id` - 外部 ID
3. 检查字段是否被修改

### 预期安全行为
- 忽略只读字段
- 或返回错误
- 不修改敏感字段

### 验证方法
```bash
# 尝试修改只读字段
curl -X PUT -H "Authorization: Bearer $TOKEN" \
  http://localhost:8080/api/v1/users/me \
  -H "Content-Type: application/json" \
  -d '{
    "display_name": "New Name",
    "id": "different-uuid",
    "created_at": "2020-01-01T00:00:00Z",
    "tenant_id": "other-tenant-id",
    "keycloak_id": "fake-keycloak-id"
  }'

# 验证修改结果
curl -H "Authorization: Bearer $TOKEN" \
  http://localhost:8080/api/v1/users/me
# 预期: id, created_at, tenant_id, keycloak_id 未变

# 数据库验证
SELECT id, created_at, keycloak_id FROM users WHERE id = '...';
```

### 修复建议
- 定义可更新字段白名单
- Service 层过滤请求字段
- 使用 DTO 模式
- 敏感字段从数据库保留

---

## 场景 2：类型混淆攻击

### 前置条件
- 具有创建/更新权限

### 攻击目标
验证参数类型验证是否严格

### 攻击步骤
1. 发送类型错误的参数：
   - 数字字段发送字符串: `"age": "twenty"`
   - 布尔字段发送字符串: `"active": "yes"`
   - 数组字段发送对象: `"ids": {"0": "id1"}`
   - 字符串字段发送数组: `"name": ["a", "b"]`
2. 发送特殊值：
   - `null`
   - `undefined`
   - 空字符串
   - 超长字符串
3. 检查服务器行为

### 预期安全行为
- 严格类型验证
- 返回 400 Bad Request
- 不崩溃或异常

### 验证方法
```bash
# 数字字段发送字符串
curl -X POST -H "Authorization: Bearer $TOKEN" \
  http://localhost:8080/api/v1/services \
  -H "Content-Type: application/json" \
  -d '{"name": "test", "timeout": "not-a-number"}'
# 预期: 400 Invalid type for timeout

# 布尔字段混淆
curl -X PUT -H "Authorization: Bearer $TOKEN" \
  http://localhost:8080/api/v1/users/me \
  -d '{"mfa_enabled": "true"}'
# 预期: 布尔 true 而非字符串 "true"

# 数组注入
curl -X POST -H "Authorization: Bearer $TOKEN" \
  http://localhost:8080/api/v1/rbac/assign \
  -d '{"role_id": ["role1", "role2"]}'
# 预期: 400 或仅处理第一个
```

### 修复建议
- 使用强类型语言特性
- Schema 验证 (JSON Schema)
- 明确拒绝错误类型
- 不进行隐式类型转换

---

## 场景 3：边界值测试

### 前置条件
- 了解字段的预期范围

### 攻击目标
验证边界条件处理

### 攻击步骤
1. 测试数值边界：
   - 最大整数: `2147483647`, `9223372036854775807`
   - 负数: `-1`, `-999999`
   - 零: `0`
   - 小数: `0.1`, `1.999999999`
2. 测试字符串边界：
   - 空字符串: `""`
   - 超长字符串: 10000+ 字符
   - Unicode: emoji, RTL 文字
   - 特殊字符: NULL 字节, 控制字符
3. 测试数组边界：
   - 空数组: `[]`
   - 大量元素: 10000+ 项

### 预期安全行为
- 合理的长度/范围限制
- 溢出保护
- 资源限制

### 验证方法
```bash
# 超长名称
curl -X POST -H "Authorization: Bearer $TOKEN" \
  http://localhost:8080/api/v1/tenants \
  -d "{\"name\": \"$(python3 -c 'print("A"*10000)')\", \"slug\": \"test\"}"
# 预期: 400 Name too long (max 255)

# 大量 ID 批量操作
curl -X DELETE -H "Authorization: Bearer $TOKEN" \
  http://localhost:8080/api/v1/services/batch \
  -d "{\"ids\": [$(seq -s, 1 10000)]}"
# 预期: 400 Too many items (max 100)

# 负数分页
curl -H "Authorization: Bearer $TOKEN" \
  "http://localhost:8080/api/v1/users?page=-1&limit=-10"
# 预期: 使用默认值或 400

# Unicode 测试
curl -X PUT -H "Authorization: Bearer $TOKEN" \
  http://localhost:8080/api/v1/users/me \
  -d '{"display_name": "Test 🎉 العربية 中文"}'
# 预期: 正常处理
```

### 修复建议
- 定义并验证字段长度限制
- 分页参数强制正整数
- 批量操作数量限制
- 正确处理 Unicode

---

## 场景 4：HTTP 方法/头篡改

### 前置条件
- 了解 API 端点

### 攻击目标
验证 HTTP 方法和头部处理

### 攻击步骤
1. 方法覆盖测试：
   - `X-HTTP-Method-Override: DELETE`
   - `X-HTTP-Method: PUT`
   - `_method=DELETE` (查询参数)
2. 头部注入测试：
   - `Host: evil.com`
   - `X-Forwarded-For: 127.0.0.1`
   - `X-Original-URL: /admin`
3. Content-Type 操纵

### 预期安全行为
- 不支持方法覆盖
- 验证关键头部
- 忽略或拒绝可疑头部

### 验证方法
```bash
# 方法覆盖
curl -X POST -H "Authorization: Bearer $TOKEN" \
  -H "X-HTTP-Method-Override: DELETE" \
  http://localhost:8080/api/v1/users/{id}
# 预期: 执行 POST 而非 DELETE

# Host 头注入
curl -H "Authorization: Bearer $TOKEN" \
  -H "Host: evil.com" \
  http://localhost:8080/api/v1/auth/password-reset
# 检查重置链接中的域名

# X-Forwarded-For 欺骗
curl -H "X-Forwarded-For: 127.0.0.1" \
  http://localhost:8080/api/v1/auth/login
# 检查是否绕过 IP 限制

# Content-Type 混淆
curl -X POST -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/xml" \
  http://localhost:8080/api/v1/users \
  -d '<user><email>test@test.com</email></user>'
# 预期: 400 或 415
```

### 修复建议
- 禁用方法覆盖头
- 固定 Host 头或验证
- 信任的代理 IP 列表
- 严格 Content-Type 验证

---

## 检查清单

| # | 场景 | 状态 | 测试日期 | 测试人员 | 发现问题 |
|---|------|------|----------|----------|----------|
| 1 | 隐藏/只读字段篡改 | ☐ | | | |
| 2 | 类型混淆攻击 | ☐ | | | |
| 3 | 边界值测试 | ☐ | | | |
| 4 | HTTP 方法/头篡改 | ☐ | | | |

---

## 参考资料

- [OWASP Input Validation](https://cheatsheetseries.owasp.org/cheatsheets/Input_Validation_Cheat_Sheet.html)
- [CWE-20: Improper Input Validation](https://cwe.mitre.org/data/definitions/20.html)
- [API Security Best Practices](https://owasp.org/www-project-api-security/)
