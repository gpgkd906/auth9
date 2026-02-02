# Webhook 管理 - CRUD 操作测试

**模块**: Webhook 管理
**测试范围**: Webhook 创建、更新、删除、启用/禁用
**场景数**: 5

---

## 数据库表结构参考

### webhooks 表
| 字段 | 类型 | 说明 |
|------|------|------|
| id | CHAR(36) | UUID 主键 |
| tenant_id | CHAR(36) | 所属租户 ID |
| name | VARCHAR(255) | Webhook 名称 |
| url | VARCHAR(500) | 目标 URL |
| secret | VARCHAR(255) | 签名密钥 |
| events | JSON | 订阅的事件类型列表 |
| enabled | BOOLEAN | 是否启用 |
| failure_count | INT | 连续失败次数 |
| created_at | TIMESTAMP | 创建时间 |

---

## 场景 1：创建 Webhook

### 初始状态
- 管理员已登录
- 存在租户 id=`{tenant_id}`

### 目的
验证 Webhook 创建功能

### 测试操作流程
1. 进入租户的「Webhook」管理页面
2. 点击「创建 Webhook」
3. 填写：
   - 名称：`User Events Webhook`
   - URL：`https://api.example.com/webhooks/auth9`
   - 事件：选择 `user.created`, `user.updated`, `user.deleted`
4. 点击「创建」

### 预期结果
- 显示创建成功
- 显示生成的 Secret
- Webhook 出现在列表中

### 预期数据状态
```sql
SELECT id, name, url, events, enabled, secret FROM webhooks
WHERE name = 'User Events Webhook' AND tenant_id = '{tenant_id}';
-- 预期: 存在记录，enabled = true，secret 非空
```

---

## 场景 2：更新 Webhook

### 初始状态
- 存在 Webhook id=`{webhook_id}`

### 目的
验证 Webhook 配置更新功能

### 测试操作流程
1. 进入租户详情 → Webhooks 标签页
2. 找到目标 Webhook，点击行末的 **✏️ 铅笔图标**（Edit 按钮）
3. 在弹出的编辑对话框中修改：
   - 添加事件：`session.revoked`
   - 修改 URL：`https://api.example.com/webhooks/auth9/v2`
4. 点击「Update webhook」保存

> **注意**: 编辑功能通过 Modal 弹窗实现，不是独立页面

### 预期结果
- 弹窗关闭
- 列表中显示更新后的配置
- Toast 提示更新成功

### 预期数据状态
```sql
SELECT url, events, updated_at FROM webhooks WHERE id = '{webhook_id}';
-- 预期: 包含新的 URL 和事件
```

---

## 场景 3：禁用 Webhook

### 初始状态
- 存在已启用的 Webhook

### 目的
验证 Webhook 禁用功能

### 测试操作流程
1. 进入租户详情 → Webhooks 标签页
2. 找到目标 Webhook，点击 **✏️ 铅笔图标** 打开编辑弹窗
3. 找到「Enabled」开关，将其关闭
4. 点击「Update webhook」保存

> **注意**: 启用/禁用开关在编辑弹窗内，不是列表行上的独立开关

### 预期结果
- 弹窗关闭
- 列表中该 Webhook 状态指示器变为灰色（已禁用）
- Test 按钮变为禁用状态

### 预期数据状态
```sql
SELECT enabled FROM webhooks WHERE id = '{webhook_id}';
-- 预期: enabled = false
```

---

## 场景 4：启用 Webhook

### 初始状态
- 存在已禁用的 Webhook

### 目的
验证 Webhook 重新启用功能

### 测试操作流程
1. 进入租户详情 → Webhooks 标签页
2. 找到目标 Webhook，点击 **✏️ 铅笔图标** 打开编辑弹窗
3. 找到「Enabled」开关，将其开启
4. 点击「Update webhook」保存

### 预期结果
- 弹窗关闭
- 列表中该 Webhook 状态指示器变为绿色（已启用）
- Test 按钮变为可用状态

### 预期数据状态
```sql
SELECT enabled FROM webhooks WHERE id = '{webhook_id}';
-- 预期: enabled = true
```

---

## 场景 5：删除 Webhook

### 初始状态
- 存在 Webhook id=`{webhook_id}`

### 目的
验证 Webhook 删除功能

### 测试操作流程
1. 找到目标 Webhook
2. 点击「删除」
3. 确认删除

### 预期结果
- 显示删除成功
- Webhook 从列表消失

### 预期数据状态
```sql
SELECT COUNT(*) FROM webhooks WHERE id = '{webhook_id}';
-- 预期: 0
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
| 1 | 创建 Webhook | ☐ | | | |
| 2 | 更新 Webhook | ☐ | | | |
| 3 | 禁用 Webhook | ☐ | | | |
| 4 | 启用 Webhook | ☐ | | | |
| 5 | 删除 Webhook | ☐ | | | |
| 6 | 认证状态检查 | ☐ | | | |
