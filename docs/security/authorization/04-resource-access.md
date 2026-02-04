# 授权安全 - 资源访问控制测试

**模块**: 授权安全
**测试范围**: 资源级访问控制
**场景数**: 5
**风险等级**: 🟠 高

---

## 背景知识

Auth9 资源访问模型：
- 每个资源属于特定租户
- 资源间存在层级关系 (Tenant → Service → Role → Permission)
- 访问控制基于资源所有权和 RBAC

---

## 场景 1：IDOR (不安全直接对象引用)

### 前置条件
- 用户 A 拥有资源 1
- 用户 B 拥有资源 2
- 用户 A 已知资源 2 的 ID

### 攻击目标
验证是否可通过 ID 访问他人资源

### 攻击步骤
1. 以用户 A 身份登录
2. 获取用户 B 的资源 ID
3. 直接访问：
   - `GET /api/v1/services/{resource_2_id}`
   - `PUT /api/v1/services/{resource_2_id}`
   - `DELETE /api/v1/services/{resource_2_id}`
4. 检查是否能访问或修改

### 预期安全行为
- 返回 403 或 404
- 不泄露资源是否存在
- 不执行任何操作

### 验证方法
```bash
# 用户 A 的 Token
TOKEN_A="..."
# 用户 B 的服务 ID
SERVICE_B_ID="..."

# 读取
curl -H "Authorization: Bearer $TOKEN_A" \
  http://localhost:8080/api/v1/services/$SERVICE_B_ID
# 预期: 403 或 404

# 修改
curl -X PUT -H "Authorization: Bearer $TOKEN_A" \
  http://localhost:8080/api/v1/services/$SERVICE_B_ID \
  -d '{"name":"hacked"}'
# 预期: 403 或 404

# 删除
curl -X DELETE -H "Authorization: Bearer $TOKEN_A" \
  http://localhost:8080/api/v1/services/$SERVICE_B_ID
# 预期: 403 或 404

# 验证资源未被修改
SELECT * FROM services WHERE id = '$SERVICE_B_ID';
```

### 修复建议
- 每次访问验证资源归属
- 使用不可预测的 ID (UUID v4)
- 404 和 403 返回相同响应 (防止枚举)
- 记录可疑访问模式

---

## 场景 2：路径遍历访问

### 前置条件
- 已知资源层级结构

### 攻击目标
验证是否可通过路径遍历访问未授权资源

### 攻击步骤
1. 分析 API 路径结构
2. 尝试路径遍历：
   - `/api/v1/tenants/../admin/settings`
   - `/api/v1/services/../../tenants/{other_tenant}`
   - `/api/v1/users/./././{admin_id}`
3. 检查响应

### 预期安全行为
- 路径规范化处理
- 不接受 `..` 或 `.` 序列
- 返回 400 或 404

### 验证方法
```bash
# 路径遍历尝试
curl -H "Authorization: Bearer $TOKEN" \
  "http://localhost:8080/api/v1/tenants/../admin/config"
# 预期: 400 或 404

curl -H "Authorization: Bearer $TOKEN" \
  "http://localhost:8080/api/v1/users/%2e%2e/admin"
# 预期: 400 或 404 (URL 编码的 ..)

# 检查服务器日志是否有异常
```

### 修复建议
- URL 路径规范化
- 拒绝包含 `..` 的路径
- 使用路由框架的安全机制
- WAF 规则阻止路径遍历

---

## 场景 3：批量操作越权

### 前置条件
- 用户有部分资源权限

### 攻击目标
验证批量操作是否检查每个资源的权限

### 攻击步骤
1. 获取有权限和无权限的资源 ID
2. 执行批量操作：
   - 批量删除混入无权限资源
   - 批量更新包含无权限资源
   - 批量导出包含无权限数据
3. 检查结果

### 预期安全行为
- 检查每个资源的权限
- 部分失败应明确报告
- 或整体事务回滚

