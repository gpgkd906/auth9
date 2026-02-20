# RBAC/ABAC - ABAC 策略管理测试

**模块**: RBAC 角色权限管理
**测试范围**: ABAC 策略草稿、发布、回滚、模拟与租户隔离
**场景数**: 5
**优先级**: 高

---

## 背景说明

ABAC 管理 API 已上线，支持按租户管理策略版本：

- `GET /api/v1/tenants/{tenant_id}/abac/policies`
- `POST /api/v1/tenants/{tenant_id}/abac/policies`
- `PUT /api/v1/tenants/{tenant_id}/abac/policies/{version_id}`
- `POST /api/v1/tenants/{tenant_id}/abac/policies/{version_id}/publish`
- `POST /api/v1/tenants/{tenant_id}/abac/policies/{version_id}/rollback`
- `POST /api/v1/tenants/{tenant_id}/abac/simulate`

Portal 页面：`/dashboard/abac`

---

## 数据库表结构参考

### abac_policy_sets 表
| 字段 | 类型 | 说明 |
|------|------|------|
| id | CHAR(36) | policy set 主键 |
| tenant_id | CHAR(36) | 租户 ID（唯一） |
| mode | VARCHAR(16) | `disabled/shadow/enforce` |
| published_version_id | CHAR(36) | 当前发布版本 |
| created_at | TIMESTAMP | 创建时间 |
| updated_at | TIMESTAMP | 更新时间 |

### abac_policy_set_versions 表
| 字段 | 类型 | 说明 |
|------|------|------|
| id | CHAR(36) | 版本主键 |
| policy_set_id | CHAR(36) | 对应 policy set |
| version_no | INT | 递增版本号 |
| status | VARCHAR(16) | `draft/published/archived` |
| policy_json | JSON | 策略文档 |
| change_note | VARCHAR(255) | 变更说明 |
| created_by | CHAR(36) | 创建者用户 ID |
| created_at | TIMESTAMP | 创建时间 |
| published_at | TIMESTAMP | 发布时间 |

---

## 场景 1：创建 ABAC 草稿版本

### 初始状态
- 已登录租户管理员账号
- 目标租户 `{tenant_id}` 已存在
- `abac_policy_sets` 中该租户可能无记录

### 目的
验证 ABAC 草稿创建成功，并正确写入版本号和策略 JSON。

### 测试操作流程
1. 打开 `/dashboard/abac`
2. 在「创建草稿」区域输入 `policy_json` 和 `change_note`
3. 点击「Create Draft」
4. 刷新页面查看版本列表

### 预期结果
- 页面显示新版本（`status=draft`）
- 版本号从 1 开始递增
- 无报错提示

### 预期数据状态
```sql
SELECT id, tenant_id, mode, published_version_id
FROM abac_policy_sets
WHERE tenant_id = '{tenant_id}';
-- 预期: 存在记录, mode 默认为 disabled

SELECT version_no, status, change_note
FROM abac_policy_set_versions
WHERE policy_set_id = '{policy_set_id}'
ORDER BY version_no DESC
LIMIT 1;
-- 预期: version_no 递增, status='draft'
```

---

## 场景 2：发布策略并切换到 shadow 模式

### 初始状态
- 存在草稿版本 `{version_id}`
- 当前未发布版本或已有旧发布版本

### 目的
验证发布动作会更新当前发布版本，并将模式切换为 `shadow`。

### 测试操作流程
1. 在版本列表找到 `{version_id}`
2. 点击「Publish (shadow)」
3. 刷新页面查看「当前状态」

### 预期结果
- `Mode` 显示 `shadow`
- `Published` 指向当前版本
- 旧发布版本（如有）状态被归档

### 预期数据状态
```sql
SELECT mode, published_version_id
FROM abac_policy_sets
WHERE tenant_id = '{tenant_id}';
-- 预期: mode='shadow', published_version_id='{version_id}'

SELECT id, status, published_at
FROM abac_policy_set_versions
WHERE policy_set_id = '{policy_set_id}'
ORDER BY version_no DESC;
-- 预期: 当前版本 status='published'，其他旧 published 版本为 archived
```

---

## 场景 3：回滚到历史版本

### 初始状态
- 同一租户至少有两个版本（例如 v1、v2）
- 当前发布为 v2

### 目的
验证回滚会将发布指针切回历史版本，并保留版本历史。

### 测试操作流程
1. 在版本列表中对 v1 点击「Rollback to This」
2. 页面刷新后查看当前发布版本
3. 重新调用列表 API 校验版本状态

