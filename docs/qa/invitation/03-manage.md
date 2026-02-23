# 邀请管理 - 管理操作测试

**模块**: 邀请管理
**测试范围**: 撤销、删除、过滤、多角色
**场景数**: 5

---

## 测试前置数据（必需）

在执行本文件场景前，先执行：

```bash
mysql -h 127.0.0.1 -P 4000 -u root auth9 < docs/qa/invitation/seed.sql
```

说明：
- `seed.sql` 会创建测试租户/服务/角色/邀请数据，并把 `admin@auth9.local` 加入租户
- 如果输出的 `admin_user_id` 为空，请先登录 Portal 完成首次登录以同步用户
- 测试租户：`invitation-test`（id=`11111111-1111-4111-8111-111111111111`）

## 场景 1：撤销邀请

### 初始状态
- 存在待处理邀请 id=`{invitation_id}`
- 可用邀请：`pending@example.com`（seed 已创建）

### 目的
验证邀请撤销功能

### 测试操作流程
1. 找到目标邀请
2. 点击「撤销」
3. 确认撤销

### 预期结果
- 显示撤销成功
- 邀请状态变为「Revoked」
- 被邀请人无法使用该链接

### 预期数据状态
```sql
SELECT status FROM invitations WHERE id = '{invitation_id}';
-- 预期: revoked
```

---

## 场景 2：删除邀请

### 初始状态
- 存在邀请 id=`{invitation_id}`
- 可用邀请：`pending@example.com`（seed 已创建）

### 目的
验证邀请删除功能

### 测试操作流程
1. 找到目标邀请
2. 点击「删除」
3. 确认删除

### 预期结果
- 显示删除成功
- 邀请从列表消失

### 预期数据状态
```sql
SELECT COUNT(*) FROM invitations WHERE id = '{invitation_id}';
-- 预期: 0
```

---

## 场景 3：邀请列表过滤

### 初始状态
- 存在多个不同状态的邀请
- seed 已包含 `pending@example.com`、`expired@example.com`、`revoked@example.com`、`accepted@example.com`

### 目的
验证邀请列表过滤功能

### 测试操作流程
1. 打开邀请列表
2. 按状态过滤：
   - 全部
   - 待处理（Pending）
   - 已接受（Accepted）
   - 已过期（Expired）
   - 已撤销（Revoked）

### 预期结果
- 每个过滤条件正确显示对应状态的邀请
- **重要：过滤器使用计算状态，而非数据库原始状态**：
  - "Pending" 仅匹配 `status='pending' AND expires_at > NOW()`（未过期的待处理邀请）
  - "Expired" 匹配 `status='expired' OR (status='pending' AND expires_at <= NOW())`
  - 因此 seed 中 `expired@example.com`（DB 状态为 pending 但已过期）会出现在 "Expired" 而非 "Pending" 过滤结果中

### 预期数据状态
```sql
-- 注意：直接查数据库状态与 UI 显示可能不一致，因为 UI 使用计算状态
-- 以下查询模拟 UI 的计算状态逻辑：
SELECT email,
  CASE
    WHEN status = 'pending' AND expires_at > NOW() THEN 'pending'
    WHEN status = 'pending' AND expires_at <= NOW() THEN 'expired'
    ELSE status
  END AS display_status
FROM invitations WHERE tenant_id = '{tenant_id}';
```

### 常见误报排查

| 症状 | 原因 | 解决 |
|------|------|------|
| "Pending" 显示 0 条，但 DB 有 pending 记录 | 该 pending 记录已过期（`expires_at <= NOW()`），UI 计算为 Expired | 检查 "Expired" 过滤器，应包含该记录 |
| 过滤数量与 `SELECT COUNT(*) ... GROUP BY status` 不一致 | DB 原始状态与 UI 计算状态不同 | 使用上方带 CASE 的查询验证 |

---

## 场景 4：邀请包含多个角色

### 初始状态
- 租户下存在多个可分配的角色

### 目的
验证邀请可以包含多个角色

### 测试操作流程
1. 创建邀请
2. 选择多个角色：Admin, Editor, Viewer
3. 发送邀请
4. 被邀请人接受

### 预期结果
- 被邀请人获得所有选定角色

### 预期数据状态
```sql
SELECT role_ids FROM invitations WHERE id = '{invitation_id}';
-- 预期: 包含 3 个角色 ID

SELECT COUNT(*) FROM user_tenant_roles utr
JOIN tenant_users tu ON tu.id = utr.tenant_user_id
WHERE tu.user_id = '{user_id}' AND tu.tenant_id = '{tenant_id}';
-- 接受后预期: 3
```

---

## 场景 5：邀请邮箱格式验证

### 初始状态
- 用户尝试创建邀请

### 目的
验证邮箱格式验证

### 测试操作流程
测试以下邮箱：
1. 有效：`user@example.com` ✓
2. 无效：`invalid-email` ✗
3. 无效：`user@` ✗

### 预期结果
- 无效邮箱被拒绝

---

## 测试数据准备

本文件使用统一的 seed 数据：

```bash
mysql -h 127.0.0.1 -P 4000 -u root auth9 < docs/qa/invitation/seed.sql
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
| 1 | 撤销邀请 | PASS | 2026-02-20 | Codex | pending@example.com 已变更为 revoked |
| 2 | 删除邀请 | PASS | 2026-02-20 | Codex | revoked@example.com 已删除 |
| 3 | 邀请列表过滤 | PASS | 2026-02-23 | Claude | 过滤器正确使用计算状态：expired@example.com 虽DB为pending但已过期，归入Expired而非Pending（设计正确） |
| 4 | 多角色邀请 | PARTIAL | 2026-02-20 | Codex | 接受页面 /invite/accept 现已修复(非404)，但创建邀请时报权限错误"Admin or owner role required" |
| 5 | 邮箱格式验证 | PASS | 2026-02-20 | Codex | 浏览器校验拦截 invalid-email / user@ |
| 6 | 认证状态检查 | PASS | 2026-02-20 | Codex | 已确认未登录自动重定向到/login |
