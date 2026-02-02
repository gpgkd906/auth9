# RBAC 权限系统

Auth9 提供灵活强大的基于角色的访问控制（RBAC）系统。

## 核心概念

### 权限模型

```
租户 (Tenant)
└── 服务 (Service)
    ├── 权限 (Permission)
    │   └── 权限点 (如 user:read, content:write)
    └── 角色 (Role)
        ├── 权限集合
        └── 角色继承
```

### 关键术语

| 术语 | 说明 | 示例 |
|------|------|------|
| **租户** | 组织单位 | "公司 A" |
| **服务** | 应用或系统 | "内容管理系统" |
| **权限** | 具体操作权限 | `content:write` |
| **角色** | 权限的集合 | "编辑者" |
| **用户角色分配** | 用户在租户中的角色 | "张三在公司A是编辑者" |

## 权限管理

### 权限命名规范

使用 `资源:操作` 格式：

```
user:read        // 读取用户
user:write       // 创建/修改用户
user:delete      // 删除用户
content:read     // 读取内容
content:write    // 创建/修改内容
content:publish  // 发布内容
report:export    // 导出报告
admin:*          // 所有管理员权限
```

### 创建权限

```bash
curl -X POST /api/v1/services/{service_id}/permissions \
  -H "Authorization: Bearer <token>" \
  -d '{
    "code": "content:write",
    "name": "编辑内容",
    "description": "允许创建和修改内容"
  }'
```

### 查看服务权限

```bash
curl /api/v1/services/{service_id}/permissions \
  -H "Authorization: Bearer <token>"
```

响应：

```json
{
  "data": [
    {
      "id": "perm-uuid",
      "service_id": "service-uuid",
      "code": "content:write",
      "name": "编辑内容",
      "description": "允许创建和修改内容"
    },
    {
      "id": "perm-uuid-2",
      "code": "content:publish",
      "name": "发布内容",
      "description": "允许发布内容到生产环境"
    }
  ]
}
```

## 角色管理

### 角色层次结构

角色支持继承，形成层次结构：

```
管理员 (Admin)
├── 所有权限
│
├── 编辑者 (Editor) - 继承部分管理员权限
│   ├── content:read
│   ├── content:write
│   ├── content:publish
│   │
│   └── 作者 (Author) - 继承编辑者权限
│       ├── content:read
│       ├── content:write
│       │
│       └── 查看者 (Viewer) - 最基础角色
│           └── content:read
```

### 创建角色

```bash
curl -X POST /api/v1/services/{service_id}/roles \
  -H "Authorization: Bearer <token>" \
  -d '{
    "name": "editor",
    "display_name": "编辑者",
    "description": "可以创建和编辑内容",
    "parent_role_id": null
  }'
```

### 为角色分配权限

```bash
curl -X POST /api/v1/roles/{role_id}/permissions \
  -H "Authorization: Bearer <token>" \
  -d '{
    "permission_ids": [
      "perm-uuid-1",
      "perm-uuid-2",
      "perm-uuid-3"
    ]
  }'
```

### 创建继承角色

```bash
# 创建子角色，继承父角色的所有权限
curl -X POST /api/v1/services/{service_id}/roles \
  -H "Authorization: Bearer <token>" \
  -d '{
    "name": "senior_editor",
    "display_name": "高级编辑",
    "description": "拥有编辑者的所有权限，外加额外权限",
    "parent_role_id": "editor-role-uuid"
  }'
```

### 查看角色权限

```bash
curl /api/v1/roles/{role_id}/permissions \
  -H "Authorization: Bearer <token>"
```

响应包含直接权限和继承权限：

```json
{
  "data": {
    "role": {
      "id": "role-uuid",
      "name": "editor",
      "display_name": "编辑者"
    },
    "direct_permissions": [
      {
        "code": "content:write",
        "name": "编辑内容"
      }
    ],
    "inherited_permissions": [
      {
        "code": "content:read",
        "name": "查看内容",
        "inherited_from": "viewer"
      }
    ],
    "all_permissions": [
      "content:read",
      "content:write"
    ]
  }
}
```

