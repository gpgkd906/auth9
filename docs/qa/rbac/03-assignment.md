# RBAC - 权限分配测试

**模块**: RBAC 角色权限管理
**测试范围**: 权限分配、用户角色分配、有效权限查询
**场景数**: 5

---

## 数据库表结构参考

### role_permissions 表
| 字段 | 类型 | 说明 |
|------|------|------|
| role_id | CHAR(36) | 角色 ID |
| permission_id | CHAR(36) | 权限 ID |

### user_tenant_roles 表
| 字段 | 类型 | 说明 |
|------|------|------|
| id | CHAR(36) | UUID 主键 |
| tenant_user_id | CHAR(36) | tenant_users 表 ID |
| role_id | CHAR(36) | 角色 ID |
| granted_at | TIMESTAMP | 授予时间 |
| granted_by | CHAR(36) | 授予者用户 ID |

---

## 场景 1：为角色分配权限

### 初始状态
- 存在角色 id=`{role_id}`
- 存在权限 id=`{permission_id}`
- 该权限尚未分配给该角色
- 若租户已启用 ABAC（`mode=enforce`），需先确认策略允许当前操作，或先切换为 `shadow/disabled`

### 目的
验证角色-权限分配功能

### 测试操作流程
1. 切换到「权限」标签页，为目标服务创建权限（点击该服务区域的「Add Permission」按钮）
2. **验证**：创建后查询 `SELECT id, code, service_id FROM permissions WHERE code = '{code}'`，确认 `service_id` 与目标服务一致
3. 切换到「角色」标签页，找到目标角色
4. 点击「管理权限」
5. 勾选要分配的权限
6. 保存

> **注意**: 在「权限」标签页中，每个服务区域都有独立的「Add Permission」按钮。请确保点击正确服务的按钮，否则权限会创建到错误的 `service_id` 下，导致分配时因 service 不匹配而静默失败。

### 预期结果
- 显示分配成功
- 权限已勾选

### 预期数据状态
```sql
SELECT role_id, permission_id FROM role_permissions
WHERE role_id = '{role_id}' AND permission_id = '{permission_id}';
-- 预期: 存在记录
```

---

## 场景 2：从角色移除权限

### 初始状态
- 角色 `{role_id}` 已有权限 `{permission_id}`

### 目的
验证权限移除功能

### 测试操作流程
1. 打开角色的权限管理
2. 取消勾选目标权限
3. 保存

### 预期结果
- 显示更新成功
- 权限不再勾选

### 预期数据状态
```sql
SELECT COUNT(*) FROM role_permissions WHERE role_id = '{role_id}' AND permission_id = '{permission_id}';
-- 预期: 0
```

---

## 场景 3：为用户分配角色

### 初始状态
- 用户 `{user_id}` 已加入租户 `{tenant_id}`
- 存在角色 `{role_id}`
- 用户尚未拥有该角色

> 本地 `./scripts/reset-docker.sh` 的默认种子数据只保证存在 `Auth9 Admin Portal` 的 `admin` 角色。
> `Auth9 Demo Service` 并不会默认带出 `Content Editor` 等业务角色。若要验证该类场景，需先在目标服务中手动创建对应角色；否则可改用已存在的 `admin` 角色验证分配流程本身。

### 目的
验证用户-角色分配功能

### 测试操作流程
1. 进入用户管理
2. 选择「管理角色」
3. 选择租户和服务
4. 勾选角色
5. 保存

### 预期结果
- 显示分配成功
- 用户角色列表显示新角色

### 预期数据状态
```sql
SELECT utr.id, utr.role_id, utr.granted_at, utr.granted_by
FROM user_tenant_roles utr
JOIN tenant_users tu ON tu.id = utr.tenant_user_id
WHERE tu.user_id = '{user_id}' AND tu.tenant_id = '{tenant_id}' AND utr.role_id = '{role_id}';
-- 预期: 存在记录
```

---

## 场景 4：移除用户的角色

### 初始状态
- 用户在租户中拥有角色 `{role_id}`

### 目的
验证用户角色移除功能

### 测试操作流程
1. 打开用户的角色管理
2. 取消勾选目标角色
3. 保存

### 预期结果
- 显示更新成功
- 角色从列表消失

### 预期数据状态
```sql
SELECT COUNT(*) FROM user_tenant_roles utr
JOIN tenant_users tu ON tu.id = utr.tenant_user_id
WHERE tu.user_id = '{user_id}' AND tu.tenant_id = '{tenant_id}' AND utr.role_id = '{role_id}';
-- 预期: 0
```

---

## 场景 5：查询用户的有效权限（含继承）

### 步骤 0：创建角色继承关系（前置数据准备）

> **重要**: 默认种子数据不包含 Editor/Viewer 角色和 content:read/write 权限。必须先手动创建。

