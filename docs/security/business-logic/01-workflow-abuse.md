# 业务逻辑 - 工作流滥用测试

**模块**: 业务逻辑安全
**测试范围**: 多步骤业务流程安全
**场景数**: 5
**风险等级**: 🔴 极高
**ASVS 5.0 矩阵ID**: M-BIZ-01
**OWASP ASVS 5.0**: V2.1,V2.2,V2.5,V8.2
**回归任务映射**: Backlog #1, #10, #20


---

## 背景知识

Auth9 包含多个关键业务流程，攻击者可能尝试篡改流程状态或跳过验证步骤：
- **Token Exchange 流程**: Identity Token → Tenant Access Token（核心授权链路）
- **邀请流程**: 创建邀请 → 发送邮件 → 接受邀请 → 加入租户
- **密码重置流程**: 请求重置 → 验证邮件 → 重置密码
- **租户生命周期**: 创建 → 配置 → 使用 → 删除
- **角色继承**: 父角色 → 子角色 → 权限解析

---

## 场景 1：Token Exchange 流程滥用

### 前置条件
- 有效的 Identity Token
- 了解 gRPC Token Exchange API 结构

### 攻击目标
验证 Token Exchange 是否可被滥用获取未授权的 Tenant Access Token

### 攻击步骤
1. 使用有效 Identity Token 调用 ExchangeToken
2. 尝试指定非所属租户的 `tenant_id`
3. 尝试指定不存在的 `service_id`
4. 篡改 Identity Token 中的 `sub` claim 后请求交换
5. 使用过期的 Identity Token 请求交换
6. 短时间内大量交换请求，检查是否有速率限制

### 预期安全行为
- 仅允许交换用户已加入的租户的 Token
- 验证 service_id 属于目标 tenant
- Identity Token 的签名验证在交换前完成
- 过期 Token 被拒绝
- 交换操作有审计日志

### 验证方法
```bash
# 正常交换（应成功）
grpcurl -plaintext \
  -H "x-api-key: $API_KEY" \
  -d '{
    "identity_token": "'$IDENTITY_TOKEN'",
    "tenant_id": "'$MY_TENANT_ID'",
    "service_id": "'$MY_SERVICE_ID'"
  }' \
  localhost:50051 auth9.TokenService/ExchangeToken
# 预期: 返回 Tenant Access Token

# 跨租户交换（应失败）
grpcurl -plaintext \
  -H "x-api-key: $API_KEY" \
  -d '{
    "identity_token": "'$IDENTITY_TOKEN'",
    "tenant_id": "'$OTHER_TENANT_ID'",
    "service_id": "'$OTHER_SERVICE_ID'"
  }' \
  localhost:50051 auth9.TokenService/ExchangeToken
# 预期: PERMISSION_DENIED - User is not a member of this tenant

# 伪造 Token 交换
FORGED_TOKEN=$(python3 -c "
import jwt
token = jwt.encode({'sub': 'admin-user-id', 'exp': 9999999999}, 'wrong-key', algorithm='HS256')
print(token)
")
grpcurl -plaintext \
  -H "x-api-key: $API_KEY" \
  -d '{"identity_token": "'$FORGED_TOKEN'", "tenant_id": "any"}' \
  localhost:50051 auth9.TokenService/ExchangeToken
# 预期: UNAUTHENTICATED - Invalid token signature
```

### 修复建议
- Token Exchange 前严格验证 Identity Token 签名
- 查询数据库确认用户-租户关联
- service_id 验证归属 tenant_id
- 记录所有 Exchange 操作的审计日志
- 对异常 Exchange 模式（大量失败）触发告警

---

## 场景 2：邀请流程篡改

### 前置条件
- 具有 `create:invitations` 权限的 Token
- 有效的邀请链接

### 攻击目标
验证邀请流程是否可被篡改以获取未授权的角色或租户访问

### 攻击步骤
1. 创建邀请并获取邀请 Token
2. 解码邀请 Token 查看其结构
3. 尝试修改邀请 Token 中的角色信息
4. 尝试使用同一邀请 Token 多次接受
5. 尝试在邀请过期后使用
6. 用不同邮箱的用户接受指定邮箱的邀请

### 预期安全行为
- 邀请 Token 不可篡改（签名验证或服务端状态）
- 邀请仅能使用一次
- 过期邀请被拒绝
- 邀请绑定特定邮箱，其他邮箱不可使用
- 角色信息从服务端数据库读取，不信任 Token 中的角色

