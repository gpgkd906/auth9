# RBAC - 角色管理测试

**模块**: RBAC 角色权限管理
**测试范围**: 角色 CRUD 和继承
**场景数**: 5

---

## 数据库表结构参考

### roles 表
| 字段 | 类型 | 说明 |
|------|------|------|
| id | CHAR(36) | UUID 主键 |
| service_id | CHAR(36) | 所属服务 ID |
| name | VARCHAR(100) | 角色名称 |
| description | TEXT | 描述 |
| parent_role_id | CHAR(36) | 父角色 ID（继承） |
| created_at | TIMESTAMP | 创建时间 |
| updated_at | TIMESTAMP | 更新时间 |

---

## 前置条件

> **JWT Token Generation**: Always use `node .claude/skills/tools/gen_token.js` which reads the private key from `.env` (matching the Docker container). Other scripts may use hardcoded key paths that don't match.

| 症状 | 原因 | 修复方法 |
|------|------|----------|
| JWT 签名验证失败 (401) | 使用了 hardcoded key path 的脚本，与 Docker 容器中的 key 不一致 | 改用 `node .claude/skills/tools/gen_token.js`，它从 `.env` 读取私钥 |
| 403 on API calls despite valid signature | Token missing `roles` or `permissions` fields | `gen_token.js` generates Tenant Access Tokens that MUST include `roles` and `permissions` fields. Verify with: `echo $TOKEN \| cut -d. -f2 \| base64 -d \| jq '{roles, permissions}'` |

> **MFA 对 UI 测试的影响**：所有默认种子用户均已启用 MFA：
> - `admin@auth9.local` 要求 WebAuthn（硬件密钥）
> - `mfa-user@auth9.local` 要求 TOTP
>
> **UI 测试方案**（二选一）：
> 1. 临时禁用 MFA：`DELETE FROM webauthn_credentials WHERE user_id = (SELECT id FROM users WHERE email = 'admin@auth9.local');`
> 2. 改用 API 测试：使用有效 Access Token 直接调用 API，绕过 MFA 登录流程
>
> | 症状 | 原因 | 解决方法 |
> |------|------|----------|
> | MFA 重定向阻断 UI 测试流程 | admin 用户绑定了 WebAuthn 凭证 | 删除 WebAuthn 凭证或改用 API 测试 |

---

## 场景 1：角色管理入口可见性与创建角色

### 初始状态
- 存在服务 id=`{service_id}`
- 该服务下无同名角色

### 目的
验证用户可从导航入口进入角色管理，并完成角色创建

### 测试操作流程
1. 在左侧导航确认存在「角色与权限」入口
2. 点击「角色与权限」进入页面
3. 切换到「角色」标签
4. 点击「创建角色」
5. 确认弹窗中的「父角色」控件使用项目统一 Selector 组件，而非浏览器原生 `<select>`
6. 填写：
   - 服务：选择目标服务
   - 角色名称：`Editor`
   - 描述：`可以编辑内容`
   - 父角色：无
7. 点击「创建」

### 预期结果
- 显示创建成功
- 「角色与权限」入口可见且可点击
- 角色出现在列表中
- 「父角色」控件默认显示“无父角色”，展开后选项列表与项目其他 Selector 风格一致

### 预期数据状态
```sql
SELECT id, service_id, name, description, parent_role_id FROM roles
WHERE name = 'Editor' AND service_id = '{service_id}';
-- 预期: 存在记录，parent_role_id = NULL
```

---

## 场景 2：创建带继承的角色

### 初始状态
- 存在服务 id=`{service_id}`
- 该服务下存在角色 `Viewer` (id=`{viewer_role_id}`)

### 目的
验证角色继承功能

### 测试操作流程
1. 点击「创建角色」
2. 在弹窗中展开「父角色」Selector，确认候选项中包含 `Viewer`
3. 创建新角色：
   - 名称：`Editor`
   - 父角色：选择 `Viewer`
4. 点击「创建」

### 预期结果
- 角色创建成功
- 层次视图显示继承关系
- 「父角色」展开列表不应出现浏览器原生 `<option>` 样式

