# 授权安全 - 权限提升测试

**模块**: 授权安全
**测试范围**: 水平/垂直权限提升
**场景数**: 5
**风险等级**: 🔴 极高

---

## 背景知识

权限提升类型：
- **垂直提升**: 普通用户获取管理员权限
- **水平提升**: 用户 A 访问用户 B 的数据

Auth9 权限层级：
1. Platform Admin (平台管理员)
2. Tenant Owner (租户所有者)
3. Tenant Admin (租户管理员)
4. Tenant Member (租户成员)

---

## 场景 1：自我角色分配攻击

### 前置条件
- 租户成员账户

### 攻击目标
验证是否可以自我分配更高权限角色

### 攻击步骤
1. 以普通成员身份登录
2. 尝试为自己分配角色：
   - 直接调用角色分配 API
   - 修改请求中的 user_id 为自己
   - 通过批量操作包含自己
3. 检查是否成功提升权限

### 预期安全行为
- 用户不能为自己分配角色
- 返回 403 Forbidden
- 记录提权尝试

### 验证方法
```bash
# 普通成员 Token
MEMBER_TOKEN="..."

# 尝试自我分配 admin 角色
curl -X POST -H "Authorization: Bearer $MEMBER_TOKEN" \
  http://localhost:8080/api/v1/rbac/assign \
  -H "Content-Type: application/json" \
  -d '{
    "user_id": "'$SELF_USER_ID'",
    "tenant_id": "'$TENANT_ID'",
    "role_id": "'$ADMIN_ROLE_ID'"
  }'
# 预期: 403 "Cannot assign roles to yourself" 或 "Insufficient permissions"

# 验证数据库
SELECT * FROM user_tenant_roles
WHERE tenant_user_id IN (
  SELECT id FROM tenant_users
  WHERE user_id = '$SELF_USER_ID'
) AND role_id = '$ADMIN_ROLE_ID';
# 预期: 无记录
```

### 修复建议
- 禁止自我角色分配
- 角色分配需要更高权限
- 审计所有角色变更
- 敏感角色分配需要二次确认

---

## 场景 2：角色创建后门

### 前置条件
- 租户管理员账户

### 攻击目标
验证是否可以创建超出自身权限的角色

### 攻击步骤
1. 以租户管理员身份登录
2. 尝试创建角色并分配：
   - 平台级权限
   - 超出自身权限的权限
   - 系统保留权限
3. 将该角色分配给自己或同伙

### 预期安全行为
- 只能创建不超过自身权限的角色
- 平台级权限不可分配
- 系统角色不可创建

### 验证方法
```bash
# 租户管理员尝试创建带平台权限的角色
curl -X POST -H "Authorization: Bearer $TENANT_ADMIN_TOKEN" \
  http://localhost:8080/api/v1/roles \
  -H "Content-Type: application/json" \
  -d '{
    "name": "super_role",
    "service_id": "'$SERVICE_ID'",
    "permissions": ["platform:admin", "tenant:delete", "system:configure"]
  }'
# 预期: 400 "Cannot assign permissions you don't have"

# 尝试创建系统保留角色名
curl -X POST -H "Authorization: Bearer $TENANT_ADMIN_TOKEN" \
  http://localhost:8080/api/v1/roles \
  -d '{"name": "platform_admin", ...}'
# 预期: 400 "Reserved role name"
```

### 修复建议
- 验证创建者拥有所有要分配的权限
- 保留系统角色名列表
- 平台权限仅平台管理员可分配
- 角色创建需要审批流程 (可选)

---

## 场景 3：邀请链接权限提升

### 前置条件
- 租户管理员可发送邀请
- 邀请可指定角色

### 攻击目标
验证邀请机制是否可被利用提升权限

### 攻击步骤
1. 管理员创建带高权限角色的邀请
2. 攻击者获取邀请链接
3. 尝试修改邀请中的角色
4. 或使用过期/被撤销的邀请

### 预期安全行为
- 邀请 Token 包含签名的角色信息
- 不能篡改邀请中的角色
- 过期邀请无法使用