```bash
# 获取 Demo Service ID
SERVICE_ID=$(curl -s http://localhost:8080/api/v1/services \
  -H "Authorization: Bearer $TOKEN" | jq -r '.data[] | select(.name=="Auth9 Demo Service") | .id')

# 1. 创建 content:read 权限
curl -s -X POST "http://localhost:8080/api/v1/services/$SERVICE_ID/permissions" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"code": "content:read", "description": "Read content"}'

# 2. 创建 content:write 权限
curl -s -X POST "http://localhost:8080/api/v1/services/$SERVICE_ID/permissions" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"code": "content:write", "description": "Write content"}'

# 3. 创建 Viewer 角色
VIEWER_ID=$(curl -s -X POST "http://localhost:8080/api/v1/services/$SERVICE_ID/roles" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"name": "Viewer"}' | jq -r '.data.id')

# 4. 创建 Editor 角色（继承 Viewer）
EDITOR_ID=$(curl -s -X POST "http://localhost:8080/api/v1/services/$SERVICE_ID/roles" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d "{\"name\": \"Editor\", \"parent_role_id\": \"$VIEWER_ID\"}" | jq -r '.data.id')

# 5. 分配权限: content:read → Viewer, content:write → Editor
# （通过 Portal UI 或 API 分配）

# 6. 为用户分配 Editor 角色（使用场景 3 的流程）
```

### 初始状态
- 用户在租户中有角色 `Editor`
- `Editor` 继承自 `Viewer`（`parent_role_id` = Viewer.id）
- `Viewer` 有权限：`content:read`
- `Editor` 有权限：`content:write`

### 目的
验证有效权限包含继承权限

### 测试操作流程
1. 调用 API 或 gRPC 获取用户权限

### 预期结果
- 返回权限包含：`content:read`, `content:write`

### 预期数据状态
```sql
-- 验证用户角色
SELECT r.name FROM user_tenant_roles utr
JOIN roles r ON r.id = utr.role_id
JOIN tenant_users tu ON tu.id = utr.tenant_user_id
WHERE tu.user_id = '{user_id}' AND tu.tenant_id = '{tenant_id}';

-- 验证有效权限（含继承）
WITH RECURSIVE role_tree AS (
    SELECT r.id, r.parent_role_id FROM user_tenant_roles utr
    JOIN roles r ON r.id = utr.role_id
    JOIN tenant_users tu ON tu.id = utr.tenant_user_id
    WHERE tu.user_id = '{user_id}' AND tu.tenant_id = '{tenant_id}'
    UNION ALL
    SELECT r.id, r.parent_role_id FROM roles r
    JOIN role_tree rt ON rt.parent_role_id = r.id
)
SELECT DISTINCT p.code FROM role_tree rt
JOIN role_permissions rp ON rp.role_id = rt.id
JOIN permissions p ON p.id = rp.permission_id;
-- 预期: content:read, content:write
```

---

## 通用场景：认证状态检查

### 初始状态
- 用户已登录管理后台
- 页面正常显示

### 目的
验证页面正确检查认证状态，未登录或 session 失效时重定向到登录页

### 测试操作流程
1. 通过以下任一方式构造未认证状态：
   - 使用浏览器无痕/隐私窗口访问
   - 手动清除 auth9_session cookie
   - 在当前会话点击「Sign out」退出登录
2. 访问本页面对应的 URL

### 预期结果
- 页面自动重定向到 `/login`
- 不显示 dashboard 内容
- 登录后可正常访问原页面

---

## 检查清单

| # | 场景 | 状态 | 测试日期 | 测试人员 | 备注 |
|---|------|------|----------|----------|------|
| 1 | 为角色分配权限 | ☐ | | | |
| 2 | 从角色移除权限 | ☐ | | | |
| 3 | 为用户分配角色 | ☐ | | | |
| 4 | 移除用户角色 | ☐ | | | |
| 5 | 查询有效权限 | ☐ | | | |
| 6 | 认证状态检查 | ☐ | | | |

---

## Troubleshooting

| Symptom | Cause | Fix |
|---------|-------|-----|
| Checkbox appears to not respond when unchecking a role | The `handleRoleCheckedChange` function calls `submit()` (React Router form submission) which triggers loader revalidation. The checkbox state updates immediately via `setAssignedRoleIds`, but the subsequent re-fetch (`get_user_assigned_roles`) may briefly flash the old state if the API response is slow. | Wait for the form submission to complete (check `navigation.state === "idle"`) before asserting checkbox state. The final state is determined by the re-fetched data from the API. |
| Role dialog shows only 1 checkbox | The available roles depend on which service is selected. "Auth9 Admin Portal" only has 1 role (`admin`). | Select a service with multiple roles (e.g., "Invitation Test Service" which has Editor, Viewer roles) to test multi-role scenarios. |
| Permission/role assignment appears to silently fail (scenarios 2, 3) | The code flow has been verified correct end-to-end. Silent failures are typically caused by expired session or invalid accessToken in the Portal BFF layer. | 1. Check browser DevTools Network tab for 401 responses. 2. Re-login to the Portal to refresh the session and accessToken. 3. Verify the Portal BFF is passing `accessToken` to auth9-core API calls (see `app/services/api.ts`). |
