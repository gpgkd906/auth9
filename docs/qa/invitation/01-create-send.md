# 邀请管理 - 创建与发送测试

**模块**: 邀请管理
**测试范围**: 邀请创建、发送、重发
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

## 数据库表结构参考

### invitations 表
| 字段 | 类型 | 说明 |
|------|------|------|
| id | CHAR(36) | UUID 主键 |
| tenant_id | CHAR(36) | 目标租户 ID |
| email | VARCHAR(255) | 被邀请人邮箱 |
| role_ids | JSON | 接受后分配的角色 ID 列表 |
| invited_by | CHAR(36) | 邀请人用户 ID |
| token_hash | VARCHAR(255) | 邀请令牌哈希 |
| status | ENUM | pending/accepted/expired/revoked |
| expires_at | TIMESTAMP | 过期时间 |
| created_at | TIMESTAMP | 创建时间 |

---

## 场景 1：创建邀请

### 初始状态
- 管理员已登录
- 存在租户 id=`{tenant_id}`
- 邀请邮箱尚未加入该租户

### 目的
验证邀请创建和邮件发送

### 测试操作流程
1. 进入租户的「邀请管理」页面
2. 点击「邀请用户」
3. 填写：
   - 邮箱：`newuser@example.com`
   - 过期时间：`72 小时`
   - 角色：选择 `Editor`, `Viewer`
4. 点击「发送邀请」

### 预期结果
- 显示发送成功
- 邀请出现在列表中，状态为「Pending」
- 被邀请人收到邮件

### 预期数据状态
```sql
SELECT id, tenant_id, email, role_ids, status, expires_at FROM invitations
WHERE email = 'newuser@example.com' AND tenant_id = '{tenant_id}';
-- 预期: 存在记录，status = 'pending'，expires_at = 当前时间 + 72小时
```

---

## 场景 2：邀请已存在的租户成员

### 初始状态
- 用户已是租户成员
- 可用邮箱：`admin@auth9.local`（seed 已加入租户）

### 目的
验证系统拒绝邀请已存在成员

### 测试操作流程
1. 尝试邀请已是成员的用户

### 预期结果
- 显示错误：「该用户已是租户成员」

### 预期数据状态
```sql
SELECT COUNT(*) FROM invitations WHERE email = '{email}' AND tenant_id = '{tenant_id}' AND status = 'pending';
-- 预期: 0（未创建新邀请）
```

---

## 场景 3：重复邀请同一邮箱

### 初始状态
- 已存在对 `pending@example.com` 的待处理邀请

### 目的
验证重复邀请处理

### 测试操作流程
1. 尝试再次邀请 `pending@example.com`

### 预期结果
- 选项1：显示错误「已存在待处理的邀请」
- 选项2：更新现有邀请并重新发送

### 预期数据状态
```sql
SELECT COUNT(*) FROM invitations WHERE email = 'pending@example.com' AND status = 'pending';
-- 预期: 1
```

---

## 场景 4：重新发送邀请

### 初始状态
- 存在待处理邀请
- 被邀请人称未收到邮件
- 可用邀请：`pending@example.com`（seed 已创建）

### 目的
验证重新发送邀请功能

### 测试操作流程
1. 找到目标邀请
2. 点击「重新发送」

### 预期结果
- 显示发送成功
- 被邀请人收到新邮件

### 预期数据状态
```sql
SELECT updated_at FROM invitations WHERE id = '{invitation_id}';
-- 预期: updated_at 更新为当前时间
```

---

## 场景 5：不同过期时间选项

### 初始状态
- 用户创建邀请

### 目的
验证不同过期时间配置

### 测试操作流程
测试以下过期时间：
1. 24 小时
2. 48 小时
3. 72 小时
4. 7 天

### 预期结果
- 每种选项的 expires_at 正确计算

### 预期数据状态
```sql
SELECT expires_at, created_at, TIMESTAMPDIFF(HOUR, created_at, expires_at) as hours_diff
FROM invitations WHERE id = '{invitation_id}';
-- 以 72 小时为例，预期: hours_diff = 72
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
| 1 | 创建邀请 | PASS | 2026-02-06 | Codex | 邮件发送成功 |
| 2 | 邀请已存在成员 | FAIL | 2026-02-06 | Codex | 见 `docs/ticket/invitation_01-create-send_scenario2_260206_194614.md` |
| 3 | 重复邀请同一邮箱 | PASS | 2026-02-06 | Codex | 返回“已存在”错误 |
| 4 | 重新发送邀请 | FAIL | 2026-02-06 | Codex | 见 `docs/ticket/invitation_01-create-send_scenario4_260206_194834.md` |
| 5 | 不同过期时间 | PASS | 2026-02-06 | Codex | 24/48/72/168 小时符合预期 |
| 6 | 认证状态检查 | NOT RUN |  |  |  |