### 验证方法
```bash
# 批量删除
curl -X POST -H "Authorization: Bearer $TOKEN" \
  http://localhost:8080/api/v1/services/batch-delete \
  -d '{"ids": ["allowed_1", "forbidden_1", "allowed_2"]}'

# 可能的响应:
# 1. 全部失败: 400 "Permission denied for resource: forbidden_1"
# 2. 部分成功: 207 Multi-Status with details
# 3. 错误: 只删除了有权限的 (需验证)

# 验证实际删除情况
SELECT id, deleted_at FROM services WHERE id IN ('allowed_1', 'forbidden_1', 'allowed_2');
```

### 修复建议
- 批量操作前验证所有资源
- 使用事务保证原子性
- 返回详细的成功/失败状态
- 限制批量操作数量

---

## 场景 4：关联资源泄露

### 前置条件
- 资源间存在关联关系

### 攻击目标
验证通过关联资源是否能访问未授权数据

### 攻击步骤
1. 访问有权限的资源
2. 通过关联字段尝试扩展访问：
   - 服务的权限列表
   - 角色的用户列表
   - 用户的所有租户
3. 检查是否泄露未授权数据

### 预期安全行为
- 关联数据也需要权限检查
- 不泄露跨租户关联
- 敏感关联需要额外权限

### 验证方法
```bash
# 访问服务详情 (包含关联的角色)
curl -H "Authorization: Bearer $TOKEN" \
  http://localhost:8080/api/v1/services/{id}?include=roles,permissions
# 验证返回的 roles 都属于同一租户

# 查看角色的用户列表
curl -H "Authorization: Bearer $TOKEN" \
  http://localhost:8080/api/v1/roles/{id}/users
# 验证不包含其他租户用户

# 查询用户详情
curl -H "Authorization: Bearer $TOKEN" \
  http://localhost:8080/api/v1/users/{id}?include=tenants
# 验证仅返回当前租户信息
```

### 修复建议
- 关联查询添加租户过滤
- 敏感关联需要额外权限
- 使用字段级别访问控制
- GraphQL 场景特别注意深度查询

---

## 场景 5：软删除资源访问

### 前置条件
- 系统使用软删除
- 存在已软删除的资源

### 攻击目标
验证是否能访问已删除的资源

### 攻击步骤
1. 记录将要删除的资源 ID
2. 删除资源
3. 尝试访问已删除资源：
   - 直接通过 ID 访问
   - 通过搜索/列表
   - 通过关联查询
4. 检查是否能恢复或访问

### 预期安全行为
- 软删除资源不可访问
- 列表不返回已删除资源
- 关联查询排除已删除
- 恢复需要特殊权限

### 验证方法
```bash
# 删除资源
curl -X DELETE -H "Authorization: Bearer $ADMIN_TOKEN" \
  http://localhost:8080/api/v1/services/{id}

# 尝试访问
curl -H "Authorization: Bearer $TOKEN" \
  http://localhost:8080/api/v1/services/{id}
# 预期: 404

# 列表是否包含
curl -H "Authorization: Bearer $TOKEN" \
  "http://localhost:8080/api/v1/services?include_deleted=true"
# 预期: 不支持 include_deleted 或需要特殊权限

# 数据库验证
SELECT * FROM services WHERE id = '{id}';
# 应有 deleted_at 字段
```

### 修复建议
- 所有查询默认排除软删除
- 恢复功能需要管理员权限
- 定期硬删除过期数据
- 软删除数据加密或脱敏

---

## 检查清单

| # | 场景 | 状态 | 测试日期 | 测试人员 | 发现问题 |
|---|------|------|----------|----------|----------|
| 1 | IDOR 直接对象引用 | ☐ | | | |
| 2 | 路径遍历访问 | ☐ | | | |
| 3 | 批量操作越权 | ☐ | | | |
| 4 | 关联资源泄露 | ☐ | | | |
| 5 | 软删除资源访问 | ☐ | | | |

---

## 参考资料

- [OWASP IDOR](https://owasp.org/www-project-web-security-testing-guide/latest/4-Web_Application_Security_Testing/05-Authorization_Testing/04-Testing_for_Insecure_Direct_Object_References)
- [CWE-639: Authorization Bypass Through User-Controlled Key](https://cwe.mitre.org/data/definitions/639.html)
- [CWE-22: Path Traversal](https://cwe.mitre.org/data/definitions/22.html)
