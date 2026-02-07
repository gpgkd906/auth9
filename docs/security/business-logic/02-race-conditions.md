# 业务逻辑 - 竞态条件测试

**模块**: 业务逻辑安全
**测试范围**: 并发操作与 TOCTOU 漏洞
**场景数**: 4
**风险等级**: 🔴 极高
**OWASP ASVS**: V11.1

---

## 背景知识

Auth9 多个关键操作涉及"检查-执行"两步逻辑（TOCTOU: Time-of-Check to Time-of-Use），在并发场景下可能被利用：
- **密码重置 Token**: 检查有效性 → 重置密码
- **邀请接受**: 检查未使用 → 标记已接受
- **Token Exchange**: 验证权限 → 签发 Token
- **角色分配**: 检查权限 → 分配角色

Rust 的内存安全不能防止逻辑层的竞态条件，数据库层面的原子性是关键。

---

## 场景 1：密码重置 Token 并发使用

### 前置条件
- 有效的密码重置 Token
- 并发请求工具（如 `turbo-intruder`, `race-the-web`）

### 攻击目标
验证密码重置 Token 是否可在极短时间窗口内被多次使用

### 攻击步骤
1. 请求密码重置，获取 Token
2. 准备 50 个并发请求，每个请求使用相同 Token 但设置不同密码
3. 同时发送所有请求
4. 检查多少个请求成功
5. 验证最终密码是哪一个

### 预期安全行为
- 仅第一个请求成功，其余全部失败
- Token 在第一次使用后立即失效
- 不存在竞态窗口允许多次使用
- 使用数据库事务或乐观锁保证原子性

### 验证方法
```bash
# 请求密码重置
curl -X POST http://localhost:8080/api/v1/password/forgot \
  -H "Content-Type: application/json" \
  -d '{"email": "test@example.com"}'

# 从邮件/日志获取 reset token
RESET_TOKEN="obtained-reset-token"

# 并发重置 - 使用 GNU parallel
seq 1 50 | parallel -j50 \
  "curl -s -o /dev/null -w '%{http_code}' \
    -X POST http://localhost:8080/api/v1/password/reset \
    -H 'Content-Type: application/json' \
    -d '{\"token\": \"$RESET_TOKEN\", \"new_password\": \"NewPass{}!\"}'"
# 预期: 仅 1 个 200，其余 49 个 400/404

# 或使用 Python 脚本
python3 -c "
import asyncio, aiohttp

async def reset(session, i):
    async with session.post('http://localhost:8080/api/v1/password/reset',
        json={'token': '$RESET_TOKEN', 'new_password': f'NewPass{i}!'}) as resp:
        return resp.status

async def main():
    async with aiohttp.ClientSession() as session:
        tasks = [reset(session, i) for i in range(50)]
        results = await asyncio.gather(*tasks)
        success = results.count(200)
        print(f'Success: {success}, Failed: {len(results) - success}')
        assert success <= 1, f'RACE CONDITION: {success} successful resets!'

asyncio.run(main())
"
```

### 修复建议
- 使用数据库事务 + `SELECT ... FOR UPDATE` 锁定 Token 记录
- 或使用乐观锁（版本号/CAS 操作）
- Token 状态变更为原子操作
- 考虑 Redis 分布式锁作为额外保护

---

## 场景 2：邀请接受竞态条件

### 前置条件
- 有效的邀请 Token
- 同一用户多个并发请求能力

### 攻击目标
验证邀请是否可被并发接受导致重复加入或角色重复分配

### 攻击步骤
1. 创建邀请获取 Token
2. 准备 20 个并发请求同时接受该邀请
3. 检查用户是否被重复添加到租户
4. 检查角色是否被重复分配

### 预期安全行为
- 仅一个接受请求成功
- 数据库中不产生重复的 tenant_user 记录
- 角色分配不重复
- 邀请状态原子性更新

### 验证方法
```bash
# 创建邀请
INVITE=$(curl -s -X POST -H "Authorization: Bearer $ADMIN_TOKEN" \
  -H "Content-Type: application/json" \
  http://localhost:8080/api/v1/invitations \
  -d '{"email": "race@test.com", "role_ids": ["role-id"]}')
INVITE_TOKEN=$(echo $INVITE | jq -r '.token')

# 并发接受
seq 1 20 | parallel -j20 \
  "curl -s -o /dev/null -w '%{http_code}\n' \
    -X POST http://localhost:8080/api/v1/invitations/accept \
    -H 'Authorization: Bearer $USER_TOKEN' \
    -H 'Content-Type: application/json' \
    -d '{\"token\": \"$INVITE_TOKEN\"}'"
# 预期: 仅 1 个 200

# 检查数据库中是否有重复记录
# SELECT COUNT(*) FROM tenant_users WHERE user_id = 'race-user-id' AND tenant_id = 'tenant-id';
# 预期: 1
```

