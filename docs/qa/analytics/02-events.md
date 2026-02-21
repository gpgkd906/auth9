# 分析与统计 - 登录事件列表测试

**模块**: 分析与统计
**测试范围**: 登录事件列表、筛选、分页
**场景数**: 5

---

## 场景 1：查看登录事件列表

### 初始状态
- 管理员已登录
- 系统有登录事件数据

### 目的
验证登录事件列表正确显示

### 测试操作流程
1. 进入「分析」→「登录事件」

### 预期结果
- 显示事件列表表格，包含列：
  - Time（时间）
  - Event（事件类型，带颜色标记）
  - User（用户邮箱或 ID）
  - IP Address（IP 地址）
  - Device（设备类型）
  - Details（失败原因或位置）
- 事件按时间倒序排列
- 显示总事件数

### 预期数据状态
```sql
SELECT id, user_id, event_type, ip_address, device_type, failure_reason, created_at
FROM login_events
ORDER BY created_at DESC
LIMIT 50;
```

---

## 场景 2：分页浏览事件

### 初始状态
- 管理员已登录
- 系统有超过 50 条登录事件

### 目的
验证事件列表分页功能

### 测试操作流程
1. 进入「分析」→「登录事件」
2. 滚动到页面底部
3. 点击「Next」或页码切换到下一页
4. 点击「Previous」返回上一页

### 预期结果
- 显示分页信息「Page X of Y」
- 点击下一页加载新数据
- 数据不重复
- 页码正确更新

### 预期数据状态
```sql
SELECT COUNT(*) FROM login_events;
-- 预期: 总数 > 50

SELECT id, created_at FROM login_events
ORDER BY created_at DESC
LIMIT 50 OFFSET 50;
-- 预期: 第二页的数据
```

---

## 场景 3：识别不同事件类型

### 初始状态
- 管理员已登录
- 系统有不同类型的登录事件

### 目的
验证不同事件类型的视觉区分

### 测试操作流程
1. 进入「分析」→「登录事件」
2. 观察不同类型事件的显示

### 预期结果
- **Login Success**：绿色标记 ✓
- **Social Login**：绿色标记
- **Wrong Password**：红色标记 ✗
- **MFA Failed**：红色标记
- **Account Locked**：橙色标记 🔒
- 每种类型有对应的图标和颜色

### 预期数据状态
```sql
SELECT DISTINCT event_type FROM login_events;
-- 预期: 包含多种事件类型
```

---

## 场景 4：查看失败事件详情

### 初始状态
- 管理员已登录
- 系统有失败的登录事件

### 目的
验证失败事件显示失败原因

### 测试操作流程
1. 进入「分析」→「登录事件」
2. 找到一个失败事件（如 Wrong Password）
3. 查看 Details 列

### 预期结果
- 显示失败原因（如 "Invalid password"）
- 原因以红色文字显示
- 成功事件可能显示位置信息

### 预期数据状态
```sql
SELECT event_type, failure_reason, location
FROM login_events
WHERE event_type IN ('failed_password', 'failed_mfa', 'locked')
LIMIT 10;
```

---

## 场景 5：按用户或租户筛选事件

### 初始状态
- 管理员已登录
- 系统有多个用户的登录事件

### 目的
验证按用户/租户筛选事件功能

### 测试操作流程
1. 进入「用户管理」页面
2. 选择一个用户
3. 查看该用户的登录历史/事件
4. 或：通过 API 按 tenant_id 筛选

### 预期结果
- 只显示该用户/租户的登录事件
- 分页正常工作
- 统计数据只计算筛选结果

### 预期数据状态
```sql
SELECT COUNT(*) FROM login_events WHERE user_id = '{user_id}';

SELECT * FROM login_events
WHERE user_id = '{user_id}'
ORDER BY created_at DESC
LIMIT 50;
```

---

## 事件类型参考

| 事件类型 | 说明 | 颜色 |
|---------|------|------|
| success | 密码登录成功 | 绿色 |
| social | 社交登录成功 | 绿色 |
| failed_password | 密码错误 | 红色 |
| failed_mfa | MFA 验证失败 | 红色 |
| locked | 账户被锁定 | 橙色 |

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
| 1 | 查看登录事件列表 | ☐ | | | |
| 2 | 分页浏览事件 | ☐ | | | |
| 3 | 识别不同事件类型 | ☐ | | | |
| 4 | 查看失败事件详情 | ☐ | | | |
| 5 | 按用户或租户筛选事件 | ☐ | | | |
| 6 | 认证状态检查 | ☐ | | | |
