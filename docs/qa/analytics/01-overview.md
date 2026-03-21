# 分析与统计 - 概览与统计测试

**模块**: 分析与统计
**测试范围**: 统计概览、数据可视化、时间范围筛选
**场景数**: 5

---

## 数据库表结构参考

### login_events 表
| 字段 | 类型 | 说明 |
|------|------|------|
| id | CHAR(36) | UUID 主键 |
| user_id | CHAR(36) | 用户 ID |
| tenant_id | CHAR(36) | 租户 ID |
| event_type | VARCHAR(50) | 事件类型 |
| ip_address | VARCHAR(45) | IP 地址 |
| user_agent | TEXT | User Agent |
| device_type | VARCHAR(20) | 设备类型 |
| location | VARCHAR(255) | 地理位置 |
| failure_reason | TEXT | 失败原因 |
| created_at | TIMESTAMP | 事件时间 |

### event_type 值（VARCHAR(50)）
| 值 | 说明 |
|----|------|
| success | 登录成功 |
| social | 社交登录成功（旧版） |
| failed_password | 密码错误 |
| failed_mfa | MFA 验证失败 |
| locked | 账户锁定 |
| federation_success | 联邦登录成功（OAuth2/SAML） |
| federation_failed | 联邦登录失败 |
| identity_linked | 用户绑定外部身份 |
| identity_unlinked | 用户解绑外部身份 |

---

## 场景 1：分析概览入口可见性与页面查看

### 初始状态
- 管理员已登录
- 系统有登录事件数据

> **前置门禁**: 本文档默认“登录流程已单独验收通过”。
> 若从未认证状态进入本场景，需先按 `docs/qa/auth/01-oidc-login.md` 完成登录验证。
> 如果 `/login` 页面本身出现跳转、按钮、CSP 或联邦登录问题，应归类到认证文档，不应作为 Analytics 缺陷建票。

### 目的
验证分析概览页面正确显示统计数据

### 测试操作流程
1. 进入「分析」→「概览」

### 预期结果
- 显示统计卡片：
  - Total Logins（总登录次数）
  - Success Rate（成功率）
  - Failed Attempts（失败次数）
  - Unique Users（独立用户数）
- 显示登录趋势图表
- 显示设备类型分布
- 默认显示过去 7 天数据

### 预期数据状态
```sql
SELECT COUNT(*) as total,
       SUM(CASE WHEN event_type IN ('success', 'social') THEN 1 ELSE 0 END) as success_count,
       COUNT(DISTINCT user_id) as unique_users
FROM login_events
WHERE created_at >= DATE_SUB(NOW(), INTERVAL 7 DAY);
```

---

## 场景 2：切换统计时间范围

### 初始状态
- 管理员已登录
- 在分析概览页面

### 目的
验证时间范围筛选功能

### 测试操作流程
1. 进入「分析」→「概览」
2. 点击时间范围选择器
3. 选择「Last 30 days」
4. 观察数据变化

### 预期结果
- 统计数据重新加载
- 显示过去 30 天的数据
- 图表横轴调整为 30 天范围
- 所有统计卡片数据更新

### 预期数据状态
```sql
SELECT COUNT(*) as total,
       SUM(CASE WHEN event_type IN ('success', 'social') THEN 1 ELSE 0 END) as success_count
FROM login_events
WHERE created_at >= DATE_SUB(NOW(), INTERVAL 30 DAY);
```

---

## 场景 3：查看每日统计

### 初始状态
- 管理员已登录
- 系统有多天的登录数据

### 目的
验证每日统计数据的准确性

> **API 说明**:
> - `period=daily` 表示「最近 24 小时的统计汇总」，不是「每日明细」
> - 每日明细（每天各事件类型的趋势）通过 **`/api/v1/analytics/daily-trend`** 端点获取
> - 区别：`login-stats` 返回聚合数字，`daily-trend` 返回逐日明细数组

### 测试操作流程
1. 进入「分析」→「概览」
2. 查看每日趋势图表（对应 `/api/v1/analytics/daily-trend` 端点）
3. 查看某一天的详细数据

