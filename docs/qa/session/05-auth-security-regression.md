# 会话与安全 - 鉴权与令牌安全回归测试

**模块**: 会话与安全
**测试范围**: 管理员端点鉴权、refresh 会话一致性、OIDC 回调令牌泄露、限流键安全
**场景数**: 5
**优先级**: 高

---

## 去重说明

本文件仅覆盖以下“待修复缺陷”的回归验证，避免与已有文档重复：
- `session/01-session.md` 已覆盖会话功能可用性，但未覆盖“普通用户越权调用管理员强制登出端点”。
- `integration/03-rate-limiting.md` 已覆盖基础限流行为，但未覆盖“`x-tenant-id` 伪造绕过”和“路径高基数键膨胀”。
- `auth/01-oidc-login.md` 已覆盖登录流程成功路径，但未覆盖“回调重定向 URL 泄露 token 参数”。

---

## 场景 1：普通用户调用管理员强制登出端点应被拒绝

### 初始状态
- 存在普通用户 Token：`{normal_user_token}`
- 存在目标用户 id：`{target_user_id}`，且有至少 1 条活跃会话

### 目的
验证 `/api/v1/admin/users/{id}/logout` 具备管理员鉴权，普通用户不能执行。

### 测试操作流程
1. 记录目标用户当前活跃会话数。
2. 使用普通用户 Token 调用管理员强制登出端点：
   ```bash
   curl -i -X POST "http://localhost:8080/api/v1/admin/users/{target_user_id}/logout" \
     -H "Authorization: Bearer {normal_user_token}"
   ```
3. 再次查询目标用户活跃会话数。

### 预期结果
- 第 2 步返回 `403 Forbidden`
- 不执行会话撤销

### 预期数据状态
```sql
SELECT COUNT(*) AS active_count
FROM sessions
WHERE user_id = '{target_user_id}' AND revoked_at IS NULL;
-- 预期: 与步骤 1 查询值一致
```

---

## 场景 2：refresh 后 token 会话可追踪且可被强退立即失效

### 初始状态
- 用户已通过 OIDC 登录，拥有 `{refresh_token}`
- 管理员 Token：`{admin_token}`
- 当前用户 id：`{user_id}`

### 目的
验证 refresh 产生的 access token 与会话撤销模型一致（可被 `sid/session` 撤销）。

### 测试操作流程
1. 用 refresh token 换取新 access token：
   ```bash
   curl -s -X POST "http://localhost:8080/api/v1/auth/token" \
     -H "Content-Type: application/json" \
     -d '{
       "grant_type":"refresh_token",
       "client_id":"auth9-portal",
       "refresh_token":"{refresh_token}"
     }'
   ```
2. 使用新 access token 调用受保护接口（应成功）：
   ```bash
   curl -i "http://localhost:8080/api/v1/auth/userinfo" \
     -H "Authorization: Bearer {refreshed_access_token}"
   ```
3. 管理员强制登出该用户：
   ```bash
   curl -i -X POST "http://localhost:8080/api/v1/admin/users/{user_id}/logout" \
     -H "Authorization: Bearer {admin_token}"
   ```
4. 使用第 1 步 token 再次调用 userinfo。

### 预期结果
- 第 2 步返回 200
- 第 4 步返回 401（或统一未授权错误）
- 表示 refresh 后 token 受会话撤销控制

### 预期数据状态
```sql
SELECT COUNT(*) AS active_count
FROM sessions
WHERE user_id = '{user_id}' AND revoked_at IS NULL;
-- 预期: 0
```

---

## 场景 3：OIDC callback 重定向 URL 不应包含 access_token/id_token

### 初始状态
- 已完成 `/api/v1/auth/authorize` 并获取有效 `code/state`
- 浏览器或抓包工具可查看 302 `Location`

### 目的
验证 callback 不通过 URL query 传递敏感 token。

### 测试操作流程
1. 调用 callback 并保留响应头：
   ```bash
   curl -i "http://localhost:8080/api/v1/auth/callback?code={code}&state={state}"
   ```
2. 检查 `Location` 跳转地址。
3. 复制跳转 URL 到日志/历史中，验证其中不含 token 字段。

### 预期结果
- `Location` 中不出现 `access_token=`
- `Location` 中不出现 `id_token=`
- 令牌应仅通过安全通道（HttpOnly Cookie 或后端会话）传递

---

## 场景 4：伪造 x-tenant-id 轮换请求不能绕过限流

### 初始状态
- 同一来源 IP，具备请求能力
- 已知某端点限流阈值（例如 10 次/60 秒）

### 目的
验证限流键不信任外部 `x-tenant-id`，防止通过 header 轮换绕过。

### 测试操作流程
1. 在 60 秒内快速发送 30 次请求，每次更换 `x-tenant-id`：
   ```bash
   for i in $(seq 1 30); do
     curl -s -o /dev/null -w "%{http_code}\n" \
       -H "x-tenant-id: spoof-$i" \
       "http://localhost:8080/api/v1/tenants"
   done
   ```
2. 统计是否出现 429。

### 预期结果
- 在达到阈值后应返回 429
- 不能因为切换 `x-tenant-id` 持续绕过限流

---

## 场景 5：动态路径限流键应折叠为模板路径，避免高基数

### 初始状态
- Redis 可访问
- 有可用用户 token：`{user_token}`

### 目的
验证限流 endpoint 键按模板路径聚合（例如 `/api/v1/users/{id}`），避免 key 爆炸。

### 测试操作流程
1. 调用 50 个不同用户路径（若无真实用户可用占位测试端点）：
   ```bash
   for i in $(seq 1 50); do
     curl -s -o /dev/null \
       -H "Authorization: Bearer {user_token}" \
       "http://localhost:8080/api/v1/users/00000000-0000-0000-0000-$(printf '%012d' $i)"
   done
   ```
2. 检查 Redis 中限流 key 数量。

### 预期结果
- 同一逻辑端点下 Redis key 数量应保持低基数（按模板聚合）
- 不应出现每个 `id` 生成一组独立限流键

### 预期数据状态
```bash
redis-cli --raw KEYS "auth9:ratelimit:*:GET:/api/v1/users/*" | wc -l
# 预期: 接近 1（或固定小常数），而不是 50
```

---

## 检查清单

| # | 场景 | 状态 | 测试日期 | 测试人员 | 备注 |
|---|------|------|----------|----------|------|
| 1 | 普通用户调用管理员强制登出端点应被拒绝 | ☐ | | | |
| 2 | refresh 后 token 会话可追踪且可被强退立即失效 | ☐ | | | |
| 3 | OIDC callback 重定向 URL 不应包含 access_token/id_token | ☐ | | | |
| 4 | 伪造 x-tenant-id 轮换请求不能绕过限流 | ☐ | | | |
| 5 | 动态路径限流键应折叠为模板路径，避免高基数 | ☐ | | | |