### 修复建议
- `tenant_users` 表使用 `UNIQUE INDEX (user_id, tenant_id)`
- 邀请接受使用数据库事务
- 接受前使用 `SELECT ... FOR UPDATE` 锁定邀请记录
- 依赖唯一约束作为最终防线

---

## 场景 3：并发 Token Exchange

### 前置条件
- 有效的 Identity Token
- gRPC 并发请求工具

### 攻击目标
验证高并发 Token Exchange 是否可能绕过权限检查或导致不一致

### 攻击步骤
1. 准备有效的 Identity Token
2. 在一个线程中并发发起 100 个 Token Exchange 请求
3. 同时在另一个线程中删除用户的租户成员资格
4. 检查删除成员资格后是否仍能成功 Exchange
5. 收集所有成功签发的 Token，验证权限一致性

### 预期安全行为
- 成员资格删除后，新的 Exchange 请求立即失败
- 不存在"成员检查通过但签发时已被删除"的窗口
- 所有签发的 Token 权限与签发时刻的数据库状态一致

### 验证方法
```bash
# 使用 ghz 进行 gRPC 负载测试
ghz --insecure \
  --call auth9.TokenService/ExchangeToken \
  --data '{"identity_token":"'$ID_TOKEN'","tenant_id":"'$TENANT_ID'","service_id":"'$SERVICE_ID'"}' \
  --metadata '{"x-api-key":"'$API_KEY'"}' \
  --connections=10 \
  --concurrency=100 \
  --total=1000 \
  localhost:50051

# 同时在另一个终端删除成员资格
# 然后检查删除时间点之后的 Exchange 是否全部失败

# 收集所有成功的 Token，解码检查权限
for token in $TOKENS; do
  echo $token | cut -d. -f2 | base64 -d 2>/dev/null | jq .roles
done
# 预期: 所有 Token 的权限一致
```

### 修复建议
- Token Exchange 中的权限查询和签发为原子操作
- 成员资格变更时立即清理相关缓存
- 签发 Token 前再次确认权限（双重检查）
- 短 Token 有效期减少窗口影响

---

## 场景 4：租户 Slug 竞态创建

### 前置条件
- 具有创建租户权限的 Token
- 并发请求工具

### 攻击目标
验证租户 slug 唯一性检查在并发创建时是否存在竞态条件

### 攻击步骤
1. 准备 20 个并发请求，全部使用相同的 slug 创建租户
2. 同时发送
3. 检查是否创建了多个同 slug 的租户
4. 如果数据库有唯一约束，检查错误是否被正确处理

### 预期安全行为
- 仅一个创建请求成功，其余失败
- 数据库唯一约束防止重复 slug
- 失败请求返回 409 Conflict
- 不产生部分创建的脏数据

### 验证方法
```bash
# 并发创建同 slug 租户
seq 1 20 | parallel -j20 \
  "curl -s -w '\n%{http_code}' \
    -X POST http://localhost:8080/api/v1/tenants \
    -H 'Authorization: Bearer $TOKEN' \
    -H 'Content-Type: application/json' \
    -d '{\"name\": \"Race Tenant {}\", \"slug\": \"race-test-slug\"}'"
# 预期: 1 个 201，19 个 409

# 验证只有一个租户
curl -H "Authorization: Bearer $TOKEN" \
  "http://localhost:8080/api/v1/tenants?search=race-test-slug"
# 预期: 仅返回 1 个结果
```

### 修复建议
- `tenants` 表 `slug` 列使用 `UNIQUE INDEX`
- 应用层捕获数据库唯一约束冲突，返回 409
- 不仅依赖应用层查重，数据库约束是最终防线
- 考虑使用 `INSERT ... ON DUPLICATE KEY` 模式

---

## 检查清单

| # | 场景 | 状态 | 测试日期 | 测试人员 | 发现问题 |
|---|------|------|----------|----------|----------|
| 1 | 密码重置 Token 并发使用 | ☐ | | | |
| 2 | 邀请接受竞态条件 | ☐ | | | |
| 3 | 并发 Token Exchange | ☐ | | | |
| 4 | 租户 Slug 竞态创建 | ☐ | | | |

---

## 测试工具

```bash
# GNU parallel - 简单并发
apt install parallel

# race-the-web - 专门的竞态条件测试
# https://github.com/TheHackerDev/race-the-web
race-the-web config.toml

# turbo-intruder (Burp Suite 扩展)
# 使用 gate 模式确保请求同时发送

# ghz - gRPC 负载/并发测试
# https://ghz.sh/
ghz --insecure --concurrency=100 --total=1000 localhost:50051

# Python aiohttp - 自定义并发脚本
pip install aiohttp
```

---

## 参考资料

- [OWASP Race Condition](https://owasp.org/www-community/vulnerabilities/Race_condition)
- [CWE-362: Concurrent Execution using Shared Resource with Improper Synchronization](https://cwe.mitre.org/data/definitions/362.html)
- [CWE-367: TOCTOU Race Condition](https://cwe.mitre.org/data/definitions/367.html)
- [PortSwigger Race Conditions](https://portswigger.net/web-security/race-conditions)
