# 授权安全 - 租户隔离测试

**模块**: 授权安全
**测试范围**: 多租户数据隔离
**场景数**: 4
**风险等级**: 🔴 极高
**ASVS 5.0 矩阵ID**: M-AUTHZ-01
**OWASP ASVS 5.0**: V8.1,V8.2,V4.2
**回归任务映射**: Backlog #2, #20


---

## 背景知识

Auth9 是多租户系统，核心隔离要求：
- 用户只能访问其所属租户的数据
- 租户管理员只能管理本租户资源
- 平台管理员可跨租户操作

关键数据表：
- `tenants` - 租户信息
- `tenant_users` - 用户-租户关联
- `services` - 租户下的服务
- `roles` / `permissions` - 租户下的 RBAC

---

## 场景 1：跨租户数据访问 (IDOR)

### 前置条件
- 用户 A 属于租户 1
- 用户 A 不属于租户 2
- 租户 2 存在数据

### 攻击目标
验证是否可以访问其他租户的数据

### 攻击步骤
1. 以用户 A 身份登录
2. 尝试访问租户 2 的资源：
   - `GET /api/v1/tenants/{tenant_2_id}`
   - `GET /api/v1/tenants/{tenant_2_id}/users`
   - `GET /api/v1/services?tenant_id={tenant_2_id}`
3. 尝试枚举租户 ID

### 预期安全行为
- 返回 403 Forbidden
- 不泄露租户是否存在
- 审计日志记录访问尝试

### 验证方法
```bash
# 获取用户 A 的 Token (属于租户 1)
TOKEN_A="..."

# 尝试访问租户 2
curl -H "Authorization: Bearer $TOKEN_A" \
  http://localhost:8080/api/v1/tenants/{tenant_2_id}
# 预期: 403 {"error": "Access denied"}

# 尝试列出租户 2 的用户
curl -H "Authorization: Bearer $TOKEN_A" \
  http://localhost:8080/api/v1/tenants/{tenant_2_id}/users
# 预期: 403

# 检查审计日志
SELECT * FROM audit_logs
WHERE action = 'access_denied'
ORDER BY created_at DESC LIMIT 10;
```

### 修复建议
- 所有 API 检查租户归属
- 使用 JWT 中的 tenant_id 而非请求参数
- 实现租户作用域中间件
- 记录所有跨租户访问尝试

---

## 场景 2：批量操作租户泄露

### 前置条件
- 具有列表查询权限的用户

### 攻击目标
验证列表 API 是否泄露其他租户数据

### 攻击步骤
1. 调用各种列表 API：
   - `GET /api/v1/users`
   - `GET /api/v1/services`
   - `GET /api/v1/roles`
   - `GET /api/v1/audit-logs`
2. 检查返回数据是否仅限当前租户
3. 尝试通过分页/过滤枚举其他租户数据

### 预期安全行为
- 列表 API 自动过滤为当前租户
- 不返回其他租户的任何数据
- 分页不暴露总数信息

### 验证方法
```bash
# 列出用户
curl -H "Authorization: Bearer $TOKEN" \
  http://localhost:8080/api/v1/users
# 验证: 所有返回用户都属于当前租户

# 检查审计日志
curl -H "Authorization: Bearer $TOKEN" \
  http://localhost:8080/api/v1/audit-logs
# 验证: 仅返回当前租户的日志

# SQL 验证
SELECT COUNT(*) FROM (
  -- API 返回的用户 ID 列表
) AS api_users
WHERE user_id NOT IN (
  SELECT user_id FROM tenant_users WHERE tenant_id = 'current_tenant'
);
# 预期: 0
```

### 修复建议
- 所有查询默认添加租户过滤
- Repository 层强制租户隔离
- 使用租户作用域的数据库连接
- 单元测试覆盖隔离逻辑

---

## 场景 3：关联资源跨租户访问

### 前置条件
- 用户 A 属于租户 1
- 租户 2 下有服务、角色等资源

### 攻击目标
验证关联资源的跨租户访问

### 攻击步骤
1. 获取租户 2 的服务 ID
2. 尝试操作该服务：
   - `GET /api/v1/services/{tenant_2_service_id}`
   - `PUT /api/v1/services/{tenant_2_service_id}`
   - `GET /api/v1/services/{tenant_2_service_id}/roles`
3. 尝试将角色分配给租户 2 的用户