### 验证方法
```bash
# 正常邀请 (member 角色)
INVITE_TOKEN="..."

# 尝试篡改邀请 Token 中的角色
# (如果是 JWT 格式，尝试修改 payload)

# 接受邀请时尝试覆盖角色
curl -X POST http://localhost:8080/api/v1/invitations/accept \
  -d '{
    "token": "'$INVITE_TOKEN'",
    "role": "admin"
  }'
# 预期: 忽略 role 参数，使用邀请中的角色

# 验证最终角色
SELECT r.name FROM user_tenant_roles utr
JOIN roles r ON r.id = utr.role_id
WHERE tenant_user_id = '...';
# 预期: member (不是 admin)
```

### 修复建议
- 邀请 Token 签名包含角色
- 接受时不接受角色参数
- 验证邀请有效性和过期
- 高权限邀请需要审批

---

## 场景 4：租户所有权转移攻击

### 前置条件
- 租户存在多个管理员
- 了解所有权转移机制

### 攻击目标
验证租户所有权是否可被非法获取

### 攻击步骤
1. 以租户管理员身份登录 (非所有者)
2. 尝试将自己设为所有者：
   - 直接修改租户设置
   - 通过角色分配获取 owner
   - 删除当前所有者
3. 检查是否成功接管租户

### 预期安全行为
- 只有当前所有者可转移所有权
- owner 角色不可通过普通分配获得
- 不能删除唯一所有者

### 验证方法
```bash
# 管理员尝试自我提升为 owner
curl -X PUT -H "Authorization: Bearer $ADMIN_TOKEN" \
  http://localhost:8080/api/v1/tenants/{tenant_id} \
  -d '{"owner_id": "'$SELF_ID'"}'
# 预期: 403 "Only owner can transfer ownership"

# 尝试分配 owner 角色
curl -X POST -H "Authorization: Bearer $ADMIN_TOKEN" \
  http://localhost:8080/api/v1/rbac/assign \
  -d '{
    "user_id": "'$SELF_ID'",
    "role_id": "'$OWNER_ROLE_ID'"
  }'
# 预期: 403 "Cannot assign owner role"

# 尝试删除所有者
curl -X DELETE -H "Authorization: Bearer $ADMIN_TOKEN" \
  http://localhost:8080/api/v1/users/{owner_id}/tenants/{tenant_id}
# 预期: 400 "Cannot remove tenant owner"
```

### 修复建议
- 所有权转移需要当前所有者确认
- owner 角色特殊处理，不走普通分配
- 禁止删除唯一所有者
- 所有权变更发送通知

---

## 场景 5：API 密钥权限提升

### 前置条件
- 具有 API 密钥管理权限

### 攻击目标
验证 API 密钥是否可获取超出权限

### 攻击步骤
1. 创建 API 密钥时尝试指定高权限
2. 检查 API 密钥的 scope 限制
3. 使用 API 密钥尝试越权操作

### 预期安全行为
- API 密钥权限不超过创建者权限
- scope 严格限制
- 敏感操作不可通过 API 密钥执行

### 验证方法
```bash
# 普通用户创建 API 密钥
curl -X POST -H "Authorization: Bearer $USER_TOKEN" \
  http://localhost:8080/api/v1/api-keys \
  -d '{
    "name": "test-key",
    "scopes": ["admin:*", "platform:*"]
  }'
# 预期: scopes 被过滤为用户实际权限

# 使用 API 密钥尝试管理操作
curl -H "X-API-Key: $API_KEY" \
  http://localhost:8080/api/v1/tenants
# 预期: 403 如果 key 没有该权限
```

### 修复建议
- API 密钥继承创建者权限的子集
- 严格验证 scope 参数
- 敏感操作排除 API 密钥访问
- API 密钥有独立的审计日志

---

## 检查清单

| # | 场景 | 状态 | 测试日期 | 测试人员 | 发现问题 |
|---|------|------|----------|----------|----------|
| 1 | 自我角色分配攻击 | ☐ | | | |
| 2 | 角色创建后门 | ☐ | | | |
| 3 | 邀请链接权限提升 | ☐ | | | |
| 4 | 租户所有权转移攻击 | ☐ | | | |
| 5 | API 密钥权限提升 | ☐ | | | |

---

## 参考资料

- [OWASP Privilege Escalation](https://owasp.org/www-project-web-security-testing-guide/latest/4-Web_Application_Security_Testing/05-Authorization_Testing/03-Testing_for_Privilege_Escalation)
- [CWE-269: Improper Privilege Management](https://cwe.mitre.org/data/definitions/269.html)
- [CWE-250: Execution with Unnecessary Privileges](https://cwe.mitre.org/data/definitions/250.html)
