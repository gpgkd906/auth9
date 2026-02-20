# 授权安全 - RBAC 权限绕过测试

**模块**: 授权安全
**测试范围**: 角色权限检查绕过
**场景数**: 5
**风险等级**: 🔴 极高
**ASVS 5.0 矩阵ID**: M-AUTHZ-02
**OWASP ASVS 5.0**: V8.1,V8.3,V8.4
**回归任务映射**: Backlog #2, #9, #20


---

## 背景知识

Auth9 授权模型（RBAC + ABAC）：
- **Permission**: 最小权限单位 (如 `user:read`, `user:write`)
- **Role**: 权限集合，支持继承
- **User-Tenant-Role**: 用户在特定租户下的角色
- **ABAC Policy**: 在 RBAC 通过后执行的属性条件约束（`disabled/shadow/enforce`）

权限检查流程：
1. 解析 JWT Token
2. 提取 roles/permissions
3. 检查是否包含所需权限（RBAC）
4. 对启用 ABAC 的租户执行属性策略评估

---

## 场景 1：直接权限绕过

### 前置条件
- 普通用户 (无管理权限)
- 已知管理员 API 端点

### 攻击目标
验证无权限用户是否可访问管理功能

### 攻击步骤
1. 以普通用户登录
2. 尝试直接访问管理端点：
   - `POST /api/v1/users` (创建用户)
   - `DELETE /api/v1/users/{id}` (删除用户)
   - `PUT /api/v1/tenants/{id}` (修改租户)
   - `POST /api/v1/services` (创建服务)
3. 检查响应

### 预期安全行为
- 返回 403 Forbidden
- 不执行任何操作
- 记录未授权访问尝试

### 验证方法
```bash
# 普通用户 Token
USER_TOKEN="..."

# 尝试创建用户
curl -X POST -H "Authorization: Bearer $USER_TOKEN" \
  http://localhost:8080/api/v1/users \
  -H "Content-Type: application/json" \
  -d '{"email":"new@test.com","password":"Test123!"}'
# 预期: 403 {"error": "Insufficient permissions", "required": "user:create"}

# 尝试删除用户
curl -X DELETE -H "Authorization: Bearer $USER_TOKEN" \
  http://localhost:8080/api/v1/users/{user_id}
# 预期: 403

# 验证数据未被修改
SELECT COUNT(*) FROM users WHERE email = 'new@test.com';
# 预期: 0
```

### 修复建议
- 每个端点明确定义所需权限
- 使用权限检查中间件
- 默认拒绝原则
- 权限不足时不执行任何操作

---

## 场景 2：权限继承绕过

### 前置条件
- 角色继承结构：`viewer` → `editor` → `admin`
- 用户仅有 `viewer` 角色

### 攻击目标
验证角色继承是否可被绕过

### 攻击步骤
1. 检查 viewer 角色的权限
2. 尝试执行 editor 或 admin 权限的操作
3. 检查是否能通过请求参数提升权限

### 预期安全行为
- 仅能使用直接分配的权限
- 继承链正确计算
- 不能通过参数注入角色

### 验证方法
```bash
# 获取当前用户权限
curl -H "Authorization: Bearer $VIEWER_TOKEN" \
  http://localhost:8080/api/v1/users/me/permissions
# 应仅包含 viewer 权限

# 尝试编辑操作
curl -X PUT -H "Authorization: Bearer $VIEWER_TOKEN" \
  http://localhost:8080/api/v1/services/{id} \
  -d '{"name":"Modified"}'
# 预期: 403

# 检查角色继承计算
SELECT r.name, p.code
FROM roles r
JOIN role_permissions rp ON r.id = rp.role_id
JOIN permissions p ON p.id = rp.permission_id
WHERE r.name = 'viewer';
```

### 修复建议
- 正确实现角色继承计算
- 缓存计算后的权限集
- 单元测试覆盖继承逻辑
- 避免循环继承

---

## 场景 3：HTTP 方法绕过

### 前置条件
- API 对某些方法有权限限制

### 攻击目标
验证是否可通过 HTTP 方法绕过权限

### 攻击步骤
1. 找到受限端点 (如 `DELETE /api/v1/users/{id}`)
2. 尝试使用其他方法访问：
   - `GET` (可能返回敏感信息)
   - `POST` (X-HTTP-Method-Override)
   - `PATCH`
   - `OPTIONS`
3. 检查响应

### 预期安全行为
- 每个方法单独检查权限
- 不支持方法覆盖头
- OPTIONS 不泄露敏感信息

### 验证方法
```bash
# 检查是否支持方法覆盖
curl -X POST -H "Authorization: Bearer $USER_TOKEN" \
  -H "X-HTTP-Method-Override: DELETE" \
  http://localhost:8080/api/v1/users/{id}
# 预期: 不应执行 DELETE

# OPTIONS 请求
curl -X OPTIONS http://localhost:8080/api/v1/users/{id}
# 预期: 仅返回允许的方法，不执行操作

# PATCH vs PUT 权限
curl -X PATCH -H "Authorization: Bearer $USER_TOKEN" \
  http://localhost:8080/api/v1/users/{id} \
  -d '{"name":"Modified"}'
# 预期: 与 PUT 相同的权限检查
```