### 预期结果
- 显示每日登录次数趋势
- 可以看到成功/失败的分布
- 数据与数据库记录一致

### 预期数据状态
```sql
SELECT DATE(created_at) as date,
       event_type,
       COUNT(*) as count
FROM login_events
WHERE created_at >= DATE_SUB(NOW(), INTERVAL 7 DAY)
GROUP BY DATE(created_at), event_type
ORDER BY date;
```

---

## 场景 4：查看设备类型统计

### 初始状态
- 管理员已登录
- 系统有不同设备类型的登录数据

### 目的
验证设备类型统计的准确性

### 测试操作流程
1. 进入「分析」→「概览」
2. 查看设备类型分布图表/卡片

### 预期结果
- 显示各设备类型的占比：
  - Desktop
  - Mobile
  - Tablet
  - Other
- 数据与实际登录记录一致

### 预期数据状态
```sql
SELECT device_type, COUNT(*) as count
FROM login_events
WHERE created_at >= DATE_SUB(NOW(), INTERVAL 7 DAY)
GROUP BY device_type;
```

---

## 场景 5：无数据时的显示

### 初始状态
- 管理员已登录
- 系统无登录事件数据（或选择无数据的时间范围）

### 目的
验证无数据时的空状态显示

### 测试操作流程
1. 进入「分析」→「概览」
2. 选择一个没有数据的时间范围（如未来日期）

> **API 日期格式**: `start`/`end` 参数支持两种格式：
> - ISO 8601 完整格式: `2027-01-01T00:00:00Z`
> - 日期格式: `2027-01-01`（start 自动补为 00:00:00，end 自动补为 23:59:59）
>
> 示例: `GET /api/v1/analytics/login-stats?start=2027-01-01&end=2027-01-31`

### 预期结果
- 统计卡片显示 0 或 N/A
- 图表显示空状态或提示无数据
- 页面不报错，正常渲染

### 预期数据状态
```sql
SELECT COUNT(*) FROM login_events
WHERE created_at BETWEEN '{start_date}' AND '{end_date}';
-- 预期: 0
```

---

## 已修复问题

| 日期 | 问题 | 修复 |
|------|------|------|
| 2026-02-03 | Analytics 页面显示 "Failed to load analytics" | 修复 `login_event.rs` 中 SQL 查询的类型不匹配：MySQL/TiDB 的 `SUM()` 返回 `DECIMAL`，需用 `CAST(... AS SIGNED)` 转换为 `i64` |
| 2026-03-21 | `start`/`end` 日期参数使用 `YYYY-MM-DD` 格式时被忽略，静默回退到默认 7 天 | `analytics.rs` 的 `parse_date_param()` 现同时支持 ISO 8601 和日期格式 |

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

### 常见误报排查

| 现象 | 原因 | 解决 |
|------|------|------|
| 在进入 Analytics 前卡在 `/login`，于是提交了 Analytics 缺陷票 | 实际失败点是认证前置步骤，不是 Analytics 页面 | 先执行 `docs/qa/auth/01-oidc-login.md`；仅当登录成功后进入 `/dashboard/analytics` 仍异常，才记录 Analytics 工单 |
| 点击「Sign in with password」后看到浏览器安全/CSP 控制台信息，就认定 Analytics 页面不可用 | QA 在未进入目标页面前就中断，且把登录链路噪音归因到 Analytics | 以”是否成功完成登录并进入 `/dashboard/analytics`”作为判定标准；认证链路问题单独归档到 Auth |

---

## 检查清单

| # | 场景 | 状态 | 测试日期 | 测试人员 | 备注 |
|---|------|------|----------|----------|------|
| 1 | 查看分析概览页面 | ☐ | | | |
| 2 | 切换统计时间范围 | ☐ | | | |
| 3 | 查看每日统计 | ☐ | | | |
| 4 | 查看设备类型统计 | ☐ | | | |
| 5 | 无数据时的显示 | ☐ | | | |
| 6 | 认证状态检查 | ☐ | | | |