## 用户角色分配

### 为用户分配角色

```bash
curl -X POST /api/v1/rbac/assign \
  -H "Authorization: Bearer <token>" \
  -d '{
    "user_id": "user-uuid",
    "tenant_id": "tenant-uuid",
    "role_ids": [
      "editor-role-uuid",
      "reporter-role-uuid"
    ]
  }'
```

### 查看用户角色

```bash
curl "/api/v1/rbac/user-roles?user_id=user-uuid&tenant_id=tenant-uuid" \
  -H "Authorization: Bearer <token>"
```

响应：

```json
{
  "data": {
    "user_id": "user-uuid",
    "tenant_id": "tenant-uuid",
    "roles": [
      {
        "id": "role-uuid",
        "name": "editor",
        "display_name": "编辑者",
        "service_name": "内容管理系统",
        "granted_at": "2024-01-01T00:00:00Z",
        "granted_by": "admin-uuid"
      }
    ],
    "all_permissions": [
      "content:read",
      "content:write",
      "content:publish",
      "report:view"
    ]
  }
}
```

### 撤销用户角色

```bash
curl -X DELETE /api/v1/rbac/revoke \
  -H "Authorization: Bearer <token>" \
  -d '{
    "user_id": "user-uuid",
    "tenant_id": "tenant-uuid",
    "role_id": "role-uuid"
  }'
```

## 权限检查

### 在应用中检查权限

#### 方式 1: 通过 Token 中的权限

```rust
// Token 中包含权限列表
let claims = validate_token(token)?;

if claims.permissions.contains(&"content:write".to_string()) {
    // 允许操作
} else {
    return Err(Error::Forbidden);
}
```

#### 方式 2: 通过 gRPC 实时查询

```rust
let response = client.get_user_roles(GetUserRolesRequest {
    user_id: user_id.to_string(),
    tenant_id: tenant_id.to_string(),
    service_id: service_id.to_string(),
}).await?;

let permissions = response.into_inner().permissions;

if permissions.contains(&"content:write".to_string()) {
    // 允许操作
}
```

### 使用中间件检查

```rust
use axum::middleware;

// 定义权限中间件
async fn require_permission(
    permission: &str,
) -> impl Fn(Request, Next) -> Future<Output = Response> {
    move |req: Request, next: Next| {
        let perm = permission.to_string();
        async move {
            let token = extract_token(&req)?;
            let claims = validate_token(token)?;
            
            if !claims.permissions.contains(&perm) {
                return Err(Error::Forbidden);
            }
            
            Ok(next.run(req).await)
        }
    }
}

// 应用到路由
let app = Router::new()
    .route("/api/content", post(create_content))
    .layer(middleware::from_fn(require_permission("content:write")));
```

## 权限策略

### 最小权限原则

为用户分配完成工作所需的最小权限集：

```
✅ 好的做法：
- 查看者: content:read
- 作者: content:read, content:write
- 编辑: content:read, content:write, content:publish

❌ 不好的做法：
- 所有用户都是管理员
- 给作者分配删除用户的权限
```

### 基于场景的角色

根据实际使用场景定义角色：

```json
{
  "roles": [
    {
      "name": "content_creator",
      "display_name": "内容创作者",
      "permissions": [
        "content:read",
        "content:write",
        "draft:create",
        "draft:edit"
      ]
    },
    {
      "name": "content_reviewer",
      "display_name": "内容审核者",
      "permissions": [
        "content:read",
        "review:create",
        "review:approve",
        "review:reject"
      ]
    },
    {
      "name": "content_publisher",
      "display_name": "内容发布者",
      "permissions": [
        "content:read",
        "content:publish",
        "content:unpublish"
      ]
    }
  ]
}
```

### 临时权限

对于临时需求，使用时间限制的角色分配：

