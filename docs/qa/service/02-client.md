# 服务管理 - 客户端管理测试

**模块**: 服务与客户端管理
**测试范围**: 客户端 CRUD、密钥管理
**场景数**: 5

---

## 数据库表结构参考

### clients 表
| 字段 | 类型 | 说明 |
|------|------|------|
| id | CHAR(36) | UUID 主键 |
| service_id | CHAR(36) | 所属服务 ID |
| client_id | VARCHAR(255) | 客户端 ID（唯一） |
| client_secret_hash | VARCHAR(255) | 客户端密钥哈希 |
| name | VARCHAR(255) | 客户端名称 |
| created_at | TIMESTAMP | 创建时间 |

---

## 场景 1：创建客户端

### 初始状态
- 存在服务 id=`{service_id}`

### 目的
验证为服务创建新客户端

### 测试操作流程
1. 进入服务详情页
2. 点击「创建客户端」 (UI: + 号按钮)
3. 填写：
   - Description：`Mobile App Client` (UI 仅支持输入描述，Client ID 自动生成)
4. 点击「创建」

### 预期结果
- 显示创建成功
- 显示 Client Secret（仅此一次）
- 显示自动生成的 Client ID
- 客户端出现在列表中

### 预期数据状态
```sql
SELECT id, service_id, client_id, name FROM clients WHERE name = 'Mobile App Client';
-- 预期: 存在记录，client_id 为 UUID 格式，client_secret_hash 非空
```

---

## 场景 2：重新生成客户端密钥

### 初始状态
- 存在客户端 id=`{client_id}`
- 记录当前 client_secret_hash 值

### 目的
验证客户端密钥重新生成功能

### 测试操作流程
1. 找到目标客户端
2. 点击「重新生成密钥」
3. 确认操作

### 预期结果
- 显示新的 Client Secret
- 旧密钥立即失效

### 预期数据状态
```sql
SELECT client_secret_hash FROM clients WHERE id = '{client_id}';
-- 预期: client_secret_hash 与之前不同

-- 验证旧密钥无法使用
```

---

## 场景 3：删除客户端

### 初始状态
- 服务有 2 个客户端
- 目标客户端 id=`{client_id}`

### 目的
验证客户端删除功能

### 测试操作流程
1. 找到目标客户端
2. 点击「删除」
3. 确认删除

### 预期结果
- 显示删除成功
- 客户端从列表消失

### 预期数据状态
```sql
SELECT COUNT(*) FROM clients WHERE id = '{client_id}';
-- 预期: 0
```

---

## 场景 4：验证客户端凭证（正确）

### 初始状态
- 存在客户端，已知 client_id 和 client_secret

### 目的
验证正确凭证通过验证

### 测试操作流程
1. 使用正确的 client_id 和 client_secret 调用 API

### 预期结果
- 请求成功

### 预期数据状态
```sql
-- 无数据变化
```

---

## 场景 5：验证客户端凭证（错误）

### 初始状态
- 存在客户端

### 目的
验证错误凭证被拒绝

### 测试操作流程
1. 使用错误的 client_secret 调用 API

### 预期结果
- 返回 401 Unauthorized

### 预期数据状态
```sql
-- 可选：验证审计日志记录失败尝试
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
| 1 | 创建客户端 | ☐ | | | |
| 2 | 重新生成密钥 | ☐ | | | |
| 3 | 删除客户端 | ☐ | | | |
| 4 | 验证凭证（正确） | ☐ | | | |
| 5 | 验证凭证（错误） | ☐ | | | |
| 6 | 认证状态检查 | ☐ | | | |
