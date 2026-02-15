# 集成测试 - 并发操作测试

**模块**: 集成测试
**测试范围**: 并发操作、竞态条件、数据一致性
**场景数**: 4
**优先级**: 高

---

## 测试说明

本文档测试系统在并发操作场景下的数据一致性和安全性。这些测试需要使用专门的并发测试工具（如 JMeter、k6）或脚本来执行。

---

## 场景 1：并发创建相同邮箱的用户

### 初始状态
- 系统中不存在邮箱 `concurrent@example.com` 的用户
- 准备 10 个并发请求，同时创建相同邮箱的用户

### 目的
验证系统在并发情况下正确处理重复邮箱约束

### 测试操作流程
1. 使用并发测试工具（如 k6）准备 10 个并发请求
2. 每个请求尝试创建用户：
   ```bash
   POST /api/v1/users
   {
     "email": "concurrent@example.com",
     "username": "concurrent-user-${VU_ID}",
     "password": "Test123!",
     "first_name": "Concurrent",
     "last_name": "Test"
   }
   ```
3. 同时发送所有 10 个请求
4. 检查所有响应状态码
5. 查询数据库验证最终状态

### 预期结果
- 只有 1 个请求成功（状态码 201）
- 其余 9 个请求失败（状态码 400 或 409），错误信息为「Email already exists」
- 数据库中只存在 1 条记录

### 预期数据状态
```sql
SELECT COUNT(*) FROM users WHERE email = 'concurrent@example.com';
-- 预期: 1

SELECT COUNT(*) FROM audit_logs 
WHERE resource_type = 'user' AND action = 'create' 
  AND JSON_EXTRACT(details, '$.email') = 'concurrent@example.com';
-- 预期: 1（只记录成功的创建）
```

---

## 场景 2：并发密码重置令牌生成

### 初始状态
- 用户 `reset@example.com` 已存在
- 准备 20 个并发密码重置请求

### 目的
验证系统正确处理并发密码重置，避免生成多个有效令牌

### 测试操作流程
1. 使用 k6 发送 20 个并发请求：
   ```bash
   POST /api/v1/auth/forgot-password
   {
     "email": "reset@example.com"
   }
   ```
2. 所有请求同时发送
3. 检查数据库中 `password_reset_tokens` 表
4. 尝试使用所有返回的令牌

### 预期结果
- 所有请求返回成功（状态码 200）
- 数据库中只有 1 个有效令牌（`used_at` 为 NULL）
- 其他令牌要么不存在，要么已被标记为失效
- 只有最新的令牌可以成功重置密码

### 预期数据状态
```sql
SELECT COUNT(*) FROM password_reset_tokens 
WHERE user_id = (SELECT id FROM users WHERE email = 'reset@example.com')
  AND used_at IS NULL 
  AND expires_at > NOW();
-- 预期: 1（只有最新的令牌有效）

SELECT COUNT(*) FROM password_reset_tokens 
WHERE user_id = (SELECT id FROM users WHERE email = 'reset@example.com')
  AND created_at >= DATE_SUB(NOW(), INTERVAL 1 MINUTE);
-- 预期: 可能是 1-20 之间（取决于实现策略：覆盖 vs 创建新的）
```

---

## 场景 3：并发权限分配操作

### 初始状态
- 租户中存在角色 `editor`
- 存在用户 `user@example.com` 在该租户中
- 准备 10 个并发请求，将同一角色分配给同一用户

### 目的
验证系统避免重复分配相同权限

### 测试操作流程
1. 使用 k6 发送 10 个并发请求：
   ```bash
   POST /api/v1/tenants/{tenant_id}/users/{user_id}/roles
   {
     "role_id": "{editor_role_id}"
   }
   ```
2. 同时发送所有请求
3. 查询 `user_tenant_roles` 表验证结果

### 预期结果
- 只有 1 个请求成功（状态码 201）
- 其余 9 个请求返回 409 Conflict（角色已存在）或成功返回幂等响应
- 数据库中只有 1 条 `user_tenant_roles` 记录

### 预期数据状态
```sql
SELECT COUNT(*) FROM user_tenant_roles utr
JOIN tenant_users tu ON tu.id = utr.tenant_user_id
WHERE tu.user_id = '{user_id}' 
  AND tu.tenant_id = '{tenant_id}'
  AND utr.role_id = '{editor_role_id}';
-- 预期: 1
```

---

## 场景 4：并发 Webhook 事件触发

### 初始状态
- 租户配置了 Webhook URL: `https://webhook.example.com/auth9`
- 准备触发 100 个用户创建事件（每个触发 1 个 Webhook）

### 目的
验证 Webhook 系统在高并发下的可靠性

### 测试操作流程
1. 使用脚本快速创建 100 个用户
2. 每个用户创建触发 `user.created` Webhook 事件
3. 监控 Webhook 服务器接收到的请求数量
4. 检查 `webhook_deliveries` 表（如果存在）

### 预期结果
- 100 个 Webhook 请求最终都被发送（可能有重试）
- 每个请求的签名验证通过
- Webhook 服务器接收到正确的事件数据
- 无请求丢失
- 重试机制正常工作（失败的请求会自动重试）

### 预期数据状态
```sql
-- 假设有 webhook_deliveries 表记录发送历史
SELECT COUNT(*) FROM webhook_deliveries 
WHERE webhook_id = '{webhook_id}' 
  AND event_type = 'user.created'
  AND created_at >= DATE_SUB(NOW(), INTERVAL 5 MINUTE)
  AND status = 'delivered';
-- 预期: 100

-- 检查失败的发送
SELECT COUNT(*) FROM webhook_deliveries 
WHERE webhook_id = '{webhook_id}' 
  AND status = 'failed'
  AND retry_count < max_retry;
-- 预期: 0（或小数量，取决于网络）
```

---

## 测试工具推荐

### k6 示例脚本
```javascript
import http from 'k6/http';
import { check } from 'k6';

export let options = {
  vus: 10,           // 10 个虚拟用户
  duration: '30s',   // 持续 30 秒
};

export default function() {
  let payload = JSON.stringify({
    email: 'concurrent@example.com',
    username: `user-${__VU}-${__ITER}`,
    password: 'Test123!',
  });

  let params = {
    headers: {
      'Content-Type': 'application/json',
      'Authorization': 'Bearer YOUR_TOKEN',
    },
  };

  let res = http.post('http://localhost:8080/api/v1/users', payload, params);
  
  check(res, {
    'status is 201 or 409': (r) => r.status === 201 || r.status === 409,
  });
}
```

---

## 问题报告格式

如发现并发问题，请详细记录：

```markdown
## Concurrency Bug: [简短描述]

**测试场景**: #X
**并发数**: 10/50/100
**复现率**: 10/10, 5/10, 偶现

**现象**:
- 数据库中出现 X 条重复记录
- 死锁日志: [粘贴日志]
- 响应时间异常: P95=5000ms

**数据库状态**: [SQL 查询结果]
**日志**: [相关错误日志]
```
