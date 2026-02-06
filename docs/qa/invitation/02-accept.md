# 邀请管理 - 接受邀请测试

**模块**: 邀请管理
**测试范围**: 接受邀请流程
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
- 接受邀请场景必须使用真实邀请邮件中的链接（seed 数据中的 `token_hash` 不可用于接受）
- 建议为每个场景在 Portal 中新建邀请以获取有效链接；过期/撤销/已接受可在创建后再通过 UI 或 DB 调整状态
- 测试租户：`invitation-test`（id=`11111111-1111-4111-8111-111111111111`）

## 场景 1：接受邀请（新用户）

### 初始状态
- 存在邀请 id=`{invitation_id}`，status=`pending`
- 邀请邮箱不是系统中的已有用户
- 建议先执行 `01-create-send` 场景 1，使用邮件链接继续测试

### 目的
验证新用户接受邀请的完整流程

### 测试操作流程
1. 点击邀请邮件中的链接
2. 跳转到注册页面
3. 完成注册
4. 系统自动将用户添加到租户

### 预期结果
- 用户成功注册
- 用户自动加入目标租户
- 用户获得邀请中指定的角色
- 邀请状态变为「Accepted」

### 预期数据状态
```sql
SELECT status, accepted_at FROM invitations WHERE id = '{invitation_id}';
-- 预期: status = 'accepted'，accepted_at 有值

SELECT tu.id FROM tenant_users tu JOIN users u ON u.id = tu.user_id
WHERE u.email = 'newuser@example.com' AND tu.tenant_id = '{tenant_id}';
-- 预期: 存在记录

SELECT r.name FROM user_tenant_roles utr
JOIN roles r ON r.id = utr.role_id
JOIN tenant_users tu ON tu.id = utr.tenant_user_id
JOIN users u ON u.id = tu.user_id
WHERE u.email = 'newuser@example.com';
-- 预期: 返回邀请中指定的角色
```

---

## 场景 2：接受邀请（已有用户）

### 初始状态
- 存在邀请给已注册用户
- 该用户不是目标租户成员

### 目的
验证已有用户接受邀请流程

### 测试操作流程
1. 用户点击邀请链接
2. 用户登录（已有账户）
3. 系统将用户添加到租户

### 预期结果
- 用户加入目标租户
- 获得邀请中指定的角色

### 预期数据状态
```sql
SELECT tu.id FROM tenant_users tu
WHERE tu.user_id = '{user_id}' AND tu.tenant_id = '{tenant_id}';
-- 预期: 存在记录
```

---

## 场景 3：使用过期邀请

### 初始状态
- 存在邀请，expires_at 已过期

### 目的
验证过期邀请处理

### 测试操作流程
1. 点击过期邀请的链接
2. 尝试接受邀请

### 预期结果
- 显示错误：「邀请已过期」
- 用户未被添加到租户

### 预期数据状态
```sql
SELECT status FROM invitations WHERE id = '{invitation_id}';
-- 预期: expired 或仍为 pending
```

---

## 场景 4：使用已撤销的邀请

### 初始状态
- 邀请状态为 `revoked`

### 目的
验证已撤销邀请无法使用

### 测试操作流程
1. 尝试使用已撤销邀请的链接

### 预期结果
- 显示错误：「邀请已被撤销」

---

## 场景 5：使用已接受的邀请

### 初始状态
- 邀请状态为 `accepted`

### 目的
验证已接受的邀请无法重复使用

### 测试操作流程
1. 尝试再次使用已接受的邀请链接

### 预期结果
- 显示提示：「邀请已被使用」

---

## 通用场景：认证状态检查

### 初始状态
- 用户已登录管理后台
- 页面正常显示

### 目的
验证页面正确检查认证状态，未登录或 session 失效时重定向到登录页

### 测试操作流程
1. 关闭浏览器
2. 重新打开浏览器，访问本页面对应的 URL

### 预期结果
- 页面自动重定向到 `/login`
- 不显示 dashboard 内容
- 登录后可正常访问原页面

---

## 检查清单

| # | 场景 | 状态 | 测试日期 | 测试人员 | 备注 |
|---|------|------|----------|----------|------|
| 1 | 接受邀请（新用户） | PASS | 2026-02-06 | Codex | newuser3@example.com |
| 2 | 接受邀请（已有用户） | PASS | 2026-02-06 | Codex | existing2@example.com |
| 3 | 使用过期邀请 | PASS | 2026-02-06 | Codex | expired2@example.com |
| 4 | 使用已撤销邀请 | PASS | 2026-02-06 | Codex | revoked2@example.com |
| 5 | 使用已接受邀请 | PASS | 2026-02-06 | Codex | existing2@example.com（二次访问） |
| 6 | 认证状态检查 | NOT RUN |  |  |  |