### 预期安全行为
- 所有资源访问检查租户归属
- 关联操作验证双方租户一致性
- 返回 403 或 404

### 验证方法
```bash
# 访问其他租户的服务
curl -H "Authorization: Bearer $TOKEN_A" \
  http://localhost:8080/api/v1/services/{tenant_2_service_id}
# 预期: 403 或 404

# 尝试为其他租户用户分配角色
curl -X POST -H "Authorization: Bearer $TOKEN_A" \
  http://localhost:8080/api/v1/rbac/assign \
  -d '{
    "user_id": "'$TENANT_2_USER_ID'",
    "tenant_id": "'$TENANT_1_ID'",
    "role_id": "'$ROLE_ID'"
  }'
# 预期: 400 "User not in tenant"
```

### 修复建议
- 资源访问先查询归属租户
- 关联操作验证所有实体租户一致
- 使用数据库约束或应用层检查
- 防止 ID 猜测 (使用 UUID)

---

## 场景 4：管理员权限边界测试

### 前置条件
- 租户 1 的管理员
- 平台管理员

### 攻击目标
验证不同管理员的权限边界

### 攻击步骤
1. 租户管理员尝试：
   - 访问其他租户
   - 创建新租户
   - 访问平台级设置
2. 检查权限边界是否正确

### 预期安全行为
- 租户管理员仅能管理本租户
- 平台管理员才能创建/管理租户
- 系统设置仅平台管理员可访问

### 验证方法
```bash
# 租户管理员尝试创建租户
curl -X POST -H "Authorization: Bearer $TENANT_ADMIN_TOKEN" \
  http://localhost:8080/api/v1/tenants \
  -d '{"name":"New Tenant","slug":"new-tenant"}'
# 预期: 403 "Platform admin required"

# 租户管理员尝试访问系统设置
curl -H "Authorization: Bearer $TENANT_ADMIN_TOKEN" \
  http://localhost:8080/api/v1/system/email
# 预期: 403

# 验证权限矩阵
```

### 修复建议
- 明确定义权限层级
- API 层检查调用者角色
- 敏感端点添加额外校验
- 完善权限矩阵文档

---

## 检查清单

| # | 场景 | 状态 | 测试日期 | 测试人员 | 发现问题 |
|---|------|------|----------|----------|----------|
| 1 | 跨租户数据访问 (IDOR) | ☐ | | | |
| 2 | 批量操作租户泄露 | ☐ | | | |
| 3 | 关联资源跨租户访问 | ☐ | | | |
| 4 | 管理员权限边界测试 | ☐ | | | |

---

## 参考资料

- [OWASP IDOR Prevention](https://cheatsheetseries.owasp.org/cheatsheets/Insecure_Direct_Object_Reference_Prevention_Cheat_Sheet.html)
- [CWE-639: Authorization Bypass Through User-Controlled Key](https://cwe.mitre.org/data/definitions/639.html)
- [Multi-tenancy Security Best Practices](https://docs.microsoft.com/en-us/azure/architecture/guide/multitenant/considerations/security)

---


---

## 标准化回归 Checklist（ASVS 5.0）

**矩阵ID**: M-AUTHZ-01  
**适用控制**: V8.1,V8.2,V4.2  
**关联任务**: Backlog #2, #20  
**建议回归频率**: 每次发布前 + 缺陷修复后必跑  
**场景总数**: 4

### 执行清单
- [ ] M-AUTHZ-01-C01 | 控制: V8.1 | 任务: #2, #20 | 动作: 执行文档内相关攻击步骤并记录证据
- [ ] M-AUTHZ-01-C02 | 控制: V8.2 | 任务: #2, #20 | 动作: 执行文档内相关攻击步骤并记录证据
- [ ] M-AUTHZ-01-C03 | 控制: V4.2 | 任务: #2, #20 | 动作: 执行文档内相关攻击步骤并记录证据

### 回归记录表
| 检查项ID | 执行结果(pass/fail) | 风险等级 | 证据（请求/响应/日志/截图） | 备注 |
|---|---|---|---|---|
|  |  |  |  |  |

### 退出准则
1. 所有检查项执行完成，且高风险项无 `fail`。
2. 如存在 `fail`，必须附带漏洞单号、修复计划和复测结论。
3. 回归报告需同时记录矩阵ID与 Backlog 任务号，便于跨版本追溯。