### 修复建议
- 为每个 HTTP 方法配置权限
- 禁用 X-HTTP-Method-Override
- 确保 PATCH/PUT 权限一致
- 正确处理 OPTIONS 请求

---

## 场景 4：参数级权限绕过

### 前置条件
- 用户有部分资源的权限

### 攻击目标
验证参数级别的权限检查

### 攻击步骤
1. 尝试批量操作绕过：
   - 批量删除包含无权限资源
   - 批量更新混入无权限资源
2. 尝试字段级绕过：
   - 更新只读字段 (如 `created_at`)
   - 更新敏感字段 (如 `role`)
3. 尝试嵌套资源访问

### 预期安全行为
- 批量操作检查每个资源
- 忽略或拒绝敏感字段更新
- 嵌套资源单独检查权限

### 验证方法
```bash
# 批量删除包含无权限资源
curl -X DELETE -H "Authorization: Bearer $TOKEN" \
  http://localhost:8080/api/v1/users \
  -d '{"ids": ["allowed_id", "forbidden_id"]}'
# 预期: 整体失败或仅删除有权限的

# 尝试更新敏感字段
curl -X PUT -H "Authorization: Bearer $USER_TOKEN" \
  http://localhost:8080/api/v1/users/me \
  -d '{"role_in_tenant": "owner", "created_at": "2020-01-01"}'
# 预期: 忽略 role_in_tenant 和 created_at

# 验证实际更新的字段
SELECT role_in_tenant, created_at FROM tenant_users WHERE user_id = '...';
```

### 修复建议
- 使用白名单定义可更新字段
- 批量操作逐个检查权限
- 敏感字段在 Service 层过滤
- 返回清晰的错误信息

---

## 场景 5：Token 权限与数据库不一致

### 前置条件
- 用户曾有某权限
- 权限已被撤销

### 攻击目标
验证权限撤销后 Token 是否仍有效

### 攻击步骤
1. 用户获取包含 `admin` 角色的 Token
2. 管理员撤销用户的 `admin` 角色
3. 用户使用旧 Token 访问管理功能
4. 检查是否仍有权限

### 预期安全行为
- Token 权限应与数据库实时同步
- 或 Token 短期过期强制刷新
- 敏感操作实时验证权限

### 验证方法
```bash
# 步骤 1: 获取 Token
ADMIN_TOKEN=$(get_token_for_user_with_admin_role)

# 步骤 2: 撤销角色
curl -X DELETE -H "Authorization: Bearer $SUPER_ADMIN_TOKEN" \
  http://localhost:8080/api/v1/users/{user_id}/tenants/{tenant_id}/roles/{admin_role_id}

# 步骤 3: 使用旧 Token
curl -H "Authorization: Bearer $ADMIN_TOKEN" \
  http://localhost:8080/api/v1/users
# 预期: 403 (如果实时验证) 或在 Token 过期后失败

# 验证数据库权限状态
SELECT * FROM user_tenant_roles WHERE tenant_user_id = '...';
```

### 修复建议
- 敏感操作实时查询数据库权限
- Token 有效期不超过 15 分钟
- 实现权限变更时的 Token 吊销
- 缓存权限时设置较短 TTL

---

## 检查清单

| # | 场景 | 状态 | 测试日期 | 测试人员 | 发现问题 |
|---|------|------|----------|----------|----------|
| 1 | 直接权限绕过 | ☐ | | | |
| 2 | 权限继承绕过 | ☐ | | | |
| 3 | HTTP 方法绕过 | ☐ | | | |
| 4 | 参数级权限绕过 | ☐ | | | |
| 5 | Token 权限与数据库不一致 | ☐ | | | |

---

## 参考资料

- [OWASP Access Control Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Access_Control_Cheat_Sheet.html)
- [CWE-285: Improper Authorization](https://cwe.mitre.org/data/definitions/285.html)
- [CWE-863: Incorrect Authorization](https://cwe.mitre.org/data/definitions/863.html)

---


---

## 标准化回归 Checklist（ASVS 5.0）

**矩阵ID**: M-AUTHZ-02  
**适用控制**: V8.1,V8.3,V8.4  
**关联任务**: Backlog #2, #9, #20  
**建议回归频率**: 每次发布前 + 缺陷修复后必跑  
**场景总数**: 5

### 执行清单
- [ ] M-AUTHZ-02-C01 | 控制: V8.1 | 任务: #2, #9, #20 | 动作: 执行文档内相关攻击步骤并记录证据
- [ ] M-AUTHZ-02-C02 | 控制: V8.3 | 任务: #2, #9, #20 | 动作: 执行文档内相关攻击步骤并记录证据
- [ ] M-AUTHZ-02-C03 | 控制: V8.4 | 任务: #2, #9, #20 | 动作: 执行文档内相关攻击步骤并记录证据

### 回归记录表
| 检查项ID | 执行结果(pass/fail) | 风险等级 | 证据（请求/响应/日志/截图） | 备注 |
|---|---|---|---|---|
|  |  |  |  |  |

### 退出准则
1. 所有检查项执行完成，且高风险项无 `fail`。
2. 如存在 `fail`，必须附带漏洞单号、修复计划和复测结论。
3. 回归报告需同时记录矩阵ID与 Backlog 任务号，便于跨版本追溯。