### 验证方法
```bash
# 创建邀请
INVITATION=$(curl -s -X POST -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  http://localhost:8080/api/v1/invitations \
  -d '{"email": "invited@test.com", "role_ids": ["viewer-role-id"]}')
INVITE_TOKEN=$(echo $INVITATION | jq -r '.token')

# 第一次接受（应成功）
curl -X POST http://localhost:8080/api/v1/invitations/accept \
  -H "Authorization: Bearer $USER_TOKEN" \
  -d '{"token": "'$INVITE_TOKEN'"}'
# 预期: 200 OK

# 第二次接受（应失败）
curl -X POST http://localhost:8080/api/v1/invitations/accept \
  -H "Authorization: Bearer $USER_TOKEN" \
  -d '{"token": "'$INVITE_TOKEN'"}'
# 预期: 400/409 - Invitation already accepted

# 篡改角色 - 如果 token 是 JWT，修改 claims
# 如果 token 是 UUID，尝试用其他邀请的 token
curl -X POST http://localhost:8080/api/v1/invitations/accept \
  -H "Authorization: Bearer $DIFFERENT_USER_TOKEN" \
  -d '{"token": "'$INVITE_TOKEN'"}'
# 预期: 403 - Email mismatch
```

### 修复建议
- 邀请状态在数据库中管理，不依赖 Token 携带角色信息
- 接受邀请时验证当前用户邮箱与邀请邮箱匹配
- 原子化操作防止重复接受
- 邀请过期时间 ≤ 7 天

---

## 场景 3：租户生命周期攻击

### 前置条件
- 具有租户管理权限的 Token
- 至少两个租户

### 攻击目标
验证租户删除后关联资源是否正确清理，防止孤儿数据被利用

### 攻击步骤
1. 创建租户并添加用户、角色、服务
2. 记录所有关联资源的 ID
3. 删除租户
4. 使用记录的 ID 尝试直接访问已删除租户的资源
5. 尝试创建相同 slug 的新租户，检查是否继承旧数据
6. 检查已删除租户用户的 Token 是否仍然有效

### 预期安全行为
- 租户删除时级联清理所有关联数据（tenant_users, services, roles, webhooks, invitations）
- 删除租户后其资源不可通过 ID 直接访问
- slug 重用不会继承旧数据
- 已删除租户的 Token 在验证时失败（或进入黑名单）

### 验证方法
```bash
# 创建测试租户
TENANT=$(curl -s -X POST -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  http://localhost:8080/api/v1/tenants \
  -d '{"name": "Delete Test", "slug": "delete-test"}')
TENANT_ID=$(echo $TENANT | jq -r '.id')

# 创建关联资源
SERVICE_ID=$(curl -s -X POST -H "Authorization: Bearer $TENANT_TOKEN" \
  -H "Content-Type: application/json" \
  http://localhost:8080/api/v1/services \
  -d '{"name": "test-service"}' | jq -r '.id')

# 删除租户
curl -X DELETE -H "Authorization: Bearer $TOKEN" \
  http://localhost:8080/api/v1/tenants/$TENANT_ID
# 预期: 200/204

# 尝试访问已删除租户的服务
curl -H "Authorization: Bearer $TOKEN" \
  http://localhost:8080/api/v1/services/$SERVICE_ID
# 预期: 404 Not Found

# 重建同 slug 租户
curl -s -X POST -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  http://localhost:8080/api/v1/tenants \
  -d '{"name": "Delete Test Reborn", "slug": "delete-test"}'
# 预期: 新租户，不包含旧数据
```

### 修复建议
- 实现完整的级联删除（参照 CLAUDE.md 中的删除对象/关联表映射）
- 删除租户时将所有关联 Token 加入黑名单
- 软删除情况下确保软删除资源不可通过 API 访问
- 定期清理孤儿数据

---

## 场景 4：角色继承循环与权限爆炸

### 前置条件
- 具有 `create:roles` 和 `update:roles` 权限的 Token
- 支持角色父子关系的 RBAC 系统

### 攻击目标
验证角色继承是否能防止循环引用和权限爆炸

### 攻击步骤
1. 创建角色 A，设置父角色为 B
2. 更新角色 B，设置父角色为 C
3. 更新角色 C，设置父角色为 A（形成循环）
4. 查询用户权限，观察是否无限递归或崩溃
5. 创建深层继承链（>100 层），检查栈溢出
6. 创建大量角色互相继承，测试权限解析性能

### 预期安全行为
- 设置父角色时检测循环引用，返回错误
- 限制继承深度（如 ≤ 10 层）
- 权限解析有超时或递归深度限制
- 循环检测错误信息明确

### 验证方法
```bash
# 创建三个角色
ROLE_A=$(curl -s -X POST -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  http://localhost:8080/api/v1/roles \
  -d '{"name": "Role A", "service_id": "'$SERVICE_ID'"}' | jq -r '.id')

ROLE_B=$(curl -s -X POST -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  http://localhost:8080/api/v1/roles \
  -d '{"name": "Role B", "service_id": "'$SERVICE_ID'", "parent_role_id": "'$ROLE_A'"}' | jq -r '.id')

ROLE_C=$(curl -s -X POST -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  http://localhost:8080/api/v1/roles \
  -d '{"name": "Role C", "service_id": "'$SERVICE_ID'", "parent_role_id": "'$ROLE_B'"}' | jq -r '.id')

# 尝试创建循环
curl -X PUT -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  http://localhost:8080/api/v1/roles/$ROLE_A \
  -d '{"parent_role_id": "'$ROLE_C'"}'
# 预期: 400 - Circular role inheritance detected

# 检查深层继承性能
time curl -H "Authorization: Bearer $TOKEN" \
  http://localhost:8080/api/v1/users/$USER_ID/roles?tenant_id=$TENANT_ID
# 预期: 响应时间 < 500ms，即使有复杂继承链
```