### 预期结果
- 当前发布版本变为 v1
- v2 变为 `archived`
- 页面显示回滚成功，无 4xx/5xx

### 预期数据状态
```sql
SELECT published_version_id, mode
FROM abac_policy_sets
WHERE tenant_id = '{tenant_id}';
-- 预期: published_version_id 为回滚目标版本

SELECT version_no, status
FROM abac_policy_set_versions
WHERE policy_set_id = '{policy_set_id}'
ORDER BY version_no ASC;
-- 预期: 目标版本='published'，其余非 draft 版本='archived'
```

---

## 场景 4：策略模拟（allow/deny 命中规则）

### 初始状态
- 存在至少一个可用策略版本（发布版或草稿 JSON）
- 策略包含 allow/deny 规则（例如办公时间 deny/owner allow）

### 目的
验证模拟接口返回正确决策及命中规则列表。

### 测试操作流程
1. 在 `/dashboard/abac` 的「策略模拟」输入：
   - `Action`: `user_manage`
   - `Resource Type`: `tenant`
   - `subject/resource/request/env` JSON
2. 点击「Run Simulation」
3. 记录返回的 `decision` 与命中规则

### 预期结果
- 返回 `allow` 或 `deny`
- `matched_allow_rule_ids`、`matched_deny_rule_ids` 与策略条件一致
- 输入非法 JSON 时返回错误提示

### 预期数据状态
```sql
SELECT published_version_id
FROM abac_policy_sets
WHERE tenant_id = '{tenant_id}';
-- 预期: 模拟操作不修改数据库记录
```

---

## 场景 5：租户隔离与权限校验

### 初始状态
- 准备两个租户：`{tenant_a}`、`{tenant_b}`
- 使用 `{tenant_a}` 的 tenant token 登录

### 目的
验证 ABAC 管理接口仅允许操作当前租户，不能跨租户访问。

### 测试操作流程
1. 使用 `{tenant_a}` token 调用 `{tenant_a}` 的 ABAC API（应成功）
2. 使用相同 token 调用 `{tenant_b}` 的 ABAC API（应失败）
3. 使用无 `abac:*`/`rbac:*` 的普通成员 token 再次调用（应失败）

### 预期结果
- 同租户管理员可访问
- 跨租户或低权限角色返回 `403 Forbidden`
- 不会写入 `{tenant_b}` 的策略数据

### 预期数据状态
```sql
SELECT tenant_id, COUNT(*) AS version_count
FROM abac_policy_sets ps
LEFT JOIN abac_policy_set_versions psv ON psv.policy_set_id = ps.id
WHERE tenant_id IN ('{tenant_a}', '{tenant_b}')
GROUP BY tenant_id;
-- 预期: tenant_b 的 version_count 不因 tenant_a token 调用而变化
```

---

## 通用场景：API 回归验证（curl）

### 测试操作流程
1. 使用租户管理员 token 执行以下请求

```bash
TOKEN="{tenant_admin_access_token}"
TENANT_ID="{tenant_id}"

curl -sS -X GET "http://localhost:8080/api/v1/tenants/${TENANT_ID}/abac/policies" \
  -H "Authorization: Bearer ${TOKEN}"

curl -sS -X POST "http://localhost:8080/api/v1/tenants/${TENANT_ID}/abac/simulate" \
  -H "Authorization: Bearer ${TOKEN}" \
  -H "Content-Type: application/json" \
  -d '{
    "simulation": {
      "action": "user_manage",
      "resource_type": "tenant",
      "subject": {"roles": ["admin"]},
      "resource": {"tenant_id": "'${TENANT_ID}'"},
      "request": {"ip": "127.0.0.1"},
      "env": {"hour": 10}
    }
  }'
```

### 预期结果
- 接口返回 200
- `simulate` 返回 `decision` 与策略条件一致

---

## 检查清单

| # | 场景 | 状态 | 测试日期 | 测试人员 | 备注 |
|---|------|------|----------|----------|------|
| 1 | 创建 ABAC 草稿版本 | ☐ | | | |
| 2 | 发布策略并切换到 shadow 模式 | ☐ | | | |
| 3 | 回滚到历史版本 | ☐ | | | |
| 4 | 策略模拟（allow/deny 命中规则） | ☐ | | | |
| 5 | 租户隔离与权限校验 | ☐ | | | |
| 6 | API 回归验证（curl） | ☐ | | | |