### 预期数据状态
```sql
SELECT name, parent_role_id FROM roles WHERE name = 'Editor' AND service_id = '{service_id}';
-- 预期: parent_role_id = '{viewer_role_id}'
```

---

## 场景 3：更新角色

### 初始状态
- 存在角色 id=`{role_id}`，name=`Editor`

### 目的
验证角色更新功能

### 测试操作流程
1. 找到目标角色
2. 点击「编辑」
3. 确认编辑弹窗中的「父角色」Selector 默认值与当前角色继承关系一致
4. 修改：
   - 名称：`Content Editor`
   - 描述：`可以编辑和发布内容`
5. 如需变更继承关系，展开「父角色」Selector 重新选择父角色
6. 保存

### 预期结果
- 显示更新成功
- 列表显示新名称
- 若未修改父角色，提交后 `parent_role_id` 保持原值；若切换为“无父角色”，提交后 `parent_role_id` 置空

### 预期数据状态
```sql
SELECT name, description, updated_at FROM roles WHERE id = '{role_id}';
-- 预期: name = 'Content Editor'
```

---

## 场景 4：删除角色

### 初始状态
- 存在角色 id=`{role_id}`
- 该角色有权限关联
- 该角色已分配给用户

### 目的
验证角色删除的级联处理

### 测试操作流程
1. 找到目标角色
2. 点击「删除」
3. 确认删除

### 预期结果
- 显示删除成功
- 角色从列表消失

### 预期数据状态
```sql
SELECT COUNT(*) FROM roles WHERE id = '{role_id}';
-- 预期: 0

SELECT COUNT(*) FROM role_permissions WHERE role_id = '{role_id}';
-- 预期: 0

SELECT COUNT(*) FROM user_tenant_roles WHERE role_id = '{role_id}';
-- 预期: 0
```

---

## 场景 5：删除有子角色的角色

### 初始状态
- 存在父角色 `Admin` (id=`{admin_role_id}`)
- 存在子角色 `Super Admin`，parent_role_id = `{admin_role_id}`

### 目的
验证删除父角色时的处理

### 测试操作流程
1. 尝试删除 `Admin` 角色

### 预期结果
- 选项1：显示错误「该角色有子角色，无法删除」
- 选项2：删除成功，子角色 parent_role_id 置为 NULL

### 预期数据状态
```sql
-- 根据实现方式验证
SELECT parent_role_id FROM roles WHERE name = 'Super Admin';
```

---

## 通用场景：认证状态检查

### 初始状态
- 用户未登录（无有效 session cookie）

### 目的
验证页面正确检查认证状态，未登录或 session 失效时重定向到登录页

### 测试操作流程
1. 打开浏览器的隐私/无痕模式（确保无遗留 cookie）
2. 直接访问本页面对应的 URL（如 `/dashboard/roles`）

### 预期结果
- 页面自动重定向到 `/login`
- 不显示 dashboard 内容
- 登录后可正常访问原页面

### 注意事项
> **Session 为持久化 Cookie（maxAge: 8 小时）**：关闭浏览器不会清除 session。
> 要测试未认证状态，请使用以下方法之一：
> 1. 使用浏览器隐私/无痕模式（推荐）
> 2. 手动点击「Sign out」退出登录
> 3. 手动清除 `auth9_session` cookie
> 4. 等待 session 过期（8 小时）
>
> | 症状 | 原因 | 解决方法 |
> |------|------|----------|
> | 关闭浏览器后重新打开仍可访问 Dashboard | 持久化 Cookie 未过期 | 使用无痕模式或手动清除 Cookie |
> | 清除 Cookie 后页面未跳转 | 浏览器缓存 | 强制刷新（Ctrl+Shift+R） |

---

## 检查清单

| # | 场景 | 状态 | 测试日期 | 测试人员 | 备注 |
|---|------|------|----------|----------|------|
| 1 | 创建角色 | ☐ | | | |
| 2 | 创建带继承的角色 | ☐ | | | |
| 3 | 更新角色 | ☐ | | | |
| 4 | 删除角色 | ☐ | | | |
| 5 | 删除有子角色的角色 | ☐ | | | |
| 6 | 认证状态检查 | ☐ | | | |