### 修复建议
- 更新父角色时执行循环检测（图遍历/DFS）
- 限制最大继承深度
- 权限解析结果缓存
- 解析过程中加入已访问集合防止循环

---

## 场景 5：系统设置安全降级攻击

### 前置条件
- 具有 `update:settings` 权限的 Token（通常为管理员）

### 攻击目标
验证攻击者获取管理员权限后是否可以通过修改系统设置降低整体安全水位

### 攻击步骤
1. 读取当前系统设置
2. 尝试降低密码策略（最小长度设为 1，禁用复杂度要求）
3. 尝试禁用 MFA 要求
4. 尝试放宽 Rate Limit 配置
5. 尝试修改 Session 超时为极长时间
6. 检查这些修改是否有审计日志

### 预期安全行为
- 关键安全设置有最低阈值限制（如密码最小长度 ≥ 8）
- 安全降级操作需要二次确认或更高权限
- 所有设置变更记录到审计日志
- 安全设置变更触发告警通知

### 验证方法
```bash
# 尝试设置极弱密码策略
curl -X PUT -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  http://localhost:8080/api/v1/system/settings \
  -d '{
    "password_policy": {
      "min_length": 1,
      "require_uppercase": false,
      "require_lowercase": false,
      "require_digits": false,
      "require_special": false
    }
  }'
# 预期: 400 - Password minimum length cannot be less than 8

# 尝试设置极长 Session 超时
curl -X PUT -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  http://localhost:8080/api/v1/system/settings \
  -d '{"session_timeout_hours": 876000}'
# 预期: 400 - Session timeout exceeds maximum (如 720 小时)

# 检查审计日志中是否记录了设置变更尝试
curl -H "Authorization: Bearer $TOKEN" \
  "http://localhost:8080/api/v1/audit?resource_type=system_settings&limit=10"
# 预期: 所有变更尝试（含失败的）都有记录
```

### 修复建议
- 安全相关设置设置硬下限
- 安全降级操作需要 step-up authentication
- 设置变更发送通知到所有管理员
- 审计日志记录新旧值对比

---

## 检查清单

| # | 场景 | 状态 | 测试日期 | 测试人员 | 发现问题 |
|---|------|------|----------|----------|----------|
| 1 | Token Exchange 流程滥用 | ☐ | | | |
| 2 | 邀请流程篡改 | ☐ | | | |
| 3 | 租户生命周期攻击 | ☐ | | | |
| 4 | 角色继承循环与权限爆炸 | ☐ | | | |
| 5 | 系统设置安全降级攻击 | ☐ | | | |

---

## 参考资料

- [OWASP Business Logic Security](https://owasp.org/www-community/vulnerabilities/Business_logic_vulnerability)
- [CWE-840: Business Logic Errors](https://cwe.mitre.org/data/definitions/840.html)
- [CWE-841: Improper Enforcement of Behavioral Workflow](https://cwe.mitre.org/data/definitions/841.html)
- [OWASP Testing Guide - Business Logic Testing](https://owasp.org/www-project-web-security-testing-guide/latest/4-Web_Application_Security_Testing/10-Business_Logic_Testing/)

---


---

## 标准化回归 Checklist（ASVS 5.0）

**矩阵ID**: M-BIZ-01  
**适用控制**: V2.1,V2.2,V2.5,V8.2  
**关联任务**: Backlog #1, #10, #20  
**建议回归频率**: 每次发布前 + 缺陷修复后必跑  
**场景总数**: 5

### 执行清单
- [ ] M-BIZ-01-C01 | 控制: V2.1 | 任务: #1, #10, #20 | 动作: 执行文档内相关攻击步骤并记录证据
- [ ] M-BIZ-01-C02 | 控制: V2.2 | 任务: #1, #10, #20 | 动作: 执行文档内相关攻击步骤并记录证据
- [ ] M-BIZ-01-C03 | 控制: V2.5 | 任务: #1, #10, #20 | 动作: 执行文档内相关攻击步骤并记录证据
- [ ] M-BIZ-01-C04 | 控制: V8.2 | 任务: #1, #10, #20 | 动作: 执行文档内相关攻击步骤并记录证据

### 回归记录表
| 检查项ID | 执行结果(pass/fail) | 风险等级 | 证据（请求/响应/日志/截图） | 备注 |
|---|---|---|---|---|
|  |  |  |  |  |

### 退出准则
1. 所有检查项执行完成，且高风险项无 `fail`。
2. 如存在 `fail`，必须附带漏洞单号、修复计划和复测结论。
3. 回归报告需同时记录矩阵ID与 Backlog 任务号，便于跨版本追溯。