```bash
curl -X POST /api/v1/rbac/assign \
  -H "Authorization: Bearer <token>" \
  -d '{
    "user_id": "user-uuid",
    "tenant_id": "tenant-uuid",
    "role_ids": ["temp-admin-uuid"],
    "expires_at": "2024-12-31T23:59:59Z"
  }'
```

## 审计和监控

### 记录权限变更

所有权限和角色变更都会记录到审计日志：

```bash
curl "/api/v1/audit-logs?resource_type=role&action=role.assigned" \
  -H "Authorization: Bearer <token>"
```

### 权限使用分析

```bash
# 查询最常用的权限
curl "/api/v1/analytics/permission-usage?start_date=2024-01-01" \
  -H "Authorization: Bearer <token>"
```

### 异常权限告警

设置告警监控异常的权限分配：

```yaml
alerts:
  - name: 非工作时间权限分配
    condition: role.assigned outside 09:00-18:00
    action: notify_security
  
  - name: 管理员权限分配
    condition: role.assigned AND role.name = 'admin'
    action: notify_security
  
  - name: 大量权限变更
    condition: count(role.assigned) > 10 within 1h
    action: notify_admin
```

## 高级特性

### 权限通配符

支持通配符匹配：

```
content:*      // 所有内容相关权限
*:read         // 所有读取权限
admin:*        // 所有管理员权限
```

### 条件权限

基于条件的权限判断（计划中的功能）：

```json
{
  "permission": "content:edit",
  "conditions": {
    "resource_owner": "self",  // 只能编辑自己的内容
    "resource_status": "draft" // 只能编辑草稿状态
  }
}
```

### 权限组

将相关权限组织成组：

```json
{
  "permission_groups": {
    "content_management": [
      "content:read",
      "content:write",
      "content:publish",
      "content:delete"
    ],
    "user_management": [
      "user:read",
      "user:write",
      "user:delete",
      "role:assign"
    ]
  }
}
```

## 最佳实践

### 1. 权限粒度

```
✅ 适中的粒度：
- user:read
- user:write
- user:delete

❌ 过粗：
- user:manage (太宽泛)

❌ 过细：
- user:edit_name
- user:edit_email
- user:edit_avatar
```

### 2. 角色命名

```
✅ 清晰的名称：
- content_editor
- user_manager
- report_viewer

❌ 模糊的名称：
- role1
- temp_role
- new_role
```

### 3. 定期审查

- 每季度审查角色和权限配置
- 清理不再使用的角色
- 更新权限描述
- 审查用户角色分配

### 4. 文档化

为每个权限和角色编写清晰的文档：

```json
{
  "code": "content:publish",
  "name": "发布内容",
  "description": "允许将内容发布到生产环境。需要先通过审核。",
  "requires": ["content:write"],
  "examples": [
    "发布文章",
    "发布产品页面",
    "发布营销内容"
  ],
  "restrictions": [
    "不能发布未审核的内容",
    "必须遵守发布流程"
  ]
}
```

## 迁移和版本管理

### 权限迁移

当权限模型变更时：

```bash
# 1. 备份当前配置
curl /api/v1/rbac/export > rbac_backup.json

# 2. 应用新的权限结构
curl -X POST /api/v1/rbac/migrate \
  -H "Authorization: Bearer <token>" \
  -d @migration_plan.json

# 3. 验证迁移结果
curl /api/v1/rbac/validate
```

### 版本控制

将 RBAC 配置纳入版本控制：

```yaml
# rbac_config.yaml
version: "1.0"
services:
  - name: content_management
    permissions:
      - code: content:read
        name: 查看内容
      - code: content:write
        name: 编辑内容
    roles:
      - name: editor
        permissions:
          - content:read
          - content:write
```

## 相关文档

- [多租户管理](多租户管理.md)
- [用户操作指南](../userguide/USER_GUIDE.md)
- [REST API](REST-API.md)
- [最佳实践](最佳实践.md)
