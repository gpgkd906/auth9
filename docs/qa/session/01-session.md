# 会话管理 - 会话操作测试

**模块**: 会话与安全
**测试范围**: 会话查看、撤销、强制登出
**场景数**: 5

---

## 数据库表结构参考

### sessions 表
| 字段 | 类型 | 说明 |
|------|------|------|
| id | CHAR(36) | UUID 主键 |
| user_id | CHAR(36) | 用户 ID |
| keycloak_session_id | VARCHAR(255) | Keycloak 会话 ID |
| device_type | VARCHAR(50) | 设备类型 |
| device_name | VARCHAR(255) | 设备名称 |
| ip_address | VARCHAR(45) | IP 地址 |
| location | VARCHAR(255) | 地理位置 |
| last_active_at | TIMESTAMP | 最后活跃时间 |
| created_at | TIMESTAMP | 创建时间 |
| revoked_at | TIMESTAMP | 撤销时间 |

---

## 场景 1：查看当前会话列表

### 初始状态
- 用户已在多个设备登录
- 至少有 3 个活跃会话

### 目的
验证会话列表正确显示

### 测试操作流程
1. 登录系统
2. 进入「设置」→「会话管理」

### 预期结果
- 显示所有活跃会话
- 当前会话有特殊标记
- 每个会话显示：设备类型、IP、位置、最后活跃时间

### 预期数据状态
```sql
SELECT id, device_type, ip_address, location, last_active_at FROM sessions
WHERE user_id = '{user_id}' AND revoked_at IS NULL ORDER BY last_active_at DESC;
-- 预期: 返回所有未撤销的会话
```

---

## 场景 2：撤销单个会话

### 初始状态
- 用户有会话 id=`{session_id}`（非当前会话）

### 目的
验证撤销特定会话功能

### 测试操作流程
1. 找到目标会话
2. 点击「撤销」
3. 确认撤销

### 预期结果
- 显示撤销成功
- 目标会话从列表消失
- 该设备被强制登出

### 预期数据状态
```sql
SELECT revoked_at FROM sessions WHERE id = '{session_id}';
-- 预期: revoked_at 有值
```

---

## 场景 3：撤销所有其他会话

### 初始状态
- 用户有 5 个活跃会话
- 当前会话 id=`{current_session_id}`

### 目的
验证一键撤销其他所有会话

### 测试操作流程
1. 点击「撤销所有其他会话」
2. 确认操作

### 预期结果
- 显示撤销成功
- 只保留当前会话
- 所有其他设备被强制登出

### 预期数据状态
```sql
SELECT id, revoked_at FROM sessions WHERE user_id = '{user_id}';
-- 预期: 4 条 revoked_at 有值，1 条为 NULL
```

---

## 场景 4：管理员强制用户登出

### 初始状态
- 管理员已登录
- 目标用户有活跃会话

### 目的
验证管理员强制登出功能

### 测试操作流程
1. 进入用户管理
2. 找到目标用户
3. 点击「强制登出」
4. 确认操作

### 预期结果
- 显示操作成功
- 目标用户所有会话被撤销
- 用户被强制登出

### 预期数据状态
```sql
SELECT COUNT(*) FROM sessions WHERE user_id = '{target_user_id}' AND revoked_at IS NULL;
-- 预期: 0
```

---

## 场景 5：会话自动过期

### 初始状态
- 用户有会话，已超过会话超时时间

### 目的
验证会话超时机制

### 测试操作流程
1. 创建会话
2. 等待超过超时时间
3. 尝试使用该会话访问系统

### 预期结果
- 访问被拒绝
- 用户被要求重新登录

### 预期数据状态
```sql
SELECT last_active_at FROM sessions WHERE id = '{session_id}';
-- 验证 last_active_at 超过允许的空闲时间
```

---

## 检查清单

| # | 场景 | 状态 | 测试日期 | 测试人员 | 备注 |
|---|------|------|----------|----------|------|
| 1 | 查看会话列表 | ☐ | | | |
| 2 | 撤销单个会话 | ☐ | | | |
| 3 | 撤销所有其他会话 | ☐ | | | |
| 4 | 管理员强制登出 | ☐ | | | |
| 5 | 会话自动过期 | ☐ | | | |
