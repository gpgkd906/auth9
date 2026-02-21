# 会话与安全 - 登录事件测试

**模块**: 会话与安全
**测试范围**: 登录事件记录、分析
**场景数**: 5

---

## 架构说明

Auth9 采用 Headless Keycloak 架构，登录事件的产生和记录涉及两个系统：

1. **登录操作发生在 Keycloak** → 测试可通过 Auth9 登录入口触发 OIDC 流程（底层由 Keycloak 执行用户名/密码与 MFA 验证）
2. **事件通过 Redis Stream 异步传递** → Keycloak 事件先进入 `auth9:keycloak:events`，由 auth9-core 后台消费者拉取处理
3. **Auth9 Core 记录和分析** → Auth9 消费事件后写入 `login_events` 表，并触发安全检测（如暴力破解告警）

兼容说明：
- `POST /api/v1/keycloak/events` 作为兼容入口仍可用于回归测试，但不再是默认主链路。

**关键点**：Auth9 不直接处理用户名/密码验证，所有认证均通过 Keycloak OIDC 流程完成。本文档不要求必须手工访问 Keycloak 登录页 URL。

---

## 测试前置数据（必需）

在执行本文件场景前，先执行：

```bash
mysql -h 127.0.0.1 -P 4000 -u root auth9 < docs/qa/session/seed.sql
```

说明：
- `seed.sql` 会创建管理员与目标用户的会话数据
- 管理员：`admin@auth9.local`
- 目标用户：`target@example.com`

---

## 数据库表结构参考

### login_events 表
| 字段 | 类型 | 说明 |
|------|------|------|
| id | BIGINT | 自增主键 |
| user_id | CHAR(36) | 用户 ID |
| email | VARCHAR(320) | 用户邮箱 |
| event_type | ENUM | success/failed_password/failed_mfa/locked/social |
| ip_address | VARCHAR(45) | IP 地址 |
| device_type | VARCHAR(50) | 设备类型 |
| failure_reason | VARCHAR(255) | 失败原因 |
| created_at | TIMESTAMP | 事件时间 |

---

## 场景 1：登录成功事件记录

### 初始状态
- 用户使用正确凭证登录

### 目的
验证成功登录事件被正确记录

### 测试操作流程
1. 用户使用正确密码登录
2. 登录成功

### 预期结果
- 登录成功
- 事件被记录

### 预期数据状态
```sql
SELECT event_type, ip_address, device_type, created_at FROM login_events
WHERE user_id = '{user_id}' ORDER BY created_at DESC LIMIT 1;
-- 预期: event_type = 'success'
```

---

## 场景 2：登录失败事件记录

### 初始状态
- 用户使用错误密码登录

### 目的
验证失败登录事件被正确记录

### 测试操作流程
1. 用户输入错误密码
2. 登录失败

### 预期结果
- 登录失败
- 失败事件被记录

### 预期数据状态
```sql
SELECT event_type, failure_reason FROM login_events
WHERE email = 'user@example.com' ORDER BY created_at DESC LIMIT 1;
-- 预期: event_type = 'failed_password'
```

---

## 场景 3：MFA 失败事件记录

### 初始状态
- 用户启用了 MFA
- 用户输入错误的 MFA 代码

### 目的
验证 MFA 失败事件被记录

### 测试操作流程
1. 正确输入密码
2. 在 MFA 验证界面输入错误代码

### 预期结果
- MFA 验证失败
- 事件被记录

### 预期数据状态
```sql
SELECT event_type, failure_reason FROM login_events
WHERE user_id = '{user_id}' ORDER BY created_at DESC LIMIT 1;
-- 预期: event_type = 'failed_mfa'
```

---

## 场景 4：账户锁定事件

### 初始状态
- 用户连续多次登录失败

### 目的
验证账户锁定机制和事件记录

### 测试操作流程
1. 连续 5 次使用错误密码登录

### 预期结果
- 账户被临时锁定
- 锁定事件被记录

### 预期数据状态
```sql
SELECT event_type, created_at FROM login_events
WHERE user_id = '{user_id}' ORDER BY created_at DESC LIMIT 6;
-- 预期: 最后一条为 'locked'
```

---

## 场景 5：登录分析统计

### 初始状态
- 系统有一定数量的登录事件数据

### 目的
验证登录分析功能

### 测试操作流程
1. 进入「分析」页面
2. 选择时间范围：7天/14天/30天/90天

### 预期结果
- 显示总登录次数
- 显示成功/失败比例
- 显示按设备类型分布

### 预期数据状态
```sql
SELECT
    COUNT(*) as total,
    SUM(CASE WHEN event_type = 'success' THEN 1 ELSE 0 END) as success,
    COUNT(DISTINCT user_id) as unique_users
FROM login_events WHERE created_at >= DATE_SUB(NOW(), INTERVAL 7 DAY);
```

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
| 1 | 登录成功事件 | ☐ | | | |
| 2 | 登录失败事件 | ☐ | | | |
| 3 | MFA 失败事件 | ☐ | | | |
| 4 | 账户锁定事件 | ☐ | | | |
| 5 | 登录分析统计 | ☐ | | | |
| 6 | 认证状态检查 | ☐ | | | |
